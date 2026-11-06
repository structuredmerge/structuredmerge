#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "net/http"
require "open3"
require "optparse"
require "shellwords"
require "time"
require "uri"

RUST_REPO = File.expand_path(ENV.fetch("STRUCTUREDMERGE_RUST_REPO", File.expand_path("..", __dir__)))
CRATES_IO_USER_AGENT = "structuredmerge-release-script (https://github.com/structuredmerge/structuredmerge)"
STRUCTUREDMERGE_REPOSITORY = "https://github.com/structuredmerge/structuredmerge"
# Already-published versions retain their original repository metadata.
STRUCTUREDMERGE_PRE_SPLIT_REPOSITORY = "https://github.com/structuredmerge/structuredmerge-rust"
STRUCTUREDMERGE_HOMEPAGE = "https://structuredmerge.org"
STRUCTUREDMERGE_LICENSE = "AGPL-3.0-only OR PolyForm-Small-Business-1.0.0"
PUBLISH_RETRY_BUFFER_SECONDS = 10
# A fresh crate takes time to show up on crates.io and in its sparse index, so
# registry waits back off exponentially instead of polling at a fixed rate. The
# schedule is kettle-dev's release availability probe: up to 15 attempts,
# min(10 * 1.6**(n-1), 120)s between them, overridable through the same
# STRUCTUREDMERGE_REGISTRY_PROBE_* variables the PyPI and RubyGems checks use.
# Checks run immediately (the API is cheap and a crate is usually live at once),
# so there is no initial delay here.
def registry_probe_setting(name, default)
  value = Integer(ENV.fetch(name, ""), 10)
  value.positive? ? value : default
rescue ArgumentError
  default
end

REGISTRY_PROBE_ATTEMPTS = registry_probe_setting("STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS", 15)
REGISTRY_PROBE_INTERVAL = registry_probe_setting("STRUCTUREDMERGE_REGISTRY_PROBE_INTERVAL", 10)
REGISTRY_PROBE_MAX_INTERVAL = registry_probe_setting("STRUCTUREDMERGE_REGISTRY_PROBE_MAX_INTERVAL", 120)
# cargo publish failures that mean "a dependency published moments ago is not in the index yet".
INDEX_LAG_PATTERN = /no matching package named|failed to select a version for the requirement|candidate versions found which didn't match/
DUPLICATE_PUBLISH_PATTERN = /already (?:uploaded|published|exists)|version .+ already exists/i

def registry_probe_delay(attempt)
  [(REGISTRY_PROBE_INTERVAL * 1.6**(attempt - 1)).round, REGISTRY_PROBE_MAX_INTERVAL].min
end

LEGACY_CRATES = [
  ["tree-haver", "tree-haver"],
  ["ast-merge", "ast-merge"],
  ["plain-merge", "plain-merge"],
  ["json-merge", "json-merge"],
  ["yaml-merge", "yaml-merge"],
  ["toml-merge", "structuredmerge-toml-merge"],
  ["markdown-merge", "markdown-merge"],
  ["ruby-merge", "ruby-merge"],
  ["go-merge", "go-merge"],
  ["rust-merge", "rust-merge"],
  ["typescript-merge", "typescript-merge"],
  ["ast-template", "ast-template"],
  ["binary-merge", "structuredmerge-binary-merge"],
  ["zip-merge", "structuredmerge-zip-merge"],
  ["yaml-serde-merge", "yaml-serde-merge"],
  ["pest-toml-merge", "pest-toml-merge"],
  ["pulldown-cmark-merge", "pulldown-cmark-merge"],
].freeze

# Preserve legacy leaf releases while deriving typed roots and their complete
# dependency-first closure from Cargo metadata. Never substitute a hand-maintained
# list for the facade's transitive implementation dependencies.
TYPED_CRATES = JSON.parse(File.read(File.join(RUST_REPO, "contracts/typed-release-inventory.json")))
  .fetch("dependency_first_order").map do |item|
    [File.basename(File.dirname(item.fetch("manifest"))), item.fetch("package")]
  end.freeze
CRATES = (TYPED_CRATES + LEGACY_CRATES.reject { |_, name| TYPED_CRATES.any? { |_, typed| typed == name } }).freeze

options = {
  push: true,
  push_git: false,
  skip_tests: false,
  tag: false,
  preflight_only: false,
}

parser = OptionParser.new do |opts|
  opts.banner = "Usage: release_rust_crates.rb [options]"

  opts.on("--list", "Validate the local inventory and list selected crates without packaging, network, or publication") do
    options[:list] = true
  end

  opts.on("--push", "Publish each crate to crates.io after local validation (default)") do
    options[:push] = true
  end

  opts.on("--no-push", "--dry-run", "Package and validate locally without publishing to crates.io") do
    options[:push] = false
  end

  opts.on("--push-git", "After all releases are live, push the Rust repo branch and release tag") do
    options[:push_git] = true
  end

  opts.on("--tag", "After all selected releases are live, create a shared vVERSION tag") do
    options[:tag] = true
  end

  opts.on("--skip-tests", "Skip workspace cargo test before packaging") do
    options[:skip_tests] = true
  end

  opts.on("--preflight-only", "Validate local package metadata, packages, and crates.io state without publishing") do
    options[:preflight_only] = true
  end

  opts.on("--only CRATE", "Release only one crate package name or crate directory") do |crate_name|
    options[:only] = crate_name
  end

  opts.on("--start-at CRATE", "Start at a crate package name or crate directory in the publish order") do |crate_name|
    options[:start_at] = crate_name
  end
end

parser.parse!

def sh(cmd)
  Shellwords.join(cmd)
end

def run_command(cmd, chdir:)
  puts "\n$ #{sh(cmd)}"
  output = +""
  status = nil
  Open3.popen2e(*cmd, chdir: chdir) do |stdin, outerr, wait_thread|
    stdin.close
    outerr.each do |line|
      print line
      output << line
    end
    status = wait_thread.value
  end

  [status.success?, output]
end

def run!(cmd, chdir:)
  success, = run_command(cmd, chdir: chdir)
  return if success

  raise "Command failed: #{sh(cmd)}"
end

def package_preflight!(crates, allow_dirty: false)
  command = ["cargo", "package"]
  command << "--allow-dirty" if allow_dirty
  command.concat(crates.flat_map { |crate| ["-p", crate[:name]] })
  run!(command, chdir: RUST_REPO)
end

def capture!(cmd, chdir:)
  output, status = Open3.capture2e(*cmd, chdir: chdir)
  raise "Command failed: #{sh(cmd)}\n#{output}" unless status.success?

  output
end

def validate_release_metadata!(crates)
  output = capture!(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], chdir: RUST_REPO)
  packages = JSON.parse(output).fetch("packages").to_h { |package| [package.fetch("name"), package] }
  problems = []

  crates.each do |crate|
    package = packages[crate[:name]]
    unless package
      problems << "#{crate[:name]}: missing from cargo metadata"
      next
    end

    checks = {
      "version" => package["version"] == crate[:version],
      "authors" => package["authors"].is_a?(Array) && package["authors"].any? { |author| !author.strip.empty? },
      "description" => package["description"].is_a?(String) && !package["description"].strip.empty?,
      # Homepage is optional in Cargo's publish metadata. When present, keep
      # it consistent; its absence must never be treated as an ownership clash.
      "homepage" => package["homepage"].nil? || package["homepage"] == STRUCTUREDMERGE_HOMEPAGE,
      "repository" => package["repository"] == STRUCTUREDMERGE_REPOSITORY,
      "license" => package["license"] == STRUCTUREDMERGE_LICENSE,
      "crates.io publication permission" => package["publish"].nil? || package["publish"].include?("crates-io"),
    }
    missing = checks.filter_map { |field, valid| field unless valid }
    problems << "#{crate[:name]}: missing or incorrect #{missing.join(", ")}" unless missing.empty?
  end

  return if problems.empty?

  raise "Rust release metadata preflight failed; no crates were published:\n#{problems.join("\n")}"
end

def validate_crates_io_auth!
  capture!(["cargo", "owner", "--list", "tree-haver"], chdir: RUST_REPO)
rescue RuntimeError => error
  raise "Could not authenticate Cargo with crates.io before release validation; no crates were published.\n#{error.message}"
end

def ensure_clean_git!
  status = capture!(%w[git status --porcelain], chdir: RUST_REPO)
  return if status.empty?

  raise "Rust repo has uncommitted changes; commit before releasing.\n#{status}"
end

def cargo_package_field(cargo_toml, field)
  in_package = false
  File.readlines(cargo_toml).each do |line|
    stripped = line.strip
    if stripped.start_with?("[") && stripped.end_with?("]")
      in_package = stripped == "[package]"
      next
    end
    next unless in_package
    next unless stripped.start_with?("#{field} ")

    match = stripped.match(/\A#{Regexp.escape(field)}\s*=\s*"([^"]+)"/)
    return match[1] if match
  end

  nil
end

def crates_io_get(path)
  uri = URI("https://crates.io/api/v1/#{path}")
  request = Net::HTTP::Get.new(uri)
  request["User-Agent"] = CRATES_IO_USER_AGENT
  request["Accept"] = "application/json"
  response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == "https",
    open_timeout: 10, read_timeout: 30) do |http|
    http.request(request)
  end

  [response, response.body]
end

def structuredmerge_crate_metadata?(crate)
  [STRUCTUREDMERGE_REPOSITORY, STRUCTUREDMERGE_PRE_SPLIT_REPOSITORY].include?(crate["repository"])
end

def crate_released?(name, version)
  path = "crates/#{URI.encode_www_form_component(name)}"
  response, body = crates_io_get(path)
  return false if response.is_a?(Net::HTTPNotFound)

  unless response.is_a?(Net::HTTPSuccess)
    raise "Could not check crates.io release state for #{name} #{version}: HTTP #{response.code}\n#{body}"
  end

  payload = JSON.parse(body)
  crate = payload.fetch("crate")
  unless structuredmerge_crate_metadata?(crate)
    raise "Crates.io package #{name.inspect} has repository metadata outside the StructuredMerge repositories.\n" \
      "repository=#{crate["repository"].inspect}\n" \
      "license=#{crate["license"].inspect}"
  end

  released_version = payload.fetch("versions").find { |entry| entry["num"] == version }
  return false unless released_version

  unless released_version["license"] == STRUCTUREDMERGE_LICENSE
    raise "Crate #{name.inspect} #{version} is live on crates.io, but has unexpected license metadata.\n" \
      "license=#{released_version["license"].inspect}"
  end

  true
end

def publish_rate_limit_retry_at(output)
  match = output.match(/try again after ([A-Z][a-z]{2}, \d{2} [A-Z][a-z]{2} \d{4} \d{2}:\d{2}:\d{2} GMT)/)
  return nil unless match

  Time.httpdate(match[1])
rescue ArgumentError
  nil
end

def sleep_until_retry(retry_at)
  wait_seconds = [(retry_at - Time.now).ceil + PUBLISH_RETRY_BUFFER_SECONDS, PUBLISH_RETRY_BUFFER_SECONDS].max
  puts "\ncrates.io publish rate limit hit; sleeping #{wait_seconds}s until #{retry_at.utc.iso8601} plus buffer."
  sleep(wait_seconds)
end

def publish_crate!(crate)
  index_attempt = 0
  duplicate_attempt = 0
  loop do
    success, output = run_command(["cargo", "publish", "-p", crate[:name]], chdir: RUST_REPO)
    return if success

    # Handle an ambiguous upload or a concurrent publisher: the server may have
    # accepted the crate even when the client did not receive its success reply.
    return if crate_released?(crate[:name], crate[:version])

    retry_at = publish_rate_limit_retry_at(output)
    if retry_at
      sleep_until_retry(retry_at)
      next
    end

    if output.match?(DUPLICATE_PUBLISH_PATTERN) && (duplicate_attempt += 1) < REGISTRY_PROBE_ATTEMPTS
      wait = registry_probe_delay(duplicate_attempt)
      puts "\n#{crate[:name]} #{crate[:version]} was reported as already published; retrying registry confirmation " \
        "in #{wait}s (attempt #{duplicate_attempt}/#{REGISTRY_PROBE_ATTEMPTS})."
      sleep(wait)
      next
    end

    # A dependency published moments ago may not be in the crates.io index yet.
    if output.match?(INDEX_LAG_PATTERN) && (index_attempt += 1) < REGISTRY_PROBE_ATTEMPTS
      wait = registry_probe_delay(index_attempt)
      puts "\n#{crate[:name]} waits on a dependency not yet in the crates.io index; retrying in #{wait}s " \
        "(attempt #{index_attempt}/#{REGISTRY_PROBE_ATTEMPTS})."
      sleep(wait)
      next
    end

    raise "Command failed: #{sh(["cargo", "publish", "-p", crate[:name]])}"
  end
end

def wait_for_crate_release!(crate)
  REGISTRY_PROBE_ATTEMPTS.times do |index|
    attempt = index + 1
    return if crate_released?(crate[:name], crate[:version])

    if attempt == REGISTRY_PROBE_ATTEMPTS
      raise "Release confirmation failed: #{crate[:name]} #{crate[:version]} is not live on crates.io " \
        "after #{REGISTRY_PROBE_ATTEMPTS} attempt(s)."
    end

    wait = registry_probe_delay(attempt)
    puts "Waiting #{wait}s for #{crate[:name]} #{crate[:version]} to become visible on crates.io " \
      "(attempt #{attempt}/#{REGISTRY_PROBE_ATTEMPTS})..."
    sleep(wait)
  end
end

def tag_exists?(tag_name)
  system("git", "rev-parse", "-q", "--verify", "refs/tags/#{tag_name}",
    chdir: RUST_REPO, out: File::NULL, err: File::NULL)
end

def preflight_release_tag!(tag_name, check_remote:)
  head_commit = capture!(%w[git rev-parse HEAD], chdir: RUST_REPO).strip
  local_tag = tag_exists?(tag_name)
  if local_tag
    tag_commit = capture!(["git", "rev-parse", "#{tag_name}^{commit}"], chdir: RUST_REPO).strip
    unless tag_commit == head_commit
      raise "Release tag #{tag_name} points to #{tag_commit}, but HEAD is #{head_commit}; refusing to reuse it."
    end
  end

  return unless check_remote

  refs = capture!(
    ["git", "ls-remote", "origin", "refs/tags/#{tag_name}", "refs/tags/#{tag_name}^{}"],
    chdir: RUST_REPO
  )
  remote_tag = refs.lines.filter_map do |line|
    sha, ref = line.split("\t", 2)
    ref = ref&.strip
    [sha, ref] if ["refs/tags/#{tag_name}", "refs/tags/#{tag_name}^{}"].include?(ref)
  end.to_h
  return if remote_tag.empty?

  remote_commit = remote_tag["refs/tags/#{tag_name}^{}"] || remote_tag["refs/tags/#{tag_name}"]
  unless remote_commit == head_commit
    raise "Remote release tag #{tag_name} points to #{remote_commit}, but HEAD is #{head_commit}; refusing to publish."
  end

  remote_object = remote_tag["refs/tags/#{tag_name}"]
  if local_tag
    local_object = capture!(["git", "rev-parse", "refs/tags/#{tag_name}"], chdir: RUST_REPO).strip
    unless local_object == remote_object
      raise "Local and remote release tag objects differ for #{tag_name}; refusing to publish before resolving the tag."
    end
  else
    run!(["git", "fetch", "--no-tags", "origin", "refs/tags/#{tag_name}:refs/tags/#{tag_name}"], chdir: RUST_REPO)
  end
end

def version_minor(version)
  match = version.match(/\A(\d+)\.(\d+)\.\d+(?:[-+].*)?\z/)
  raise "Crate version #{version.inspect} is not a simple semver version" unless match

  "#{match[1]}.#{match[2]}"
end

def ensure_one_minor_version!(crates)
  minor_versions = crates.group_by { |crate| version_minor(crate[:version]) }
  return if minor_versions.one?

  details = minor_versions.map do |minor, grouped_crates|
    "#{minor}: #{grouped_crates.map { |crate| "#{crate[:name]}@#{crate[:version]}" }.join(", ")}"
  end
  raise "Rust crates must share one major.minor version before release.\n#{details.join("\n")}"
end

raise "Could not find Rust repo at #{RUST_REPO}" unless Dir.exist?(RUST_REPO)

capture!(["python", File.join(RUST_REPO, "workspace-scripts/check_typed_release_inventory.py")], chdir: RUST_REPO)

all_crates = CRATES.map do |crate_dir_name, expected_name|
  crate_dir = File.join(RUST_REPO, "crates", crate_dir_name)
  cargo_toml = File.join(crate_dir, "Cargo.toml")
  raise "Missing Cargo.toml: #{cargo_toml}" unless File.exist?(cargo_toml)

  actual_name = cargo_package_field(cargo_toml, "name")
  version = cargo_package_field(cargo_toml, "version")
  raise "Crate order expected #{expected_name}, found #{actual_name}" unless actual_name == expected_name
  raise "Missing version in #{cargo_toml}" unless version

  { dir_name: crate_dir_name, dir: crate_dir, name: actual_name, version: version }
end

ensure_one_minor_version!(all_crates)

crates = all_crates.dup
if options[:only]
  crates.select! { |crate| crate[:dir_name] == options[:only] || crate[:name] == options[:only] }
  raise "Unknown crate for --only: #{options[:only]}" if crates.empty?
end

if options[:start_at]
  start_index = crates.index { |crate| crate[:dir_name] == options[:start_at] || crate[:name] == options[:start_at] }
  raise "Unknown crate for --start-at: #{options[:start_at]}" unless start_index

  crates = crates.drop(start_index)
end

if (options[:tag] || options[:push_git]) && (options[:only] || options[:start_at])
  raise "--tag and --push-git require the complete release set; remove --only/--start-at and rerun to resume safely"
end

versions = crates.map { |crate| crate[:version] }.uniq
tag_name = versions.one? ? "v#{versions.first}" : nil

if options[:list]
  puts JSON.pretty_generate(crates.map { |crate| crate.slice(:dir_name, :name, :version) })
  exit
end

puts "Selected Rust crates: #{crates.map { |crate| crate[:name] }.join(", ")}"
puts "Crate versions: #{versions.join(", ")}"
puts "crates.io publish: #{options[:push] && !options[:preflight_only] ? "enabled" : "disabled for this run"}"

ensure_clean_git! if options[:push] || options[:push_git] || options[:tag]

raise "--tag requires selected crates to share one version; found #{versions.join(", ")}" if options[:tag] && !tag_name
raise "--push-git requires --tag or crates with a shared version" if options[:push_git] && !tag_name
preflight_release_tag!(tag_name, check_remote: options[:push]) if options[:tag]

validate_release_metadata!(crates)
validate_crates_io_auth! if options[:push] && !options[:preflight_only]

registry_states = {}
if options[:push] || options[:preflight_only]
  puts "\nPreflighting crates.io names and existing releases before packaging or publishing..."
  crates.each do |crate|
    released = crate_released?(crate[:name], crate[:version])
    registry_states[crate[:name]] = released
    puts "  #{crate[:name]} #{crate[:version]}: #{released ? "already released" : "available to publish"}"
  end
end

if options[:preflight_only]
  package_preflight!(crates, allow_dirty: !options[:push])
  puts "\nPreflight complete for #{crates.length} crates; nothing was published."
  exit
end

run!(%w[cargo test], chdir: RUST_REPO) unless options[:skip_tests]

# Validate every selected package before the first irreversible upload. A full
# rerun is the resume mechanism: exact versions already live are skipped below.
package_preflight!(crates)

published_crates = []
skipped_crates = []

crates.each do |crate|
  puts "\n=== #{crate[:name]} #{crate[:version]} ==="

  if options[:push] && registry_states.fetch(crate[:name])
    puts "Skipping #{crate[:name]} #{crate[:version]}; already released on crates.io."
    skipped_crates << crate[:name]
    next
  end

  next unless options[:push]

  publish_crate!(crate)
  wait_for_crate_release!(crate)
  published_crates << crate[:name]
end

if options[:push]
  crates.each do |crate|
    next if registry_states.fetch(crate[:name])

    wait_for_crate_release!(crate)
  end

  if options[:tag]
    run!(["git", "tag", "-a", tag_name, "-m", "Release Rust crates #{versions.first}"], chdir: RUST_REPO) unless tag_exists?(tag_name)
  end

  if options[:push_git]
    run!(%w[git push origin HEAD], chdir: RUST_REPO)
    run!(["git", "push", "origin", tag_name], chdir: RUST_REPO) if options[:tag]
  end

  puts "\nPublished #{published_crates.length} crate(s) to crates.io: #{published_crates.join(", ")}"
  puts "Skipped #{skipped_crates.length} already-released crate(s): #{skipped_crates.join(", ")}" unless skipped_crates.empty?
else
  puts "\nLocal validation complete for #{crates.length} crate(s); no crates were published."
  puts "Skipping release tag creation because --no-push cannot confirm live releases." if options[:tag]
end
