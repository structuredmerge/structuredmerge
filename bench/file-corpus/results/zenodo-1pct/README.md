# Zenodo merge-conflict corpus, 1% profile slice: results

Per-case results for the [Zenodo corpus](https://zenodo.org/records/22165346) (`merge_cases.tar.gz`, 34,067,268,005 bytes, MD5 `d1e0164970eebf9062d760c6dc2d85bd`).
Sample: 1% of cases for json/jsonc/json5/py/go/rs/ts/tsx/sh = 1,037 cases in 10 repository-balanced shards (see `provenance/`).

`Expected` in the corpus is a historical resolution, **not a semantic oracle**. Exact agreement is not correctness, and disagreement is not necessarily wrong (e.g. cocos2d `showFPS`: base true, ours false, theirs unchanged; StructuredMerge returns false, `Expected` true).

Each `.jsonl` row records, per case: StructuredMerge exit/markers/exact/normalized match, Git baseline equivalents, and artifact paths + SHA-256 of the saved merge outputs and reports. The artifacts themselves are not committed; they live on the benchmark VM data disk (`/srv/structuredmerge-bench/results/*-artifacts/`).

## Runs

| File | Build | Notes |
|---|---|---|
| `shard-00-0bb5685-prefix.jsonl` | PR #7 (before any fix) | Legacy path used `plain_merge::merge_text` for non-Go/JSON files; **py/sh/ts "clean" output had newlines collapsed** ([plans#25](https://github.com/structuredmerge/plans/issues/25)). Kept as evidence. |
| `shard-00-2f793d9.jsonl`, `shard-01-2f793d9.jsonl` | [#8](https://github.com/structuredmerge/structuredmerge/pull/8) | Interim line-merge fallback. Superseded. |
| `shard-00-ff469f2.jsonl`, `shard-01-ff469f2.jsonl` | [#9](https://github.com/structuredmerge/structuredmerge/pull/9) | Parity with smorg-rb: unsupported language → `git merge-file` → plain text. Current. |

## Summary (current build, rev ff469f2)

| Shard | Cases | SM clean | SM exact = Expected | SM conflict markers | Git conflict markers |
|---|---|---|---|---|---|
| 0 | 41 | 7 | 3 | 34 | 41 |
| 1 | 77 | 5 | 3 | 72 | 77 |

## Scope caveat

These runs use the legacy `merge-driver` path, which has structured providers only for Go and JSON; every other file type goes through the `git merge-file` fallback. They do **not** measure the typed/benchmark-adapter lanes (bash, rust, typescript, ...). Tracked in [plans#26](https://github.com/structuredmerge/plans/issues/26). No shards beyond 1 have been run.

Infrastructure: [structuredmerge/devops `infra/bench`](https://github.com/structuredmerge/devops/tree/main/infra/bench) ([#73](https://github.com/structuredmerge/devops/pull/73): VM no longer replaced on cloud-init edits). Runner: [`../../run_cases.py`](../../run_cases.py) ([#7](https://github.com/structuredmerge/structuredmerge/pull/7)).
