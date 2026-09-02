# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"
require "digest"
require "json"
require "weakref"

module HostPrototypeFixtures
  class IdentityWorkflowHost
    attr_reader :callback_thread_ids, :detached_requests, :requests, :shutdown_count, :typed_requests

    def initialize(id = "ruby.identity")
      @id = id
      @requests = []
      @callback_thread_ids = []
      @typed_requests = []
      @detached_requests = []
      @shutdown_count = 0
    end

    def descriptor
      JSON.generate("id" => @id)
    end

    def version
      "test"
    end

    def execute_batch(request)
      @callback_thread_ids << Thread.current.object_id
      @requests << request
      request
    end

    def execute_typed_batch(request, source)
      @typed_requests << [request, source]
      source
    end

    def execute_detached_batch(request)
      envelope = JSON.parse(request)
      blob = envelope.fetch("blob")
      source = File.binread(blob.fetch("path"))
      raise "detached blob length mismatch" unless source.bytesize == blob.fetch("byte_length")
      raise "detached blob digest mismatch" unless
        Digest::SHA256.hexdigest(source) == blob.fetch("sha256")

      envelope.fetch("items").each do |item|
        bytes = source.byteslice(item.fetch("offset"), item.fetch("byte_length"))
        raise "detached segment digest mismatch: #{item.fetch("source_id")}" unless
          Digest::SHA256.hexdigest(bytes) == item.fetch("sha256")
      end
      @detached_requests << [request, envelope, source]
      request
    end

    def shutdown
      @shutdown_count += 1
    end
  end

  class IdentityParserHost < IdentityWorkflowHost
    def probe_batch(request)
      requests << request
      request
    end

    def parse_batch(request)
      requests << request
      request
    end
  end

  class LifecycleWorkflowHost
    attr_reader :events, :shutdown_count

    def initialize(id = nil)
      if defined?(@id)
        @events << :initialize
      else
        @id = id || raise(ArgumentError, "id is required")
        @events = []
        @shutdown_count = 0
      end
    end
    public :initialize

    def version
      @events << :version
      "test"
    end

    def descriptor
      @events << :descriptor
      raise "provider published before initialization" if
        StructuredmergeHostPrototypeCore.registered_workflow_hosts.include?(@id)

      JSON.generate("id" => @id, "capabilities" => ["identity"])
    end

    def execute_batch(request)
      request
    end

    def execute_typed_batch(_request, source)
      source
    end

    def execute_detached_batch(request)
      request
    end

    def shutdown
      @shutdown_count += 1
    end
  end
end

RSpec.describe StructuredmergeHostPrototype do
  def identity_cases
    path = File.expand_path(
      "../../../../fixtures/diagnostics/slice-1033-ruby-generated-host-prototype/identity-corpus.json",
      __dir__
    )
    JSON.parse(File.binread(path)).fetch("cases")
  end

  def materialize_identity_payload(recipe)
    case recipe.fetch("kind")
    when "hex"
      [recipe.fetch("value")].pack("H*")
    when "utf8"
      recipe.fetch("value").encode(Encoding::UTF_8).b
    when "repeat_byte"
      recipe.fetch("byte").chr.b * recipe.fetch("count")
    else
      raise "unsupported identity payload recipe: #{recipe.inspect}"
    end
  end

  def verify_identity_bytes(bytes, test_case)
    expect(bytes.bytesize).to eq(test_case.fetch("byte_length")), test_case.fetch("id")
    expect(Digest::SHA256.hexdigest(bytes)).to eq(test_case.fetch("sha256")), test_case.fetch("id")
  end

  before do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
    StructuredmergeHostPrototypeCore.clear_parser_hosts
  end

  after do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
    StructuredmergeHostPrototypeCore.clear_parser_hosts
  end

  it "round trips the shared byte corpus through a Ruby parser host" do
    provider = HostPrototypeFixtures::IdentityParserHost.new("ruby.parser.identity")
    StructuredmergeHostPrototypeCore.register_parser_host(provider, "ruby.parser.identity")

    identity_cases.each do |test_case|
      payload = materialize_identity_payload(test_case.fetch("payload"))
      verify_identity_bytes(payload, test_case)

      probe = described_class.probe_with_parser("ruby.parser.identity", payload.bytes).pack("C*")
      parse = described_class.parse_with_parser("ruby.parser.identity", payload.bytes).pack("C*")

      verify_identity_bytes(probe, test_case)
      verify_identity_bytes(parse, test_case)
      expect(probe).to eq(payload), test_case.fetch("id")
      expect(parse).to eq(payload), test_case.fetch("id")
    end

    expect(provider.requests).to all(have_attributes(encoding: Encoding::ASCII_8BIT))
    expect(described_class.registered_parser_hosts).to eq(["ruby.parser.identity"])
  end

  it "loads the native extension and exposes a version" do
    expect(described_class::VERSION).to match(/\A\d+\.\d+\.\d+/)
  end

  it "round trips the shared byte corpus through a Ruby workflow host" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.identity")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    identity_cases.each do |test_case|
      payload = materialize_identity_payload(test_case.fetch("payload"))
      verify_identity_bytes(payload, test_case)
      result = described_class.execute_identity("ruby.identity", payload.bytes).pack("C*")

      verify_identity_bytes(result, test_case)
      expect(result).to eq(payload), test_case.fetch("id")
    end

    expect(provider.requests).to all(have_attributes(encoding: Encoding::ASCII_8BIT))
    expect(described_class.registered_workflow_hosts).to eq(["ruby.identity"])
  end

  it "round trips the shared byte corpus with generated typed batch headers" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.typed-identity")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.typed-identity")

    identity_cases.each do |test_case|
      payload = materialize_identity_payload(test_case.fetch("payload"))
      result = described_class.execute_typed_identity(
        "ruby.typed-identity",
        [test_case.fetch("id")],
        [test_case.fetch("byte_length")],
        [test_case.fetch("sha256")],
        payload.bytes
      ).pack("C*")

      verify_identity_bytes(result, test_case)
      expect(result).to eq(payload), test_case.fetch("id")
    end

    provider.typed_requests.zip(identity_cases).each do |(request, source), test_case|
      item = request.items.fetch(0)
      expect(request.schema).to eq("structuredmerge.host-batch/v1")
      expect(item.source_id).to eq(test_case.fetch("id"))
      expect(item.offset).to eq(0)
      expect(item.byte_length).to eq(test_case.fetch("byte_length"))
      expect(item.sha256).to eq(test_case.fetch("sha256"))
      expect(source.encoding).to eq(Encoding::ASCII_8BIT)
      verify_identity_bytes(source, test_case)
    end
  end

  it "preserves distinct IDs and byte ranges in a multi-item typed batch" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.typed-batch")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.typed-batch")
    payloads = ["alpha\r\n".b, "\x00\xffbeta".b]
    source = payloads.join.b
    digests = payloads.map { |payload| Digest::SHA256.hexdigest(payload) }

    result = described_class.execute_typed_identity(
      "ruby.typed-batch",
      %w[source:1 source:2],
      payloads.map(&:bytesize),
      digests,
      source.bytes
    ).pack("C*")

    request, callback_source = provider.typed_requests.fetch(0)
    expect(result).to eq(source)
    expect(callback_source).to eq(source)
    expect(request.items.map(&:source_id)).to eq(%w[source:1 source:2])
    expect(request.items.map(&:offset)).to eq([0, payloads.fetch(0).bytesize])
    expect(request.items.map(&:byte_length)).to eq(payloads.map(&:bytesize))
    expect(request.items.map(&:sha256)).to eq(digests)
  end

  it "rejects invalid typed batch metadata before invoking Ruby" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.typed-invalid")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.typed-invalid")

    expect do
      described_class.execute_typed_identity(
        "ruby.typed-invalid",
        ["source:1"],
        [1],
        ["not-the-digest"],
        [97]
      )
    end.to raise_error(RuntimeError, /digest mismatch for source "source:1"/)
    expect(provider.typed_requests).to be_empty
  end

  it "fails closed when a typed identity callback changes source bytes" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.typed-changed")
    def provider.execute_typed_batch(request, source)
      super
      source.dup << "changed"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.typed-changed")
    payload = "original".b

    expect do
      described_class.execute_typed_identity(
        "ruby.typed-changed",
        ["source:1"],
        [payload.bytesize],
        [Digest::SHA256.hexdigest(payload)],
        payload.bytes
      )
    end.to raise_error(RuntimeError, /typed identity host changed source bytes/)
  end

  it "round trips the shared byte corpus through canonical JSON and detached blobs" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.detached-identity")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.detached-identity")

    identity_cases.each do |test_case|
      payload = materialize_identity_payload(test_case.fetch("payload"))
      result = described_class.execute_detached_identity(
        "ruby.detached-identity",
        [test_case.fetch("id")],
        [test_case.fetch("byte_length")],
        [test_case.fetch("sha256")],
        payload.bytes
      ).pack("C*")

      request, envelope, callback_source = provider.detached_requests.last
      expect(result).to eq(payload), test_case.fetch("id")
      expect(callback_source).to eq(payload), test_case.fetch("id")
      expect(request.encoding).to eq(Encoding::ASCII_8BIT)
      expect(JSON.generate(JSON.parse(request))).to eq(request)
      expect(envelope.fetch("schema")).to eq("structuredmerge.host-batch/v1")
      expect(envelope.fetch("transport")).to eq("detached_local_file")
      expect(envelope.fetch("items").fetch(0).fetch("source_id")).to eq(test_case.fetch("id"))
      expect(File).not_to exist(envelope.fetch("blob").fetch("path"))
    end
  end

  it "detects detached blob mutation and removes the scoped file" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.detached-mutation")
    detached_path = nil
    provider.define_singleton_method(:execute_detached_batch) do |request|
      envelope = JSON.parse(request)
      detached_path = envelope.fetch("blob").fetch("path")
      File.binwrite(detached_path, "mutated")
      request
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.detached-mutation")
    payload = "original".b

    expect do
      described_class.execute_detached_identity(
        "ruby.detached-mutation",
        ["source:1"],
        [payload.bytesize],
        [Digest::SHA256.hexdigest(payload)],
        payload.bytes
      )
    end.to raise_error(RuntimeError, /detached identity host changed source bytes/)
    expect(File).not_to exist(detached_path)
  end

  it "fails closed when a provider name is registered twice" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.identity")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")
    end.to raise_error(RuntimeError, /provider already registered/)
  end

  it "rejects providers that omit required workflow methods" do
    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(Object.new, "incomplete")
    end.to raise_error(RuntimeError, /missing required method: descriptor/i)
  end

  it "shuts down and removes an unregistered provider" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.identity")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    described_class.unregister_workflow_host("ruby.identity")

    expect(provider.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
    expect do
      described_class.execute_identity("ruby.identity", [])
    end.to raise_error(RuntimeError, /provider not registered/)
  end

  it "contains Ruby callback exceptions" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.failure")
    def provider.execute_batch(_request)
      raise "callback exploded"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.failure")

    expect do
      described_class.execute_identity("ruby.failure", [])
    end.to raise_error(RuntimeError, /Ruby method 'execute_batch' failed: callback exploded/)
  end

  it "rejects malformed callback return values" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.malformed")
    def provider.execute_batch(_request)
      Object.new
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.malformed")

    expect do
      described_class.execute_identity("ruby.malformed", [])
    end.to raise_error(RuntimeError, /Failed to convert Ruby 'execute_batch' return value/)
  end

  it "does not retain the registry lock while invoking Ruby" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.reentrant")
    def provider.execute_batch(request)
      raise "provider disappeared during callback" unless
        StructuredmergeHostPrototypeCore.registered_workflow_hosts.include?("ruby.reentrant")

      request
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.reentrant")

    expect(described_class.execute_identity("ruby.reentrant", [0, 255])).to eq([0, 255])
  end

  it "removes a provider even when shutdown fails" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.shutdown-failure")
    def provider.shutdown
      super
      raise "shutdown exploded"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.shutdown-failure")

    expect do
      described_class.unregister_workflow_host("ruby.shutdown-failure")
    end.to raise_error(RuntimeError, /Ruby method 'shutdown' failed: shutdown exploded/)
    expect(provider.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
    expect do
      described_class.unregister_workflow_host("ruby.shutdown-failure")
    end.to raise_error(RuntimeError, /provider not registered/)
    expect(provider.shutdown_count).to eq(1)
  end

  it "clears every provider and aggregates shutdown failures" do
    first = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.first")
    second = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.second")
    def first.shutdown
      super
      raise "first exploded"
    end
    def second.shutdown
      super
      raise "second exploded"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(first, "ruby.first")
    StructuredmergeHostPrototypeCore.register_workflow_host(second, "ruby.second")

    expect do
      described_class.clear_workflow_hosts
    end.to raise_error(RuntimeError, /ruby\.first.*first exploded.*ruby\.second.*second exploded/)
    expect(first.shutdown_count).to eq(1)
    expect(second.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
  end

  it "supports repeated calls entered from Ruby threads" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.thread-entry")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.thread-entry")

    threads = Array.new(4) do |thread_id|
      Thread.new do
        Array.new(25) do |call_id|
          payload = [thread_id, call_id, 0, 255]
          described_class.execute_identity("ruby.thread-entry", payload)
        end
      end
    end

    results = threads.flat_map(&:value)
    expect(results.length).to eq(100)
    expect(results).to include([0, 0, 0, 255], [3, 24, 0, 255])
    expect(provider.requests.length).to eq(100)
  end

  it "dispatches concurrent native Rust workers onto a Ruby runtime thread" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.native-workers")
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.native-workers")
    provider_ref = WeakRef.new(provider)
    provider = nil # rubocop:disable Lint/UselessAssignment -- deliberately release the strong GC root
    GC.start
    expect(provider_ref.weakref_alive?).to be(true)

    caller_thread_id = Thread.current.object_id
    payloads = Array.new(8) { |index| [index, 0, 255, 13, 10] }

    tasks = payloads.map do |payload|
      described_class.start_identity_worker("ruby.native-workers", payload)
    end
    results = {}
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 5
    until results.length == tasks.length
      tasks.each do |task_id|
        next if results.key?(task_id)

        result = described_class.poll_identity_worker(task_id)
        results[task_id] = result unless result.nil?
      end
      raise "native workers timed out" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

      sleep(0.001)
    end

    expect(tasks.map { |task_id| results.fetch(task_id) }).to match_array(payloads)
    expect(provider_ref.callback_thread_ids.length).to eq(payloads.length)
    expect(provider_ref.callback_thread_ids).to all(eq(provider_ref.callback_thread_ids.first))
    expect(provider_ref.callback_thread_ids).not_to include(caller_thread_id)
  end

  it "contains Ruby exceptions raised for native Rust workers" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.native-failure")
    def provider.execute_batch(_request)
      raise "native callback exploded"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.native-failure")
    task_id = described_class.start_identity_worker("ruby.native-failure", [0, 255])
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 5

    loop do
      begin
        result = described_class.poll_identity_worker(task_id)
      rescue RuntimeError => e
        expect(e.message).to include("Ruby method 'execute_batch' failed: native callback exploded")
        break
      end
      raise "native worker returned without its Ruby exception" unless result.nil?
      raise "native worker timed out" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

      sleep(0.001)
    end
  end

  it "validates and initializes a provider before publication" do
    provider = HostPrototypeFixtures::LifecycleWorkflowHost.new("ruby.lifecycle")

    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.lifecycle")

    expect(provider.events).to eq(%i[version descriptor initialize])
    expect(described_class.registered_workflow_hosts).to eq(["ruby.lifecycle"])
  end

  it "rejects descriptor IDs that differ from the registration identity" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.descriptor")

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.registration")
    end.to raise_error(RuntimeError, /descriptor id.*does not match registration name/)
    expect(described_class.registered_workflow_hosts).to be_empty
  end

  it "rejects malformed descriptors and capability lists" do
    malformed = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.malformed-descriptor")
    def malformed.descriptor
      "not JSON"
    end
    excessive = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.excessive-capabilities")
    def excessive.descriptor
      JSON.generate("id" => "ruby.excessive-capabilities", "capabilities" => Array.new(65, "x"))
    end

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(malformed, "ruby.malformed-descriptor")
    end.to raise_error(RuntimeError, /invalid provider descriptor JSON/)
    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(excessive, "ruby.excessive-capabilities")
    end.to raise_error(RuntimeError, /exceeds 64 capabilities/)
    expect(described_class.registered_workflow_hosts).to be_empty
  end

  it "rejects a missing or malformed provider version" do
    missing = Object.new
    def missing.descriptor
      '{"id":"ruby.missing-version"}'
    end
    def missing.execute_batch(request)
      request
    end
    def missing.execute_typed_batch(_request, source)
      source
    end
    def missing.execute_detached_batch(request)
      request
    end
    malformed = HostPrototypeFixtures::IdentityWorkflowHost.new("ruby.malformed-version")
    def malformed.version
      Object.new
    end

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(missing, "ruby.missing-version")
    end.to raise_error(RuntimeError, /Ruby method 'version' failed/)
    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(malformed, "ruby.malformed-version")
    end.to raise_error(RuntimeError, /Failed to convert Ruby 'version' return value/)
    expect(described_class.registered_workflow_hosts).to be_empty
  end

  it "does not reinitialize on duplicate registration" do
    first = HostPrototypeFixtures::LifecycleWorkflowHost.new("ruby.duplicate")
    second = HostPrototypeFixtures::LifecycleWorkflowHost.new("ruby.duplicate")
    StructuredmergeHostPrototypeCore.register_workflow_host(first, "ruby.duplicate")

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(second, "ruby.duplicate")
    end.to raise_error(RuntimeError, /provider already registered/)
    expect(first.events).to eq(%i[version descriptor initialize])
    expect(second.events).to be_empty
  end

  it "cleans up but does not publish after initialization failure" do
    provider = HostPrototypeFixtures::LifecycleWorkflowHost.new("ruby.initialize-failure")
    def provider.initialize
      events << :initialize
      raise "initialize exploded"
    end

    expect do
      StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.initialize-failure")
    end.to raise_error(RuntimeError, /Ruby method 'initialize' failed: initialize exploded/)
    expect(provider.events).to eq(%i[version descriptor initialize])
    expect(provider.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
  end
end
