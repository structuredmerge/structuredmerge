# Canonical diagnostic records

`structuredmerge_core::portable_diagnostic` implements the typed
`structuredmerge.diagnostic/v1` record. It is integrated into the existing
`OperationResult` diagnostics array, not a second operation envelope.

Records carry typed category/severity/origin, result-local identity and
sequence, optional request/operation correlation, ordered source and subject
references, causal/related references, structured data, native extensions and
metadata. Unknown compatible fields survive forwarding, including fields in
origins, references, spans, points and ranges. Native codes stay in
`origin.native_code`; neither native exceptions nor human messages become
portable codes.

The diagnostic-scope validator checks schema, canonical namespaced code syntax,
unique nonempty IDs, contiguous sequences, preceding causes, local related
references, request identity, exact source IDs/roles/byte spans/points, and
subject resolution supplied by the owning result. It enforces operation-phase
order and parse diagnostic role order. The result validator resolves known
conflict/change/structural-path/classification subjects and rejects unsupported
references instead of silently accepting them. A canonical `merge_conflict`
diagnostic cannot substitute for an absent conflict record.

## Migration is explicit

`DiagnosticRecord` distinguishes schema-bearing canonical records from
schema-less migration records. Invalid canonical records cannot be retried as
migration records. Duplicate top-level diagnostic fields are rejected. Mixed
canonical/migration arrays are rejected by the result validator.

`migrate_diagnostic` currently maps only the two observed Slice 1025 reasons:
`parse-error`/`unexpected-eof` and `merge-conflict`/`edit-edit`. Unknown reasons
fail closed, without message matching or mechanical punctuation replacement.
The caller supplies actual origin evidence; versions and native codes are not
invented. Exact source identity comes from the normalized request. The entire
original record is retained in a versioned migration extension, including its
unknown fields, while its message and metadata remain available on the new
record. Missing conflict alternatives, subject associations or causal facts
are not manufactured by this diagnostic-only conversion.

## Evidence and remaining work

Tests round-trip and validate every diagnostic in the Slice 1028 fixture,
including native parser failure, its causal classification failure, unresolved
conflict and resolved conflict audit diagnostics. Adversarial cases cover
sequence gaps, duplicate IDs, forward/self causes, missing related/subject
references, phase reversal, source corruption, native-code substitution,
schema downgrade, duplicate schema fields and unknown-field forwarding.
Migration is exercised through serialization and validation of the existing
operation result.

This does not prove ID-generation provenance or stable discovery/tie-breaking
across real parallel provider runs: a single serialized ID cannot prove it was
not generated from time or a parser address. Provider projection/execution
must supply that evidence. Recovery negotiation, policy-elevated warnings,
canonical conflict alternatives/resolution, complete subject namespaces,
runtime failure projection, generated bindings and consumer adoption remain
open. No full Slice 1028 conformance, artifact or release gate is claimed.
