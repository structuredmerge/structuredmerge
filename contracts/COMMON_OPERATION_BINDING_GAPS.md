# Common-operation binding integration status

The Rust facade now exposes `execute_operation` and
`execute_operation_controlled`, accepting the existing common `OperationRequest`
and returning `OperationResult`. They normalize inline content/bytes, use the
registered `ParserHost` through TreeHaver and apply one execution context across
normalization and execution. References are rejected without opening files.
Rust-only closure/resolver and evidence-validation methods are not binding APIs.

## Generator trial

A local Alef Ruby/Python generation trial included the common request, policy,
result, diagnostic and conflict DTOs plus the two functions. It generated output
but neither native target compiled. The active `alef.toml` and generated bindings
were restored using Alef to the preceding working export set. No generated code
was hand-patched; no prototype or JSON operation fallback was added.

Resolved configuration issues during the trial:

- Exported DTO map fields now use explicit map types instead of the `Metadata`
  alias, which Alef treated as an opaque object. This is the identical Rust type
  and serde contract, not a second transport representation.
- Exclude Rust-only validation methods and their internal dependency types from
  the candidate binding surface. They otherwise pulled closures, source maps and
  registry execution types into generated code.

Remaining observed compilation issues:

- Enum-keyed maps (`SourceRole` to operation source/span) need usable generated
  key equality/hashing and conversion of keys in both directions. Generated
  conversion left core enum keys in maps requiring binding enum keys.
- Nested `Vec<BTreeMap<String, Value>>` conversion does not recursively convert
  the binding's map/value representation.
- Generated defaults assume required data-enum fields have `Default` in Python;
  Ruby untagged-record lowering also assumed defaults for required diagnostic
  and conflict records. Invalid fabricated defaults are not an acceptable fix.

The Python policy constructor also currently serializes dictionaries/kwargs via
Python `json.dumps`; typed variant construction must be checked before claiming
an ergonomic typed input boundary. Successful compilation will not by itself
close constructor, sum-type round-trip, canonical-record, or installed-call gates.

Reproduction evidence is retained locally under kernel `tmp/`:
`common-bindings-config.patch`, `common-bindings-generation.log`,
`common-bindings-python-build.log`, and `common-bindings-ruby-build.log`.
The patch is the candidate configuration, not an approved second configuration
or a release artifact. Fixes must live in the generator, with regression tests;
common contracts must not be weakened to fit generated code. Alef changes remain
local under the maintainer's instruction. Upstream-only generation and release
readiness are still unmet.
