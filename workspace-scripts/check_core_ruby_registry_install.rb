#!/usr/bin/env ruby
# frozen_string_literal: true

# Post-publication RubyGems consumer gate. Unlike check_core_ruby_artifact.rb,
# this installs the exact registry gem and never uses the source checkout as a
# Ruby load path.
require "bundler"
require "digest"
require "fileutils"
require "json"
require "open3"
require "rbconfig"
require "rubygems/package"
require_relative "artifact_workspace"

USAGE = "usage: check_core_ruby_registry_install.rb --version VERSION [--gem LOCAL_GEM]"
if ARGV == ["--help"]
  puts USAGE
  exit
end
options = {}
args = ARGV.dup
until args.empty?
  flag = args.shift
  value = args.shift
  abort USAGE unless %w[--version --gem].include?(flag) && !value.to_s.empty? && !options.key?(flag)
  options[flag] = value
end
abort USAGE unless options.key?("--version") && Gem::Version.correct?(options.fetch("--version"))
version = Gem::Version.new(options.fetch("--version"))
local_gem = File.realpath(options.fetch("--gem")) if options.key?("--gem")
if local_gem
  archive = Gem::Package.new(local_gem)
  archive.verify
  abort "expected structuredmerge-core archive" unless archive.spec.name == "structuredmerge-core"
  abort "local archive version does not match --version" unless archive.spec.version == version
end

root = File.expand_path("..", __dir__)
FileUtils.mkdir_p(File.join(root, "tmp"))
ArtifactWorkspace.open(root: root, prefix: "core-ruby-registry-install-",
  disposable: %w[consumer gems]) do |stage|
  consumer = File.join(stage, "consumer")
  gem_home = File.join(stage, "gems")
  FileUtils.mkdir_p(consumer)
  File.write(File.join(consumer, "Gemfile"), <<~GEMFILE)
    source "https://rubygems.org"
    gem "structuredmerge-core", "= #{version}"
    gem "rspec", "~> 3.0"
    gem "rbs", ">= 3.0"
  GEMFILE
  FileUtils.cp(File.join(root, "packages/ruby/spec/structuredmerge_core_spec.rb"), File.join(consumer, "core_spec.rb"))
  FileUtils.cp(File.join(root, "packages/ruby/spec/native_merge_fixture.rb"), File.join(consumer, "native_merge_fixture.rb"))
  FileUtils.cp_r(File.join(root, "e2e/ruby/spec"), File.join(consumer, "generated"))
  test_app = File.join(consumer, "test_app")
  FileUtils.cp_r(File.join(root, "test_apps/ruby"), test_app)
  FileUtils.cp(File.join(consumer, "native_merge_fixture.rb"), File.join(test_app, "native_merge_fixture.rb"))
  psych_facts = File.join(consumer, "psych_facts.rb")
  FileUtils.cp(File.join(root, "crates/yaml-merge/tests/support/psych_facts.rb"), psych_facts)

  env = Bundler.unbundled_env.merge(
    "GEM_HOME" => gem_home, "GEM_PATH" => gem_home,
    "BUNDLE_GEMFILE" => File.join(consumer, "Gemfile"),
    "BUNDLE_APP_CONFIG" => File.join(consumer, ".bundle"),
    "BUNDLE_USER_HOME" => File.join(consumer, ".bundle-user"),
    "BUNDLE_PATH" => nil, "RUBYLIB" => nil, "RUBYOPT" => nil,
    "STRUCTUREDMERGE_EXPECT_GEM_HOME" => gem_home,
    "STRUCTUREDMERGE_EXPECT_VERSION" => version.to_s,
    "STRUCTUREDMERGE_PSYCH_FACTS" => psych_facts,
    "CARGO_BUILD_JOBS" => "1"
  )
  run = lambda do |*command, directory: consumer|
    output, status = Open3.capture2e(env, *command, chdir: directory, unsetenv_others: true)
    puts output
    raise "registry install gate failed: #{command.first(3).join(" ")}" unless status.success?
  end
  install = if local_gem
              [RbConfig.ruby, "-S", "gem", "install", local_gem, "--no-document", "--source", "https://rubygems.org"]
            else
              [RbConfig.ruby, "-S", "gem", "install", "structuredmerge-core", "--version", version.to_s,
               "--no-document", "--clear-sources", "--source", "https://rubygems.org"]
            end
  attempts = local_gem ? 1 : 5
  begin
    run.call(*install)
  rescue RuntimeError
    attempts -= 1
    raise if attempts.zero? || local_gem

    sleep 10
    retry
  end
  run.call(RbConfig.ruby, "-S", "bundle", "install", "--jobs", "4")
  run.call(RbConfig.ruby, "-e", <<~'VERIFY', directory: consumer)
    require "rubygems"
    gem "structuredmerge-core", "= #{ENV.fetch("STRUCTUREDMERGE_EXPECT_VERSION")}"
    spec = Gem.loaded_specs.fetch("structuredmerge-core")
    expected_version = Gem::Version.new(ENV.fetch("STRUCTUREDMERGE_EXPECT_VERSION"))
    abort "wrong installed version: #{spec.version}" unless spec.version == expected_version
    expected_home = File.realpath(ENV.fetch("STRUCTUREDMERGE_EXPECT_GEM_HOME"))
    abort "core gem escaped isolated GEM_HOME" unless File.realpath(spec.full_gem_path).start_with?(expected_home + File::SEPARATOR)
    require "structuredmerge_core"
    native_paths = $LOADED_FEATURES.select { |path| path.include?("structuredmerge_core_rb") }
    abort "native extension did not load from installed core gem" if native_paths.empty?
    abort "native extension escaped installed core gem" unless native_paths.all? { |path| File.realpath(path).start_with?(File.realpath(spec.full_gem_path) + File::SEPARATOR) }
    abort "typed core has no native merge API" unless StructuredmergeCore.respond_to?(:execute_operation)
    puts JSON.generate(version: spec.version.to_s, gem_path: spec.full_gem_path, native_paths: native_paths)
  VERIFY
  installed_core_dirs = Dir.glob(File.join(gem_home, "gems", "structuredmerge-core-#{version}*")).select do |path|
    File.directory?(path)
  end
  abort "expected one installed structuredmerge-core gem directory" unless installed_core_dirs.one?
  signature_dir = File.join(installed_core_dirs.fetch(0), "sig")
  abort "installed core signatures are missing" unless File.directory?(signature_dir)
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rbs", "-I",
    signature_dir, "validate")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "core_spec.rb")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "generated")
  run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "spec", directory: test_app)
  report = {
    "package" => "structuredmerge-core", "version" => version.to_s,
    "installation_source" => local_gem || "https://rubygems.org",
    "gem_home" => gem_home, "isolated_load_path" => true,
    "installed_core_tests" => "passed", "generated_e2e_tests" => "passed",
    "consumer_app_tests" => "passed", "type_declarations" => "validated",
    "publication_gate" => false, "registry_install" => local_gem ? "local-candidate" : "passed"
  }
  report["archive_sha256"] = Digest::SHA256.file(local_gem).hexdigest if local_gem
  File.write(File.join(stage, "report.json"), JSON.pretty_generate(report) + "\n")
  puts JSON.generate(report)
end
