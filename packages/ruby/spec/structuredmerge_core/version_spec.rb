# frozen_string_literal: true

require "anonymous_loader"
require "structuredmerge_core"
require "version_gem/rspec"

RSpec.describe StructuredmergeCore::Version do
  it_behaves_like("a Version module", described_class)

  it "executes the version file for coverage without redefining constants" do
    dir = File.expand_path("../../lib/structuredmerge_core", __dir__)
    paths = %w[version.rb version_gem.rb].map { |file| File.join(dir, file) }.select { |path| File.file?(path) }
    anonymous_namespace = AnonymousLoader.load(files: paths)

    expect(anonymous_namespace::StructuredmergeCore::Version::VERSION).to(eq(described_class::VERSION))
  end
end
