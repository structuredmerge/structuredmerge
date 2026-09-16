# frozen_string_literal: true

# Test-only native provider and request setup. Merge semantics remain in Rust.
require "structuredmerge_core"
require "digest"
require ENV.fetch("STRUCTUREDMERGE_PSYCH_FACTS") {
  File.expand_path("../../../crates/yaml-merge/tests/support/psych_facts.rb", __dir__)
}

class TypedPsychHost
  attr_reader :calls, :received_batch

  def initialize
    @calls = 0
  end

  def descriptor
    StructuredmergeCore::ParserProviderDescriptor.new(
      id: "ruby.typed.psych", family: "native", runtime: RUBY_ENGINE,
      package: "psych", package_version: Psych::VERSION,
      parser: "psych", parser_version: Psych::VERSION,
      grammar: nil, grammar_version: nil, languages: ["yaml"], dialects: [],
      contracts: ["structuredmerge.parse-result/v1"],
      capabilities: ["diagnostics", "native_extensions", "source_spans"], probe_id: "psych.available",
      priority: 0, metadata: {}, extensions: []
    )
  end

  def probe_batch(request)
    raise "untyped probe callback" unless request.is_a?(StructuredmergeCore::ProbeBatchRequest)
    StructuredmergeCore::ProbeBatchResult.new(items: request.items.map do |_item|
      StructuredmergeCore::ParserProbeResult.new(available: true, loadable: true)
    end)
  end

  def parse_batch(request)
    @received_batch = request
    @calls += 1
    raise "untyped parse callback" unless request.is_a?(StructuredmergeCore::ParseBatchRequest)
    StructuredmergeCore::ParseBatchResult.new(items: request.items.map do |item|
      source = item.source
      descriptor = source.descriptor
      endings = descriptor.line_endings
      # Adapt native typed fields to the existing test-only AST projector;
      # no JSON string or merge operation crosses the generated callback.
      facts = project({
        "request_id" => item.request_id,
        "source" => {"bytes" => source.bytes, "descriptor" => {
          "source_id" => descriptor.source_id, "role" => descriptor.role.to_s,
          "byte_length" => descriptor.byte_length, "sha256" => descriptor.sha256,
          "encoding" => descriptor.encoding.to_s, "bom" => descriptor.bom,
          "line_endings" => {"lf" => endings.lf, "crlf" => endings.crlf, "bare_cr" => endings.bare_cr},
          "final_newline" => descriptor.final_newline
        }}
      })
      StructuredmergeCore::ParseOutput.new(**facts)
    end)
  end
end

module NativeMergeFixture
  extend self

  def merge_requests(sources, shared_source_id: nil, roles: %w[base ours theirs])
    roles.zip(sources).map do |role, source|
      descriptor = StructuredmergeCore::SourceDescriptor.new(
        source_id: shared_source_id || (role == "base" ? "merge-output" : role), role: role, byte_length: source.bytesize,
        sha256: Digest::SHA256.hexdigest(source), encoding: "utf8", bom: source.start_with?("\uFEFF"),
        line_endings: StructuredmergeCore::LineEndings.new(
          lf: source.count("\n") - source.scan("\r\n").length,
          crlf: source.scan("\r\n").length, bare_cr: 0
        ), final_newline: source.end_with?("\n")
      )
      StructuredmergeCore::ParseRequest.new(
        schema: "structuredmerge.parse-request/v1", request_id: role,
        source: StructuredmergeCore::SourceInput.new(descriptor: descriptor, bytes: source.bytes),
        language: "yaml", dialect: nil,
        selection: StructuredmergeCore::ParserSelection.new(backend_id: "ruby.typed.psych", preference: [], required_capabilities: []),
        options: StructuredmergeCore::ParseOptions.new(comments: false, tokens: false, diagnostics: true, native_extensions: true),
        metadata: {}, extra: {}
      )
    end.reverse # semantic roles, not argument positions
  end

  def merge_limits
    StructuredmergeCore::ParseLimits.new(max_batch_items: 3, max_input_bytes: 10000, max_nodes: 1000, max_diagnostics: 20)
  end

  def native_merge_profiles
    StructuredmergeCore.native_merge_profiles
  end

  def run_yaml_native_merge(base, ours, theirs)
    host = TypedPsychHost.new
    StructuredmergeCore.register_parser_host(host)
    begin
      result = StructuredmergeCore.merge_yaml_mapping(merge_requests([base, ours, theirs]), merge_limits)
      raise "fixture did not call the native parser" unless host.calls.positive?
      result
    ensure
      StructuredmergeCore.unregister_parser_host("ruby.typed.psych")
    end
  end
end
