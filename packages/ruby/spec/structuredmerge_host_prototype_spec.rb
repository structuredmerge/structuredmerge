# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"

RSpec.describe StructuredmergeHostPrototype do
  class IdentityWorkflowHost
    attr_reader :requests, :shutdown_count

    def initialize
      @requests = []
      @shutdown_count = 0
    end

    def descriptor
      '{"id":"ruby.identity"}'
    end

    def execute_batch(request)
      @requests << request
      request
    end

    def shutdown
      @shutdown_count += 1
    end
  end

  before do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
  end

  after do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
  end

  it "loads the native extension and exposes a version" do
    expect(described_class::VERSION).to match(/\A\d+\.\d+\.\d+/)
  end

  it "round trips arbitrary bytes through a Ruby workflow host" do
    provider = IdentityWorkflowHost.new
    payload = "\x00\xff\r\nA\x00".b
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    result = described_class.execute_identity("ruby.identity", payload.bytes)

    expect(result.pack("C*")).to eq(payload)
    expect(provider.requests).to contain_exactly(payload)
    expect(provider.requests.first.encoding).to eq(Encoding::ASCII_8BIT)
    expect(described_class.registered_workflow_hosts).to eq(["ruby.identity"])
  end

  it "fails closed when a provider name is registered twice" do
    provider = IdentityWorkflowHost.new
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    expect {
      StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")
    }.to raise_error(RuntimeError, /provider already registered/)
  end

  it "rejects providers that omit required workflow methods" do
    expect {
      StructuredmergeHostPrototypeCore.register_workflow_host(Object.new, "incomplete")
    }.to raise_error(RuntimeError, /missing required method: descriptor/i)
  end

  it "shuts down and removes an unregistered provider" do
    provider = IdentityWorkflowHost.new
    StructuredmergeHostPrototypeCore.register_workflow_host(provider, "ruby.identity")

    described_class.unregister_workflow_host("ruby.identity")

    expect(provider.shutdown_count).to eq(1)
    expect(described_class.registered_workflow_hosts).to be_empty
    expect {
      described_class.execute_identity("ruby.identity", [])
    }.to raise_error(RuntimeError, /provider not registered/)
  end
end
