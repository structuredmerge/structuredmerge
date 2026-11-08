# frozen_string_literal: true

require "version_gem"

require_relative "version"

StructuredmergeCore::Version.class_eval do
  extend(VersionGem::Basic)
end
