#!/usr/bin/env ruby
# frozen_string_literal: true

# Pre-install integrity/compatibility check, not a runtime or provenance gate.
require "digest"
require "json"
require "rbconfig"
require "rubygems/package"

usage = "usage: verify_core_ruby_export.rb ARTIFACT.gem REPORT.json"
if ARGV == ["--help"]
  puts usage
  exit
end
abort usage unless ARGV.length == 2

begin
  artifact, report_path = ARGV
  report = JSON.parse(File.read(report_path))
  check = ->(condition, message) { raise ArgumentError, message unless condition }
  check.call(report.fetch("artifact") == File.basename(artifact), "artifact filename mismatch")
  check.call(report.fetch("sha256") == Digest::SHA256.file(artifact).hexdigest, "artifact digest mismatch")
  check.call(report.fetch("mode") == "package-only", "expected package-only report")
  {
    "installed_merge_tests" => "not_run", "generated_e2e_tests" => "not_run",
    "type_declarations" => "not_validated", "publication_gate" => false,
    "source_gem_gate" => false, "linkage_check" => "passed",
    "api_review_baseline" => "ruby source surface matched",
  }.each do |key, value|
    check.call(report.fetch(key) == value, "unexpected producer verification state: #{key}")
  end

  archive = Gem::Package.new(artifact)
  spec = archive.spec
  check.call(spec.name == "structuredmerge-core", "unexpected package name")
  {
    "package" => spec.name, "version" => spec.version.to_s,
    "platform" => spec.platform.to_s,
    "required_ruby_version" => spec.required_ruby_version.to_s,
  }.each do |key, value|
    check.call(report.fetch(key) == value, "archive metadata mismatch: #{key}")
  end
  abi = RbConfig::CONFIG.fetch("ruby_version")
  major, minor = RUBY_VERSION.split(".").map(&:to_i)
  check.call(report.fetch("ruby_abi") == abi, "Ruby ABI mismatch")
  check.call(report.fetch("ruby").split(".").first(2) == RUBY_VERSION.split(".").first(2), "Ruby minor mismatch")
  check.call(spec.platform.to_s == Gem::Platform.local.to_s, "platform mismatch")
  requirement = Gem::Requirement.new(">= #{major}.#{minor}.0", "< #{major}.#{minor + 1}.0")
  check.call(spec.required_ruby_version == requirement, "expected current Ruby minor restriction")
  check.call(spec.required_ruby_version.satisfied_by?(Gem::Version.new(RUBY_VERSION)), "unsupported Ruby version")
  check.call(spec.extensions.empty? && spec.executables.empty?, "unexpected build step or executable")
  expected_files = [
    "AGPL-3.0-only.md", "PolyForm-Small-Business-1.0.0.md", "README.md",
    "lib/structuredmerge_core.rb", "lib/structuredmerge_core/native.rb",
    "lib/structuredmerge_core/version.rb", "sig/types.rbs",
    "lib/structuredmerge_core_rb/#{abi}/structuredmerge_core_rb.#{RbConfig::CONFIG.fetch('DLEXT')}",
  ].sort
  check.call(archive.contents.sort == expected_files, "archive file allowlist mismatch")
  check.call(report.fetch("files") == expected_files, "report file allowlist mismatch")
  puts JSON.generate("artifact" => File.expand_path(artifact), "sha256" => report.fetch("sha256"),
    "package" => spec.name, "version" => spec.version.to_s,
    "pre_install_check" => "passed", "runtime_tests" => "not_run")
rescue StandardError => error
  abort "core export verification failed: #{error.message}"
end
