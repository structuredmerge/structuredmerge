#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "fileutils"
require "json"
require "open3"
require "rbconfig"
require "rubygems/package"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
DEFAULT_SAMPLE_COUNT = 5

LOAD_PROGRAM = <<~RUBY
  require "json"
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  require "structuredmerge_host_prototype"
  loaded = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  puts JSON.generate(
    "ruby" => RUBY_DESCRIPTION,
    "require_ns" => loaded - started,
    "version" => StructuredmergeHostPrototype::VERSION
  )
RUBY

PARSER_LOAD_PROGRAM = <<~'RUBY'
  require "json"
  source = %({"name":"structuredmerge","enabled":true}\n).b
  load_started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  require "structuredmerge_host_prototype"
  load_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  provider = "rust.tslp.json.cold-path"
  register_started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  StructuredmergeHostPrototype.register_tslp_parser_host(provider, "json")
  register_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  parse_started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  result = StructuredmergeHostPrototype.parse_with_parser(provider, source.bytes).pack("C*")
  parse_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  raise "native TSLP parser changed source bytes" unless result == source
  puts JSON.generate(
    "require_ns" => load_finished - load_started,
    "register_ns" => register_finished - register_started,
    "first_parse_ns" => parse_finished - parse_started,
    "provider" => provider,
    "registered_providers" => StructuredmergeHostPrototype.registered_parser_hosts,
    "source_bytes" => source.bytesize,
    "source_sha256" => Digest::SHA256.hexdigest(source),
    "result_sha256" => Digest::SHA256.hexdigest(result)
  )
RUBY

FIRST_MERGE_PROGRAM = <<~RUBY
  require "json"
  load_started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  require "structuredmerge_host_prototype"
  load_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  merge_started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  result = JSON.parse(
    StructuredmergeHostPrototype.merge_json_three_way(
      '{"left":1,"right":1}'.b,
      '{"left":2,"right":1}'.b,
      '{"left":1,"right":2}'.b,
      "json"
    )
  )
  merge_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  raise "first merge was not clean" unless result.fetch("outcome") == "clean"
  raise "first merge output changed" unless result.fetch("output") == '{"left":2,"right":2}'
  puts JSON.generate(
    "require_ns" => load_finished - load_started,
    "first_merge_ns" => merge_finished - merge_started,
    "outcome" => result.fetch("outcome"),
    "output_sha256" => Digest::SHA256.hexdigest(result.fetch("output"))
  )
RUBY

def monotonic_nanoseconds
  Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
end

def gem_path(input)
  path = File.expand_path(input)
  return path if File.file?(path)

  candidates = Dir.glob(File.join(path, "structuredmerge_host_prototype-*-*.gem"))
  abort "expected one platform gem in #{path}, found #{candidates.length}" unless candidates.one?

  candidates.first
end


def isolated_environment(gem_home)
  {
    "BUNDLE_BIN_PATH" => nil,
    "BUNDLE_GEMFILE" => nil,
    "BUNDLE_PATH" => nil,
    "GEM_HOME" => gem_home,
    "GEM_PATH" => gem_home,
    "RUBYLIB" => nil,
    "RUBYOPT" => nil
  }
end


def run_command!(environment, *command)
  started = monotonic_nanoseconds
  stdout, stderr, status = Open3.capture3(environment, *command, chdir: ROOT)
  duration = monotonic_nanoseconds - started
  abort "#{command.join(" ")} failed:\n#{stdout}#{stderr}" unless status.success?

  {
    "process_duration_ns" => duration,
    "stdout_bytes" => stdout.bytesize,
    "stderr_bytes" => stderr.bytesize,
    "stderr_sha256" => stderr.empty? ? nil : Digest::SHA256.hexdigest(stderr),
    "stdout" => stdout
  }
end


def run_ruby_sample!(environment, program, sample)
  command = run_command!(environment, RbConfig.ruby, "-rdigest", "-e", program)
  payload = JSON.parse(command.delete("stdout"))
  command.merge("sample" => sample, "child" => payload)
end


def summarize(samples, child_key)
  values = samples.map { |sample| sample.fetch("child").fetch(child_key) }.sort
  {
    "samples" => samples.length,
    "minimum_ns" => values.first,
    "median_ns" => values.fetch(values.length / 2),
    "maximum_ns" => values.last
  }
end


def cargo_package_version(name)
  output, status = Open3.capture2(
    "cargo", "metadata", "--locked", "--format-version", "1", chdir: ROOT
  )
  abort "cargo metadata failed" unless status.success?

  package = JSON.parse(output).fetch("packages").find { |candidate| candidate.fetch("name") == name }
  abort "cargo metadata omitted #{name}" unless package

  package.fetch("version")
end


# rubocop:disable-next Metrics/AbcSize, Metrics/MethodLength
def measure(gem_file, output_path, sample_count)
  spec = Gem::Package.new(gem_file).spec
  Dir.mktmpdir("structuredmerge-cold-path-") do |gem_home|
    environment = isolated_environment(gem_home)
    install = run_command!(
      environment,
      RbConfig.ruby,
      "-S",
      "gem",
      "install",
      gem_file,
      "--install-dir",
      gem_home,
      "--no-document"
    )
    install.delete("stdout")

    cold_start = Array.new(sample_count) do |index|
      run_ruby_sample!(environment, LOAD_PROGRAM, index + 1)
    end
    parser_load = Array.new(sample_count) do |index|
      run_ruby_sample!(environment, PARSER_LOAD_PROGRAM, index + 1)
    end
    first_merge = Array.new(sample_count) do |index|
      run_ruby_sample!(environment, FIRST_MERGE_PROGRAM, index + 1)
    end

    report = {
      "schema" => "structuredmerge.ruby-cold-path-measurement/v1",
      "clock" => "CLOCK_MONOTONIC nanoseconds",
      "artifact" => {
        "name" => File.basename(gem_file),
        "byte_length" => File.size(gem_file),
        "sha256" => Digest::SHA256.file(gem_file).hexdigest,
        "package" => spec.name,
        "version" => spec.version.to_s,
        "platform" => spec.platform.to_s
      },
      "environment" => {
        "ruby" => RUBY_DESCRIPTION,
        "host_cpu" => RbConfig::CONFIG.fetch("host_cpu"),
        "host_os" => RbConfig::CONFIG.fetch("host_os"),
        "tree_sitter_language_pack_crate" => cargo_package_version("tree-sitter-language-pack")
      },
      "methodology" => {
        "fresh_gem_home" => true,
        "fresh_process_per_runtime_sample" => true,
        "sample_count" => sample_count,
        "filesystem_and_network_caches" => "uncontrolled",
        "timing_thresholds" => "none; correctness and evidence shape are gated"
      },
      "measurements" => {
        "cold_install" => install,
        "cold_start" => {
          "summary" => summarize(cold_start, "require_ns"),
          "samples" => cold_start
        },
        "parser_load" => {
          "operation" => "register native TSLP JSON provider and parse exact source bytes",
          "summary" => summarize(parser_load, "first_parse_ns"),
          "samples" => parser_load
        },
        "first_merge" => {
          "operation" => "public generated API JSON three-way merge in a fresh process",
          "summary" => summarize(first_merge, "first_merge_ns"),
          "samples" => first_merge
        }
      }
    }
    FileUtils.mkdir_p(File.dirname(output_path))
    File.binwrite(output_path, JSON.pretty_generate(report) << "\n")
  end
  puts output_path
end

input = ARGV.fetch(0) do
  abort "usage: ruby measure_ruby_cold_paths.rb GEM_OR_DIRECTORY [OUTPUT_PATH]"
end
artifact = gem_path(input)
output = File.expand_path(ARGV[1] || artifact.sub(/\.gem\z/, ".cold-path.json"))
samples = Integer(ENV.fetch("STRUCTUREDMERGE_COLD_PATH_SAMPLES", DEFAULT_SAMPLE_COUNT.to_s), 10)
abort "STRUCTUREDMERGE_COLD_PATH_SAMPLES must be positive" unless samples.positive?

measure(artifact, output, samples)
