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

RUST_REPO = File.expand_path("..", __dir__)
CRATES_IO_USER_AGENT = "structuredmerge-release-script (https://github.com/structuredmerge/structuredmerge-rust)"
STRUCTUREDMERGE_REPOSITORY = "https://github.com/structuredmerge/structuredmerge-rust"
STRUCTUREDMERGE_HOMEPAGE = "https://structuredmerge.org"
STRUCTUREDMERGE_LICENSE = "AGPL-3.0-only OR PolyForm-Small-Business-1.0.0"
PUBLISH_RETRY_BUFFER_SECONDS = 10
RELEASE_CONFIRM_TIMEOUT_SECONDS = 15 * 60
RELEASE_CONFIRM_POLL_SECONDS = 15

CRATES = [
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
  ["kettle-rusty", "kettle-rusty"],
].freeze

options = {
  push: true,
  push_git: false,
  skip_tests: false,
  tag: false,
}

parser = OptionParser.new do |opts|
  opts.banner = "Usage: release_rust_crates.rb [options]"

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

def capture!(cmd, chdir:)
  output, status = Open3.capture2e(*cmd, chdir: chdir)
  raise "Command failed: #{sh(cmd)}\n#{output}" unless status.success?

  output
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
  response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == "https") do |http|
    http.request(request)
  end

  [response, response.body]
end

def structuredmerge_crate_metadata?(crate)
  crate["repository"] == STRUCTUREDMERGE_REPOSITORY &&
    crate["homepage"] == STRUCTUREDMERGE_HOMEPAGE
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
    raise "Crate name #{name.inspect} is already owned by a different project on crates.io.\n" \
      "repository=#{crate["repository"].inspect}\n" \
      "homepage=#{crate["homepage"].inspect}\n" \
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
  loop do
    success, output = run_command(["cargo", "publish", "-p", crate[:name]], chdir: RUST_REPO)
    return if success

    retry_at = publish_rate_limit_retry_at(output)
    if retry_at
      sleep_until_retry(retry_at)
      next
    end

    raise "Command failed: #{sh(["cargo", "publish", "-p", crate[:name]])}"
  end
end

def wait_for_crate_release!(crate)
  deadline = Time.now + RELEASE_CONFIRM_TIMEOUT_SECONDS
  loop do
    return if crate_released?(crate[:name], crate[:version])

    raise "Release confirmation failed: #{crate[:name]} #{crate[:version]} is not live on crates.io." if Time.now >= deadline

    puts "Waiting #{RELEASE_CONFIRM_POLL_SECONDS}s for #{crate[:name]} #{crate[:version]} to become visible on crates.io..."
    sleep(RELEASE_CONFIRM_POLL_SECONDS)
  end
end

def tag_exists?(tag_name)
  system("git", "rev-parse", "-q", "--verify", "refs/tags/#{tag_name}",
    chdir: RUST_REPO, out: File::NULL, err: File::NULL)
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

versions = crates.map { |crate| crate[:version] }.uniq
tag_name = versions.one? ? "v#{versions.first}" : nil

puts "Selected Rust crates: #{crates.map { |crate| crate[:name] }.join(", ")}"
puts "Crate versions: #{versions.join(", ")}"
puts "crates.io publish: #{options[:push] ? "enabled; pass --no-push for local validation only" : "disabled; local validation only"}"

ensure_clean_git! if options[:push] || options[:push_git] || options[:tag]

raise "--tag requires selected crates to share one version; found #{versions.join(", ")}" if options[:tag] && !tag_name
raise "--push-git requires --tag or crates with a shared version" if options[:push_git] && !tag_name

run!(%w[cargo test], chdir: RUST_REPO) unless options[:skip_tests]

published_crates = []
skipped_crates = []

crates.each do |crate|
  puts "\n=== #{crate[:name]} #{crate[:version]} ==="

  if options[:push] && crate_released?(crate[:name], crate[:version])
    puts "Skipping #{crate[:name]} #{crate[:version]}; already released on crates.io."
    skipped_crates << crate[:name]
    next
  end

  run!(["cargo", "package", "-p", crate[:name]], chdir: RUST_REPO)

  next unless options[:push]

  publish_crate!(crate)
  wait_for_crate_release!(crate)
  published_crates << crate[:name]
end

if options[:push]
  crates.each do |crate|
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
