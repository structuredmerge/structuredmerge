#!/usr/bin/env ruby
# frozen_string_literal: true

# Retained regression boundary only. This neither builds a gem nor restores the
# abandoned prototype as a product, dependency of the typed core, or release gate.
require "fileutils"
require "rbconfig"
require "shellwords"
require "tmpdir"

abort "usage: bundle exec ruby build_legacy_ruby_regression.rb" unless ARGV.empty?
root = File.expand_path("..", __dir__)
package = File.join(root, "packages/ruby")
extension = "structuredmerge_host_prototype_core_rb.#{RbConfig::CONFIG.fetch('DLEXT')}"
FileUtils.mkdir_p(File.join(root, "tmp"))
stage = Dir.mktmpdir("legacy-regression-build-", File.join(root, "tmp"))
extconf = File.join(package, "ext/structuredmerge_host_prototype_core_rb/native/extconf.rb")
make = Shellwords.split(ENV.fetch("MAKE", RbConfig::CONFIG["MAKE"] || "make"))
abort "MAKE must name a build command" if make.empty?

# Use the historical rb_sys build configuration out of tree. Its source manifest
# stays anchored to extconf.rb; intermediate files stay in this fresh directory.
system(RbConfig.ruby, extconf, chdir: stage, exception: true)
system(*make, "RB_SYS_CARGO_TARGET_DIR=#{File.join(root, 'target')}",
  chdir: stage, exception: true)
built = File.join(stage, extension)
abort "regression build did not produce #{extension}" unless File.file?(built)
FileUtils.cp(built, File.join(package, "lib", extension))
puts "Built checkout-only legacy regression extension for Ruby #{RUBY_VERSION}: #{built}"
