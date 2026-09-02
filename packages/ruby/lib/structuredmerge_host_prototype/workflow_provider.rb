# frozen_string_literal: true

require "digest"
require "json"

module StructuredmergeHostPrototype
  # Adapts a Ruby StructuredMerge provider to Alef's coarse WorkflowHost bridge.
  # rubocop:disable-next Metrics/AbcSize, Metrics/ClassLength, Metrics/MethodLength -- validation and dispatch form one transport boundary
  class WorkflowProvider
    REQUEST_SCHEMA = "structuredmerge.operation-request/v1"
    RESULT_SCHEMA = "https://structuredmerge.org/schemas/provider-result/v1.json"
    REQUEST_SEGMENT_ID = "structuredmerge.operation-request"
    SOURCE_ARGUMENTS = {
      "analyze" => {"source" => :source},
      "diff2" => {"before" => :before_source, "after" => :after_source},
      "merge2" => {"incoming" => :incoming_source, "current" => :current_source},
      "merge3" => {"base" => :base_source, "ours" => :ours_source, "theirs" => :theirs_source}
    }.freeze

    class Error < StandardError; end

    attr_reader :host_id, :provider

    def initialize(host_id:, provider:)
      @host_id = host_id.to_s
      @provider = provider
      raise ArgumentError, "host_id cannot be empty" if @host_id.empty?
    end

    def version
      provider.package_version.to_s
    end

    def descriptor
      operations = Array(value(provider.capabilities, :operations)).map(&:to_s)
      JSON.generate(
        "id" => host_id,
        "implementation" => "ruby-structuredmerge-provider",
        "capabilities" => ["typed-source-segments", "opaque-extensions", *operations.map { |item| "operation:#{item}" }],
        "provider" => {
          "provider_id" => provider.provider_id,
          "family" => provider.family,
          "capabilities" => provider.capabilities
        }
      )
    end

    def execute_batch(_request)
      raise Error, "StructuredMerge workflow providers require typed source segments"
    end

    def execute_cancellable_batch(_task_id, request)
      execute_batch(request)
    end

    def execute_async_batch(request)
      execute_batch(request)
    end

    def execute_typed_batch(batch, source)
      segments = materialize_segments(batch, source)
      operation_request = parse_operation_request(segments.delete(REQUEST_SEGMENT_ID))
      operation = operation_request.fetch("operation")
      provider_request = build_provider_request(operation_request, operation, segments)
      result = provider.public_send(operation, provider_request)
      encode_result(operation_request, operation, result)
    end

    def execute_detached_batch(_request)
      raise Error, "StructuredMerge workflow providers require an operation envelope"
    end

    private

    def materialize_segments(batch, source)
      raise Error, "unsupported host batch schema: #{batch.schema.inspect}" unless batch.schema == "structuredmerge.host-batch/v1"

      batch.items.each_with_object({}) do |item, segments|
        raise Error, "duplicate host source segment: #{item.source_id}" if segments.key?(item.source_id)

        bytes = source.byteslice(item.offset, item.byte_length)
        raise Error, "host source segment is outside the source buffer: #{item.source_id}" unless bytes && bytes.bytesize == item.byte_length
        raise Error, "host source segment digest mismatch: #{item.source_id}" unless Digest::SHA256.hexdigest(bytes) == item.sha256

        segments[item.source_id] = bytes
      end
    end

    def parse_operation_request(bytes)
      raise Error, "typed batch is missing #{REQUEST_SEGMENT_ID}" unless bytes

      request = JSON.parse(bytes)
      raise Error, "unsupported operation request schema: #{request["schema"].inspect}" unless request["schema"] == REQUEST_SCHEMA
      request
    rescue JSON::ParserError => e
      raise Error, "invalid operation request JSON: #{e.message}"
    end

    def build_provider_request(operation_request, operation, segments)
      source_arguments = SOURCE_ARGUMENTS.fetch(operation) do
        raise Error, "unsupported workflow operation: #{operation.inspect}"
      end
      selection = operation_request.fetch("provider_selection")
      validate_provider_selection(selection, operation)
      parser_selection = operation_request.fetch("parser_selection")
      validate_parser_selection(parser_selection)
      sources = operation_request.fetch("sources")

      {
        provider_id: selection.fetch("provider_id"),
        family: selection.fetch("family"),
        dialect: selection.fetch("dialect"),
        profile_id: selection.fetch("profile_id"),
        backend: parser_selection.fetch("backend"),
        metadata: operation_request.fetch("metadata", {}),
        extensions: operation_request.fetch("extensions", [])
      }.merge(source_arguments.to_h do |role, argument|
        descriptor = sources.fetch(role)
        [argument, materialize_source(descriptor, segments)]
      end)
    rescue KeyError => e
      raise Error, "incomplete operation request: #{e.message}"
    end

    def validate_provider_selection(selection, operation)
      raise Error, "operation requested a different provider" unless selection.fetch("provider_id").to_s == provider.provider_id.to_s
      raise Error, "operation requested a different provider family" unless selection.fetch("family").to_s == provider.family.to_s
      return if Array(value(provider.capabilities, :operations)).map(&:to_s).include?(operation)

      raise Error, "provider does not support operation: #{operation}"
    end

    def validate_parser_selection(selection)
      backend = selection.fetch("backend").to_s
      supported = Array(value(provider.capabilities, :backends)).map(&:to_s)
      raise Error, "provider does not support parser backend: #{backend}" unless supported.include?(backend)
    end

    def materialize_source(descriptor, segments)
      source_id = descriptor.fetch("source_id")
      bytes = segments.fetch(source_id) { raise Error, "missing source segment: #{source_id}" }
      raise Error, "source descriptor length mismatch: #{source_id}" unless bytes.bytesize == descriptor.fetch("byte_length")
      raise Error, "source descriptor digest mismatch: #{source_id}" unless Digest::SHA256.hexdigest(bytes) == descriptor.fetch("sha256")

      encoding = descriptor.fetch("encoding", "utf-8")
      return bytes unless encoding.downcase == "utf-8"

      bytes.dup.force_encoding(Encoding::UTF_8).tap do |source|
        raise Error, "source is not valid UTF-8: #{source_id}" unless source.valid_encoding?
      end
    end

    def encode_result(operation_request, operation, result)
      raise Error, "workflow provider returned #{result.class}, expected Hash" unless result.is_a?(Hash)
      raise Error, "workflow provider returned an unsupported result schema" unless value(result, :schema).to_s == RESULT_SCHEMA
      raise Error, "workflow provider returned a different operation" unless value(result, :operation).to_s == operation

      forwarded_extensions = Array(operation_request.fetch("extensions", []))
      provider_extensions = Array(value(result, :extensions))
      response = result.merge(
        request_id: operation_request.fetch("request_id"),
        extensions: forwarded_extensions + provider_extensions
      )
      JSON.generate(response)
    rescue KeyError => e
      raise Error, "incomplete operation request: #{e.message}"
    end

    def value(hash, key)
      hash.key?(key) ? hash[key] : hash[key.to_s]
    end
  end
end
