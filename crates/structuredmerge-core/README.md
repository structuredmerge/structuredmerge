# StructuredMerge core

The typed facade for the shared Rust kernel and its generated language bindings.
It has no dependency on `structuredmerge-host-prototype-core`.

Implementation starts with the source/operation boundary from specification
Slices 722 and 1024–1027:

- immutable, validated source storage is owned by TreeHaver;
- source bytes, digests, encoding and line-ending evidence are preserved;
- semantic roles distinguish analyze, diff2, directional merge2 and merge3;
- merge-provider and parser-backend selections are separate typed values.

`OperationInputs` validates inputs before provider dispatch. It is not yet the
complete operation envelope: policy, extensions, result evidence, parser
selection, and merge execution follow in subsequent implementation slices.
The facade now exposes typed parser-host registration and `parse_sources`.
TreeHaver owns the registry, source validation, selection, dispatch, and result
validation. Callback batches contain generated records, not whole-operation
JSON strings. Native syntax failures retain their parse diagnostics.

Ruby and Python bindings are generated from the active `alef.toml`. Initial
Ruby/Psych and installed Python-wheel/LibCST callback smoke tests pass with
targeted local Alef fixes. Upstream Alef 0.89.0 cannot yet reproduce compiling
bindings for this surface: callback-only return DTO conversions and PyO3
trait-parameter construction/error conversion require upstream fixes. This is
development evidence, not an upstream-generation or publication gate pass.

The development generator is upstream v0.89.0 plus local commit `acd0469`
in the separate `tmp/alef-typed-bridge` checkout at workspace level. Its
`target/debug/alef generate --skip-compile --clean` produced this checkpoint;
generated sources were not hand-edited. `alef.toml` still names the official
release, not a fork dependency. Reproducible generation with the declared
upstream pin is therefore an **open gate**, not a passing check. Replace this
development checkpoint with output from a released upstream fix before release.

The facade also exposes `merge_yaml_mapping`, a concrete projection of the
existing YAML block-mapping operation. Generated Ruby/Psych tests exercise real
Rust-owned analysis, matching, conflicts, and rendering. Shared result fields
are retained, and native syntax rejection carries a typed parse result with its
source role. This profile-specific entry point is not the complete Slice 1025
provider-result envelope; service and unsupported-profile failures still use
the preliminary `CoreError` bridge.

`merge_python_declarations` now derives whole top-level Python owners in Rust
from native LibCST syntax facts. It shares parse/verification orchestration and
the owner merge engine with YAML. Installed-wheel tests cover assignments,
function/class bodies, conflicts, syntax failure, exact-byte retention, and
unsupported forms. See [`python-merge`](../python-merge/README.md) for the bounded
profile and remaining gates. Both operations return `NativeMergeResult`.

The complete operation and typed failure/preservation envelopes,
runtime lifecycle stress tests, clean Ruby artifact installation, broader
Python public DTO ergonomics, and registry publication remain unfinished.
Legacy Ruby packaging files still coexist with the new package and must be
removed or isolated before publication; the current gem file glob must not
ship the inherited prototype files.

The next gates include clean Ruby installed-artifact evidence and complete
operation/preservation contracts for both runtimes. Passing an encoded
operation through a host-owned merge does not meet that requirement. Existing
kernel mechanics should be reused; new parallel merge algorithms and
independent parser-selection registries are not intended.

Run the boundary tests with:

```sh
cargo test -p structuredmerge-core -p tree-haver --locked
```
