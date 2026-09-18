# frozen_string_literal: true

require_relative "native_merge_fixture"
require "weakref"
require "json"
require "tmpdir"
require "fileutils"
require "open3"
require "rbconfig"
require "rbs"

if (expected_home = ENV["STRUCTUREDMERGE_EXPECT_GEM_HOME"])
  installed = Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path
  raise "core must load from the isolated installed gem" unless installed.start_with?(File.expand_path(expected_home) + File::SEPARATOR)
  raise "prototype was activated" if Gem.loaded_specs.keys.any? { |name| name.include?("host_prototype") }
  loaded_native = $LOADED_FEATURES.select { |path| path.include?("structuredmerge_core_rb") }
  raise "native extension came from outside the installed gem" if loaded_native.empty? || loaded_native.any? { |path| !path.start_with?(installed + File::SEPARATOR) }
end

RSpec.describe StructuredmergeCore do
  include NativeMergeFixture

  it "keeps compiled batch parser pinning distinct from requested policy selection" do
    core = described_class
    core.register_parser_host(TypedPsychHost.new)
    original = common_request("analyze", ["a: one\n"])
    operation = core::OperationRequest.new(schema: original.schema, request_id: original.request_id,
      operation: original.operation, sources: original.sources, provider_selection: original.provider_selection,
      parser_selection: core::OperationParserSelection.new(preference: ["ruby.typed.psych"], required_capabilities: [], extra: {}),
      extensions: [], metadata: {}, extra: {})
    item = core::WorkflowOperation.new(operation: operation, parser_language: "yaml", parser_dialect: nil,
      parse_options: core::ParseOptions.new(comments: false, tokens: false, diagnostics: false, native_extensions: true))
    result = core.execute_workflow_batch("kernel.yaml", core::WorkflowBatchRequest.new(items: [item]), workflow_limits)
    expect(result.execution_owner).to eq(:kernel)
    expect(result.approved_as_default).to be(false)
    expect(result.results.first.ok).to be(true)
    parser = result.results.first.profile.parser
    expect(parser.requested_backend).to be_nil
    expect(parser.selected_backend).to eq("ruby.typed.psych")
    expect(parser.selection_mode).to eq("policy")
  ensure
    core.unregister_parser_provider("ruby.typed.psych") if core
  end

  it "lists compiled workflows without granting host retirement authority" do
    inventory = described_class.workflow_registry_inventory
    profiles = described_class.operation_profile_catalog.profiles
    expect(inventory.providers.map(&:provider_id)).to include(*profiles.map(&:provider_id))
    reserved = Object.new
    reserved.define_singleton_method(:descriptor) { inventory.providers.find { |provider| provider.provider_id == "kernel.json" } }
    reserved.define_singleton_method(:execute_batch) { |*| raise "reserved host must never execute" }
    expect { described_class.register_workflow_host(reserved) }.to raise_error(RuntimeError, /workflow.reserved_provider/)
    expect { described_class.replace_workflow_host(reserved, inventory.generation) }.to raise_error(RuntimeError, /workflow.reserved_provider/)
    profiles.each do |profile|
      expect { described_class.unregister_workflow_host(profile.provider_id, inventory.generation) }.to raise_error(RuntimeError, /workflow.reserved_provider/)
    end
    expect(described_class.workflow_registry_inventory.generation).to eq(inventory.generation)
  end

  it "delivers native Psych facts and shared cancellation to a coarse workflow callback" do
    core = described_class
    parser = TypedPsychHost.new
    parser_errors = []
    parse_batch = parser.method(:parse_batch)
    parser.define_singleton_method(:parse_batch) do |batch|
      parse_batch.call(batch)
    rescue => error
      parser_errors << "#{error.class}: #{error.message}"
      raise
    end
    core.register_parser_host(parser)
    host = TypedWorkflowHost.new
    provider_id = host.descriptor.provider_id
    request = workflow_request
    limits = workflow_limits
    generation = core.register_workflow_host(host)
    begin
      execution = core.execute_workflow_batch(provider_id, request, limits)
    rescue RuntimeError
      expect(parser_errors).to be_empty
      raise
    end
    expect(host.calls.length).to eq(1)
    expect(host.calls.first.items.length).to eq(2)
    expect(execution.results.map(&:request_id)).to eq(%w[workflow-0 workflow-1])
    expect(execution.execution_owner).to eq(:host)
    expect(execution.approved_as_default).to be(false)
    expect(execution.results.all? { |result| JSON.parse(result.analysis.extra.fetch("native_node_count")) > 0 }).to be(true)
    host.cancel = true
    control = core.create_operation_control
    expect { core.execute_workflow_batch_controlled(provider_id, request, limits, control) }.to raise_error(RuntimeError, /execution.cancelled/)
    expect(control.is_cancelled).to be(true)
    expect(host.calls.length).to eq(2)
  ensure
    core.unregister_workflow_host(provider_id, generation) if generation
    core.unregister_parser_host("ruby.typed.psych")
  end

  def register_ephemeral_workflow(tag)
    host = TypedWorkflowHost.new(tag)
    reference = WeakRef.new(host)
    [reference, described_class.register_workflow_host(host)]
  end

  it "retains workflow callbacks across GC and releases retired registrations" do
    described_class.register_parser_host(TypedPsychHost.new)
    8.times do |index|
      reference, generation = register_ephemeral_workflow(index.to_s)
      begin
        GC.start
        GC.compact if GC.respond_to?(:compact)
        expect(reference.weakref_alive?).to be_truthy
        execution = described_class.execute_workflow_batch("ruby.psych.workflow", workflow_request, workflow_limits)
        expect(JSON.parse(execution.results.first.metadata.fetch("host_tag"))).to eq(index.to_s)
      ensure
        described_class.unregister_workflow_host("ruby.psych.workflow", generation)
      end
      50.times do
        Thread.pass
        GC.start
        break unless reference.weakref_alive?
      end
      expect(reference.weakref_alive?).to be_falsey
    end
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "keeps in-flight workflow snapshots through reentrant replacement and rejects stale writes" do
    described_class.register_parser_host(TypedPsychHost.new)
    original = TypedWorkflowHost.new("original")
    replacement = TypedWorkflowHost.new("replacement")
    generation = described_class.register_workflow_host(original)
    inventory = described_class.workflow_registry_inventory
    current = generation
    original.before_return = lambda do
      current = described_class.replace_workflow_host(replacement, generation)
      expect { described_class.unregister_workflow_host("ruby.psych.workflow", generation) }.to raise_error(RuntimeError, /StaleGeneration/)
      expect { described_class.replace_workflow_host(TypedWorkflowHost.new("stale"), generation) }.to raise_error(RuntimeError, /StaleGeneration/)
    end
    execution = described_class.execute_workflow_batch("ruby.psych.workflow", workflow_request, workflow_limits)
    expect(JSON.parse(execution.provider.metadata.fetch("host_tag"))).to eq("original")
    expect(execution.results.map { |result| JSON.parse(result.metadata.fetch("host_tag")) }).to eq(["original"] * 2)
    expect(execution.selections.map(&:provider_generation)).to eq([generation] * 2)
    expect(original.calls.length).to eq(1)
    expect(replacement.calls).to be_empty
    expect(JSON.parse(inventory.providers.find { |provider| provider.provider_id == "ruby.psych.workflow" }.metadata.fetch("host_tag"))).to eq("original")
    subsequent = described_class.execute_workflow_batch("ruby.psych.workflow", workflow_request, workflow_limits)
    expect(JSON.parse(subsequent.provider.metadata.fetch("host_tag"))).to eq("replacement")
    expect(subsequent.selections.map(&:provider_generation)).to eq([current] * 2)
    expect(replacement.calls.length).to eq(1)
  ensure
    described_class.unregister_workflow_host("ruby.psych.workflow", current) if current
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "lets a workflow callback unregister itself without retrying or losing its batch" do
    described_class.register_parser_host(TypedPsychHost.new)
    host = TypedWorkflowHost.new
    generation = described_class.register_workflow_host(host)
    retired = false
    host.before_return = lambda do
      described_class.unregister_workflow_host("ruby.psych.workflow", generation)
      retired = true
    end
    execution = described_class.execute_workflow_batch("ruby.psych.workflow", workflow_request, workflow_limits)
    expect(execution.results.length).to eq(2)
    expect(host.calls.length).to eq(1)
    expect(described_class.workflow_registry_inventory.providers.map(&:provider_id)).not_to include("ruby.psych.workflow")
    expect { described_class.execute_workflow_batch("ruby.psych.workflow", workflow_request, workflow_limits) }.to raise_error(RuntimeError, /workflow/)
    expect(host.calls.length).to eq(1)
  ensure
    described_class.unregister_workflow_host("ruby.psych.workflow", generation) if generation && !retired
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "overlaps workflow callbacks on four Ruby threads without crossing thread-local context" do
    described_class.register_parser_host(TypedPsychHost.new)
    host = TypedWorkflowHost.new
    arrivals = Queue.new
    releases = []
    closing = false
    observations = []
    host.before_return = lambda do
      observations << [Thread.current.object_id, Thread.current.thread_variable_get(:workflow_worker)]
      gate = Queue.new
      releases << gate
      arrivals << gate
      raise "workflow callback barrier timed out" unless closing || gate.pop(timeout: 10)
    end
    generation = described_class.register_workflow_host(host)
    request, limits = workflow_request, workflow_limits
    workers = 4.times.map do |index|
      Thread.new do
        Thread.current.thread_variable_set(:workflow_worker, index)
        4.times.map do
          result = described_class.execute_workflow_batch("ruby.psych.workflow", request, limits)
          [Thread.current.object_id, index, result.results.length]
        end
      rescue => error
        error
      end
    end
    4.times do
      gates = 4.times.map do
        gate = arrivals.pop(timeout: 10)
        expect(gate).to be_a(Queue)
        gate
      end
      gates.each { |gate| gate << true }
    end
    results = workers.flat_map do |worker|
      raise "workflow worker did not finish" unless worker.join(15)
      value = worker.value
      raise value if value.is_a?(Exception)
      value
    end
    expect(observations).to match_array(results.map { |thread, index, _| [thread, index] })
    expect(results.map(&:last)).to eq([2] * 16)
    expect(results.map(&:first).uniq.length).to eq(4)
    expect(host.calls.length).to eq(16)
  ensure
    closing = true
    releases&.each { |gate| gate << true }
    workers&.each { |worker| worker.join(15) }
    described_class.unregister_workflow_host("ruby.psych.workflow", generation) if generation
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "discards a late workflow result after cross-thread cancellation and registry retirement" do
    described_class.register_parser_host(TypedPsychHost.new)
    host = TypedWorkflowHost.new
    arrived, release = Queue.new, Queue.new
    host.before_return = lambda do
      arrived << true
      raise "workflow cancellation barrier timed out" unless release.pop(timeout: 10)
    end
    generation = described_class.register_workflow_host(host)
    retired = false
    control = described_class.create_operation_control
    request, limits = workflow_request, workflow_limits
    worker = Thread.new do
      described_class.execute_workflow_batch_controlled("ruby.psych.workflow", request, limits, control)
    rescue RuntimeError => error
      error
    end
    begin
      expect(arrived.pop(timeout: 10)).to be(true)
      control.cancel
      described_class.unregister_workflow_host("ruby.psych.workflow", generation)
      retired = true
    ensure
      release << true
    end
    raise "cancelled workflow worker did not finish" unless worker.join(15)
    expect(worker.value).to be_a(RuntimeError)
    expect(worker.value.message).to include("execution.cancelled")
    expect(host.calls.length).to eq(1)
    expect(described_class.workflow_registry_inventory.providers.map(&:provider_id)).not_to include("ruby.psych.workflow")
  ensure
    release << true if release
    worker&.join(15)
    described_class.unregister_workflow_host("ruby.psych.workflow", generation) if generation && !retired
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "exits fresh runtimes with registered, retired and cancelled-drained workflow hosts" do
    script = <<~'RUBY'
      require_relative "native_merge_fixture"
      include NativeMergeFixture
      mode = ARGV.fetch(0)
      StructuredmergeCore.register_parser_host(TypedPsychHost.new)
      host = TypedWorkflowHost.new
      generation = StructuredmergeCore.register_workflow_host(host)
      request, limits = workflow_request, workflow_limits
      if mode == "drained"
        entered, release = Queue.new, Queue.new
        host.before_return = lambda do
          entered << true
          raise "workflow not released" unless release.pop(timeout: 5)
        end
        control = StructuredmergeCore.create_operation_control
        worker = Thread.new do
          StructuredmergeCore.execute_workflow_batch_controlled("ruby.psych.workflow", request, limits, control)
          "unexpected success"
        rescue RuntimeError => error
          error.message
        end
        begin
          raise "workflow never entered" unless entered.pop(timeout: 5)
          StructuredmergeCore.unregister_workflow_host("ruby.psych.workflow", generation)
          control.cancel
        ensure
          release << true
          raise "workflow failed to drain" unless worker.join(5)
        end
        raise "late result was accepted" unless worker.value.include?("execution.cancelled")
      else
        result = StructuredmergeCore.execute_workflow_batch("ruby.psych.workflow", request, limits)
        raise "wrong result count" unless result.results.length == 2
        StructuredmergeCore.unregister_workflow_host("ruby.psych.workflow", generation) if mode == "retired"
      end
      raise "workflow did not finish once" unless host.calls.length == 1
      host = nil
      GC.start
      GC.compact if GC.respond_to?(:compact)
      puts "ready-to-exit:#{mode}"
    RUBY
    %w[registered retired drained].each do |mode|
      3.times do
        Open3.popen2e(RbConfig.ruby, "-rbundler/setup", "-e", script, mode, chdir: __dir__) do |input, output, process|
          input.close
          reader = Thread.new { output.read }
          begin
            expect(process.join(20)).not_to be_nil, "workflow runtime exit timed out: #{mode}"
            expect(process.value.success?).to be(true), reader.value
            expect(reader.value.strip).to eq("ready-to-exit:#{mode}")
          ensure
            if process.alive?
              Process.kill("KILL", process.pid)
              process.join
            end
            reader.join
          end
        end
      end
    end
  end

  it "separates capability declarations, parser eligibility and default approval" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    query = lambda do |profile = "kernel.yaml.native_mapping.v1", operation = :merge3, backend = "ruby.typed.psych"|
      described_class::CapabilityQuery.new(profile_id: profile, operation: operation, dialect: nil,
        parser_selection: described_class::ParserSelection.new(backend_id: backend, preference: [], required_capabilities: []))
    end
    probes = []
    original = host.method(:probe_batch)
    host.define_singleton_method(:probe_batch) do |request|
      probes << request
      original.call(request)
    end
    inventory = described_class.capability_manifest([], merge_limits)
    expect(inventory).to be_a(described_class::CapabilityManifest)
    expect(inventory.schema).to eq("structuredmerge.typed-capability-manifest/v1")
    expect(inventory.profiles.profiles.length).to eq(8)
    expect(probes).to be_empty
    expect { described_class.capability_manifest([query.call, query.call("unknown")], merge_limits) }.to raise_error(RuntimeError, /capability.unknown_profile/)
    expect(probes).to be_empty
    observations = described_class.capability_manifest([
      query.call("kernel.yaml.native_mapping.v1", :merge2), query.call,
      query.call("kernel.yaml.native_mapping.v1", :merge3, "missing")
    ], merge_limits).observations
    expect(observations[0].operation_declared).to be(false)
    expect(observations[0].parser_report).to be_nil
    expect(observations[0].parser_eligible).to be_nil
    expect(observations[1].parser_eligible).to be(true)
    expect(observations[1].parser_report.selected_backend).to eq("ruby.typed.psych")
    expect(observations[1].parser_request.options.native_extensions).to be(true)
    expect(observations[2].parser_eligible).to be(false)
    expect(observations.map(&:approved_as_default)).to eq([false, false, false])
    expect(host.calls).to eq(0)
    control = described_class.create_operation_control
    control.cancel
    expect { described_class.capability_manifest_controlled([], merge_limits, control) }.to raise_error(RuntimeError, /execution.cancelled/)
    expect { described_class.capability_manifest([query.call] * (merge_limits.max_batch_items + 1), merge_limits) }.to raise_error(RuntimeError, /resource.limit/)
    host.define_singleton_method(:probe_batch) { |_request| raise "injected capability probe failure" }
    fault = described_class.capability_manifest([query.call], merge_limits).observations.first
    expect(fault.parser_eligible).to be(false)
    expect(fault.parser_report.candidates.first.probe_fault).not_to be_nil
    expect(fault.approved_as_default).to be(false)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "retains capability snapshot identity when a probe retires its host" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    query = described_class::CapabilityQuery.new(profile_id: "kernel.yaml.native_mapping.v1",
      operation: :merge3, dialect: nil, parser_selection: described_class::ParserSelection.new(
        backend_id: "ruby.typed.psych", preference: [], required_capabilities: []))
    probes = []
    original = host.method(:probe_batch)
    host.define_singleton_method(:probe_batch) do |request|
      StructuredmergeCore.unregister_parser_host("ruby.typed.psych") if probes.empty?
      probes << request
      original.call(request)
    end
    manifest = described_class.capability_manifest([query, query], merge_limits)
    expect(probes.length).to eq(2)
    manifest.observations.each do |item|
      expect(item.parser_eligible).to be(true)
      expect(item.parser_report.generation).to eq(manifest.parsers.generation)
      expect(item.parser_report.digest).to eq(manifest.parsers.descriptor_digest)
    end
    expect(described_class.parser_registry_inventory.providers).to be_empty
    expect(host.calls).to eq(0)
  ensure
    if described_class.parser_registry_inventory.providers.any? { |provider| provider.id == "ruby.typed.psych" }
      described_class.unregister_parser_host("ruby.typed.psych")
    end
  end

  it "exposes the runtime classes, readers and methods declared by the installed RBS" do
    signature = File.join(Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path, "sig/types.rbs")
    namespace = RBS::Parser.parse_signature(File.read(signature)).last.find do |node|
      node.is_a?(RBS::AST::Declarations::Module) && node.name.to_s == "StructuredmergeCore"
    end
    expect(namespace).not_to be_nil
    aggregate_failures do
      namespace.members.each do |declaration|
        case declaration
        when RBS::AST::Declarations::Class
          name = declaration.name.name
          expect(described_class.const_defined?(name, false)).to be(true), "missing runtime class #{name}"
          next unless described_class.const_defined?(name, false)

          runtime = described_class.const_get(name, false)
          expect(runtime).to be_a(Class)
          declaration.members.each do |member|
            case member
            when RBS::AST::Members::AttrReader
              expect(runtime.public_instance_methods).to include(member.name)
            when RBS::AST::Members::MethodDefinition
              if member.name == :initialize
                expect(runtime.singleton_methods).to include(:new)
              elsif member.kind == :singleton
                expect(runtime.singleton_methods).to include(member.name)
              else
                expect(runtime.public_instance_methods).to include(member.name)
              end
            end
          end
        when RBS::AST::Members::MethodDefinition
          expect(described_class.singleton_methods).to include(declaration.name)
        end
      end
    end
  end

  it "declares source roles as symbol values rather than nonexistent runtime classes" do
    signature = File.join(Gem.loaded_specs.fetch("structuredmerge-core").full_gem_path, "sig/types.rbs")
    namespace = RBS::Parser.parse_signature(File.read(signature)).last.first
    role_alias = namespace.members.find do |node|
      node.is_a?(RBS::AST::Declarations::TypeAlias) && node.name.name == :enum_SourceRole
    end
    expect(role_alias).not_to be_nil
    expect(role_alias.type).to be_a(RBS::Types::Union)
    literals = role_alias.type.types.map(&:literal)
    expect(literals).to contain_exactly(:source, :before, :after, :incoming, :current, :base, :ours, :theirs, :output)
    roles = merge_requests(["a: 1\n"] * 3).map { |request| request.source.descriptor.role }
    # The shared fixture deliberately reverses requests to test role-based
    # routing. This assertion checks the value representation, not batch order.
    expect(roles).to contain_exactly(:base, :ours, :theirs)
    expect(described_class.const_defined?(:SourceRole, false)).to be(false)
  end

  it "exits fresh runtimes with registered, retired and cancelled-drained callbacks" do
    script = <<~'RUBY'
      require_relative "native_merge_fixture"
      include NativeMergeFixture
      mode = ARGV.fetch(0)
      host = TypedPsychHost.new
      StructuredmergeCore.register_parser_host(host)
      requests = merge_requests(["a: one\n"] * 3)
      limits = merge_limits
      if mode == "drained"
        entered, release = Queue.new, Queue.new
        host.define_singleton_method(:parse_batch) do |request|
          entered << true
          raise "callback not released" unless release.pop(timeout: 5)
          super(request)
        end
        control = StructuredmergeCore.create_operation_control
        worker = Thread.new do
          begin
            StructuredmergeCore.parse_sources_controlled(requests, limits, control)
            "unexpected success"
          rescue RuntimeError => error
            error.message
          end
        end
        begin
          raise "callback never entered" unless entered.pop(timeout: 5)
          StructuredmergeCore.unregister_parser_host("ruby.typed.psych")
          control.cancel
        ensure
          release << true
          raise "operation failed to drain" unless worker.join(5)
        end
        raise "late result was accepted" unless worker.value.include?("execution.cancelled")
        raise "callback did not finish" unless host.calls == 1
      else
        results = StructuredmergeCore.parse_sources(requests, limits)
        raise "parse failed" unless results.length == 3 && results.all? { |item| item.parsed.ok }
        raise "callback did not finish" unless host.calls == 1
        StructuredmergeCore.unregister_parser_host("ruby.typed.psych") if mode == "retired"
      end
      host = nil
      GC.start
      GC.compact if GC.respond_to?(:compact)
      puts "ready-to-exit:#{mode}"
    RUBY
    %w[registered retired drained].each do |mode|
      3.times do
        Open3.popen2e(RbConfig.ruby, "-rbundler/setup", "-e", script, mode, chdir: __dir__) do |input, output, process|
          input.close
          reader = Thread.new { output.read }
          begin
            expect(process.join(20)).not_to be_nil, "runtime exit timed out: #{mode}"
            expect(process.value.success?).to be(true), reader.value
            expect(reader.value.strip).to eq("ready-to-exit:#{mode}")
          ensure
            if process.alive?
              Process.kill("KILL", process.pid)
              process.join
            end
            reader.join
          end
        end
      end
    end
  end

  it "lists common operation scope without probing or granting default authority" do
    host = TypedPsychHost.new
    probe_calls = 0
    host.define_singleton_method(:probe_batch) do |_request|
      probe_calls += 1
      raise "catalog must not probe"
    end
    described_class.register_parser_host(host)
    generation = described_class.parser_registry_inventory.generation
    catalog = described_class.operation_profile_catalog
    expect(catalog).to be_a(described_class::OperationProfileCatalog)
    expect(catalog.schema).to eq("structuredmerge.operation-profile-catalog/v1")
    expect(catalog.profiles.length).to eq(8)
    ids = catalog.profiles.map(&:id)
    expect(ids).to eq(ids.sort)
    catalog.profiles.each do |profile|
      expect(profile.parser_available).to be_nil
      expect(profile.approved_as_default).to be(false)
      expect(profile.experimental).to be(true)
      expect(profile.semantic_runtime).to eq("rust")
      expect(profile.limitations).not_to be_empty
      operations = profile.operations.map(&:to_s)
      case profile.provider_id
      when "kernel.git.json" then expect(operations).to eq(["merge3"])
      when "kernel.yaml" then expect(operations).to eq(["analyze", "diff2", "merge3"])
      else expect(operations).to eq(["analyze", "diff2", "merge2", "merge3"])
      end
    end
    expect(described_class.parser_registry_inventory.generation).to eq(generation)
    expect(probe_calls).to eq(0)
    expect(host.calls).to eq(0)
    expect(described_class.native_merge_profiles.length).to eq(2)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "atomically replaces an in-flight parser and rejects stale generations" do
    replacement = TypedPsychHost.new
    original = TypedPsychHost.new
    described_class.register_parser_host(original)
    generation = described_class.parser_registry_inventory.generation
    expect { described_class.replace_parser_host(replacement, nil) }.to raise_error(TypeError)
    expect { described_class.replace_parser_host(replacement, generation - 1) }.to raise_error(RuntimeError, /StaleGeneration/)
    expect(described_class.parser_registry_inventory.generation).to eq(generation)
    original.define_singleton_method(:parse_batch) do |request|
      committed = StructuredmergeCore.replace_parser_host(replacement, generation)
      raise "unexpected generation" unless committed == generation + 1
      super(request)
    end
    requests = [merge_requests(["a: one\n"] * 3).first]
    first = described_class.parse_sources(requests, merge_limits)
    expect(first.first.parsed.ok).to be(true)
    expect(original.calls).to eq(1)
    expect(replacement.calls).to eq(0)
    second = described_class.parse_sources(requests, merge_limits)
    expect(second.first.parsed.ok).to be(true)
    expect(original.calls).to eq(1)
    expect(replacement.calls).to eq(1)
    expect { described_class.replace_parser_host(original, generation) }.to raise_error(RuntimeError, /StaleGeneration/)
    described_class.unregister_parser_host("ruby.typed.psych")
    latest = described_class.parser_registry_inventory.generation
    expect { described_class.replace_parser_host(replacement, latest) }.to raise_error(RuntimeError, /UnknownId/)
  ensure
    if described_class.parser_registry_inventory.providers.any? { |provider| provider.id == "ruby.typed.psych" }
      described_class.unregister_parser_host("ruby.typed.psych")
    end
  end

  it "reports request-specific parser eligibility without parsing source" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    query = lambda do |backend, comments = false|
      described_class::ParserSelectionRequest.new(language: "yaml", dialect: nil,
        selection: described_class::ParserSelection.new(backend_id: backend, preference: [], required_capabilities: []),
        options: described_class::ParseOptions.new(comments: comments, tokens: false, diagnostics: false, native_extensions: false))
    end
    report = described_class.parser_selection_report(query.call("ruby.typed.psych"), merge_limits)
    expect(report.selected_backend).to eq("ruby.typed.psych")
    candidate = report.candidates.find { |item| item.backend_id == "ruby.typed.psych" }
    expect(candidate.available).to be(true)
    expect(candidate.loadable).to be(true)
    unsupported = described_class.parser_selection_report(query.call("ruby.typed.psych", true), merge_limits)
    expect(unsupported.selected_backend).to be_nil
    rejected = unsupported.candidates.find { |item| item.backend_id == "ruby.typed.psych" }
    expect(rejected.rejections).to include("missing_capability:comments")
    expect(rejected.available).to be_nil
    missing = described_class.parser_selection_report(query.call("missing.inventory.parser"), merge_limits)
    expect(missing.selected_backend).to be_nil
    expect(missing.candidates.map(&:selected)).not_to include(true)
    expect(missing.candidates.find { |item| item.backend_id == "ruby.typed.psych" }.available).to be_nil
    control = described_class.create_operation_control
    control.cancel
    expect { described_class.parser_selection_report_controlled(query.call("ruby.typed.psych"), merge_limits, control) }.to raise_error(RuntimeError, /execution.cancelled/)
    expect(host.calls).to eq(0)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "observes owned parser declarations without probing or changing registration" do
    host = TypedPsychHost.new
    def host.probe_batch(_request) = raise("inventory must not probe")
    described_class.register_parser_host(host)
    before = described_class.parser_registry_inventory
    expect(before).to be_a(described_class::ParserRegistryInventory)
    expect(before.schema).to eq("structuredmerge.parser-registry-inventory/v1")
    ids = before.providers.map(&:id)
    expect(ids).to eq(ids.sort)
    expect(ids).to include("ruby.typed.psych")
    before.providers.find { |provider| provider.id == "ruby.typed.psych" }.languages.clear
    unchanged = described_class.parser_registry_inventory
    expect(unchanged.providers.find { |provider| provider.id == "ruby.typed.psych" }.languages).to include("yaml")
    expect(unchanged.generation).to eq(before.generation)
    expect(unchanged.descriptor_digest).to eq(before.descriptor_digest)
    described_class.unregister_parser_host("ruby.typed.psych")
    removed = described_class.parser_registry_inventory
    expect(removed.providers.map(&:id)).not_to include("ruby.typed.psych")
    expect(removed.generation).to be > before.generation
    expect(before.providers.map(&:id)).to include("ruby.typed.psych")
    expect(host.calls).to eq(0)
  ensure
    if described_class.parser_registry_inventory.providers.any? { |provider| provider.id == "ruby.typed.psych" }
      described_class.unregister_parser_host("ruby.typed.psych")
    end
  end

  it "executes nested JSON common merges in Rust with render and conflict evidence" do
    provider_id = "ruby.common.json"
    described_class.register_language_pack_parser(provider_id, "json")
    request = lambda do |operation, texts, dialect = "json"|
      original = common_request(operation, texts)
      described_class::OperationRequest.new(schema: original.schema, request_id: original.request_id,
        operation: original.operation, sources: original.sources, extensions: [], metadata: {}, extra: {},
        provider_selection: described_class::MergeProviderSelection.new(provider_id: "kernel.json", family: "json", dialect: dialect,
          profile_id: "kernel.json.nested.v1", required_capabilities: [operation], extra: {}),
        parser_selection: described_class::OperationParserSelection.new(backend: provider_id, preference: [], required_capabilities: [], extra: {}))
    end
    directional = described_class.execute_operation(request.call("merge2", ['{"x":{"add":2}}', '{"x":{"keep":1}}']), merge_limits)
    expect(directional.ok).to be(true)
    expect(JSON.parse(directional.output)).to eq("x" => {"keep" => 1, "add" => 2})
    proof = JSON.parse(directional.render_report.fetch("evidence"))
    expect(proof.dig("baseline", "role")).to eq("current")
    expect(proof.fetch("edits")).not_to be_empty
    merged = described_class.execute_operation(request.call("merge3", ['{"a":1,"b":2}', '{"a":3,"b":2}', '{"a":1,"b":4}']), merge_limits)
    expect(merged.ok).to be(true)
    expect(JSON.parse(merged.output)).to eq("a" => 3, "b" => 4)
    conflict = described_class.execute_operation(request.call("merge3", ['{"a":1}', '{"a":2}', '{"a":3}']), merge_limits)
    expect(conflict.ok).to be(false)
    expect(conflict.output).to be_nil
    expect(conflict.conflicts.first.canonical.alternatives.length).to eq(3)
    diff = described_class.execute_operation(request.call("diff2", ['{"x":1}', '{"x":2}']), merge_limits)
    expect(diff.ok).to be(true)
    expect(diff.output).to be_nil
    expect(diff.changes.map(&:path)).to eq([nil, "", "/x"])
    expect(JSON.parse(diff.diff.extra.fetch("document_bytes_compared"))).to be(true)
    trivia = described_class.execute_operation(request.call("diff2", ["{}", "{}\r\n"]), merge_limits)
    expect(trivia.ok).to be(true)
    expect(trivia.changes.length).to eq(1)
    expect(trivia.changes.first.subject_ref).to eq("json.document")
    described_class.unregister_parser_provider(provider_id)
    described_class.register_language_pack_parser(provider_id, "json5")
    analysis = described_class.execute_operation(request.call("analyze", ["{} /* unclaimed */"], "json5"), merge_limits)
    expect(analysis.ok).to be(true)
    expect(analysis.output).to be_nil
    owners = JSON.parse(analysis.analysis.extra.fetch("owners"))
    comments = JSON.parse(analysis.analysis.extra.fetch("comment_regions"))
    expect(owners.first.fetch("id")).to eq("json:")
    expect(comments.first.fetch("owner_id")).to eq("json:")
    expect(comments.first.fetch("attachment_resolved")).to be(false)
    expect(JSON.parse(analysis.analysis.extra.fetch("diagnostics")).first.fetch("code")).to eq("json.comment_attachment_unresolved")
  ensure
    described_class.unregister_parser_provider(provider_id)
  end

  it "registers cached-only grammars without acquisition or replacement" do
    provider_id = "ruby.typed.cached.missing"
    descriptor = described_class.register_cached_language_pack_parser(provider_id, "not-a-real-grammar")
    begin
      expect(JSON.parse(descriptor.metadata.fetch("grammar_policy"))).to eq("cached-only")
      expect { described_class.register_language_pack_parser(provider_id, "json") }.to raise_error(RuntimeError, /DuplicateId/)
      query = described_class::ParserSelectionRequest.new(language: "not-a-real-grammar", dialect: nil,
        selection: described_class::ParserSelection.new(backend_id: provider_id, preference: [], required_capabilities: []),
        options: described_class::ParseOptions.new(comments: false, tokens: false, diagnostics: false, native_extensions: false))
      expect(described_class.parser_selection_report(query, merge_limits).selected_backend).to be_nil
    ensure
      described_class.unregister_parser_provider(provider_id)
    end
  end

  it "registers the cached-only Rust language pack in the typed TreeHaver registry" do
    provider_id = "ruby.typed.tslp.json"
    descriptor = described_class.register_cached_language_pack_parser(provider_id, "json")
    begin
      expect(descriptor.runtime).to eq("rust")
      expect(descriptor.languages).to eq(["json"])
      expect { described_class.register_language_pack_parser(provider_id, "python") }.to raise_error(RuntimeError, /DuplicateId/)
      request = lambda do |text|
        source = merge_requests([text], roles: ["source"]).first.source
        described_class::ParseRequest.new(schema: "structuredmerge.parse-request/v1", request_id: "json",
          source: source, language: "json", dialect: nil,
          selection: described_class::ParserSelection.new(backend_id: provider_id, preference: [], required_capabilities: []),
          options: described_class::ParseOptions.new(comments: true, diagnostics: true, tokens: false, native_extensions: true),
          metadata: {}, extra: {})
      end
      result = described_class.parse_sources([request.call("// note\r\n{\"é\": [true]}")], merge_limits).first
      expect(result.backend.id).to eq(provider_id)
      expect(result.selection.selected_backend).to eq(provider_id)
      expect(result.parsed.ok).to be(true)
      expect(result.parsed.comments.length).to eq(1)
      comment = result.parsed.nodes.find { |node| node.id == result.parsed.comments.first.node_id }
      expect(comment.extensions.first.schema).to eq("tree-haver.tree-sitter.node/v1")
      expect(comment.extensions.first.payload).to eq('{"extra":true}')
      expect(result.parsed.nodes.flat_map(&:children).map(&:field_name)).to include("key")
      broken = described_class.parse_sources([request.call('{"x":')], merge_limits).first.parsed
      expect(broken.ok).to be(false)
      expect(broken.nodes).not_to be_empty
      expect(broken.nodes.any?(&:has_error)).to be(true)
      expect(broken.diagnostics.first.blocking).to be(true)
    ensure
      described_class.unregister_parser_provider(provider_id)
    end
    expect { described_class.parse_sources([request.call("{}")], merge_limits) }.to raise_error(RuntimeError, /selection.no_parser/)
  end

  it "executes typed common operations through the registered native Psych provider" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    cases = {"analyze" => ["a: one\n"], "diff2" => ["a: one\n", "a: two\n"],
      "merge3" => ["a: one\nb: two\n", "a: ours\nb: two\n", "a: one\nb: theirs\n"]}
    cases.each do |operation, texts|
      request = common_request(operation, texts)
      expect(request.sources.length).to eq(texts.length)
      expect(request.operation).to be_a(described_class::OperationPolicy)
      expect(request.operation.public_send(operation)).not_to be_nil
      request = common_request(operation, texts, policy: request.operation)
      result = described_class.execute_operation(request, merge_limits)
      expect(result.ok).to be(true)
      expect(result.request_id).to eq(request.request_id)
      case operation
      when "analyze" then expect(result.analysis).to be_a(described_class::ResultAnalysis)
      when "diff2" then expect(result.changes).not_to be_empty
      else
        expect(result.output).to eq("a: ours\nb: theirs\n")
        expect(result.verification.output_reparsed).to be(true)
      end
    end
    expect(host.calls).to eq(4)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "classifies parser result limits as resource limits in common operation diagnostics" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    limits = described_class::ParseLimits.new(max_batch_items: 3, max_input_bytes: 10000,
      max_nodes: 1, max_diagnostics: 20)
    {"analyze" => 1, "diff2" => 2, "merge3" => 3}.each do |operation, count|
      result = described_class.execute_operation(common_request(operation, Array.new(count, "a: one\nb: two\n")), limits)
      expect(result.ok).to be(false)
      expect(result.output).to be_nil
      expect(result.analysis).to be_nil
      expect(result.verification.classification_reached).to be(false)
      expect(result.diagnostics.length).to eq(1)
      diagnostic = result.diagnostics.first.canonical
      expect(diagnostic.code).to eq("resource.limit")
      expect(diagnostic.category).to eq(:resource_limit)
      expect(diagnostic.blocking).to be(true)
      expect(diagnostic.origin.backend_id).to eq("ruby.typed.psych")
    end
    expect(host.calls).to eq(3)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "rejects wrong typed policy payloads and cancelled common operations before callbacks" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    request = common_request("analyze", ["a: one\n"])
    control = described_class.create_operation_control
    control.cancel
    expect { described_class.execute_operation_controlled(request, merge_limits, control) }
      .to raise_error(RuntimeError, /execution.cancelled/)
    expect { described_class::OperationPolicy.from_analyze(described_class::DiffPolicy.new(extra: {})) }.to raise_error(TypeError)
    expect(host.calls).to eq(0)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "round-trips every policy variant without losing optional values or typed payloads" do
    extra = {"future" => "[null,false,7]"}
    policies = {
      "analyze" => described_class::AnalyzePolicy.new(comments: false, ownership: true, extra: extra),
      "diff2" => described_class::DiffPolicy.new(equivalence: [], source_preservation_evidence: false, extra: extra),
      "merge2" => described_class::DirectionalMergePolicy.new(directional_merge: "template-into-current", render_policy: "source-preserving", extra: extra),
      "merge3" => described_class::ThreeWayMergePolicy.new(render_policy: "source-preserving", labels: {"ours" => "local"}, conflict_marker_size: 9, extra: extra)
    }
    policies.each do |name, payload|
      policy = described_class::OperationPolicy.public_send("from_#{name}", payload)
      source_count = {"analyze" => 1, "diff2" => 2, "merge2" => 2, "merge3" => 3}.fetch(name)
      request = common_request(name, Array.new(source_count, "a: one\n"), policy: policy)
      restored = request.operation
      expect(restored).to be_a(described_class::OperationPolicy)
      policies.each_key { |other| expect(restored.public_send(other)).to be_nil unless other == name }
      value = restored.public_send(name)
      expect(value).to be_a(payload.class)
      expect(value.extra).to eq(extra)
      case name
      when "analyze"
        expect(value.comments).to be(false)
        expect(value.ownership).to be(true)
        expect(value.tokens).to be_nil
      when "diff2"
        expect(value.equivalence).to eq([])
        expect(value.source_preservation_evidence).to be(false)
      when "merge2"
        expect(value.directional_merge).to eq("template-into-current")
      else
        expect(value.labels).to eq({"ours" => "local"})
        expect(value.conflict_marker_size).to eq(9)
      end
    end
    expect { described_class::OperationPolicy.new }.to raise_error(TypeError)
  end

  it "preserves canonical record variants and payloads across the installed native boundary" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    result = described_class.execute_operation(common_request("merge3", ["a: one\n", "a: ours\n", "a: theirs\n"]), merge_limits)
    expect(result.ok).to be(false)
    expect(result.output).to be_nil
    record = result.conflicts.fetch(0)
    expect(record).to be_a(described_class::ConflictRecord)
    expect(record.migration).to be_nil
    conflict = record.canonical
    expect(conflict).to be_a(described_class::PortableConflict)
    expect(conflict.code).to eq("merge.edit_edit")
    expect(conflict.roles.map(&:to_s)).to eq(%w[base ours theirs])
    restored = described_class::ConflictRecord.from_canonical(conflict)
    expect(restored.canonical.id).to eq(conflict.id)
    expect(restored.canonical.decision_ids).to eq(conflict.decision_ids)
    expect(restored.canonical.alternatives.map(&:source_id)).to eq(conflict.alternatives.map(&:source_id))
    diagnostic = result.diagnostics.fetch(0)
    expect(diagnostic).to be_a(described_class::DiagnosticRecord)
    expect(diagnostic.migration).to be_nil
    expect(diagnostic.canonical.code).to eq("merge.edit_edit")
    restored_diagnostic = described_class::DiagnosticRecord.from_canonical(diagnostic.canonical)
    expect(restored_diagnostic.canonical.id).to eq(diagnostic.canonical.id)
    expect(restored_diagnostic.canonical.blocking).to be(true)
    legacy = described_class::ResultDiagnostic.new(id: "legacy", severity: "error", category: "unsupported",
      code: "unsupported", message: "legacy", blocking: true, metadata: {}, extra: {})
    migration = described_class::DiagnosticRecord.from_migration(legacy)
    expect(migration.canonical).to be_nil
    expect(migration.migration.id).to eq("legacy")
    expect { described_class::DiagnosticRecord.from_canonical(legacy) }.to raise_error(TypeError)
    expect { described_class::DiagnosticRecord.new }.to raise_error(TypeError)
    expect { described_class::ConflictRecord.new }.to raise_error(TypeError)
    legacy_conflict = described_class::ResultConflict.new(id: "legacy-conflict", category: "structural", roles: [],
      source_regions: [], localized: false, resolution: "unresolved", metadata: {}, extra: {})
    migration_conflict = described_class::ConflictRecord.from_migration(legacy_conflict)
    expect(migration_conflict.canonical).to be_nil
    expect(migration_conflict.migration.id).to eq("legacy-conflict")
    expect { described_class::ConflictRecord.from_canonical(legacy_conflict) }.to raise_error(TypeError)
    rejected = described_class.execute_operation(common_request("analyze", ["a: [\n"]), merge_limits)
    expect(rejected.ok).to be(false)
    parse_diagnostic = rejected.diagnostics.fetch(0).canonical
    expect(parse_diagnostic.code).to eq("parse.rejected")
    expect(parse_diagnostic.source_refs.fetch(0).role.to_s).to eq("source")
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  def diff_request(sources, roles: %w[before after])
    described_class::NativeDiffRequest.new(request_id: "diff-ruby", profile_id: "kernel.yaml.native_mapping.v1",
      parses: merge_requests(sources, roles: roles))
  end

  it "classifies native YAML diff in Rust without a merge or output parse" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    result = described_class.diff_native_owners(diff_request(["# header\r\na: one\r\nb: two", "# header\r\na: edited\r\nc: three"]), merge_limits)
    expect(result.ok).to be(true)
    expect(result.request_id).to eq("diff-ruby")
    expect(result.diff.changes.map(&:kind)).to eq(%i[edited deleted added])
    expect(result.diff.changes.map(&:id)).to eq(%w[change-0 change-1 change-2])
    expect(result.diff.changes[0].before.range.start_byte).to eq("# header\r\n".bytesize)
    expect(result.diff.changes[1].after).to be_nil
    expect(result.diff.changes[2].before).to be_nil
    expect(result.input_parses.map { |item| item.parsed.source.role }).to eq(%i[before after])
    expect(result).not_to respond_to(:output)
    expect(host.calls).to eq(1)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "retains diff syntax and unsupported-analysis failures without partial changes" do
    described_class.register_parser_host(TypedPsychHost.new)
    result = described_class.diff_native_owners(diff_request(["a: [\n", "a: two\n"]), merge_limits)
    expect(result.ok).to be(false)
    expect(result.diff).to be_nil
    expect(result.input_parses.first.parsed.source.role).to eq(:before)
    expect(result.input_parses.first.parsed.ok).to be(false)
    result = described_class.diff_native_owners(diff_request(["a: one\n", "a: one\na: two\n"]), merge_limits)
    expect(result.ok).to be(false)
    expect(result.diff).to be_nil
    expect(result.analysis_rejections.map(&:source_role)).to eq([:after])
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "rejects diff role substitution and cancellation before callbacks" do
    host = TypedPsychHost.new
    described_class.register_parser_host(host)
    expect { described_class.diff_native_owners(diff_request(["a: one", "a: two"], roles: %w[incoming current]), merge_limits) }
      .to raise_error(RuntimeError, /invalid_diff_inputs/)
    control = described_class.create_operation_control
    control.cancel
    expect { described_class.diff_native_owners_controlled(diff_request(["a: one", "a: two"]), merge_limits, control) }
      .to raise_error(RuntimeError, /execution.cancelled/)
    expect(host.calls).to eq(0)
  ensure
    described_class.unregister_parser_host("ruby.typed.psych")
  end

  it "plans directory content through the typed API without writing it" do
    FileUtils.mkdir_p("tmp")
    Dir.mktmpdir("typed-template-", "tmp") do |root|
      template = File.join(root, "template")
      destination = File.join(root, "destination")
      FileUtils.mkdir_p([template, destination])
      File.binwrite(File.join(template, "README.md"), "# é\r\n")
      options = described_class::TemplateSessionOptions.new(mode: "plan", template_root: template, destination_root: destination,
        context: described_class::TemplateDestinationContext.new(project_name: "widget"), default_strategy: "raw_copy",
        overrides: [], replacements: {}, allowed_families: nil, config: nil)
      report = described_class.plan_template_directory(options)
      expect(report.runner_report.plan_report.summary.create).to eq(1)
      expect(report.runner_report.preview.result_files.fetch("README.md")).to eq("# é\r\n")
      expect(Dir.children(destination)).to be_empty
      expect(File.binread(File.join(template, "README.md"))).to eq("# é\r\n".b)
    end
  end

  it "reports typed template options and profiles without applying templates" do
    options = described_class::TemplateSessionOptions.new(mode: "plan", template_root: "", destination_root: "",
      context: described_class::TemplateDestinationContext.new(project_name: nil), default_strategy: "merge",
      overrides: [], replacements: {}, allowed_families: nil, config: nil)
    report = described_class.report_template_options(options)
    expect(report.ready).to be(false)
    expect(report.diagnostics.map(&:reason)).to eq(%w[missing_destination_root missing_template_root])
    report = described_class.report_template_profile(described_class::TemplateProfileRequest.new(
      profile_name: "missing", profiles: {}, options: options))
    expect(report.diagnostics.map(&:reason)).to include("missing_profile")
    profile = described_class::DirectorySessionProfile.new(mode: "apply",
      context: described_class::TemplateDestinationContext.new(project_name: "widget"), default_strategy: "raw_copy",
      overrides: [], replacements: { "NAME" => "widget" }, allowed_families: ["markdown"], config: nil)
    configured = described_class::TemplateSessionOptions.new(mode: "plan", template_root: "/not-read/templates", destination_root: "/not-read/destination",
      context: described_class::TemplateDestinationContext.new(project_name: nil), default_strategy: "merge",
      overrides: [], replacements: {}, allowed_families: nil, config: nil)
    report = described_class.report_template_profile(described_class::TemplateProfileRequest.new(
      profile_name: "known", profiles: { "known" => profile }, options: configured))
    expect(report.ready).to be(true)
    expect(report.resolved_options.context.project_name).to eq("widget")
    expect(report.resolved_options.replacements).to eq("NAME" => "widget")
    expect(report.mode).to eq(:apply)
    options = described_class::TemplateSessionOptions.new(mode: "apply", template_root: "", destination_root: "",
      context: described_class::TemplateDestinationContext.new(project_name: nil), default_strategy: "merge",
      overrides: [], replacements: {}, allowed_families: nil, config: nil)
    expect { described_class.plan_template_directory(options) }.to raise_error(RuntimeError, /template.request.invalid/)
  end

  it "reports structural operations as typed ordered profiles without selecting source" do
    boundary = described_class.report_structural_boundary
    expect(boundary.package).to eq("ast-crispr")
    expect(boundary.metadata.source).to eq("legacy_crispr_reference")
    expect(boundary.implementations.map(&:language)).to eq(%w[go ruby rust typescript])
    expect(boundary.relationship.ast_merge).not_to be_empty
    limit = described_class.report_structural_limit(described_class::CrisprLimitRequest.new(constraints: nil, counts: [0, 1, 2]))
    expect(limit.description).to eq("== 1")
    expect(limit.allowed).to eq([false, true, false])
    limit = described_class.report_structural_limit(described_class::CrisprLimitRequest.new(constraints: [
      described_class::CrisprLimitConstraint.new(operator: "at_least", value: 1),
      described_class::CrisprLimitConstraint.new(operator: "at_most", value: 2)
    ], counts: [0, 1, 2, 3]))
    expect(limit.description).to eq(">= 1 and <= 2")
    expect(limit.allowed).to eq([false, true, true, false])
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
