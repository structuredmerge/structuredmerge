#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "rbconfig"

ROOT = File.expand_path("..", __dir__)
PACKAGE_ROOT = File.join(ROOT, "packages", "ruby")
CONTRACT_PATH = File.join(ROOT, "contracts", "ruby-api-v1.json")
EXTENSION_NAME = "structuredmerge_host_prototype_core_rb"

$LOAD_PATH.unshift(File.join(PACKAGE_ROOT, "lib")) unless ENV["STRUCTUREDMERGE_API_USE_INSTALLED"] == "true"
require "structuredmerge_host_prototype"
require "structuredmerge_host_prototype/workflow_provider"

def method_arities(owner, singleton:)
  names = singleton ? owner.singleton_methods(false) : owner.public_instance_methods(false)
  names.sort.to_h do |name|
    method = singleton ? owner.method(name) : owner.instance_method(name)
    [name.to_s, method.arity]
  end
end

actual = {
  "schema" => "structuredmerge.ruby-api/v1",
  "facade_constants" => StructuredmergeHostPrototype.constants(false).map(&:to_s).sort,
  "facade_methods" => StructuredmergeHostPrototype.singleton_methods(false).map(&:to_s).sort,
  "native_constants" => StructuredmergeHostPrototypeCore.constants(false).map(&:to_s).sort,
  "native_method_arities" => method_arities(StructuredmergeHostPrototypeCore, singleton: true),
  "workflow_provider_method_arities" => method_arities(
    StructuredmergeHostPrototype::WorkflowProvider,
    singleton: false
  )
}
expected = JSON.parse(File.read(CONTRACT_PATH))
unless actual == expected
  warn "Ruby API contract mismatch"
  warn "expected:\n#{JSON.pretty_generate(expected)}"
  warn "actual:\n#{JSON.pretty_generate(actual)}"
  exit 1
end

native_feature = $LOADED_FEATURES.reverse.find do |feature|
  File.basename(feature).start_with?("#{EXTENSION_NAME}.")
end
abort "could not identify the loaded #{EXTENSION_NAME} extension" unless native_feature

expected_basename = "#{EXTENSION_NAME}.#{RbConfig::CONFIG.fetch("DLEXT")}"
actual_basename = File.basename(native_feature)
abort "native ABI filename mismatch: expected #{expected_basename}, loaded #{actual_basename}" unless actual_basename == expected_basename

puts "Ruby API/native ABI contract verified"
