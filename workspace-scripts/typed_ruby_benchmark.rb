#!/usr/bin/env ruby
# frozen_string_literal: true

# Retained benchmark transport, not a merge implementation or native package.
begin
  require "json"
  require "structuredmerge_core"
  installed = Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path
  expected = File.realpath(ENV.fetch("STRUCTUREDMERGE_BENCHMARK_GEM_HOME"))
  raise "benchmark requires the isolated installed core gem" unless File.realpath(installed).start_with?(expected + File::SEPARATOR)
  require_relative "../packages/ruby/spec/native_merge_fixture"
rescue LoadError, StandardError => error
  warn "typed-core benchmark setup: #{error.message}"
  exit 2 # Exit 1 is reserved for a typed conflict, never a setup failure.
end

module TypedRubyBenchmark
  extend self

  def execute(operation, family, dialect, texts, request_id)
    raise ArgumentError, "unsupported typed Ruby benchmark combination" unless operation == "merge3" && family == "yaml" && dialect == "yaml"
    raise ArgumentError, "incorrect source count" unless texts.length == 3
    unless @registered
      StructuredmergeCore.register_parser_host(TypedPsychHost.new)
      @registered = true
    end
    request = NativeMergeFixture.common_request(operation, texts, request_id: request_id)
    result = StructuredmergeCore.execute_operation(request, StructuredmergeCore::ParseLimits.new(
      max_batch_items: 3, max_input_bytes: 16 * 1024 * 1024, max_nodes: 1_000_000, max_diagnostics: 1000))
    status = result.ok ? 0 : (result.conflicts.empty? ? 2 : 1)
    output = result.ok ? result.output : result.conflicted_output
    diagnostics = result.diagnostics.map do |record|
      item = record.canonical
      {"category" => item.category.to_s, "code" => item.code, "message" => item.message}
    end
    [status, output, {"ok" => result.ok, "provider_id" => result.provider.provider_id,
      "profile_id" => result.profile.profile_id, "conflict_count" => result.conflicts.length,
      "diagnostics" => diagnostics, "output" => output}]
  end

  def session
    $stdin.each_line do |line|
      next if line.strip.empty?
      started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
      request_id = operation = nil
      begin
        request = JSON.parse(line)
        request_id = request.fetch("request_id")
        raise ArgumentError, "unsupported adapter schema" unless request.fetch("schema_version") == "structuredmerge.benchmark.adapter-request/v1"
        operation = request.fetch("operation")
        texts = %w[base ours theirs].map do |role|
          text = request.fetch("sources").fetch(role).unpack1("m0").force_encoding(Encoding::UTF_8)
          raise ArgumentError, "source must be UTF-8" unless text.valid_encoding?
          text
        end
        selector = request.fetch("selector")
        status, output, result = execute(operation, selector.fetch("family"), selector.fetch("dialect"), texts, request_id)
        stderr = ""
      rescue ArgumentError, KeyError, TypeError => error
        status, output, result, stderr = 2, nil, {}, error.message
      end
      puts JSON.generate(schema_version: "structuredmerge.benchmark.adapter-response/v1",
        request_id: request_id, operation: operation, process_id: Process.pid, status: status,
        duration_ns: Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond) - started,
        output_base64: [output || ""].pack("m0"), result: result, stderr: stderr)
      $stdout.flush
    end
  end

  def main(args)
    if args == ["benchmark-provider-session"]
      session
      return 0
    end
    raise ArgumentError, "expected base ours theirs path marker-size" unless args.length == 5 && args[4] == "7"
    texts = args.first(3).map do |path|
      text = File.binread(path).force_encoding(Encoding::UTF_8)
      raise ArgumentError, "source must be UTF-8" unless text.valid_encoding?
      text
    end
    status, output, result = execute("merge3", ENV["AST_MERGE_FAMILY"], ENV["AST_MERGE_DIALECT"], texts, "benchmark.cold.merge3")
    File.binwrite(args[1], output) unless output.nil?
    result.fetch("diagnostics").each do |item|
      warn "typed-core: #{item.fetch('category')}: #{item.fetch('code')}: #{item.fetch('message')}"
    end
    status
  end
end

begin
  exit TypedRubyBenchmark.main(ARGV)
rescue StandardError => error
  warn "typed-core benchmark: #{error.message}"
  exit 2
end
