# frozen_string_literal: true

require_relative "native_merge_fixture"
require "weakref"

if (expected_home = ENV["STRUCTUREDMERGE_EXPECT_GEM_HOME"])
  installed = Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path
  raise "core must load from the isolated installed gem" unless installed.start_with?(File.expand_path(expected_home) + File::SEPARATOR)
  raise "prototype was activated" if Gem.loaded_specs.keys.any? { |name| name.include?("host_prototype") }
  loaded_native = $LOADED_FEATURES.select { |path| path.include?("structuredmerge_core_rb") }
  raise "native extension came from outside the installed gem" if loaded_native.empty? || loaded_native.any? { |path| !path.start_with?(installed + File::SEPARATOR) }
end

RSpec.describe StructuredmergeCore do
  include NativeMergeFixture

  it "reports structural operations as typed ordered profiles without selecting source" do
    match = described_class.report_structural_match(described_class::CrisprMatchRequest.new(
      start_boundary: "future", end_boundary: "owner_end_plus_trailing_gap", payload_kind: "comment_owned_body"))
    expect(match.known_start_boundary).to be(false)
    expect(match.trailing_gap_extended).to be(true)
    expect(match.comment_anchored).to be(true)
    selection = described_class.report_structural_selection(described_class::CrisprSelectionRequest.new(
      owner_scope: "", owner_selector: "", selector_kind: "", selection_intent: "",
      comment_region: nil, include_trailing_gap: true))
    expect(selection.comment_region).to be_nil
    expect(selection.owner_selector).to eq("line_bound_statements")
    destination = described_class.report_structural_destination(described_class::CrisprDestinationRequest.new(
      resolution_kind: "", resolution_source: "future", anchor_boundary: "", used_if_missing: true))
    expect(destination.append_fallback).to be(true)
    expect(destination.used_if_missing).to be(true)
    expect(destination.known_resolution_source).to be(false)
    requests = %w[replace future].map do |kind|
      described_class::CrisprOperationRequest.new(operation_kind: kind, source_requirement: "required",
        destination_requirement: "none", replacement_source: "explicit_text",
        captures_source_text: true, supports_if_missing: false)
    end
    report = described_class.report_structural_operations(requests)
    expect(report.operation_count).to eq(2)
    expect(report.operation_kinds).to eq(%w[replace future])
    expect(report.operation_profiles.first.requires_source).to be(true)
    expect(report.operation_profiles.first.known_operation_kind).to be(true)
    expect(report.operation_profiles.last.known_operation_kind).to be(false)
    expect(report.operation_profiles.last.operation_family).to eq("unknown")
  end

  it "applies explicit UTF-8 byte edits in Rust without a parser host" do
    text = "\uFEFFé: one\r\nlast"
    source = described_class::SourceInput.new(
      descriptor: described_class::SourceDescriptor.new(
        source_id: "edit-source", role: "source", byte_length: text.bytesize,
        sha256: Digest::SHA256.hexdigest(text), encoding: "utf8", bom: true,
        line_endings: described_class::LineEndings.new(lf: 0, crlf: 1, bare_cr: 0), final_newline: false
      ), bytes: text.bytes
    )
    limits = described_class::SourceEditLimits.new(max_input_bytes: 100, max_output_bytes: 100, max_edits: 2)
    request = described_class::SourceEditRequest.new(request_id: "edit-1", source: source,
      edits: [described_class::ExplicitSourceEdit.new(start_byte: 7, end_byte: 10, replacement: "two")])
    result = described_class.apply_explicit_source_edits(request, limits)
    expect(result.output).to eq("\uFEFFé: two\r\nlast")
    expect(result.request_id).to eq("edit-1")
    expect(result.edit_count).to eq(1)
    expect(result.source.sha256).to eq(source.descriptor.sha256)
    invalid = described_class::SourceEditRequest.new(request_id: "invalid", source: source,
      edits: [described_class::ExplicitSourceEdit.new(start_byte: 4, end_byte: 5, replacement: "x")])
    expect { described_class.apply_explicit_source_edits(invalid, limits) }.to raise_error(RuntimeError, /source_edit.rejected:/)
  end

  def register_ephemeral_parser
    host = TypedPsychHost.new
    reference = WeakRef.new(host)
    described_class.register_parser_host(host)
    reference
  end

  it "retains registered callbacks across GC and releases them after unregister" do
    12.times do
      reference = register_ephemeral_parser
      GC.start
      GC.compact if GC.respond_to?(:compact)
      expect(reference.weakref_alive?).to be_truthy
      result = described_class.merge_yaml_mapping(
        merge_requests(["a: one\nb: two\n", "a: ours\nb: two\n", "a: one\nb: theirs\n"]), merge_limits)
      expect(result.output).to eq("a: ours\nb: theirs\n")
      described_class.unregister_parser_host("ruby.typed.psych")
      # The generated dispatcher exits asynchronously after its final sender drops.
      50.times do
        Thread.pass
        GC.start
        break unless reference.weakref_alive?
      end
      expect(reference.weakref_alive?).to be_falsey
      failed = described_class.merge_yaml_mapping(merge_requests(["a: one\n"] * 3), merge_limits)
      expect(failed.input_failure.code).to eq("selection.no_parser")
    end
  ensure
    begin
      described_class.unregister_parser_host("ruby.typed.psych")
    rescue RuntimeError
      # Already unregistered on the successful path.
    end
  end

  it "keeps an in-flight parse alive when its callback unregisters the provider" do
    host = Class.new(TypedPsychHost) do
      def parse_batch(request)
        StructuredmergeCore.unregister_parser_host("ruby.typed.psych")
        GC.start
        super
      end
    end.new
    described_class.register_parser_host(host)
    requests = merge_requests(["a: one\n"] * 3)
    results = described_class.parse_sources(requests, merge_limits)
    expect(results.length).to eq(3)
    expect(results.map { |result| result.parsed.ok }).to eq([true, true, true])
    expect(host.calls).to eq(1)
    expect { described_class.parse_sources(requests, merge_limits) }.to raise_error(RuntimeError, /selection\.no_parser:/)
    replacement = TypedPsychHost.new
    described_class.register_parser_host(replacement)
    expect(described_class.parse_sources(requests, merge_limits).map { |result| result.parsed.ok }).to eq([true, true, true])
    expect(replacement.calls).to eq(1)
    expect(host.calls).to eq(1)
  ensure
    begin
      described_class.unregister_parser_host("ruby.typed.psych")
    rescue RuntimeError
      # Callback removal may have completed before a failed assertion.
    end
  end

  it "overlaps native callbacks on two Ruby threads without crossing results" do
    arrivals = Queue.new
    releases = Queue.new
    host = TypedPsychHost.new
    host.define_singleton_method(:parse_batch) do |request|
      arrivals << true
      raise "concurrent callback barrier timed out" unless releases.pop(timeout: 10)
      super(request)
    end
    described_class.register_parser_host(host)
    batches = 2.times.map { |index| merge_requests(["worker: value#{index}\n"] * 3) }
    limits = merge_limits
    workers = batches.map do |requests|
      Thread.new { described_class.parse_sources(requests, limits) }
    end
    2.times { expect(arrivals.pop(timeout: 10)).to be(true) }
    described_class.unregister_parser_host("ruby.typed.psych")
    replacement = TypedPsychHost.new
    described_class.register_parser_host(replacement)
    2.times { releases << true }
    results = workers.map do |worker|
      raise "concurrent parse did not complete" unless worker.join(15)
      worker.value
    end
    expect(host.calls).to eq(2)
    expect(replacement.calls).to eq(0)
    batches.zip(results).each do |requests, parsed|
      expect(parsed.length).to eq(3)
      expect(parsed.map { |result| result.parsed.ok }).to eq([true, true, true])
      expect(parsed.map { |result| result.parsed.source.sha256 }).to eq(requests.map { |request| request.source.descriptor.sha256 })
    end
    described_class.parse_sources(batches.first, limits)
    expect(replacement.calls).to eq(1)
    expect(host.calls).to eq(2)
  ensure
    2.times { releases << true } if releases
    workers&.each { |worker| worker.join(15) }
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "rejects expired deadlines and discards late input and verification results" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    requests = merge_requests(["a: one\nb: two\n", "a: ours\nb: two\n", "a: one\nb: theirs\n"])
    limits = lambda do |millis|
      described_class::ParseLimits.new(max_batch_items: 3, max_input_bytes: 10000,
        max_nodes: 1000, max_diagnostics: 20, timeout_millis: millis)
    end
    %i[parse_sources merge_yaml_mapping].each do |operation|
      expect { described_class.public_send(operation, requests, limits.call(0)) }.to raise_error(RuntimeError, /execution\.deadline_exceeded:/)
    end
    expect(host.calls).to eq(0)
    host.define_singleton_method(:parse_batch) do |request|
      result = super(request)
      sleep 0.15
      result
    end
    %i[parse_sources merge_yaml_mapping].each do |operation|
      expect { described_class.public_send(operation, requests, limits.call(100)) }.to raise_error(RuntimeError, /execution\.deadline_exceeded:/)
    end
    expect(host.calls).to eq(2)
    host.define_singleton_method(:parse_batch) do |request|
      result = super(request)
      sleep 0.15 if request.items.first.source.descriptor.role.to_s == "output"
      result
    end
    expect { described_class.merge_yaml_mapping(requests, limits.call(100)) }.to raise_error(RuntimeError, /execution\.deadline_exceeded:/)
    expect(host.calls).to eq(4)
    host.define_singleton_method(:parse_batch) do |request|
      super(request)
      sleep 0.15
      raise "native failure after deadline"
    end
    %i[parse_sources merge_yaml_mapping].each do |operation|
      expect { described_class.public_send(operation, requests, limits.call(100)) }.to raise_error(RuntimeError, /execution\.deadline_exceeded:/)
    end
    expect(host.calls).to eq(6)
    host.singleton_class.remove_method(:parse_batch)
    expect(described_class.merge_yaml_mapping(requests, limits.call(nil)).output).to eq("a: ours\nb: theirs\n")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "shares cancellation across threads and rejects late input and verification" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    requests = merge_requests(["a: one\nb: two\n", "a: ours\nb: two\n", "a: one\nb: theirs\n"])
    limits = merge_limits
    cancelled = described_class.create_operation_control
    expect(cancelled.is_cancelled).to be(false)
    2.times { cancelled.cancel }
    %i[parse_sources_controlled merge_yaml_mapping_controlled].each do |operation|
      expect { described_class.public_send(operation, requests, limits, cancelled) }.to raise_error(RuntimeError, /execution\.cancelled:/)
    end
    expect(host.calls).to eq(0)
    [[:parse_sources_controlled, false], [:merge_yaml_mapping_controlled, false], [:merge_yaml_mapping_controlled, true]].each do |operation, output_phase|
      control = described_class.create_operation_control
      arrivals, releases = Queue.new, Queue.new
      host.define_singleton_method(:parse_batch) do |request|
        result = super(request)
        if (request.items.first.source.descriptor.role.to_s == "output") == output_phase
          arrivals << true
          raise "cancellation barrier timed out" unless releases.pop(timeout: 10)
        end
        result
      end
      worker = Thread.new do
        described_class.public_send(operation, requests, limits, control)
      rescue RuntimeError => error
        error
      end
      begin
        expect(arrivals.pop(timeout: 10)).to be(true)
        control.cancel
        expect(control.is_cancelled).to be(true)
        releases << true
        expect(worker.join(15)).to eq(worker)
        expect(worker.value).to be_a(RuntimeError)
        expect(worker.value.message).to start_with("execution.cancelled:")
      ensure
        releases << true
        worker.join(15)
        host.singleton_class.remove_method(:parse_batch)
      end
    end
    fresh = described_class.create_operation_control
    expect(fresh.is_cancelled).to be(false)
    expect(described_class.merge_yaml_mapping_controlled(requests, limits, fresh).output).to eq("a: ours\nb: theirs\n")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "does not let a late callback failure override cancellation" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    requests = merge_requests(["a: one\n"] * 3)
    %i[parse_sources_controlled merge_yaml_mapping_controlled].each do |operation|
      control = described_class.create_operation_control
      host.define_singleton_method(:parse_batch) do |_request|
        control.cancel
        raise "native failure after cancellation"
      end
      expect { described_class.public_send(operation, requests, merge_limits, control) }.to raise_error(RuntimeError, /execution\.cancelled:/)
    end
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
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
    expect(result.diagnostics.length).to eq(1)
    expect(result.diagnostics.first.severity.to_s).to eq("error")
    expect(result.diagnostics.first.category.to_s).to eq("parse_error")
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
