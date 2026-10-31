#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 EVALUATOR_DIR QUEUE_TSV CACHE_DIR" >&2
  exit 2
fi

evaluator=$(realpath "$1")
queue=$(realpath "$2")
cache=$(realpath -m "$3")
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
smorg_bin=${SMORG_BIN:-smorg-rs}
state_dir=${BENCH_STATE_DIR:-"$evaluator/.structuredmerge-bench"}
command -v "$smorg_bin" >/dev/null 2>&1 || { echo "Missing SM binary: $smorg_bin" >&2; exit 2; }
project_root=$(git -C "$project_dir/../.." rev-parse --show-toplevel)
if [[ -n "$(git -C "$project_root" status --porcelain)" ]]; then
  echo "StructuredMerge checkout must be clean before benchmarking" >&2
  exit 2
fi
mkdir -p "$state_dir"

while IFS=$'\t' read -r run_name repository_csv merge_count; do
  [[ -z "$run_name" || "$run_name" == \#* ]] && continue
  if [[ ! "$run_name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Invalid run name in queue: $run_name" >&2
    exit 2
  fi
  if [[ ! "$merge_count" =~ ^[1-9][0-9]*$ ]]; then
    echo "Invalid merge count for $run_name: $merge_count" >&2
    exit 2
  fi
  [[ "$repository_csv" = /* ]] || repository_csv="$evaluator/$repository_csv"
  [[ -f "$repository_csv" ]] || { echo "Missing CSV: $repository_csv" >&2; exit 2; }
  marker="$state_dir/$run_name.complete"
  [[ -f "$marker" ]] && continue

  export SMORG_BIN="$smorg_bin"
  export PATH="$(dirname "$(command -v "$smorg_bin")"):$PATH"
  cd "$evaluator"
  ./run.sh "$repository_csv" "smorg-$run_name" "$merge_count" "$cache"
  {
    printf 'run_name=%s\n' "$run_name"
    printf 'repository_csv=%s\n' "$repository_csv"
    printf 'repository_csv_sha256=%s\n' "$(sha256sum "$repository_csv" | awk '{print $1}')"
    printf 'merge_count=%s\n' "$merge_count"
    printf 'completed_at_utc=%s\n' "$(date -u +%FT%TZ)"
    printf 'kernel_revision=%s\n' "$(git -C "$project_dir/../.." rev-parse HEAD)"
    printf 'evaluator_revision=%s\n' "$(git -C "$evaluator" rev-parse HEAD)"
    printf 'smorg_binary=%s\n' "$(command -v "$smorg_bin")"
    printf 'smorg_binary_sha256=%s\n' "$(sha256sum "$(command -v "$smorg_bin")" | awk '{print $1}')"
  } > "$marker.tmp"
  mv "$marker.tmp" "$marker"
done < "$queue"
