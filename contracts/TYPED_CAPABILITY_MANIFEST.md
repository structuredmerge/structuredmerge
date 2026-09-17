# Typed capability manifest

The Rust facade's `capability_manifest(queries, limits)` composes operation
declarations, parser inventory, and explicitly requested parser observations.
Its schema is `structuredmerge.typed-capability-manifest/v1`; it is not the old
prototype's JSON-string manifest and does not preserve its production claims.
Ruby/Python export and installed-consumer validation remain pending.

An empty query list returns all eight implemented common-operation profile
declarations and a parser inventory without probing. Nonempty queries name a
profile, operation, optional dialect, and ordinary TreeHaver parser selection.
Unknown profile IDs reject the entire batch before callbacks. An undeclared
operation or dialect is reported with no probe and unknown parser eligibility.
Queries are bounded by `ParseLimits.max_batch_items`. Byte/node limits do not
apply because there is no source input. Cancellation and deadlines apply even
to empty requests, and around callbacks; callbacks are cooperative, not forcibly
terminated.

All reports use one immutable registry snapshot. Inventory generation/digest
match each selection report even if a host unregisters or replaces itself while
probing. A returned observation is not a reservation of that parser for a later
operation. Descriptor identity, selection rejections, availability/loadability,
and probe faults remain in the ordinary TreeHaver reports.

For declared operation/dialect combinations, parser language and requirements
match common-operation dispatch: JSONC/JSON5 select the `json5` parser language,
TSX selects `tsx`, and native owner profiles require the native-extension channel.
JSON analysis additionally requires comments and diagnostics. Parser options are
shared with the execution path rather than independently advertised defaults.

The manifest deliberately separates three facts:

- `operation_declared` / `dialect_declared`: implemented profile scope, still
  constrained by the catalog's syntax and policy limitations.
- `parser_eligible`: a parser was selected under the recorded request. `None`
  means no probe was attempted for an unsupported combination. `false` is not
  synonymous with "not installed": inspect candidate rejections and faults.
- `approved_as_default`: the profile's explicit authority decision, currently
  false for every profile. Successful probing never promotes it.

No source is parsed and no merge runs. Eligibility does not prove native facts
contain a required extension schema, source syntax/policy support, preservation,
semantic correctness, or release readiness. Those checks still belong to actual
operation execution and the plan's conformance/authority gates. The catalog's
`parser_available` fields remain unknown; request-specific observations do not
rewrite static declarations into unconditional availability claims.

Tests in `crates/structuredmerge-core/tests/capabilities.rs` cover non-loading
inventory, invalid-batch preflight, unsupported combinations, parser-language
mapping/options, reentrant removal with snapshot consistency, unavailable/faulted
selection, serialization, resource limits, cancellation, and deadlines. Existing
common-operation tests guard the shared option refactor. Binding exposure and
cross-runtime tests are the next integration step, not established by Rust tests.
