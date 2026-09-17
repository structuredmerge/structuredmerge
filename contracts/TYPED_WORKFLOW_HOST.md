# Typed WorkflowHost implementation status

`structuredmerge_core::workflow` implements the initial Rust boundary specified
by the spec repository's `TYPED_WORKFLOW_HOST_CONTRACT.md`. It uses
`ast_merge::provider_registry` and `provider_selection`; no prototype dependency
or additional registry implementation is introduced.

The facade exports registration, generation-checked replacement/removal,
inventory and explicit-provider batch execution, with a controlled variant.
Hosts receive existing typed operation requests and TreeHaver-validated parse
facts, then return existing typed operation results in one coarse batch. The
kernel marks execution host-owned and not approved as default.

Tests use instrumented Rust providers and an analysis callback that computes
its result from supplied parse facts. They prove request/source correlation,
coarse batching, opaque request forwarding, cumulative limits, malformed-batch
rejection before probes, prepared-payload limits, returned identity/cardinality
validation, no retry, fault redaction, reserved-field shadowing rejection,
cancellation precedence and in-flight registry retirement. They are boundary
tests, not native parser or merge semantic parity evidence.

Verification is recorded in `tmp/workflow-final-tests.log`: core and ast-merge
tests and strict Clippy. Existing native opt-in tests are not claimed by this
run. Ruby/Python bindings do not yet expose this interface. Next add the required
Alef exports and generated trait bridges, then validate real native providers
against isolated installed artifacts before declaring that boundary complete.

The initial error path returns `CoreError`; full portable failure/selection
envelopes remain open. Host availability, versioned parser-profile support,
family-default workflow execution, allowed delegation, complete portable batch
conformance and broad binding-runtime guarantees also remain open. No existing
kernel profile or native default changes, and no new package is published.
