#!/usr/bin/env bash
# Move every tree-sitter-language-pack / alef / polylint pin to the latest release,
# regenerate what alef owns, and run the gates. Safe to run locally or from the
# scheduled pin-bump workflow. Writes tmp/pin-bump/{summary.md,failures.txt}.
#
# Exit 0 whether or not anything changed; check `git status` or summary.md. A failed
# gate is recorded in failures.txt (the workflow then opens a draft PR) rather than
# aborting, so a human always gets to look at the result.
set -uo pipefail
cd "$(dirname "$0")/.."

# The interpreter that has the test dependencies; the formatter venv put on PATH below
# would otherwise shadow it.
py="$(command -v python3)"

out=tmp/pin-bump
rm -rf "$out"
mkdir -p "$out"
: > "$out/failures.txt"

gate() { # gate NAME COMMAND...
  local name="$1"
  shift
  echo "::group::$name"
  if "$@"; then
    echo "::endgroup::"
  else
    echo "::endgroup::"
    echo "$name" >> "$out/failures.txt"
  fi
}

if "$py" workspace-scripts/check_latest_pins.py > "$out/check.txt" 2>&1; then
  echo "all pins are already the latest release"
  echo "all pins are current" > "$out/summary.md"
  exit 0
fi

"$py" workspace-scripts/check_latest_pins.py --write | tee "$out/write.txt"
grep -E ' -> |checksum' "$out/write.txt" > "$out/changes.txt" || true

# The pinned alef installs from the reviewed checksum table, then regenerates.
alef_version="$(tr -d '[:space:]' < workspace-scripts/alef-version)"
bash workspace-scripts/install_alef_binary.sh workspace-scripts/alef-version tmp/alef-bin
export PATH="$PWD/tmp/alef-bin:$PATH"
if ! command -v poly > /dev/null; then
  "$py" -m venv tmp/alef-tools
  tmp/alef-tools/bin/python -m pip install --quiet polylint
  export PATH="$PWD/tmp/alef-tools/bin:$PATH"
fi

if grep -q alef "$out/changes.txt"; then
  gate "alef generate" alef generate
  gate "alef test-apps generate" alef test-apps generate
  gate "alef docs" alef docs
  gate "alef e2e generate" alef e2e generate
  # alef rewrites poly.toml on every run, dropping the file_safety exclusion for
  # the generated Ruby extension; the committed one is authoritative.
  git checkout HEAD -- poly.toml
fi
gate "alef verify" alef verify --exit-code
gate "typed release inventory" "$py" workspace-scripts/check_typed_release_inventory.py
# Version-only API surface changes need a human compatibility review (--record), so a
# baseline mismatch fails the gate and turns the PR into a draft.
gate "typed API baselines" "$py" workspace-scripts/check_typed_api_baselines.py
gate "workspace pytest" "$py" -m pytest workspace-scripts/tests -q
gate "cargo test" cargo test --workspace --exclude structuredmerge-core-py --no-fail-fast
gate "pins current" "$py" workspace-scripts/check_latest_pins.py

{
  echo "## Pin bump"
  echo
  sed 's/^/- /' "$out/changes.txt"
  echo
  if [ -s "$out/failures.txt" ]; then
    echo "### Gates that failed (needs a human)"
    sed 's/^/- /' "$out/failures.txt"
  else
    echo "All gates passed: alef verify, typed inventory/API baselines, workspace pytest, cargo test."
  fi
} > "$out/summary.md"
cat "$out/summary.md"
