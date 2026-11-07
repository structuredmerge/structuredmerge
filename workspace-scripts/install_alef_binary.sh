#!/usr/bin/env bash
# Install the reviewed Linux x86_64 Alef release binary without compiling it.
set -euo pipefail

version_file="${1:-workspace-scripts/alef-version}"
install_dir="${2:?usage: install_alef_binary.sh VERSION_FILE INSTALL_DIR}"
alef_version="$(tr -d '[:space:]' < "$version_file")"

if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  printf 'Alef binary installer only supports Linux x86_64 (got %s/%s)\n' "$(uname -s)" "$(uname -m)" >&2
  exit 1
fi

case "$alef_version" in
  0.128.0)
    alef_sha256=696400c3737881868ffd96e0f44beac4bdbbecd0e5fa8539e1cb1fd22e9bf365
    ;;
  *)
    printf 'No reviewed Linux binary checksum for Alef %s\n' "$alef_version" >&2
    exit 1
    ;;
esac

download_dir="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/alef-${alef_version}-linux-x86_64"
archive="$download_dir/alef-x86_64-unknown-linux-gnu.tar.gz"
mkdir -p "$download_dir" "$install_dir"
curl --fail --location --silent --show-error \
  "https://github.com/xberg-io/alef/releases/download/v${alef_version}/alef-x86_64-unknown-linux-gnu.tar.gz" \
  --output "$archive"
printf '%s  %s\n' "$alef_sha256" "$archive" | sha256sum --check
tar -xzf "$archive" -C "$download_dir"
install -m 0755 "$download_dir/alef-x86_64-unknown-linux-gnu/alef" "$install_dir/alef"

installed_version="$("$install_dir/alef" --version | sed -n 's/^alef //p')"
if [[ "$installed_version" != "$alef_version" ]]; then
  printf 'Expected Alef %s, installed %s\n' "$alef_version" "$installed_version" >&2
  exit 1
fi
