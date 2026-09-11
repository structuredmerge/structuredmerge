#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "optparse"
require "rubygems/package"

ROOT = File.expand_path("..", __dir__)
PACKAGE_NAME = "structuredmerge_host_prototype"
DEFAULT_PLATFORMS = %w[
  x86_64-linux
  aarch64-linux
  arm64-darwin
  x86_64-darwin
  x64-mingw-ucrt
].freeze

options = { platforms: DEFAULT_PLATFORMS }
OptionParser.new do |parser|
  parser.banner = "usage: validate_ruby_release_artifacts.rb DIST [--platforms PLATFORM,...]"
  parser.on("--platforms LIST", "Expected platform gem suffixes for this validation run") do |value|
    options[:platforms] = value.split(",").map(&:strip).reject(&:empty?).uniq.sort
  end
end.parse!(ARGV)

input = File.expand_path(ARGV.fetch(0) { abort "missing artifact directory" }, ROOT)
abort "artifact directory does not exist: #{input}" unless File.directory?(input)

platforms = options.fetch(:platforms).sort
gem_files = Dir.glob(File.join(input, "#{PACKAGE_NAME}-*.gem")).sort
expected_count = platforms.length + 1
abort "expected #{expected_count} gems, found #{gem_files.length}" unless gem_files.length == expected_count

specs = gem_files.to_h do |path|
  [path, Gem::Package.new(path).spec]
end
versions = specs.values.map { |spec| spec.version.to_s }.uniq
abort "artifact versions differ: #{versions.inspect}" unless versions.one?

source_files = specs.select { |_path, spec| spec.platform.to_s == "ruby" }
abort "expected exactly one source gem" unless source_files.length == 1

platform_specs = specs.reject { |_path, spec| spec.platform.to_s == "ruby" }
actual_platforms = platform_specs.values.map { |spec| spec.platform.to_s }.sort
abort "platforms differ: expected #{platforms.inspect}, got #{actual_platforms.inspect}" unless actual_platforms == platforms

platform_specs.each do |gem_path, spec|
  basename = File.basename(gem_path, ".gem")
  provenance_path = File.join(input, "#{basename}.provenance.json")
  cold_path = File.join(input, "#{basename}.cold-path.json")
  abort "missing provenance manifest for #{basename}" unless File.file?(provenance_path)
  abort "missing cold-path report for #{basename}" unless File.file?(cold_path)

  provenance = JSON.parse(File.read(provenance_path))
  abort "invalid provenance schema for #{basename}" unless provenance["schema"] == "structuredmerge.distribution-artifact/v1"
  artifact = provenance.fetch("artifact")
  abort "provenance artifact name mismatch for #{basename}" unless artifact.fetch("name") == File.basename(gem_path)
  abort "provenance byte length mismatch for #{basename}" unless artifact.fetch("byte_length") == File.size(gem_path)
  digest = Digest::SHA256.file(gem_path).hexdigest
  abort "provenance digest mismatch for #{basename}" unless artifact.fetch("sha256") == digest

  package = provenance.fetch("package")
  abort "provenance package mismatch for #{basename}" unless package.fetch("name") == spec.name
  abort "provenance version mismatch for #{basename}" unless package.fetch("version") == spec.version.to_s
  abort "provenance platform mismatch for #{basename}" unless package.fetch("platform") == spec.platform.to_s
  abort "provenance native-platform list is incomplete for #{basename}" unless
    (platforms - Array(provenance.fetch("supported_native_platforms"))).empty?

  cold = JSON.parse(File.read(cold_path))
  abort "invalid cold-path schema for #{basename}" unless cold["schema"] == "structuredmerge.ruby-cold-path-measurement/v1"
  cold_artifact = cold.fetch("artifact")
  abort "cold-path artifact name mismatch for #{basename}" unless cold_artifact.fetch("name") == File.basename(gem_path)
  abort "cold-path byte length mismatch for #{basename}" unless cold_artifact.fetch("byte_length") == File.size(gem_path)
  abort "cold-path digest mismatch for #{basename}" unless cold_artifact.fetch("sha256") == digest
  abort "cold-path version mismatch for #{basename}" unless cold_artifact.fetch("version") == spec.version.to_s
  abort "cold-path platform mismatch for #{basename}" unless cold_artifact.fetch("platform") == spec.platform.to_s
  %w[cold_start parser_load first_merge].each do |measurement|
    abort "cold-path report missing #{measurement} for #{basename}" unless cold.fetch("measurements").key?(measurement)
  end
end

puts JSON.generate(
  "package" => PACKAGE_NAME,
  "version" => versions.fetch(0),
  "source_gems" => source_files.length,
  "platform_gems" => actual_platforms,
  "validated" => true
)
