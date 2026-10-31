# frozen_string_literal: true

require "rubygems"

# Assertions shared by the post-release installation gate and its unit tests.
module CoreRubyRegistryInstallContract
  class VerificationError < StandardError; end

  module_function

  def verify!(spec:, expected_version:, gem_home:, native_paths:)
    raise VerificationError, "structuredmerge-core is not installed" unless spec
    raise VerificationError, "unexpected installed gem name" unless spec.name == "structuredmerge-core"
    unless Gem::Version.new(spec.version.to_s) == Gem::Version.new(expected_version.to_s)
      raise VerificationError, "wrong installed version: #{spec.version}"
    end

    installed_root = File.realpath(gem_home)
    gem_path = File.realpath(spec.full_gem_path)
    raise VerificationError, "core gem escaped isolated GEM_HOME" unless nested_path?(installed_root, gem_path)

    paths = Array(native_paths)
    raise VerificationError, "native extension did not load from installed core gem" if paths.empty?
    real_paths = paths.map { |path| File.realpath(path) }
    unless real_paths.all? { |path| nested_path?(gem_path, path) }
      raise VerificationError, "native extension escaped installed core gem"
    end

    { version: spec.version.to_s, gem_path: gem_path, native_paths: real_paths }
  rescue Errno::ENOENT => error
    raise VerificationError, "installed package path is missing: #{error.message}"
  end

  def nested_path?(parent, path)
    path.start_with?(parent + File::SEPARATOR)
  end
  private_class_method :nested_path?
end
