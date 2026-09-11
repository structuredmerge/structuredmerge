# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"
require "digest"
require "fileutils"
require "json"
require "open3"

module ParserMergeBenchmark
  WARMUP_SAMPLES = 5
  MEASURED_SAMPLES = 30
  COLD_SAMPLES = 5

  CASES = {
    "normalized_json_parse" => {
      method: "parse_normalized_with_tslp",
      args: ["json", '{"answer":42,"label":"café"}', "json"]
    },
    "json_three_way_merge" => {
      method: "merge_json_three_way",
      args: [
        '{"left":1,"right":1}',
        '{"left":2,"right":1}',
        '{"left":1,"right":2}',
        "json"
      ]
    },
    "go_three_way_merge" => {
      method: "merge_go_three_way",
      args: [
        "package main\n\nfunc left() int { return 1 }\nfunc right() int { return 1 }\n",
        "package main\n\nfunc left() int { return 2 }\nfunc right() int { return 1 }\n",
        "package main\n\nfunc left() int { return 1 }\nfunc right() int { return 2 }\n",
        "go"
      ]
    },
    "rust_three_way_merge" => {
      method: "merge_rust_three_way",
      args: [
        "fn left() -> i32 { 1 }\nfn right() -> i32 { 1 }\n",
        "fn left() -> i32 { 2 }\nfn right() -> i32 { 1 }\n",
        "fn left() -> i32 { 1 }\nfn right() -> i32 { 2 }\n",
        "rust"
      ]
    },
    "typescript_three_way_merge" => {
      method: "merge_typescript_three_way",
      args: [
        "function left(): number { return 1; }\nfunction right(): number { return 1; }\n",
        "function left(): number { return 2; }\nfunction right(): number { return 1; }\n",
        "function left(): number { return 1; }\nfunction right(): number { return 2; }\n",
        "typescript"
      ]
    }
  }.freeze

  module_function

  def monotonic_nanoseconds
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  end

  def percentile(samples, fraction)
    sorted = samples.sort
    sorted.fetch([(fraction * sorted.length).ceil - 1, 0].max)
  end

  def invoke(test_case)
    StructuredmergeHostPrototype.public_send(test_case.fetch(:method), *test_case.fetch(:args))
  end

  def verify(result, test_case)
    parsed = JSON.parse(result)
    raise "#{test_case.fetch(:method)} returned an unsuccessful result" if parsed["ok"] == false

    Digest::SHA256.hexdigest(result)
  end

  def warm_samples(test_case)
    WARMUP_SAMPLES.times { verify(invoke(test_case), test_case) }
    samples = Array.new(MEASURED_SAMPLES) do |index|
      started = monotonic_nanoseconds
      result = invoke(test_case)
      duration = monotonic_nanoseconds - started
      {
        "sample" => index + 1,
        "duration_ns" => duration,
        "sha256" => verify(result, test_case)
      }
    end
    durations = samples.map { |sample| sample.fetch("duration_ns") }
    {
      "p50_ns" => percentile(durations, 0.50),
      "p95_ns" => percentile(durations, 0.95),
      "p99_ns" => percentile(durations, 0.99),
      "output_sha256" => samples.map { |sample| sample.fetch("sha256") }.uniq.fetch(0),
      "samples" => samples
    }
  end

  def cold_samples(test_case, script_path)
    code = <<~RUBY
      require #{File.expand_path("../lib/structuredmerge_host_prototype", __dir__).inspect}
      args = #{test_case.fetch(:args).inspect}
      result = StructuredmergeHostPrototype.public_send(#{test_case.fetch(:method).inspect}, *args)
      abort "unsuccessful result" if JSON.parse(result)["ok"] == false
      puts Digest::SHA256.hexdigest(result)
    RUBY
    Array.new(COLD_SAMPLES) do |index|
      started = monotonic_nanoseconds
      output, error, status = Open3.capture3(RbConfig.ruby, "-rjson", "-rdigest", "-e", code, chdir: script_path)
      duration = monotonic_nanoseconds - started
      raise error unless status.success?

      {
        "sample" => index + 1,
        "duration_ns" => duration,
        "sha256" => output.strip
      }
    end
  end

  def run(output_directory)
    script_path = File.expand_path("..", __dir__)
    warm = {}
    cold = {}
    CASES.each do |name, test_case|
      warm[name] = warm_samples(test_case)
      cold_samples_for_case = cold_samples(test_case, script_path)
      durations = cold_samples_for_case.map { |sample| sample.fetch("duration_ns") }
      cold[name] = {
        "p50_ns" => percentile(durations, 0.50),
        "p95_ns" => percentile(durations, 0.95),
        "output_sha256" => cold_samples_for_case.map { |sample| sample.fetch("sha256") }.uniq.fetch(0),
        "samples" => cold_samples_for_case
      }
      unless warm[name].fetch("output_sha256") == cold[name].fetch("output_sha256")
        raise "cold/warm output digest mismatch for #{name}"
      end
    end

    FileUtils.mkdir_p(output_directory)
    report = {
      "schema" => "structuredmerge.parser-merge-benchmark/v1",
      "ruby" => RUBY_DESCRIPTION,
      "warmup_samples" => WARMUP_SAMPLES,
      "measured_samples" => MEASURED_SAMPLES,
      "cold_samples" => COLD_SAMPLES,
      "cases" => CASES.transform_values { |test_case| test_case.slice(:method, :args) },
      "warm" => warm.transform_values { |result| result.reject { |key, _| key == "samples" } },
      "cold" => cold.transform_values { |result| result.reject { |key, _| key == "samples" } },
      "raw_samples" => {"warm" => warm, "cold" => cold}
    }
    path = File.join(output_directory, "report.json")
    File.binwrite(path, JSON.pretty_generate(report) << "\n")
    puts path
  end
end

output_directory = ARGV.fetch(0) do
  raise ArgumentError, "usage: ruby parser_merge_benchmark.rb OUTPUT_DIRECTORY"
end
ParserMergeBenchmark.run(File.expand_path(output_directory))
