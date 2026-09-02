# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"
require "digest"
require "json"

module HostPrototypeFixtures
  class IdentityWorkflowHost
    attr_reader :requests, :shutdown_count

    def initialize
      @requests = []
      @shutdown_count = 0
    end

    def descriptor
      '{"id":"ruby.identity"}'
    end

    def version
      "test"
    end

    def execute_batch(request)
      @requests << request
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
    provider = HostPrototypeFixtures::IdentityParserHost.new
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
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
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

  it "fails closed when a provider name is registered twice" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
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
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    described_class.unregister_workflow_host("ruby.identity")

    expect(provider.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
    expect do
      described_class.execute_identity("ruby.identity", [])
    end.to raise_error(RuntimeError, /provider not registered/)
  end

  it "contains Ruby callback exceptions" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
    def provider.execute_batch(_request)
      raise "callback exploded"
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.failure")

    expect do
      described_class.execute_identity("ruby.failure", [])
    end.to raise_error(RuntimeError, /Ruby method 'execute_batch' failed: callback exploded/)
  end

  it "rejects malformed callback return values" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
    def provider.execute_batch(_request)
      Object.new
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.malformed")

    expect do
      described_class.execute_identity("ruby.malformed", [])
    end.to raise_error(RuntimeError, /Failed to convert Ruby 'execute_batch' return value/)
  end

  it "does not retain the registry lock while invoking Ruby" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
    def provider.execute_batch(request)
      raise "provider disappeared during callback" unless
        StructuredmergeHostPrototypeCore.registered_workflow_hosts.include?("ruby.reentrant")

      request
    end
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.reentrant")

    expect(described_class.execute_identity("ruby.reentrant", [0, 255])).to eq([0, 255])
  end

  it "removes a provider even when shutdown fails" do
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
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
    first = HostPrototypeFixtures::IdentityWorkflowHost.new
    second = HostPrototypeFixtures::IdentityWorkflowHost.new
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
    provider = HostPrototypeFixtures::IdentityWorkflowHost.new
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
end
