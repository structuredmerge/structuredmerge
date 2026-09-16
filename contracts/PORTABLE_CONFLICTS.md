# Canonical conflict records

`structuredmerge_core::portable_conflict` represents
`structuredmerge.conflict/v1` inside the existing `OperationResult` envelope.
Canonical records retain typed category, ordered roles and alternatives,
classification, localization, resolution, record references, extensions and
metadata. Schema-bearing records cannot downgrade to legacy parsing; duplicate
top-level fields and mixed migration/canonical conflict arrays are rejected.

The validator checks exact operation roles and alternative order, including
incoming/current for merge2 and base/ours/theirs for merge3. Every merge3
conflict must report base participation. Present alternatives identify their
source; every supplied region's bounds, length and digest are verified against
that source's exact bytes. Absent alternatives cannot invent ranges. Exact
localization requires verified nonempty regions for every present alternative.
Coarse or whole-document localization does not pretend to have exact regions.
Reported output regions are checked against actual output bytes, not a
self-reported verification flag. No marker scanning is used.

Unresolved conflicts cannot coexist with success, selected roles or resolution
authorization. Resolved records remain audit evidence and require strategy,
reason, resolver and a separately supplied authorization matching conflict ID,
decision ID, strategy, selected roles and resolver. Decision/render reference
IDs must resolve in executor-supplied evidence. Diagnostic/change references
resolve within the containing result. A conflict's own lists of IDs do not
serve as an evidence catalog or authorize its resolution.

`OperationResult::validate_with_conflict_evidence` accepts that executor
evidence. The original `validate_against` supplies no external evidence and
therefore fails closed when canonical conflicts require it.
`validate_operation_results_with_evidence` correlates out-of-order results and
scopes executor evidence by request ID. Missing or unknown-request evidence
cannot be replaced by another batch item's records.

## Evidence and remaining integration

Tests round-trip both canonical fixture conflicts, including exact output
regions for the authorized select-ours case, through individual and operation
result checks. Adversarial cases cover missing base/roles, role reversal,
false localization, missing/corrupt source evidence, output mismatch,
fabricated absent ranges, missing references, duplicate IDs/fields,
schema downgrade and unauthorized resolution. Additional cases exercise
directional roles, honest whole-document scope, unknown-field forwarding and
request-scoped batch authorization.

These checks do not prove semantic classification or create policy authority.
The trusted Rust executor must supply real decision/render catalogs and
authorizations, not copy declared IDs out of an untrusted result. Structural
subject resolution, classification truth, resolved output semantics, complete
render-plan provenance and deterministic production IDs remain separate
requirements. The tests use explicit fixture evidence catalogs, not an
executing provider.

Executed Rust owner conflicts now have an evidence-backed projection described
in [NATIVE_CONFLICT_PROJECTION.md](NATIVE_CONFLICT_PROJECTION.md). There is still
no automatic arbitrary legacy-conflict conversion: the old
records lack enough classification, alternative-state and authorization facts
to fabricate canonical evidence safely. Real native-provider projection,
common operation dispatch, generated binding/consumer adoption and full
Slice 1028 conformance remain open. No release or default-authority gate is
claimed by this module.
