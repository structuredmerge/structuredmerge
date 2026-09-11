# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"
require "digest"
require "fileutils"
require "json"
require "open3"
require "rbconfig"

# The driver keeps scenario execution and evidence serialization together.
# rubocop:disable-next Metrics/ModuleLength
module HostTransportBenchmark
  WARMUP_SAMPLES = 5
  MEASURED_SAMPLES = 30
  BATCH_SIZES = {"1" => 1, "8" => 8, "32" => 32, "negotiated_maximum" => 32}.freeze
  PAYLOAD_CLASSES = {
    "small_inline" => 64,
    "medium_inline" => 16 * 1024,
    "detached_above_threshold" => 262_145
  }.freeze
  IMPLEMENTATIONS = %w[
    in_process_rust_identity
    generated_typed_values
    canonical_json_detached_bytes
  ].freeze

  class IdentityHost
    attr_reader :callback_counts, :callback_bytes

    def initialize(id)
      @id = id
      @callback_counts = Hash.new(0)
      @callback_bytes = Hash.new(0)
    end

    def descriptor
      JSON.generate("id" => @id, "capabilities" => %w[typed detached])
    end

    def version
      "benchmark-v1"
    end

    def execute_batch(request)
      request
    end

    def execute_cancellable_batch(_task_id, request)
      execute_batch(request)
    end

    def execute_async_batch(request)
      execute_batch(request)
    end

    def execute_typed_batch(_request, source)
      @callback_counts["generated_typed_values"] += 1
      @callback_bytes["generated_typed_values"] += source.bytesize
      source
    end

    def execute_detached_batch(request)
      envelope = JSON.parse(request)
      blob = envelope.fetch("blob")
      source = File.binread(blob.fetch("path"))
      raise "detached benchmark blob length mismatch" unless source.bytesize == blob.fetch("byte_length")
      raise "detached benchmark blob digest mismatch" unless
        Digest::SHA256.hexdigest(source) == blob.fetch("sha256")

      @callback_counts["canonical_json_detached_bytes"] += 1
      @callback_bytes["canonical_json_detached_bytes"] += request.bytesize + source.bytesize
      request
    end

    def shutdown; end
  end

  module_function

  def monotonic_nanoseconds
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  end

  def command_output(*command, chdir:)
    output, status = Open3.capture2(*command, chdir: chdir)
    status.success? ? output.strip : "unavailable"
  end

  def git_evidence(path)
    status = command_output("git", "status", "--short", chdir: path)
    {
      "revision" => command_output("git", "rev-parse", "HEAD", chdir: path),
      "dirty" => !status.empty?,
      "status_sha256" => status.empty? ? nil : Digest::SHA256.hexdigest(status)
    }
  end

  def percentile(samples, percentile)
    sorted = samples.sort
    sorted.fetch([(percentile * sorted.length).ceil - 1, 0].max)
  end

  def payload_arguments(total_bytes, batch_size)
    quotient, remainder = total_bytes.divmod(batch_size)
    payloads = Array.new(batch_size) do |index|
      length = quotient + (index < remainder ? 1 : 0)
      ((index * 37) % 256).chr.b * length
    end
    source = payloads.join.b
    {
      "source_ids" => Array.new(batch_size) { |index| "source:#{index + 1}" },
      "source_lengths" => payloads.map(&:bytesize),
      "source_digests" => payloads.map { |payload| Digest::SHA256.hexdigest(payload) },
      "source" => source,
      "source_sha256" => Digest::SHA256.hexdigest(source)
    }
  end

  def invoke(implementation, provider_name, arguments)
    call_arguments = [
      arguments.fetch("source_ids"),
      arguments.fetch("source_lengths"),
      arguments.fetch("source_digests"),
      arguments.fetch("source").bytes
    ]
    case implementation
    when "in_process_rust_identity"
      StructuredmergeHostPrototype.execute_in_process_identity(*call_arguments)
    when "generated_typed_values"
      StructuredmergeHostPrototype.execute_typed_identity(provider_name, *call_arguments)
    when "canonical_json_detached_bytes"
      StructuredmergeHostPrototype.execute_detached_identity(provider_name, *call_arguments)
    else
      raise "unknown implementation: #{implementation}"
    end
  end

  def verify_result(result, arguments)
    bytes = result.pack("C*")
    raise "identity byte length changed" unless bytes.bytesize == arguments.fetch("source").bytesize
    raise "identity digest changed" unless Digest::SHA256.hexdigest(bytes) == arguments.fetch("source_sha256")
  end

  # rubocop:disable-next Metrics/AbcSize, Metrics/MethodLength, Metrics/ParameterLists
  def run_scenario(implementation, batch_label, batch_size, payload_class, total_bytes, provider, provider_name)
    arguments = payload_arguments(total_bytes, batch_size)
    WARMUP_SAMPLES.times { verify_result(invoke(implementation, provider_name, arguments), arguments) }
    GC.start
    callback_count_before = provider.callback_counts.fetch(implementation, 0)
    callback_bytes_before = provider.callback_bytes.fetch(implementation, 0)
    samples = Array.new(MEASURED_SAMPLES) do |sample_index|
      started = monotonic_nanoseconds
      result = invoke(implementation, provider_name, arguments)
      duration = monotonic_nanoseconds - started
      verify_result(result, arguments)
      {
        "sample" => sample_index + 1,
        "duration_ns" => duration
      }
    end
    durations = samples.map { |sample| sample.fetch("duration_ns") }
    {
      "identity" => {
        "implementation" => implementation,
        "batch_size" => batch_label,
        "effective_batch_size" => batch_size,
        "payload_class" => payload_class,
        "source_bytes" => arguments.fetch("source").bytesize,
        "source_sha256" => arguments.fetch("source_sha256")
      },
      "summary" => {
        "p50_ns" => percentile(durations, 0.50),
        "p95_ns" => percentile(durations, 0.95),
        "p99_ns" => percentile(durations, 0.99),
        "callback_count" => provider.callback_counts.fetch(implementation, 0) - callback_count_before,
        "observed_host_callback_bytes" =>
          provider.callback_bytes.fetch(implementation, 0) - callback_bytes_before,
        "digest_equality" => true
      },
      "samples" => samples
    }
  end

  def lifecycle_samples
    Array.new(MEASURED_SAMPLES) do |index|
      provider_name = "ruby.benchmark.lifecycle.#{index}"
      provider = IdentityHost.new(provider_name)
      register_started = monotonic_nanoseconds
      StructuredmergeHostPrototypeCore.register_workflow_host(provider, provider_name)
      register_duration = monotonic_nanoseconds - register_started
      unregister_started = monotonic_nanoseconds
      StructuredmergeHostPrototype.unregister_workflow_host(provider_name)
      unregister_duration = monotonic_nanoseconds - unregister_started
      {
        "sample" => index + 1,
        "register_ns" => register_duration,
        "unregister_ns" => unregister_duration
      }
    end
  end

  def peak_resident_kibibytes
    status = File.read("/proc/self/status")
    match = status.match(/^VmHWM:\s+(\d+)\s+kB$/)
    match && Integer(match[1])
  rescue Errno::ENOENT
    nil
  end

  # rubocop:disable-next Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength
  def run(output_directory)
    rust_root = File.expand_path("../../..", __dir__)
    project_root = File.expand_path("..", rust_root)
    revisions = {
      "rust" => git_evidence(rust_root),
      "alef" => git_evidence(File.join(project_root, "vendor/alef")),
      "fixtures" => git_evidence(File.join(project_root, "fixtures"))
    }
    provider_name = "ruby.benchmark.identity"
    provider = IdentityHost.new(provider_name)
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, provider_name)

    scenarios = IMPLEMENTATIONS.product(BATCH_SIZES.to_a, PAYLOAD_CLASSES.to_a).map do |implementation, (batch_label, batch_size), (payload_class, total_bytes)|
      run_scenario(
        implementation,
        batch_label,
        batch_size,
        payload_class,
        total_bytes,
        provider,
        provider_name
      )
    end
    lifecycle = lifecycle_samples
    StructuredmergeHostPrototype.unregister_workflow_host(provider_name)

    raw = {
      "schema" => "structuredmerge.host-transport-raw-samples/v1",
      "warmup_samples_per_scenario" => WARMUP_SAMPLES,
      "measured_samples_per_scenario" => MEASURED_SAMPLES,
      "scenarios" => scenarios.map { |scenario| scenario.slice("identity", "samples") },
      "lifecycle_samples" => lifecycle
    }
    FileUtils.mkdir_p(output_directory)
    raw_path = File.join(output_directory, "raw-samples.json")
    File.binwrite(raw_path, JSON.pretty_generate(raw) << "\n")

    generated_source = File.binread(
      File.join(rust_root, "packages/ruby/ext/structuredmerge_host_prototype_core_rb/src/lib.rs")
    )
    report = {
      "schema" => "structuredmerge.host-transport-benchmark/v1",
      "clock" => "CLOCK_MONOTONIC nanoseconds",
      "correctness" => "all measured results matched source length and SHA-256",
      "environment" => {
        "ruby" => RUBY_DESCRIPTION,
        "rustc" => command_output("rustc", "--version", chdir: rust_root),
        "host_cpu" => RbConfig::CONFIG.fetch("host_cpu"),
        "host_os" => RbConfig::CONFIG.fetch("host_os"),
        "peak_resident_kibibytes" => peak_resident_kibibytes
      },
      "revisions" => revisions,
      "generated_binding_sha256" => Digest::SHA256.hexdigest(generated_source),
      "measurement" => {
        "implementations" => IMPLEMENTATIONS,
        "batch_sizes" => BATCH_SIZES,
        "payload_classes" => PAYLOAD_CLASSES,
        "warmup_samples" => WARMUP_SAMPLES,
        "measured_samples" => MEASURED_SAMPLES,
        "scenarios" => scenarios.map { |scenario| scenario.slice("identity", "summary") },
        "lifecycle" => {
          "register_p50_ns" => percentile(lifecycle.map { |sample| sample.fetch("register_ns") }, 0.50),
          "register_p95_ns" => percentile(lifecycle.map { |sample| sample.fetch("register_ns") }, 0.95),
          "unregister_p50_ns" => percentile(lifecycle.map { |sample| sample.fetch("unregister_ns") }, 0.50),
          "unregister_p95_ns" => percentile(lifecycle.map { |sample| sample.fetch("unregister_ns") }, 0.95)
        },
        "not_measured" => %w[
          allocator_counts
          per_scenario_peak_resident_memory
          gvl_hold_time
          gvl_wait_time
          internal_rust_copy_counts
        ]
      },
      "raw_samples" => {
        "path" => File.basename(raw_path),
        "sha256" => Digest::SHA256.file(raw_path).hexdigest
      },
      "transport_decision" => "undecided_pending_review"
    }
    report_path = File.join(output_directory, "report.json")
    File.binwrite(report_path, JSON.pretty_generate(report) << "\n")
    puts report_path
  ensure
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
  end
end

output_directory = ARGV.fetch(0) do
  raise ArgumentError, "usage: ruby host_transport_benchmark.rb OUTPUT_DIRECTORY"
end
HostTransportBenchmark.run(File.expand_path(output_directory))
