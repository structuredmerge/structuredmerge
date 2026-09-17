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

  def common_request(operation, texts, policy: nil, request_id: "typed-common-#{operation}")
    roles = {"analyze" => %w[source], "diff2" => %w[before after], "merge2" => %w[incoming current], "merge3" => %w[base ours theirs]}.fetch(operation)
    sources = roles.zip(texts).to_h do |role, text|
      [role, StructuredmergeCore::OperationSource.new(source_id: role, role: role,
        byte_length: text.bytesize, sha256: Digest::SHA256.hexdigest(text), encoding: "utf-8", content: text, extra: {})]
    end
    policy ||= case operation
    when "analyze"
      StructuredmergeCore::OperationPolicy.from_analyze(StructuredmergeCore::AnalyzePolicy.new(extra: {}))
    when "diff2"
      StructuredmergeCore::OperationPolicy.from_diff2(StructuredmergeCore::DiffPolicy.new(extra: {}))
    when "merge2"
      StructuredmergeCore::OperationPolicy.from_merge2(StructuredmergeCore::DirectionalMergePolicy.new(
        directional_merge: "template-into-current", render_policy: "source-preserving", extra: {}))
    else
      StructuredmergeCore::OperationPolicy.from_merge3(StructuredmergeCore::ThreeWayMergePolicy.new(
        render_policy: "source-preserving", fallback_policy: "none", extra: {}))
    end
    StructuredmergeCore::OperationRequest.new(schema: "structuredmerge.operation-request/v1", request_id: request_id,
      operation: policy, sources: sources, extensions: [], metadata: {}, extra: {},
      provider_selection: StructuredmergeCore::MergeProviderSelection.new(provider_id: "kernel.yaml", family: "yaml",
        profile_id: "kernel.yaml.native_mapping.v1", required_capabilities: [operation], extra: {}),
      parser_selection: StructuredmergeCore::OperationParserSelection.new(backend: "ruby.typed.psych", preference: [],
        required_capabilities: [], extra: {}))
  end

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

  def run_yaml_common(operation, sources)
    host = TypedPsychHost.new
    StructuredmergeCore.register_parser_host(host)
    begin
      result = StructuredmergeCore.execute_operation(common_request(operation, sources), merge_limits)
      raise "common fixture did not call the native parser" unless host.calls.positive?
      result
    ensure
      StructuredmergeCore.unregister_parser_host("ruby.typed.psych")
    end
  end

  def run_yaml_common_analyze(source)
    run_yaml_common("analyze", [source])
  end

  def run_json_common(operation, dialect, sources, git_options = nil, family = "json")
    provider_id = "ruby.fixture.#{family}"
    StructuredmergeCore.register_language_pack_parser(provider_id, %w[bash go rust typescript].include?(family) ? dialect : ((dialect == "json") ? "json" : "json5"))
    begin
      original = common_request(operation, sources)
      policy = if git_options
        StructuredmergeCore::OperationPolicy.from_merge3(StructuredmergeCore::ThreeWayMergePolicy.new(
          render_policy: "source-preserving", conflict_marker_size: git_options[0], labels: {"ours" => git_options[1]}, extra: {}))
      else
        original.operation
      end
      request = StructuredmergeCore::OperationRequest.new(schema: original.schema, request_id: original.request_id,
        operation: policy, sources: original.sources, extensions: [], metadata: {}, extra: {},
        provider_selection: StructuredmergeCore::MergeProviderSelection.new(provider_id: %w[bash go rust typescript].include?(family) ? "kernel.#{family}" : (git_options ? "kernel.git.json" : "kernel.json"), family: family, dialect: dialect,
          profile_id: %w[bash go rust typescript].include?(family) ? "kernel.#{family}.owners.v1" : (git_options ? "kernel.git.json.v1" : "kernel.json.nested.v1"), required_capabilities: [operation], extra: {}),
        parser_selection: StructuredmergeCore::OperationParserSelection.new(backend: provider_id, preference: [], required_capabilities: [], extra: {}))
      StructuredmergeCore.execute_operation(request, merge_limits)
    ensure
      StructuredmergeCore.unregister_parser_provider(provider_id)
    end
  end

  def run_json_common_analyze(dialect, source)
    run_json_common("analyze", dialect, [source])
  end

  def run_json_common_diff(dialect, before, after)
    run_json_common("diff2", dialect, [before, after])
  end

  def run_json_common_merge2(dialect, incoming, current)
    run_json_common("merge2", dialect, [incoming, current])
  end

  def run_json_common_merge3(dialect, base, ours, theirs)
    run_json_common("merge3", dialect, [base, ours, theirs])
  end

  def run_git_common_merge3(dialect, base, ours, theirs, marker_size, ours_label)
    run_json_common("merge3", dialect, [base, ours, theirs], [marker_size, ours_label])
  end

  def run_typescript_common_analyze(dialect, source)
    run_json_common("analyze", dialect, [source], nil, "typescript")
  end

  def run_typescript_common_diff(dialect, before, after)
    run_json_common("diff2", dialect, [before, after], nil, "typescript")
  end

  def run_typescript_common_merge2(dialect, incoming, current)
    run_json_common("merge2", dialect, [incoming, current], nil, "typescript")
  end

  def run_typescript_common_merge3(dialect, base, ours, theirs)
    run_json_common("merge3", dialect, [base, ours, theirs], nil, "typescript")
  end

  def run_rust_common_analyze(source)
    run_json_common("analyze", "rust", [source], nil, "rust")
  end

  def run_rust_common_diff(before, after)
    run_json_common("diff2", "rust", [before, after], nil, "rust")
  end

  def run_rust_common_merge2(incoming, current)
    run_json_common("merge2", "rust", [incoming, current], nil, "rust")
  end

  def run_rust_common_merge3(base, ours, theirs)
    run_json_common("merge3", "rust", [base, ours, theirs], nil, "rust")
  end

  def run_go_common_analyze(source)
    run_json_common("analyze", "go", [source], nil, "go")
  end

  def run_go_common_diff(before, after)
    run_json_common("diff2", "go", [before, after], nil, "go")
  end

  def run_go_common_merge2(incoming, current)
    run_json_common("merge2", "go", [incoming, current], nil, "go")
  end

  def run_go_common_merge3(base, ours, theirs)
    run_json_common("merge3", "go", [base, ours, theirs], nil, "go")
  end

  def run_bash_common_analyze(source)
    run_json_common("analyze", "bash", [source], nil, "bash")
  end

  def run_bash_common_diff(before, after)
    run_json_common("diff2", "bash", [before, after], nil, "bash")
  end

  def run_bash_common_merge2(incoming, current)
    run_json_common("merge2", "bash", [incoming, current], nil, "bash")
  end

  def run_bash_common_merge3(base, ours, theirs)
    run_json_common("merge3", "bash", [base, ours, theirs], nil, "bash")
  end

  def run_yaml_common_diff(before, after)
    run_yaml_common("diff2", [before, after])
  end

  def run_yaml_common_merge(base, ours, theirs)
    run_yaml_common("merge3", [base, ours, theirs])
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
