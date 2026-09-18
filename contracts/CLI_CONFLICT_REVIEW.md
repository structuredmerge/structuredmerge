# Read-only CLI conflict review

Both `smorg` and `smorg-rs` support `conflicts diff --json`, with optional
`--exit-code` and `--path-name`. `--` permits filenames beginning with dashes.
Duplicate options, missing/empty values and extra operands fail before reading
source or emitting JSON. Help is source-free. Existing human line summaries use
the same kernel review as JSON; output errors now return 3 instead of being ignored.

`ast-merge-git::conflict_review` owns the marker-framing state machine. This is
Git transport framing, not source-language classification or merge semantics.
It records exact source descriptors and ordered whole/ours/base/theirs byte
ranges, including empty alternatives and null absent bases. Unicode, UTF-8 BOM,
CRLF, multiple regions, configured marker widths and missing final newlines retain
their original byte offsets. The BOM remains outside the first marker range.
Standard, diff3 and zdiff3 framing is recognized; the review does not distinguish
diff3 from zdiff3 semantically.

Unlike the old CLI scanner, nested starts, missing separators, duplicate or
misordered markers and unterminated regions reject the entire review. Marker
width and token boundaries are exact; a longer run is not silently interpreted
as the configured shorter marker. Literal marker lines in source remain an
ambiguity: `semantic_conflicts_verified` is always false. Review does not invoke
a language parser, execute a merge, load a grammar or grant provider authority.

Limits: regular UTF-8 source up to 8 MiB, at most 10,000 regions, configured width
1–128. Source reads are bounded even if the file grows; this is not an adversarial
filesystem lease. Marker-size configuration retains the existing local attribute
reader, not full Git attribute-resolution conformance. No source or attribute
write occurs. Stable error codes distinguish source rejection, invalid UTF-8,
malformed markers, marker-size limits, input limits and region limits.

The shared CLI envelope has command `conflicts.diff`, the exact review or null,
canonical adapter errors, and null operation/availability/install fields. No
conflict means clean/0. A reviewed conflict means conflict/0 by default or
conflict/1 with `--exit-code`. Review failures are error/2, and output failures
are process exit 3. This is not typed semantic conflict-resolution evidence.

## Verification

- Four kernel framing tests and all 114 CLI tests pass, including the three
  explicitly enabled warm-grammar tests. Logs: `tmp/conflict-review-final-tests.log`.
- Three direct-process CLI tests exercise the shared twelve-case fixture on both
  names, malformed invocations, invalid UTF-8, missing/nonregular/oversized sources,
  exact source preservation and report channels. Each temporary source is removed.
- The portable fixture runner separately passes twelve cases per executable;
  reports in the fixtures repository are
  `tmp/cli-conflict-review-gadmhicl/report.json` and
  `tmp/cli-conflict-review-jse_33ix/report.json`.
- Three new fixture-checker self-tests and seven existing observer tests pass.
- All 74 kernel tooling tests pass; Alef verification, unchanged host API
  baselines and the 20-crate publication closure are checked separately.
- Both binaries pass the six real-Git merge cases and external-diff checks:
  `tmp/typed-cli-git-8ultdos8/report.json` and
  `tmp/typed-cli-git-mi9sin3u/report.json`. Existing discovery remains 19/20;
  fixture reports are `tmp/cli-conformance-hf4dbo9_/report.json` and
  `tmp/cli-conformance-bz364uyc/report.json`. `languages --json` is still missing.
- Current binaries are retained in `tmp/conflict-review-bin/`. Their SHA-256:
  `smorg`: `096dced81499567061f2367641fb2df09dba0f7a4ff9e9d9da40d3f4e30b3e92`;
  `smorg-rs`: `2ded9c2ddcbb1fcf6d3707c33b20f1fb5d71161e994d6c0c7070692227670ebd`.

The bounded compiler target and superseded compiled-provider binary pair are
removed after verification. Current binaries and small logs/reports remain.
This advances the conflict-review command, not runtime provider availability,
preflight, publication, hosted CI, platform support or default approval.
