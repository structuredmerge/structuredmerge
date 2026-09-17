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
run.

Alef now generates the Ruby/Python registry functions, workflow DTOs and trait
bridges. The callback receives a native `OperationControl` whose cloned Rust
handle shares the caller's cancellation state; it is not a serialized control
message. Python provider descriptors and parser requirements are native
re-exports, so their public and callback/result identities agree. Three local-only
Alef fixes supply native opaque callback arguments and resolve shared `host`
parameter names by trait type before name fallback, and retain serializable
nested request records containing data enums.

Installed-artifact tests exercise LibCST/Psych analysis callbacks over two
TreeHaver-prepared operations, Unicode/BOM source identity, typed results, one
coarse host callback and cancellation from inside that callback. The analysis
is a native-fact node count, not a migrated merge implementation. Broad runtime
lifecycle/stress checks and independently packaged native workflow providers
remain open. Build and installed-consumer evidence is retained under
`tmp/workflow-bindings-*.log`. Completed isolated runs pass 49 Python boundary
tests, 113 generated e2e tests and 114 test-app tests; Ruby passes 42 boundary
examples and 110 examples in each generated suite. Python uses 3.14.2/LibCST
1.9.0; Ruby uses 4.0.6. Reports are retained at
`tmp/core-python-artifact-cqc1qgob/report.json` and
`tmp/core-ruby-artifact-20260917-2383726-b0ukqd/report.json`.
The successful Python rebuild is `tmp/workflow-bindings-python-build.log`;
the earlier combined build log retains the diagnosed, now-fixed generator error.
Both API baselines, all 40 workspace-script tests and local Alef verification
pass. Verification's presence-only/declared-ownership entries are not content
checks. Local Alef commits `a82b77e`, `8e40b19` and `16b8ed0` remain unpushed;
464 trait-bridge and 239 PyO3 generator tests pass.
The test-only Psych projector probes whether the installed parser counts a BOM
in character columns; it always parses the original document bytes. This keeps
the isolated default Psych 5.3.1 and local Psych 5.5.0 byte ranges consistent.

The initial error path returns `CoreError`; full portable failure/selection
envelopes remain open. Host availability, versioned parser-profile support,
family-default workflow execution, allowed delegation, complete portable batch
conformance and broad binding-runtime guarantees also remain open. No existing
kernel profile or native default changes, and no new package is published.
