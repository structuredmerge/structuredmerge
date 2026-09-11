#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

SOURCE = "{\n  \"answer\": 42\n}\n"

def fail_contract(message)
  abort "installed Ruby normalized-parser contract failed: #{message}"
end

unless defined?(StructuredmergeHostPrototype)
  require "structuredmerge_host_prototype"
end

unless StructuredmergeHostPrototype.respond_to?(:parse_normalized_with_tslp)
  fail_contract "parse_normalized_with_tslp is not exported by the installed facade"
end

result = JSON.parse(
  StructuredmergeHostPrototype.parse_normalized_with_tslp("json", SOURCE, "json")
)
fail_contract "result is not successful" unless result.fetch("ok")
fail_contract "source fragments are unavailable" unless result.fetch("source_fragments_available")

nodes = result.fetch("nodes").to_h { |node| [node.fetch("id"), node] }
root = nodes.fetch(result.fetch("root_id"))
fail_contract "root kind changed" unless root.fetch("kind") == "document"
fail_contract "root source fragment changed" unless root.fetch("source_fragment") == SOURCE

pair = nodes.values.find { |node| node.fetch("kind") == "pair" }
fail_contract "JSON pair node was not returned" unless pair
fail_contract "pair source fragment changed" unless pair.fetch("source_fragment") == '"answer": 42'
fail_contract "JSON backend provenance is missing" unless result.dig("backend_capability", "backend_ref", "id")

puts "installed Ruby normalized-parser contract verified"
