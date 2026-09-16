# Request-correlated result contract

`structuredmerge_core::operation_result::OperationResult` represents the
Slice 1025 result envelope, using the existing Ruby provider-result schema.
`validate_against` checks it against a `ValidatedOperationRequest`, including
its verified source bytes. It does not execute a provider or depend on the
host-prototype facade.

The checks cover:

- Schema, request/operation correlation, explicit provider/profile selection,
  and explicit parser identity and selection mode.
- Exact consumed roles for classified outcomes, merge2 directionality, and
  merge3 base participation. An explicit `classification_reached: false`
  cannot contradict changes, conflicts, or success.
- Analyze/diff payload presence and the prohibition on merged output; clean
  and conflicted output are exclusive. Marker text is never a conflict model.
- Blocking failure evidence or unresolved structured conflicts; neither can
  coexist with success.
- Unique nonempty diagnostic/change/conflict IDs, exact ordered diff change
  references, valid conflict roles, and role-scoped changes.
- Diagnostic/change span bounds and byte-oriented points, conflict source
  identities and ranges, and exact retained-region input digests.
- Required preservation statuses, preservation report presence for clean
  merges, and reported output-reparse/structural-verification success.
- No undeclared fallback. Even a named fallback currently fails closed as
  unverified; registry-aware fallback verification is not implemented here.

`validate_operation_results` accepts results in transport arrival order,
correlates them by request ID, validates every member, and returns request
order. Missing, extra, duplicated, unknown, or invalid members reject the
whole batch without returning partially accepted results.

Compatible unknown fields and namespaced extensions survive serialization,
including fields inside source ranges. Provider-specific analysis, render
reports, fallback details and per-role change states retain their data rather
than being replaced with strings. Embedded analysis requires its Slice 1024
schema, but its complete analysis semantics are not validated here.

## Boundaries of this evidence

These are structural and source-evidence checks, not semantic verification.
True role-participation and preservation flags still need independently
produced algorithm/render evidence. A retained-region digest verifies the
input region; without an output range it cannot prove that the output retained
that region or ownership. Full render-plan/output provenance verification,
policy-required property negotiation and delegated-provider compatibility
remain required.

The diagnostic/conflict records here explicitly accept the Slice 1025
**migration shape**, including that fixture's historical hyphenated codes.
They are not the final Slice 1028 portable diagnostic/conflict records. Before
exposing this API through bindings, canonical projection must preserve origin,
source alternatives, causality, localization and resolution evidence and emit
Slice 1028 schemas/codes. No message parsing or blind hyphen replacement is an
acceptable substitute. No generated binding exports this module yet.

`tests/operation_results.rs` proves exact round trips for the four successful
fixture operations and two supplemental outcomes (parse failure and conflict
without markers). Adversarial tests cover their contradictory outcomes,
source corruption, identity/selection substitution, omitted payloads,
preservation failure/unverified/missing reports, duplicate IDs, invalid diff
references, undeclared/unverified fallback, unknown-field forwarding and batch
correlation. This is not full Slice 1025/1028 conformance, a provider capability
claim, a replacement for real execution tests, or a release gate completion.
