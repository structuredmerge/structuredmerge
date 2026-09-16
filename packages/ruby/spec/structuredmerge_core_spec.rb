# frozen_string_literal: true

require "structuredmerge_core"
require "digest"
require ENV.fetch("STRUCTUREDMERGE_PSYCH_FACTS") {
  File.expand_path("../../../crates/yaml-merge/tests/support/psych_facts.rb", __dir__)
}

if (expected_home = ENV["STRUCTUREDMERGE_EXPECT_GEM_HOME"])
  installed = Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path
  raise "core must load from the isolated installed gem" unless installed.start_with?(File.expand_path(expected_home) + File::SEPARATOR)
  raise "prototype was activated" if Gem.loaded_specs.keys.any? { |name| name.include?("host_prototype") }
  loaded_native = $LOADED_FEATURES.select { |path| path.include?("structuredmerge_core_rb") }
  raise "native extension came from outside the installed gem" if loaded_native.empty? || loaded_native.any? { |path| !path.start_with?(installed + File::SEPARATOR) }
end

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

  def merge_requests(sources, shared_source_id: nil)
    %w[base ours theirs].zip(sources).map do |role, source|
      descriptor = described_class::SourceDescriptor.new(
        source_id: shared_source_id || (role == "base" ? "merge-output" : role), role: role, byte_length: source.bytesize,
        sha256: Digest::SHA256.hexdigest(source), encoding: "utf8", bom: source.start_with?("\uFEFF"),
        line_endings: described_class::LineEndings.new(
          lf: source.count("\n") - source.scan("\r\n").length,
          crlf: source.scan("\r\n").length, bare_cr: 0
        ), final_newline: source.end_with?("\n")
      )
      described_class::ParseRequest.new(
        schema: "structuredmerge.parse-request/v1", request_id: role,
        source: described_class::SourceInput.new(descriptor: descriptor, bytes: source.bytes),
        language: "yaml", dialect: nil,
        selection: described_class::ParserSelection.new(backend_id: "ruby.typed.psych", preference: [], required_capabilities: []),
        options: described_class::ParseOptions.new(comments: false, tokens: false, diagnostics: true, native_extensions: true),
        metadata: {}, extra: {}
      )
    end.reverse # semantic roles, not argument positions
  end

  def merge_limits
    described_class::ParseLimits.new(max_batch_items: 3, max_input_bytes: 10000, max_nodes: 1000, max_diagnostics: 20)
  end

  it "declares native profile scope separately from parser availability and default approval" do
    profiles = described_class.native_merge_profiles
    expect(profiles.map(&:family)).to eq(%w[python yaml])
    expect(profiles.map(&:id).uniq.length).to eq(2)
    profiles.each do |profile|
      expect(profile.operation).to eq("merge3")
      expect(profile.semantic_runtime).to eq("rust")
      expect(profile.merge_crate).to eq("ast-merge")
      expect(profile.parse_contract).to eq("structuredmerge.parse-result/v1")
      expect(profile.experimental).to be(true)
      expect(profile.approved_as_default).to be(false)
      expect(profile.syntax_scope).not_to be_empty
      expect(profile.limitations).not_to be_empty
      expect(described_class).to respond_to(profile.entry_point)
    end
  end

  it "preserves portable service failure codes through the installed binding" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    requests = merge_requests(["a: one\n"] * 3)
    expect { described_class.parse_sources([], merge_limits) }.to raise_error(RuntimeError, /request\.invalid:/)
    limited = described_class::ParseLimits.new(max_batch_items: 3, max_input_bytes: 0, max_nodes: 1000, max_diagnostics: 20)
    %i[parse_sources merge_yaml_mapping].each do |operation|
      expect { described_class.public_send(operation, requests, limited) }.to raise_error(RuntimeError, /resource\.limit:/)
    end
    expect(host.calls).to eq(0)
    duplicated = merge_requests(["a: one\n", "a: two\n", "a: three\n"], shared_source_id: "duplicate")
    %i[parse_sources merge_yaml_mapping].each do |operation|
      expect { described_class.public_send(operation, duplicated, merge_limits) }.to raise_error(RuntimeError, /source\.invalid:/)
    end
    expect(host.calls).to eq(0)
    host.define_singleton_method(:parse_batch) do |_request|
      @calls += 1
      raise "native test failure"
    end
    expect { described_class.parse_sources(requests, merge_limits) }.to raise_error(RuntimeError, /parser\.provider_fault:.*native test failure/)
    expect(host.calls).to eq(1)
    failed = described_class.merge_yaml_mapping(requests, merge_limits)
    expect(failed.outcome.to_s).to eq("error")
    expect(failed.input_failure.code).to eq("parser.provider_fault")
    expect(failed.input_failure.backend_id).to eq("ruby.typed.psych")
    expect(failed.profile_id).to eq("kernel.yaml.native_mapping.v1")
    expect(failed.input_failure.native_message).to include("native test failure")
    expect(failed.output).to be_nil
    expect(failed.output_parse).to be_nil
    expect(failed.input_parses).to be_empty
    expect(failed.sources.map(&:source_id)).to eq(requests.reverse.map { |request| request.source.descriptor.source_id })
    expect(failed.sources.map(&:sha256)).to eq(requests.reverse.map { |request| request.source.descriptor.sha256 })
    expect(host.calls).to eq(2)
    host.define_singleton_method(:parse_batch) do |_request|
      StructuredmergeCore::ParseBatchResult.new(items: [])
    end
    expect { described_class.parse_sources(requests, merge_limits) }.to raise_error(RuntimeError, /parser\.invalid_batch:/)
    expect(described_class.merge_yaml_mapping(requests, merge_limits).input_failure.code).to eq("parser.invalid_batch")
    described_class.unregister_parser_host("ruby.typed.psych")
    begin
      expect { described_class.parse_sources(requests, merge_limits) }.to raise_error(RuntimeError, /selection\.no_parser:/)
      failed = described_class.merge_yaml_mapping(requests, merge_limits)
      expect(failed.input_failure.code).to eq("selection.no_parser")
      expect(failed.input_failure.selection.requested.backend_id).to eq("ruby.typed.psych")
      expect(failed.input_failure.selection.selected_backend).to be_nil
      expect(failed.input_failure.selection.digest).not_to be_empty
      expect(failed.sources.length).to eq(3)
    ensure
      described_class.register_parser_host(host)
    end
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "merges independent changes in Rust through generated Psych callbacks" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    sources = [
      "\uFEFF# header\r\né: 'one'  # stable\r\nbeta: two",
      "\uFEFF# header\r\né: 'ours'  # stable\r\nbeta: two",
      "\uFEFF# header\r\né: 'one'  # stable\r\nbeta: theirs"
    ]
    result = described_class.merge_yaml_mapping(merge_requests(sources), merge_limits)
    expect(result.outcome.to_s).to eq("clean")
    expect(result.profile_id).to eq("kernel.yaml.native_mapping.v1")
    expect(result.output).to eq("\uFEFF# header\r\né: 'ours'  # stable\r\nbeta: theirs")
    expect(result.conflicts).to be_empty
    expect(result.rejected_parse).to be_nil
    expect(result.input_parses.map { |parsed| parsed.parsed.source.role.to_s }).to eq(%w[base ours theirs])
    expect(result.input_parses.all? { |parsed| parsed.parsed.ok }).to be(true)
    expect(result.input_parses.map { |parsed| parsed.selection.selected_backend }.uniq).to eq(["ruby.typed.psych"])
    expect(result.input_parses.map { |parsed| parsed.selection.digest }.uniq.length).to eq(1)
    expect(host.calls).to eq(2) # input batch and Rust-requested output verification
    expect(host.received_batch.items.first.source.descriptor.role.to_s).to eq("output")
    by_role = %w[base ours theirs].zip(sources).to_h
    descriptors = result.sources.to_h { |source| [source.source_id, source] }
    expect(descriptors).not_to have_key(result.output_source.source_id)
    expect(result.output_source.sha256).to eq(Digest::SHA256.hexdigest(result.output))
    expect(result.output_source.byte_length).to eq(result.output.bytesize)
    expect(result.output_parse.parsed.ok).to be(true)
    expect(result.output_parse.parsed.source.role.to_s).to eq("output")
    expect(result.output_parse.parsed.source.sha256).to eq(result.output_source.sha256)
    expect(result.output_parse.selection.selected_backend).to eq("ruby.typed.psych")
    cursor = 0
    expect(result.source_segments).not_to be_empty
    result.source_segments.each do |segment|
      expect(segment.output_range.start_byte).to eq(cursor)
      selected = by_role.fetch(segment.source_role.to_s).byteslice(segment.source_range.start_byte...segment.source_range.end_byte)
      rendered = result.output.byteslice(segment.output_range.start_byte...segment.output_range.end_byte)
      expect(rendered).to eq(selected)
      expect(segment.sha256).to eq(Digest::SHA256.hexdigest(rendered))
      expect(descriptors.fetch(segment.source_id).role.to_s).to eq(segment.source_role.to_s)
      cursor = segment.output_range.end_byte
    end
    expect(cursor).to eq(result.output.bytesize)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "retains output verification rejection without exposing unverified output" do
    host = TypedPsychHost.new
    original = host.method(:parse_batch)
    host.define_singleton_method(:parse_batch) do |request|
      item = request.items.first
      next original.call(request) unless item.source.descriptor.role.to_s == "output"
      # Controlled provider rejection, not a claim about Psych accepting these bytes.
      diagnostic = StructuredmergeCore::ParseDiagnostic.new(
        id: "verification.reject", severity: "error", category: "parse_error",
        message: "controlled output rejection", source_role: "output", blocking: true,
        metadata: {}, extra: {}, code: "test.output_rejected", span: nil, node_id: nil
      )
      StructuredmergeCore::ParseBatchResult.new(items: [StructuredmergeCore::ParseOutput.new(
        request_id: item.request_id, source: item.source.descriptor, ok: false,
        root_id: nil, nodes: [], comments: [], diagnostics: [diagnostic], extensions: [], metadata: {}, extra: {}
      )])
    end
    described_class.register_parser_host(host)
    result = described_class.merge_yaml_mapping(merge_requests(["a: 1\nb: 2\n", "a: 3\nb: 2\n", "a: 1\nb: 4\n"]), merge_limits)
    expect(result.outcome.to_s).to eq("error")
    expect(result.output).to be_nil
    expect(result.output_source).to be_nil
    expect(result.source_segments).to be_empty
    expect(result.input_parses.length).to eq(3)
    expect(result.output_parse.parsed.ok).to be(false)
    expect(result.output_parse.parsed.diagnostics.first.code).to eq("test.output_rejected")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "retains structured verification callback faults and invalid batches" do
    host = TypedPsychHost.new
    original = host.method(:parse_batch)
    described_class.register_parser_host(host)
    {raise: "parser.provider_fault", empty: "parser.invalid_batch"}.each do |mode, code|
      calls = []
      host.define_singleton_method(:parse_batch) do |request|
        next original.call(request) unless request.items.first.source.descriptor.role.to_s == "output"
        calls << mode
        raise "verification exploded" if mode == :raise
        StructuredmergeCore::ParseBatchResult.new(items: [])
      end
      result = described_class.merge_yaml_mapping(merge_requests(["a: 1\nb: 2\n", "a: 3\nb: 2\n", "a: 1\nb: 4\n"]), merge_limits)
      expect(result.outcome.to_s).to eq("error")
      expect(result.output).to be_nil
      expect(result.output_source).to be_nil
      expect(result.output_parse).to be_nil
      expect(result.source_segments).to be_empty
      expect(result.input_parses.length).to eq(3)
      expect(result.verification_failure.code).to eq(code)
      expect(result.verification_failure.backend_id).to eq("ruby.typed.psych")
      expect(calls).to eq([mode])
      if mode == :raise
        expect(result.verification_failure.native_code).not_to be_empty
        expect(result.verification_failure.native_message).to include("verification exploded")
      else
        expect(result.verification_failure.native_code).to be_nil
      end
    end
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "does not fabricate a verification parse for whole-source selection" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    result = described_class.merge_yaml_mapping(merge_requests(["a: one\n"] * 3), merge_limits)
    expect(result.output).to eq("a: one\n")
    expect(result.output_parse).to be_nil
    expect(result.input_parses.length).to eq(3)
    expect(host.calls).to eq(1)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "returns Rust-classified conflicts with all three revision alternatives" do
    described_class.register_parser_host(TypedPsychHost.new)
    result = described_class.merge_yaml_mapping(merge_requests([
      "a: one\n", "a: ours\n", "a: theirs\n"
    ]), merge_limits)
    expect(result.outcome.to_s).to eq("conflict")
    expect(result.output).to be_nil
    expect(result.output_source).to be_nil
    expect(result.source_segments).to be_empty
    expect(result.conflicts.length).to eq(1)
    expect(result.conflicts.first.path).to eq("/a")
    expect(result.conflicts.first.alternatives.map { |alternative| alternative.revision.to_s }).to eq(%w[base ours theirs])
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "keeps malformed revision diagnostics and never emits fabricated merge output" do
    described_class.register_parser_host(TypedPsychHost.new)
    result = described_class.merge_yaml_mapping(merge_requests([
      "a: one\n", "a: [\n", "a: theirs\n"
    ]), merge_limits)
    expect(result.outcome.to_s).to eq("error")
    expect(result.output).to be_nil
    expect(result.output_source).to be_nil
    expect(result.source_segments).to be_empty
    expect(result.rejected_parse.parsed.source.role.to_s).to eq("ours")
    expect(result.sources.map { |source| source.role.to_s }).to eq(%w[base ours theirs])
    expect(result.rejected_parse.parsed.diagnostics.first.code).to eq("psych.syntax")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "prioritizes native failures before analysis with order-independent source provenance" do
    described_class.register_parser_host(TypedPsychHost.new)
    sources = ["- unsupported sequence\n", "a: [\n", "b: [\n"]
    requests = merge_requests(sources)
    [requests, requests.reverse].each do |ordered|
      result = described_class.merge_yaml_mapping(ordered, merge_limits)
      expect(result.rejected_parse.parsed.source.role.to_s).to eq("ours")
      expect(result.output).to be_nil
      expect(result.output_source).to be_nil
      expect(result.source_segments).to be_empty
      expect(result.sources.map(&:sha256)).to eq(sources.map { |source| Digest::SHA256.hexdigest(source) })
      expect(result.input_parses.map { |parsed| parsed.parsed.source.sha256 }).to eq(result.sources.map(&:sha256))
      rejected = result.input_parses.reject { |parsed| parsed.parsed.ok }
      expect(rejected.map { |parsed| parsed.parsed.source.role.to_s }).to eq(%w[ours theirs])
      expect(rejected.map { |parsed| parsed.parsed.diagnostics.first.code }).to eq(["psych.syntax", "psych.syntax"])
      expect(rejected.map { |parsed| parsed.backend.id }.uniq).to eq(["ruby.typed.psych"])
    end
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "reports unsupported analysis with complete parsed input evidence" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    result = described_class.merge_yaml_mapping(merge_requests(["a: one\n", "- unsupported sequence\n", "a: two\n"]), merge_limits)
    expect(result.outcome.to_s).to eq("error")
    expect(result.output).to be_nil
    expect(result.output_source).to be_nil
    expect(result.output_parse).to be_nil
    expect(result.source_segments).to be_empty
    expect(result.input_parses.length).to eq(3)
    expect(result.input_parses.all? { |parsed| parsed.parsed.ok }).to be(true)
    expect(result.analysis_rejections.length).to eq(1)
    rejection = result.analysis_rejections.first
    expect(rejection.code).to eq("analysis.unsupported_profile")
    expect(rejection.source_role.to_s).to eq("ours")
    expect(rejection.source_id).to eq("ours")
    expect(rejection.message).not_to be_empty
    expect(host.calls).to eq(1)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "fails closed when changed unowned comments cannot be preserved" do
    described_class.register_parser_host(TypedPsychHost.new)
    result = described_class.merge_yaml_mapping(merge_requests([
      "# base\na: one\nb: two\n", "# changed\na: ours\nb: two\n", "# base\na: one\nb: theirs\n"
    ]), merge_limits)
    expect(result.outcome.to_s).to eq("error")
    expect(result.output).to be_nil
    expect(result.diagnostics).not_to be_empty
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
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
