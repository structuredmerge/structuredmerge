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

Next: broader consumer parity review, Git and remaining family migration.
The Ruby JSON adapter's local cutover is recorded below. No
public core API signature, parser default or publication authority changed.

## Common exact-source diff2

The same explicit JSON profile now accepts diff2 with omitted or
`exact-source-owners` comparison profile and omitted, empty or `exact-source`
equivalence rules. Other comparison rules fail before parsing. The shared
TreeHaver snapshot supplies distinct Before/After inputs. Source-preservation
evidence is always included; no output or output-reparse claim is made.

Changes include a whole-document summary whenever any bytes differ, followed by
the Rust-owned nested owner comparisons. The summary catches outer comments,
CRLF/final-newline and other trivia changes without inventing their ownership.
Each record carries exact source identity, spans and digests. Ancestor and summary
subjects intentionally overlap; arrays are positional, and the result is neither
a minimal edit script nor a semantic move detector. Duplicate decoded keys fail
closed. Identical complete sources produce no changes.

The result validator checks embedded native parse graphs against request bytes,
source roles, dialect/backend and selection evidence, then recomputes all changes
and summary claims. Missing or altered classifications, ranges, digests or whole
document coverage are rejected. Compatible unknown fields survive forwarding.
This consistency check does not authenticate a foreign parser or grant semantic
authority to a fabricated result. Cancellation/deadline checks still surround
execution and validation; recursive family work is not preemptible.

Common analysis is described below: diff2's complete byte comparison does not
pretend to resolve every comment attachment. Ruby consumer migration remains open.

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
outer-trivia limitation. Common analysis now projects these facts and checks
ownership evidence; common diff2 supplements this helper with complete-document
comparison as described above.
No generated export or installed artifact was changed in this step.
JSON's 24 unit, 14 fixture and 12 typed tests pass, alongside Git/core regression
tests (native-runtime opt-in tests were not rerun in this step) and strict Clippy.
Logs: `tmp/typed-json-owner-{regression,clippy}.log`.

### Comment provenance and exact blank-run gaps

The shared `ast-merge` comment augmenter now offers companion evidence captured
while executing its existing grouping decisions. Each region retains the native
comment node IDs that supplied its lines; repeated equal-text comments remain
distinct. The legacy serialized augmentation and attachment policy are unchanged.
The JSON owner analysis exposes this map and explicitly lists unclaimed native
comments. A multiline node can contribute to more than one region or be partly
unclaimed: the map is evidence, not a disjoint render partition.

Existing blank-run layout decisions now have exact byte spans and digests in
`layout_gap_sources`. Newline indexing preserves CRLF and final whitespace bytes;
it does not discover comments or infer syntax from text. These gaps do not yet
cover all document trivia. Common analysis supplies owner-namespace projection,
native comment coverage/ambiguity reporting and attachment/controller validation.
Common diff2's document summary does not supply these decisions.

Two additional typed tests cover repeated same-line comments, multiline comments,
unclaimed trailing comments and exact CRLF/final-whitespace gaps (14 typed tests
total). Logs: `tmp/json-comment-provenance-{tests,clippy,workspace}.log`.

## Common JSON analysis

Explicit `kernel.json.nested.v1` analysis now embeds the validated native parse,
nested owners and logical identities, native comment regions, exact blank-run
layout gaps, attachments and ownership decisions. It requires comments,
diagnostics and native-extension parser capabilities. Omitted analysis policy
selects `exact-source-owners`; disabling comments/ownership/native extensions or
requesting token analysis is unsupported and fails before parsing.

The family engine resolves its native value-node owner IDs to logical owner IDs.
One exact region per native comment avoids treating intervening code as comment
text when the legacy family groups multiple same-line comments. Existing family
group references remain available. Incomplete/conflicting attachment decisions
are retained at document scope, marked unresolved and accompanied by nonblocking
analysis diagnostics and alternatives. This is not an invented sibling attachment
or permission to move an unresolved comment. Native parse extensions are retained
inside the embedded parse result, not flattened into universal semantic fields.

Layout follows the existing family blank-run policy, with exact source digests,
one controller per gap and the existing opposite-side fallback when both sides
exist. It does not claim to cover all source trivia or form a non-overlapping
render partition. Source-preserving merges continue using their executed edit
evidence; analysis is not itself an authorization to apply edits.

The common validator reconstructs the native parse and family analysis from
request bytes, then checks owner references, comment coverage, attachments,
controller decisions, digests and diagnostics. Compatible unknown fields and
passive extensions are retained. Five new common tests cover nested/scalar/array
owners, dialects, exact comment and gap evidence, unresolved comments, explicit
enrichment, native extensions, shared-gap fallback, malformed/duplicate-key input, tampering and
unsupported policies; cancellation also covers analysis. Consumer parity,
broader analysis-policy conformance remain
open. The subsequent Ruby JSON cutover is recorded below.

## Installed diff2 verification (2026-09-17)

Eleven common JSON tests pass, including diff2 coverage, cancellation, unsupported
rules, ambiguous keys, unknown-field forwarding and altered/missing evidence.
Core/JSON regressions, strict Clippy and 14 artifact/inventory audits pass.
Both isolated installed packages execute nested and trivia-only JSON diff2 through
the generated facade: Python 35 tests plus 15 existing generated fixtures; Ruby
31 plus 12. No source API baseline changed. Broader downstream/platform gates
remain open; shared JSON fixtures were added in the subsequent step below.

Logs: `tmp/json-common-diff-{tests,facade,clippy,audits,python-artifact,ruby-artifact}.log`.
Python wheel SHA-256: `6aa4f7df1d6bc13977b3b10c8bcca74b2e90531190cc2ca51d47c9d1cd465089`.
Ruby gem SHA-256: `c5e65da7ec16ed7f386091ad1fd66449f56038b0efc8d494f386d737e01363b3`.

## Installed analysis verification (2026-09-17)

All 16 common JSON tests, core/JSON regressions, strict Clippy and 14 audits pass.
The rebuilt isolated wheel passes 35 tests plus 15 existing generated fixtures;
the isolated gem passes 31 plus 12. Both execute JSON5 analysis and check native
owner/comment references and the unresolved-attachment diagnostic. Regeneration
updates the input fingerprint without changing reviewed public API source files.
Logs: `tmp/json-common-analysis-{tests,clippy,audits,python-artifact,ruby-artifact}.log`.
Wheel SHA-256: `9acc664103c9a428d0dbae8e71ac20e4f7472adc2b1cd57757bc782fa519afdf`.
Gem SHA-256: `d5d11261c7954780bb2975e744b60483ea6cb73a39eed845733d5a7476c97ea2`.

## Shared generated JSON fixtures (2026-09-17)

Fixtures commit `structuredmerge/fixtures@1e27290` adds eleven JSON cases shared
by generated Ruby and Python suites. They cover nested owner paths, unresolved
native comments, duplicate decoded keys, JSONC rejection of JSON5 syntax,
repeated-fragment and outer-trivia diffs, identical Unicode sources, directional
current/array precedence, independent merge3, canonical conflicts and selected
source comment/CRLF/final-newline preservation. Test-only call adapters register
explicit Rust language-pack providers and construct typed requests; expectations
remain in the shared fixtures, with no host-owned analysis or merge logic.

Eight test files were generated by the local Alef tool. Both existing artifacts
pass the expanded isolated suites: Python 35 runtime tests plus 26 generated
cases; Ruby 31 plus 23. The same eleven JSON cases run in both. Artifact hashes
remain those of the installed analysis step above. API baselines and 14 audit
tests pass. A repeated e2e generation reports cached/up-to-date output; this is
not an independent clean-build reproducibility claim. Optional `poly fmt` is
unavailable, so no formatter gate is claimed.

Logs: `tmp/json-shared-fixtures-{generation,bindings,python,ruby,audits,reproducibility}.log`.
Actual Ruby family/Git consumer migration, Ruby golden-master authority review,
broader policy/downstream/platform and publication gates remain open.

## Ruby JSON consumer cutover (2026-09-17)

Local Ruby main `19bddc684` moves the opt-in `rust.json` provider to typed common
operations. The compatibility class no longer inherits the prototype adapter;
it uses TreeHaver's existing typed parser registration lifecycle, explicit source
roles/digests and the JSON kernel profile. No Ruby-owned diff comparison or
source-text location lookup remains on this path. Unknown requirements fail
closed. JSON Pointer paths replace non-unique member-name identities; root and
document-summary subjects remain visible rather than discarded for compatibility.

The adapter projects generated read-only records into deterministic portable Ruby
values, retaining complete core evidence, native conflicts, canonical records,
diagnostics and exact render verification. It does not invoke executable handles
or transport opaque whole-operation JSON. UTF-8 binary strings are accepted
without transcoding or mutating caller data. No default backend changes.

The complete JSON suite passes through `kettle-test`: 93 examples against the
installed core gem (SHA-256 `d5d11261c7954780bb2975e744b60483ea6cb73a39eed845733d5a7476c97ea2`).
Native comparisons cover all three dialects and exact/independent/conflicting
merge3 outcomes. Incorrect negative-backend tags were removed from those parity
tests so they actually execute with Rust present. Repeated-fragment locations,
directionality, malformed input, conflict evidence, constraint rejection and
deterministic portable serialization also pass. Logs:
`tmp/json-ruby-consumer-{bundle,focused,parity,full}.log`.

The artifact test bundle requires the core; released-package CI remains separate.
Tool-managed locks were left local. This is local consumer integration, not broad
golden-master authority, hosted/downstream/lint/coverage or release approval. Git
and the Bash/Go/Rust/TypeScript consumers remain on the migration inventory.

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
