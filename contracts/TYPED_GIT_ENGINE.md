# Typed Git rendering migration

`ast_merge_git::typed::merge3` is the Rust engine behind the explicit common
`kernel.git.json.v1` profile (provider `kernel.git.json`, family `json`). It consumes
validated TreeHaver results for distinct base/ours/theirs sources and delegates
classification and clean rendering to the existing typed JSON engine. Parser
selection and output parsing remain caller-owned through the shared substrate;
there is no host-prototype transport, parser registry or implicit fallback here.

Clean results retain the JSON family's actual edit/retention evidence and require
output verification. Conflict results retain the full native classifications and
alternatives. The existing localized renderer supplies marker output, source-line
provenance, synthesized-fragment records, conflict locations and digests. Evidence
is bound to all three source descriptors and replayable against trusted native
decisions and explicit rendering options. Replay checks integrity, not semantic
authority of foreign classifications. It does not claim successful JSON reparsing.

Markers default to width seven, accept widths 1–1024, and honor explicit labels.
Labels must be nonempty, at most 1024 UTF-8 bytes, and contain no control characters.
Options are validated before execution even for clean/no-op inputs. Unrenderable
conflicts stay unresolved with a render error; no full-file fallback is invented.

## Limits that must survive the consumer cutover

- This is line-localized review output, not an exact AST-node-only splice.
  Original source lines retain their bytes, including CRLF; markers and missing
  final-line boundaries use synthesized LF and are recorded as such.
- The established renderer keeps ours outside conflicts. Independent theirs
  edits outside those regions are **not** incorporated into this review artifact.
  Do not advertise it as a complete partially resolved merge. A future change to
  that policy needs Rust-owned decisions, explicit evidence and shared fixtures.
- Overlapping conflict line ranges and absent alternatives cannot currently be
  rendered. Conflict-write consumers must fail closed; leave-ours can report the
  unresolved conflict without writing. No clean-output or preservation claim is
  inferred merely from a nonempty marker artifact.
- Legacy formatting scores/default-driver recommendations are not carried into
  this typed path: they were constants, not measurements of this execution.

## Remaining integration

The common facade now accepts merge3 only, explicit JSON/JSONC/JSON5 dialects,
source-preserving rendering, no fallback, marker width and base/ours/theirs labels.
It reuses shared parser negotiation and cancellation. Clean output retains normal
JSON edit/output-parse validation. Conflicts preserve canonical records and expose
`conflicted_output` with an explicitly labeled review-artifact report. Validation
reconstructs validated input parses and recomputes native conflict decisions and
rendering; altered classifications, bytes, provenance or omission fail validation.
Compatible passive fields survive. No successful/failed JSON reparse is claimed
for marker output, because no such parse was attempted.

Generate shared binding fixtures,
rebuild isolated artifacts, then migrate Ruby's opt-in Git provider and test real
Git write/leave-ours/error exits. Correct its current all-four-operations metadata
at cutover; keep unimplemented operations unsupported. Retain canonical conflicts,
render limitations and complete records across that boundary. Do not substitute
generic JSON merge output for the Git protocol or mark the legacy export migrated
before the actual consumer changes. The existing typed operation entry point is
used without new binding signatures; the Ruby consumer is not migrated yet.

Local tests: `cargo test -p ast-merge-git --locked` exercises eight typed cases and
five existing fixtures. The cases include all three dialects, real clean-output
reparsing and rejection, Unicode/CRLF, custom labels/width, tampered replay data,
absent alternatives, overlapping lines, missing final newlines and mixed independent
edits/conflicts. Log: `tmp/git-typed-render-tests.log`.

Common facade tests add clean results for all three dialects, canonical conflict
replay/tampering and compatible fields, unrenderable cases, unsupported operations,
unknown labels, injection/size/fallback rejection, parse errors and exported facade
cancellation. Logs: `tmp/git-common-{operation-tests,regressions,clippy,generation,audits}.log`.
