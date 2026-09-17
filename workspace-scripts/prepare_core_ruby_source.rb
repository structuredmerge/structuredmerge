#!/usr/bin/env ruby
# frozen_string_literal: true

# Pre-publication source packaging only. Use Alef to prepare a committed snapshot;
# never edit generated manifests in the worktree or resolve release prerequisites.
require "digest"
require "fileutils"
require "json"
require "open3"
require "optparse"
require "rbconfig"
require "rubygems/package"
require "tmpdir"
require_relative "artifact_workspace"

options = {}
parser = OptionParser.new do |opts|
  opts.banner = "usage: prepare_core_ruby_source.rb --alef EXECUTABLE --output NEW_DIRECTORY"
  opts.on("--alef EXECUTABLE") { |value| options[:alef] = File.expand_path(value) }
  opts.on("--output NEW_DIRECTORY") { |value| options[:output] = File.expand_path(value) }
  opts.on("--help") { puts opts; exit }
end
begin
  parser.parse!
rescue OptionParser::ParseError => error
  abort "#{error.message}\n#{parser}"
end
abort parser.to_s unless ARGV.empty? && options.values_at(:alef, :output).all?
abort "Alef executable missing" unless File.file?(options[:alef]) && File.executable?(options[:alef])
abort "output already exists" if File.exist?(options[:output]) || File.symlink?(options[:output])

root = File.expand_path("..", __dir__)
run = lambda do |*command, chdir: root|
  stdout, stderr, status = Open3.capture3(*command, chdir: chdir)
  raise "command failed: #{command.first}\n#{stdout}\n#{stderr}" unless status.success?
  stdout
end
revision = run.call("git", "rev-parse", "HEAD").strip
FileUtils.mkdir_p(File.join(root, "tmp"))
ArtifactWorkspace.open(root: root, prefix: "core-ruby-source-",
  disposable: %w[workspace source.tar package]) do |stage|
  puts "Source preparation report directory: #{stage}"
  workspace = File.join(stage, "workspace")
  FileUtils.mkdir_p(workspace)
  archive_path = File.join(stage, "source.tar")
  run.call("git", "archive", "--format=tar", "--output=#{archive_path}", revision)
  run.call("tar", "-xf", archive_path, "-C", workspace)
  prepare_log = run.call(options[:alef], "publish", "prepare", "--lang", "ruby", "--crate", "structuredmerge-core", chdir: workspace)
  File.write(File.join(stage, "prepare.log"), prepare_log)

  package = File.join(stage, "package")
  FileUtils.mkdir_p(package)
  original = File.join(workspace, "packages/ruby")
  files = %w[
    lib/structuredmerge_core.rb
    lib/structuredmerge_core/native.rb
    lib/structuredmerge_core/version.rb
    ext/structuredmerge_core_rb/native/Cargo.toml
    ext/structuredmerge_core_rb/native/extconf.rb
    ext/structuredmerge_core_rb/src/lib.rs
    sig/types.rbs
  ].to_h { |name| [name, File.join(original, name)] }
  files.merge!(
    "README.md" => File.join(workspace, "crates/structuredmerge-core/README.md"),
    "AGPL-3.0-only.md" => File.join(workspace, "AGPL-3.0-only.md"),
    "PolyForm-Small-Business-1.0.0.md" => File.join(workspace, "PolyForm-Small-Business-1.0.0.md")
  )
  files.each do |name, source|
    destination = File.join(package, name)
    FileUtils.mkdir_p(File.dirname(destination))
    FileUtils.cp(source, destination)
  end

  # Cargo owns the manifest format; use a TOML parser, not text substitutions, to
  # check Alef's prepared dependency tables. Python >=3.11 is a packaging tool only.
  audit = <<~PYTHON
    import json, pathlib, sys, tomllib
    path = pathlib.Path(sys.argv[1])
    manifest = tomllib.loads(path.read_text())
    def dependencies(table):
        for section in ('dependencies', 'build-dependencies', 'dev-dependencies'):
            for name, value in table.get(section, {}).items():
                if isinstance(value, dict) and any(key in value for key in ('path', 'workspace', 'git')):
                    raise SystemExit('non-registry dependency in prepared source: ' + name)
        for target in table.get('target', {}).values():
            dependencies(target)
    dependencies(manifest)
    assert not manifest.get('patch') and not manifest.get('replace')
    assert manifest['dependencies']['structuredmerge-core']['version'] == sys.argv[2]
    print(json.dumps(manifest['dependencies']['structuredmerge-core']))
  PYTHON
  spec = Dir.chdir(original) { Gem::Specification.load("structuredmerge_core.gemspec") }
  raise "unexpected generated gemspec" unless spec&.name == "structuredmerge-core"
  run.call("python", "-c", audit, File.join(package, "ext/structuredmerge_core_rb/native/Cargo.toml"), spec.version.to_s)
  baseline = JSON.parse(File.read(File.join(workspace, "contracts/typed-api/ruby/manifest.json")))
  baseline.fetch("files").each do |name, digest|
    raise "source API differs from reviewed baseline: #{name}" unless Digest::SHA256.file(File.join(package, name)).hexdigest == digest
  end
  spec.files = files.keys.sort
  spec.platform = Gem::Platform::RUBY
  spec.executables = []
  spec.extensions = ["ext/structuredmerge_core_rb/native/extconf.rb"]
  license_expression = spec.license
  spec.licenses = license_expression.split(" OR ")
  spec.metadata["license_expression"] = license_expression
  artifact = Dir.chdir(package) { File.join(package, Gem::Package.build(spec)) }
  contents = Gem::Package.new(artifact).contents.sort
  raise "source archive file mismatch" unless contents == files.keys.sort
  raise "prototype file in source gem" if contents.any? { |name| name.include?("prototype") }

  report = {
    "mode" => "source-package-only", "source_revision" => revision,
    "package" => spec.name, "version" => spec.version.to_s, "platform" => spec.platform.to_s,
    "artifact" => File.basename(artifact), "sha256" => Digest::SHA256.file(artifact).hexdigest,
    "files" => contents, "file_sha256" => files.keys.to_h { |name| [name, Digest::SHA256.file(File.join(package, name)).hexdigest] },
    "alef_executable_sha256" => Digest::SHA256.file(options[:alef]).hexdigest,
    "alef_version" => run.call(options[:alef], "--version").strip,
    "upstream_generation_verified" => false, "registry_resolution" => "not_run",
    "source_installation" => "not_run", "installed_merge_tests" => "not_run",
    "publication_gate" => false, "stage" => stage
  }
  FileUtils.mkdir_p(options[:output])
  FileUtils.cp(artifact, File.join(options[:output], File.basename(artifact)))
  File.write(File.join(options[:output], "core-ruby-source.json"), JSON.pretty_generate(report) + "\n")
  puts JSON.generate(report)
end
