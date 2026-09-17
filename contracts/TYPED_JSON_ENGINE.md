# JSON family typed-parser migration

`json_merge::typed` supplies analysis, directional merge2 and base-aware merge3
over validated TreeHaver `ParsedResult` values. This is the existing nested
JSON/JSONC/JSON5 engine, not a new top-level-only profile or a host implementation.
The old source-taking entry points remain wrappers around the same analyzer,
planner and renderer while repository consumers migrate.

The family adapts typed node facts internally to its existing analyzer's node
view. Text comes only from validated byte spans; field names come from child
edges. This view is neither a public JSON-string transport nor a second parser
registry. The typed path does not load grammars or call the old parser entry
point. Dialect validation, nested matching, conflict decisions, comments/layout,
current-preferred arrays and source-preserving edits remain Rust-owned.

Merge2 requires distinct Incoming/Current source IDs and never synthesizes a
base. Merge3 requires distinct Base/Ours/Theirs sources. Every successful render,
including selected-input/no-op outcomes, invokes a caller-supplied output parser.
The result must carry Output role, a non-input source ID, exact rendered bytes,
the destination/ours parser descriptor and the planned semantic value. Failed
verification returns no output. The orchestrator must use the same TreeHaver
snapshot and propagate resource/cancellation controls; this family callback
interface does not itself prove registry generation or preempt recursive work.

Seven native typed integration tests cover nested independent edits/conflicts,
directional current preservation, arrays, Unicode/CRLF, JSONC/JSON5 comments,
dialect rejection, unsuccessful parses, swapped roles, and output verification
failure/byte/backend/role rejection. Analysis and commented merges match the
existing engine. All JSON tests (24 unit, 14 fixture, 7 typed), five Git adapter
tests, TreeHaver regression, six opt-in provider tests and strict Clippy pass.
Logs: `tmp/typed-json-{family-regression,engine-tests,provider-regression,engine-clippy}.log`.

These tests uncovered and fixed a TreeHaver provider contract issue: native
Comment-role nodes require comment index records even when optional comment
enrichment is not requested. Enrichment controls do not suppress those facts.

## Common merge execution and preservation evidence

The common `execute_operation` facade now accepts explicit profile
`kernel.json.nested.v1`, provider `kernel.json`, family `json`, and dialect
`json`, `jsonc` or `json5` (omission selects strict JSON). Merge2 requires
`template-into-current` direction; both merge operations require
`source-preserving` rendering and no fallback. Unsupported constraints, including
unimplemented operations, version/profile requirements, policy fields, labels
and marker-size requests, fail closed. There is no default or implicit parser
registration. JSONC/JSON5 use the JSON5 grammar plus family dialect validation.

Input parsing and output verification use one TreeHaver snapshot and execution
context. Output parsing pins the current/ours backend; the renderer separately
records whether current, ours or theirs supplied the actual baseline. Deadline
and cancellation checks surround parser and family execution; recursive native
analysis/render work is not claimed to be preemptible.

`merge2_with_evidence` and `merge3_with_evidence` return the actual executed edit
plan, baseline descriptor, exact retained source/output ranges and digests.
Replacement ranges contain their rendered text but do not claim an unchanged
donor-source origin. Edit replay must reproduce the complete output; retained
regions are independently byte-checked. Even selected-source/no-op paths report
their actual source role. Failed semantic or parse verification discards the
proof and output. The common result validator checks the edit replay, retained
range projection and output parse against request/output bytes.

The preservation property is deliberately `exact-bytes-outside-executed-edits`,
not preservation of every byte inside replaced owners. The existing engine may
reformat an inline object when adding a member. This migration preserves that
existing family behavior; it does not turn synthesized replacement text into
donor provenance or make edit replay proof of semantic authorization.

Merge3 conflicts are canonical `provider_specific` records carrying the original
family category, path and alternatives in classification evidence. Present
regions receive exact input-byte digests; absent alternatives remain absent.
Unavailable native locality is not upgraded to exact locality. No fabricated
decision IDs, resolutions, markers or conflicted output are introduced.

Next: common JSON analysis/diff2 with authoritative nested owner spans, broader
generated shared JSON fixtures and actual Ruby family/Git consumer migration.
Those adapters' prototype calls and Ruby-owned diff decisions remain open. No
public core API signature, parser default or publication authority changed.

## Nested owner facts and bounded comparison (2026-09-17)

`typed::owner_analysis` now returns source-qualified owner facts for the syntax
root, every nested member and every array element. Members reference their actual
native pair/member node; elements reference their value node. Byte spans must
match those nodes exactly and carry source-byte digests. Parent references are
owner IDs, paths use JSON Pointer escaping, and array identity is positional.
Duplicate decoded keys fail explicitly instead of overwriting an owner in a map.
Nested subjects overlap ancestors; they are not a render partition.

`typed::diff_owner_sources` compares exact bytes at those spans in Rust. It
classifies added/deleted/edited subjects, including scalar roots and containing
ancestors. Equal fragments never supply locations. This is a Rust-only helper,
not the common diff2 operation: comments and whitespace outside the syntax root
are intentionally outside its comparison. The retained legacy family analysis
has its own native-node owner namespace and line-based comment/layout records;
it is not mislabeled as the common Slice 1024 analysis result.

Five additional typed tests cover repeated fragments, escaped keys, Unicode/CRLF,
native span/digest/parent consistency, nested changes, scalar roots, positional
arrays, duplicate decoded keys, wrong roles, duplicate source IDs, and the
outer-trivia limitation. Common analysis/diff2 still needs exact comment/layout
projection and comparison plus evidence validation before consumer migration.
No generated export or installed artifact was changed in this step.
JSON's 24 unit, 14 fixture and 12 typed tests pass, alongside Git/core regression
tests (native-runtime opt-in tests were not rerun in this step) and strict Clippy.
Logs: `tmp/typed-json-owner-{regression,clippy}.log`.

## Installed verification (2026-09-17)

Seven common-facade JSON tests pass, including canonical present/absent conflict
regions, root conflicts, invalid dialect syntax, unsupported constraints,
cancelled execution, output-parser failure and tampered render/parse evidence.
Core/JSON/Git regressions, strict Clippy, 23 existing Psych/LibCST runtime tests
and 14 artifact/inventory audits pass. Ruby's isolated gem passes 31 tests plus
12 existing generated fixtures; Python's isolated wheel passes 35 plus 15.
The new JSON binding checks exercise merge2, nested merge3 and conflicts in Rust;
shared generated JSON fixtures are still pending.

The Ruby artifact test exposed an enum-map round-trip bug in Alef: unit enums
were emitted as Symbols but only Strings were accepted on input. Local Alef
commit `870d1e7` fixes that conversion without arbitrary `to_s` coercion;
134 Magnus tests and the unmodified installed round-trip check pass. Regeneration
changed native Ruby conversion bodies, not reviewed Ruby/Python API signatures.
The fix remains local, as requested.

Logs: `tmp/typed-json-{common-regression,common-clippy,native-regression,common-audits,python-artifact,ruby-artifact}.log`.
Artifacts: Ruby SHA-256
`53986c5814193fda2a203f86ec24b38de11ebef2f4c9c29096d15b3506dd5ed2`;
Python `d17fb05e6604bd26624b119ec9b34303cb2b4cdfe31b6fea9d9f31bd7f4676c1`.
