#!/usr/bin/env ruby
# frozen_string_literal: true

# Development installed-artifact gate. Builds only the current Ruby ABI/platform;
# it neither publishes nor claims the source-gem or release-platform matrix gates.
require "bundler"
require "digest"
require "fileutils"
require "json"
require "open3"
require "rbconfig"
require "rubygems/package"
require "tmpdir"
require_relative "artifact_workspace"

usage = "usage: check_core_ruby_artifact.rb [--package-only OUTPUT_DIRECTORY | --provider-gem PSYCH_GEM]"
if ARGV == ["--help"]
  puts usage
  exit
end
abort usage unless ARGV.empty? || (ARGV.length == 2 && %w[--package-only --provider-gem].include?(ARGV[0]) && !ARGV[1].empty?)
export_directory = File.expand_path(ARGV[1]) if ARGV[0] == "--package-only"
provider_gem = File.realpath(ARGV[1]) if ARGV[0] == "--provider-gem"
if provider_gem
  provider_archive = Gem::Package.new(provider_gem)
  provider_archive.verify
  abort "expected a psych-merge provider gem" unless provider_archive.spec.name == "psych-merge"
  abort "provider gem lacks the typed parser host" unless provider_archive.contents.include?("lib/psych/merge/core_parser_host.rb")
end

root = File.expand_path("..", __dir__)
package_root = File.join(root, "packages/ruby")
FileUtils.mkdir_p(File.join(root, "tmp"))
ArtifactWorkspace.open(root: root, prefix: "core-ruby-artifact-",
  disposable: %w[package consumer gems]) do |stage|
  puts "Artifact report directory: #{stage}"
  gem_root = File.join(stage, "package")
  FileUtils.mkdir_p(gem_root)

  # Derive metadata from the generated gemspec, but never copy its broad file glob.
  # The source gem remains a separate, currently unproven distribution gate.
  spec = Dir.chdir(package_root) do
    Gem::Specification.load("structuredmerge_core.gemspec") || abort("cannot load core gemspec")
  end
  abort "unexpected generated package" unless spec.name == "structuredmerge-core"
  extension = "structuredmerge_core_rb.#{RbConfig::CONFIG.fetch('DLEXT')}"
  native_path = File.join(package_root, "lib", extension)
  abort "build the current native extension with bundle exec rake compile first" unless File.file?(native_path)
  abi = RbConfig::CONFIG.fetch("ruby_version")
  major, minor = RUBY_VERSION.split(".").map(&:to_i)
  copies = {
    "lib/structuredmerge_core.rb" => File.join(package_root, "lib/structuredmerge_core.rb"),
    "lib/structuredmerge_core/native.rb" => File.join(package_root, "lib/structuredmerge_core/native.rb"),
    "lib/structuredmerge_core/version.rb" => File.join(package_root, "lib/structuredmerge_core/version.rb"),
    "lib/structuredmerge_core_rb/#{abi}/#{extension}" => native_path,
    "README.md" => File.join(root, "crates/structuredmerge-core/README.md"),
    "AGPL-3.0-only.md" => File.join(root, "AGPL-3.0-only.md"),
    "PolyForm-Small-Business-1.0.0.md" => File.join(root, "PolyForm-Small-Business-1.0.0.md"),
    "sig/types.rbs" => File.join(package_root, "sig/types.rbs"),
  }
  copies.each do |destination, source|
    target = File.join(gem_root, destination)
    FileUtils.mkdir_p(File.dirname(target))
    FileUtils.cp(source, target)
    File.chmod(destination.end_with?(extension) ? 0o755 : 0o644, target)
  end
  baseline = JSON.parse(File.read(File.join(root, "contracts/typed-api/ruby/manifest.json")))
  baseline.fetch("files").each do |name, digest|
    path = File.join(gem_root, name)
    raise "gem API differs from reviewed baseline: #{name}" unless File.file?(path) && Digest::SHA256.file(path).hexdigest == digest
  end
  spec.files = copies.keys.sort
  spec.extensions = []
  spec.executables = []
  spec.platform = Gem::Platform.local
  license_expression = spec.license
  spec.licenses = license_expression.split(" OR ")
  spec.metadata["license_expression"] = license_expression
  # A binary built for one Ruby ABI must not advertise compatibility with all Rubies.
  spec.required_ruby_version = Gem::Requirement.new(">= #{major}.#{minor}.0", "< #{major}.#{minor + 1}.0")
  artifact = Dir.chdir(gem_root) { File.join(gem_root, Gem::Package.build(spec)) }
  archive = Gem::Package.new(artifact)
  raise "unexpected archive contents" unless archive.contents.sort == copies.keys.sort
  raise "prototype files leaked into core gem" if archive.contents.any? { |name| name.include?("prototype") }
  raise "binary gem retained an extension build step" unless archive.spec.extensions.empty?
  raise "binding gem ships executables" unless archive.spec.executables.empty?

  if export_directory
    # CI can build a pre-publication artifact without running/installing the test
    # harness. Keep the same allowlist, ABI restriction and API checks as the full
    # gate, and never label package-only output as installed-runtime verification.
    output, status = Open3.capture2e(RbConfig.ruby,
      File.join(root, "workspace-scripts/check_ruby_linkage.rb"), artifact)
    puts output
    abort "core artifact linkage check failed" unless status.success?
    FileUtils.mkdir_p(export_directory)
    destination = File.join(export_directory, File.basename(artifact))
    report_path = File.join(export_directory, "core-ruby-artifact.json")
    abort "refusing to overwrite an existing artifact or report" if File.exist?(destination) || File.symlink?(destination) || File.exist?(report_path) || File.symlink?(report_path)
    report = {
      "artifact" => File.basename(artifact), "sha256" => Digest::SHA256.file(artifact).hexdigest,
      "package" => spec.name, "version" => spec.version.to_s,
      "platform" => spec.platform.to_s, "ruby" => RUBY_VERSION, "ruby_abi" => abi,
      "required_ruby_version" => spec.required_ruby_version.to_s,
      "files" => archive.contents.sort, "linkage_check" => "passed",
      "api_review_baseline" => "ruby source surface matched",
      "mode" => "package-only", "installed_merge_tests" => "not_run",
      "generated_e2e_tests" => "not_run", "type_declarations" => "not_validated",
      "generated_test_app" => "not_run", "registry_install" => "not_run",
      "publication_gate" => false, "source_gem_gate" => false,
    }
    File.open(destination, File::WRONLY | File::CREAT | File::EXCL, 0o644) do |file|
      File.open(artifact, "rb") { |source| IO.copy_stream(source, file) }
    end
    File.open(report_path, File::WRONLY | File::CREAT | File::EXCL, 0o644) do |file|
      file.write(JSON.pretty_generate(report) + "\n")
    end
    puts JSON.generate(report)
    exit
  end

  consumer = File.join(stage, "consumer")
  gem_home = File.join(stage, "gems")
  FileUtils.mkdir_p(consumer)
  File.write(File.join(consumer, "Gemfile"), <<~GEMFILE)
    source "https://rubygems.org"
    gem "structuredmerge-core", "= #{spec.version}"
    gem "rspec", "~> 3.0"
    gem "rbs", ">= 3.0"
  GEMFILE
  if provider_gem
    File.open(File.join(consumer, "Gemfile"), "a") do |file|
      file.puts "gem 'psych-merge', '= #{provider_archive.spec.version}'"
      file.puts "gem 'psych', '~> 5.5.0'"
    end
  end
  FileUtils.cp(File.join(package_root, "spec/structuredmerge_core_spec.rb"), File.join(consumer, "core_spec.rb"))
  FileUtils.cp(File.join(package_root, "spec/native_merge_fixture.rb"), File.join(consumer, "native_merge_fixture.rb"))
  unless provider_gem
    FileUtils.cp(File.join(root, "crates/yaml-merge/tests/support/psych_facts.rb"), File.join(consumer, "psych_facts.rb"))
  end
  FileUtils.cp_r(File.join(root, "e2e/ruby/spec"), File.join(consumer, "generated"))
  test_app = File.join(consumer, "test_app")
  FileUtils.cp_r(File.join(root, "test_apps/ruby"), test_app)
  # Generated native fixtures need their test-only provider beside the app.
  # Keep the binding installed in GEM_HOME; never add checkout load paths.
  FileUtils.cp(File.join(consumer, "native_merge_fixture.rb"), File.join(test_app, "native_merge_fixture.rb"))
  env = Bundler.unbundled_env.merge(
    "GEM_HOME" => gem_home, "GEM_PATH" => gem_home,
    "BUNDLE_GEMFILE" => File.join(consumer, "Gemfile"),
    "BUNDLE_APP_CONFIG" => File.join(consumer, ".bundle"),
    "BUNDLE_USER_HOME" => File.join(consumer, ".bundle-user"),
    "BUNDLE_PATH" => nil, "RUBYLIB" => nil, "RUBYOPT" => nil,
    "STRUCTUREDMERGE_PSYCH_FACTS" => File.join(consumer, "psych_facts.rb"),
    # Never inherit installed-provider mode accidentally in the default gate.
    "STRUCTUREDMERGE_PSYCH_INSTALLED" => provider_gem ? "1" : nil,
    "STRUCTUREDMERGE_EXPECT_GEM_HOME" => gem_home,
    "TREE_HAVER_LANGUAGE_PACK_CACHE_DIR" => ENV.fetch("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", File.join(root, "tmp/typed-tslp-cache")),
  )
  run = lambda do |*command, directory: consumer|
    output, status = Open3.capture2e(env, *command, chdir: directory, unsetenv_others: true)
    puts output
    raise "artifact gate failed: #{command.first(3).join(' ')}" unless status.success?
  end
  run.call(RbConfig.ruby, File.join(root, "workspace-scripts/check_ruby_linkage.rb"), artifact)
  run.call(RbConfig.ruby, "-S", "gem", "install", artifact, "--no-document",
    "--clear-sources", "--source", "https://rubygems.org")
  if provider_gem
    run.call(RbConfig.ruby, "-S", "gem", "install", provider_gem, "--no-document",
      "--clear-sources", "--source", "https://rubygems.org")
  end
  run.call(RbConfig.ruby, "-S", "bundle", "install", "--jobs", "4")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rbs", "-I",
    File.join(gem_home, "gems", spec.full_name, "sig"), "validate")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "core_spec.rb")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "generated")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "spec", directory: test_app)
  report = {
    "artifact" => artifact, "sha256" => Digest::SHA256.file(artifact).hexdigest,
    "package" => spec.name, "version" => spec.version.to_s,
    "platform" => spec.platform.to_s, "ruby" => RUBY_VERSION, "ruby_abi" => abi,
    "files" => archive.contents.sort, "installed_merge_tests" => "passed",
    "linkage_check" => "passed",
    "type_declarations" => "validated",
    "api_review_baseline" => "ruby source surface matched",
    "generated_e2e_tests" => "passed",
    "generated_test_app" => provider_gem ? "passed with local gems and independent provider" : "passed with local gem and copied native-provider support",
    "native_provider_mode" => provider_gem ? "installed-provider" : "conformance-adapter",
    "registry_install" => "not_run",
    "publication_gate" => false, "source_gem_gate" => false,
  }
  if provider_gem
    report["provider_artifact"] = provider_gem
    report["provider_sha256"] = Digest::SHA256.file(provider_gem).hexdigest
    report["fixture_provider_id"] = "ruby.typed.psych"
    report["production_provider_id"] = "ruby.psych"
    FileUtils.cp(File.join(consumer, "Gemfile.lock"), File.join(stage, "Gemfile.lock"))
  end
  File.write(File.join(stage, "report.json"), JSON.pretty_generate(report) + "\n")
  puts JSON.generate(report)
end
