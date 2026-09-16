# frozen_string_literal: true

require "structuredmerge_core"
require "digest"
require_relative "../../../crates/yaml-merge/tests/support/psych_facts"

RSpec.describe StructuredmergeCore do
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
        capabilities: ["native_extensions", "source_spans"], probe_id: "psych.available",
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

  it "passes native typed batches through Psych and preserves parser diagnostics" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    requests = ["é: one\r\n", "a: [\n"].each_with_index.map do |source, index|
      descriptor = described_class::SourceDescriptor.new(
        source_id: "s#{index}", role: "source", byte_length: source.bytesize,
        sha256: Digest::SHA256.hexdigest(source), encoding: "utf8", bom: false,
        line_endings: described_class::LineEndings.new(lf: index, crlf: 1 - index, bare_cr: 0),
        final_newline: true
      )
      described_class::ParseRequest.new(
        schema: "structuredmerge.parse-request/v1", request_id: "r#{index}",
        source: described_class::SourceInput.new(descriptor: descriptor, bytes: source.bytes),
        language: "yaml", dialect: nil,
        selection: described_class::ParserSelection.new(backend_id: "ruby.typed.psych", preference: [], required_capabilities: []),
        options: described_class::ParseOptions.new(comments: false, tokens: false, diagnostics: false, native_extensions: true),
        metadata: {}, extra: {}
      )
    end
    limits = described_class::ParseLimits.new(max_batch_items: 2, max_input_bytes: 1000, max_nodes: 100, max_diagnostics: 10)
    results = described_class.parse_sources(requests, limits)
    expect(host.calls).to eq(1)
    expect(host.received_batch).to be_a(described_class::ParseBatchRequest)
    expect(results.first.parsed.ok).to be(true)
    expect(results.first.parsed.source.sha256).to eq(requests.first.source.descriptor.sha256)
    expect(results.last.parsed.ok).to be(false)
    expect(results.last.parsed.diagnostics.first.code).to eq("psych.syntax")
    expect(results.first.selection.selected_backend).to eq("ruby.typed.psych")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end
end
