#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "open3"
require "rake"
require "rubygems/package"
require "stringio"
require "zlib"

ROOT = File.expand_path("..", __dir__)
PACKAGE_ROOT = File.join(ROOT, "packages", "ruby")
REQUIRED_FILES = %w[
  lib/structuredmerge_host_prototype.rb
  lib/structuredmerge_host_prototype/native.rb
  lib/structuredmerge_host_prototype/workflow_provider.rb
].freeze

def capture!(*command, chdir: ROOT)
  output, status = Open3.capture2e(*command, chdir: chdir)
  abort "#{command.join(" ")} failed:\n#{output}" unless status.success?

  output.strip
end

def package_files(path)
  data = nil
  File.open(path, "rb") do |gem_io|
    Gem::Package::TarReader.new(gem_io) do |archive|
      entry = archive.find { |candidate| candidate.full_name == "data.tar.gz" }
      abort "#{path} does not contain data.tar.gz" unless entry

      data = entry.read
    end
  end

  files = []
  Zlib::GzipReader.wrap(StringIO.new(data)) do |gzip|
    Gem::Package::TarReader.new(gzip) do |archive|
      archive.each do |entry|
        next unless entry.file?

        content = entry.read
        files << {
          "path" => entry.full_name,
          "byte_length" => content.bytesize,
          "sha256" => Digest::SHA256.hexdigest(content)
        }
      end
    end
  end
  files.sort_by { |entry| entry.fetch("path") }
end

Dir.chdir(PACKAGE_ROOT) { load File.join(PACKAGE_ROOT, "Rakefile") }
platforms = Object.const_get(:CROSS_PLATFORMS).sort
source_spec = Object.const_get(:GEMSPEC)
default_gem = File.join(PACKAGE_ROOT, "pkg", "#{source_spec.name}-#{source_spec.version}.gem")
input = File.expand_path(ARGV[0] || default_gem)
if File.directory?(input)
  candidates = Dir.glob(File.join(input, "#{source_spec.name}-#{source_spec.version}-*.gem")).sort
  abort "expected one platform gem in #{input}, found #{candidates.length}" unless candidates.one?

  gem_path = candidates.first
else
  gem_path = input
end
abort "gem does not exist: #{gem_path}" unless File.file?(gem_path)

spec = Gem::Package.new(gem_path).spec
files = package_files(gem_path)
file_names = files.map { |entry| entry.fetch("path") }
missing = REQUIRED_FILES - file_names
abort "gem is missing required files: #{missing.join(", ")}" unless missing.empty?
native_prefix = "lib/structuredmerge_host_prototype_core_rb/"
native_extensions = %w[.bundle .dll .dylib .so]
unless file_names.any? { |name| name.start_with?(native_prefix) && native_extensions.include?(File.extname(name)) }
  abort "gem does not contain a packaged native extension under #{native_prefix}"
end

git_status = capture!("git", "status", "--porcelain", "--untracked-files=no")
alef_version = capture!("alef", "--version").lines.first.chomp
manifest = {
  "schema" => "structuredmerge.distribution-artifact/v1",
  "artifact" => {
    "name" => File.basename(gem_path),
    "byte_length" => File.size(gem_path),
    "sha256" => Digest::SHA256.file(gem_path).hexdigest,
    "contents" => files
  },
  "package" => {
    "name" => spec.name,
    "version" => spec.version.to_s,
    "platform" => spec.platform.to_s,
    "required_ruby_version" => spec.required_ruby_version.to_s
  },
  "source" => {
    "repository" => "https://github.com/structuredmerge/structuredmerge-rust",
    "revision" => capture!("git", "rev-parse", "HEAD"),
    "dirty" => !git_status.empty?
  },
  "generation" => {
    "alef" => alef_version,
    "alef_source_revision" => File.read(File.join(ROOT, "workspace-scripts", "alef-source-revision")).strip,
    "config_sha256" => Digest::SHA256.file(File.join(ROOT, "alef.toml")).hexdigest,
    "record_sha256" => Digest::SHA256.file(File.join(ROOT, ".alef-generation.toml")).hexdigest
  },
  "build" => {
    "ruby" => RUBY_DESCRIPTION,
    "rustc" => capture!("rustc", "-Vv"),
    "cargo" => capture!("cargo", "--version")
  },
  "supported_native_platforms" => platforms
}

output = File.expand_path(ARGV[1] || gem_path.sub(/\.gem\z/, ".provenance.json"))
File.write(output, "#{JSON.pretty_generate(manifest)}\n")
puts output
