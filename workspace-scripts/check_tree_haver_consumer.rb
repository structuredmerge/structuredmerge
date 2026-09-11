#!/usr/bin/env ruby
# frozen_string_literal: true

# Consumer-style probe: the caller installs both gems into one fresh GEM_HOME.
# No STRUCTUREDMERGE_RUST_DEV, path gem, or Ruby load-path override is used.
require "json"
require "tree_haver"

abort "TreeHaver was not loaded from an installed gem" unless Gem.loaded_specs.key?("tree_haver")

TreeHaver.with_backend(:rust_tslp) do
  abort "Rust host was not loaded from an installed gem" unless TreeHaver::Backends::RustTslp.available?
  TreeHaver::GrammarFinder.new(:json).register!(raise_on_missing: true)
  tree = TreeHaver.parser_for(:json).parse("{\n  \"answer\": 42\n}\n")
  root = tree.root_node
  pair = root.children.first.children.find { |child| child.type == "pair" }

  abort "unexpected TreeHaver root" unless root.type == "document"
  abort "unexpected TreeHaver source" unless root.text == "{\n  \"answer\": 42\n}\n"
  abort "JSON pair was not exposed" unless pair&.text == '"answer": 42'
  abort "Rust provenance was not exposed" unless tree.provenance.dig("backend_ref", "id")
end

puts "packaged Rust host + TreeHaver consumer contract verified"
