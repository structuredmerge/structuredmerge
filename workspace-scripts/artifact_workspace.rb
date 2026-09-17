# frozen_string_literal: true

require "fileutils"
require "json"
require "tmpdir"

# Small evidence stays in the stage; only explicitly named disposable children
# are removed. Never apply this helper to an existing checkout or worktree.
module ArtifactWorkspace
  def self.open(root:, prefix:, disposable:)
    unless disposable.all? { |name| name.is_a?(String) && !name.empty? && File.basename(name) == name && !%w[. ..].include?(name) }
      raise ArgumentError, "disposable paths must be direct child names"
    end
    scratch = File.join(root, "tmp")
    FileUtils.mkdir_p(scratch)
    stage = Dir.mktmpdir(prefix, scratch)
    begin
      yield stage
    rescue Exception => error # Includes Interrupt and abort/SystemExit; always re-raise.
      unless error.is_a?(SystemExit) && error.success?
        File.write(File.join(stage, "failure.json"), JSON.pretty_generate({
          "status" => "failed", "error" => error.class.name,
          "message" => error.message, "publication_gate" => false
        }) + "\n")
      end
      raise
    ensure
      disposable.each do |name|
        path = File.join(stage, name)
        FileUtils.remove_entry_secure(path) if File.exist?(path) || File.symlink?(path)
      end
    end
  end
end
