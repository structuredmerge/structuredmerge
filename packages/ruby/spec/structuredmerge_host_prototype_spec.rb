# frozen_string_literal: true

require_relative "../lib/structuredmerge_host_prototype"
require "digest"
require "json"

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

  before do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
    StructuredmergeHostPrototypeCore.clear_parser_hosts
  end

  after do
    StructuredmergeHostPrototypeCore.clear_workflow_hosts
    StructuredmergeHostPrototypeCore.clear_parser_hosts
  end


  it "round trips the shared byte corpus through a Ruby parser host" do
    provider = IdentityParserHost.new
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
    provider = IdentityWorkflowHost.new
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
