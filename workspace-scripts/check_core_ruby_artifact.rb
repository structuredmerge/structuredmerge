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

root = File.expand_path("..", __dir__)
package_root = File.join(root, "packages/ruby")
FileUtils.mkdir_p(File.join(root, "tmp"))
stage = Dir.mktmpdir("core-ruby-artifact-", File.join(root, "tmp"))
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
}
copies.each do |destination, source|
  target = File.join(gem_root, destination)
  FileUtils.mkdir_p(File.dirname(target))
  FileUtils.cp(source, target)
  File.chmod(destination.end_with?(extension) ? 0o755 : 0o644, target)
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

consumer = File.join(stage, "consumer")
gem_home = File.join(stage, "gems")
FileUtils.mkdir_p(consumer)
File.write(File.join(consumer, "Gemfile"), <<~GEMFILE)
  source "https://rubygems.org"
  gem "structuredmerge-core", "= #{spec.version}"
  gem "rspec", "~> 3.0"
GEMFILE
FileUtils.cp(File.join(package_root, "spec/structuredmerge_core_spec.rb"), File.join(consumer, "core_spec.rb"))
FileUtils.cp(File.join(root, "crates/yaml-merge/tests/support/psych_facts.rb"), File.join(consumer, "psych_facts.rb"))
env = Bundler.unbundled_env.merge(
  "GEM_HOME" => gem_home, "GEM_PATH" => gem_home,
  "BUNDLE_GEMFILE" => File.join(consumer, "Gemfile"),
  "BUNDLE_APP_CONFIG" => File.join(consumer, ".bundle"),
  "BUNDLE_USER_HOME" => File.join(consumer, ".bundle-user"),
  "BUNDLE_PATH" => nil, "RUBYLIB" => nil, "RUBYOPT" => nil,
  "STRUCTUREDMERGE_PSYCH_FACTS" => File.join(consumer, "psych_facts.rb"),
  "STRUCTUREDMERGE_EXPECT_GEM_HOME" => gem_home,
)
run = lambda do |*command|
  output, status = Open3.capture2e(env, *command, chdir: consumer, unsetenv_others: true)
  puts output
  raise "artifact gate failed: #{command.first(3).join(' ')}" unless status.success?
end
run.call(RbConfig.ruby, File.join(root, "workspace-scripts/check_ruby_linkage.rb"), artifact)
run.call(RbConfig.ruby, "-S", "gem", "install", artifact, "--no-document",
  "--clear-sources", "--source", "https://rubygems.org")
run.call(RbConfig.ruby, "-S", "bundle", "install", "--jobs", "4")
run.call(RbConfig.ruby, "-S", "bundle", "exec", "rspec", "core_spec.rb")
report = {
  "artifact" => artifact, "sha256" => Digest::SHA256.file(artifact).hexdigest,
  "package" => spec.name, "version" => spec.version.to_s,
  "platform" => spec.platform.to_s, "ruby" => RUBY_VERSION, "ruby_abi" => abi,
  "files" => archive.contents.sort, "installed_merge_tests" => "passed",
  "linkage_check" => "passed",
  "publication_gate" => false, "source_gem_gate" => false,
}
File.write(File.join(stage, "report.json"), JSON.pretty_generate(report) + "\n")
puts JSON.generate(report)
