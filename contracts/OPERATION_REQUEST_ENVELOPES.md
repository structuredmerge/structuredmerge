# Versioned operation request normalization

`structuredmerge_core::operation` implements the Rust request side of Slice
1025. It does not depend on the host-prototype crate. `OperationRequest` uses
the fixture's role-keyed source map and separate provider/parser selections.
`OperationPolicy` selects a typed analyze, diff2, merge2, or merge3 policy;
serde projects the discriminator and policy into the existing wire shape.

Normalization verifies schema, request identity, exact source roles, source
IDs, selection shape, source content exclusivity, encoding, byte length, and
SHA-256 before returning immutable source documents. Supplied BOM, line-ending,
and final-newline evidence must match the bytes. Missing layout evidence is
computed, not guessed or normalized. Binary bytes do not pass through strings.
Duplicate source role keys are rejected during deserialization.

Inline sources use `content` for UTF-8 or `bytes` for exact bytes. References
require an explicit caller-supplied local content resolver. The kernel does
not interpret a reference as a path or URL, access a filesystem, or download
anything. It bounds the declared source bytes before resolution, passes the
declared size to the resolver, and checks returned bytes independently. The
resolver must enforce its allocation bound; this API cannot interrupt a
misbehaving callback. Transport decoding and extension-memory limits remain
the caller's responsibility.

The validated request retains all policy, selection constraints, metadata,
namespaced extensions, and compatible unknown fields. Normalization is **not
capability negotiation or execution authorization**. Executors must handle or
reject unsupported policy/selection requirements; they must not discard them
by projecting into a narrower development request. No automatic conversion to
`NativeDiffRequest` or native merge arguments is supplied. Missing merge
fallback policy means `none` without changing the forwarded wire record.

`validate_operation_batch` preserves input order, enforces unique nonempty
request IDs within the batch, and shares request-count/source-byte budgets.
It returns no partial normalized batch on failure and performs no dispatch.
Global request-ID uniqueness across processes remains the caller's obligation.

## Evidence and remaining integration

`tests/operation_envelopes.rs` loads the shared Slice 1025 fixture and checks
all four request round trips and exact resolved bytes. Additional tests cover
forward-compatible fields/extensions, role mismatches, source corruption,
binary/multibyte layout, explicit resolver failures, selection separation,
batch ordering/isolation/limits, and duplicate serialized role keys.

This is a Rust normalization component, not full Slice 1025 conformance.
The common result envelope, operation execution/registry negotiation,
portable failure diagnostic projection, cancellation/deadline integration,
and generated binding/consumer adoption remain open. Existing `OperationInputs`
is an earlier minimal input validator, not the transport contract. The native
diff and merge development APIs likewise remain separate until they execute
this common request and produce its common result without losing evidence.
No new binding, provider capability, default authority, or release gate is
claimed by these tests.
