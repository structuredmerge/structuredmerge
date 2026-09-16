#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "rubygems/package"
require "tmpdir"

root = File.expand_path("..", __dir__)
input = File.expand_path(ARGV.fetch(0))
gems = File.directory?(input) ? Dir.glob(File.join(input, "*.gem")).sort : [input]
abort "no gems found in #{input}" if gems.empty?
scratch = File.join(root, "tmp")
FileUtils.mkdir_p(scratch)

gems.each do |path|
  package = Gem::Package.new(path)
  platform = package.spec.platform.to_s
  # Windows Ruby extensions legitimately use the Ruby import library. This
  # guard addresses Linux/macOS portability to both static and shared Rubies.
  next puts("Windows linkage requires the UCRT installed-runtime gate: #{platform}") if platform.include?("mingw")

  command = if platform.include?("linux")
    ["readelf", "-d"]
  elsif platform.include?("darwin")
    ["otool", "-L"]
  else
    abort "unsupported platform for native linkage check: #{platform}"
  end

  Dir.mktmpdir("ruby-linkage-", scratch) do |directory|
    package.extract_files(directory)
    extensions = Dir.glob(File.join(directory, "lib", "**", "*.{so,bundle,dylib}"))
    abort "no packaged native extension in #{path}" if extensions.empty?
    extensions.each do |extension|
      output, status = Open3.capture2e(*command, extension)
      abort "linkage inspection failed: #{output}" unless status.success?
      # Inspect dependency records, never source syntax or parser ownership.
      dependencies = platform.include?("linux") ? output.lines.select { |line| line.include?("(NEEDED)") } : output.lines.drop(1)
      abort "#{path} requires libruby: #{dependencies.join}" if dependencies.any? { |line| line.downcase.include?("libruby") }
    end
    puts "#{File.basename(path)}: #{extensions.length} native extension(s), no libruby dependency"
  end
end
