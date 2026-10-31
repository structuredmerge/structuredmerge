#!/usr/bin/env bash

# AST-Merging-Evaluation tool adapter. Merges branch2 into branch1 in the
# evaluator's disposable repository and lets Git invoke the SM driver only for
# formats advertised by the exact smorg-rs build under test.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 REPOSITORY BRANCH1 BRANCH2" >&2
  exit 2
fi

repo=$1
branch1=$2
branch2=$3
smorg_bin=${SMORG_BIN:-smorg-rs}

cd "$repo"
git config --local merge.smorg-rs.name "StructuredMerge"
git config --local merge.smorg-rs.driver "$smorg_bin merge-driver %O %A %B %P"
attributes=$(git rev-parse --git-path info/attributes)
"$smorg_bin" languages --gitattributes >> "$attributes"
git checkout "$branch1" --force
git merge --no-edit "$branch2"
