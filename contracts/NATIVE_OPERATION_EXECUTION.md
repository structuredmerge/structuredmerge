# Native common-operation execution

`structuredmerge_core::native_operation::execute_native_operation` executes a
validated common request using an immutable TreeHaver parser-registry snapshot
and shared execution controls. Rust owns owner analysis, diff and merge decisions.
This is an explicit experimental profile dispatcher, not a general provider
registry, generated binding export, or default-backend change.

Supported profiles are `kernel.yaml.native_mapping.v1` and
`kernel.python.native_declarations.v1`, with their existing bounded syntax and
layout restrictions. Supported operations are exact-source owner `diff2` and
source-preserving `merge3` without fallback. Requests must name a profile.
Unsupported operations, selection constraints, policy fields and required
extension capabilities fail before parsing. Parser selection remains TreeHaver's
responsibility; explicit backend requests do not trigger substitution.

Results retain request identity, passive extensions and metadata. Unknown request
fields and original selection/policy fields are nested under `request_forwarding`
so they cannot shadow reserved result fields. Parsed inputs retain their native
facts and selection evidence. Public parser-service errors use stable codes and
do not copy native exception messages.

Diff results retain both revision roles, exact owner changes, ordering and layout
evidence, without merged output. Merge conflicts use executed Rust decisions and
the canonical conflict projector. Clean merges retain verified source segments
and actual output parse evidence. Even a whole-source selection receives a fresh
native parse and owner comparison; reusing an input parse is not called reparsing.
Output source IDs use the engine's collision-free identity. Cancellation or
deadline expiry discards completed output before returning a failure result.

`exact-source-partition` preservation and structural equivalence refer only to
the implemented owner profile. They are not proofs of general language semantic
equivalence. Marker options are preserved but no conflict markers are emitted.
Complete common change/span projection, native diagnostic projection, provider
registry/policy coverage, analyze/merge2, generated binding adoption and full
portable conformance remain open. The entry point does not authorize filesystem
writes, package publication or default cutover.

The explicit CI gate `cargo test -p structuredmerge-core --test native_operations
--locked -- --ignored` runs real Ruby/Psych subprocesses. It covers composition,
whole-source output reparsing, conflict evidence, diff, malformed/unsupported
inputs, unsupported requirements, cancellation, deadlines, native output faults,
source-ID collisions and reserved result-field isolation. It is not an installed
Ruby/Python artifact test; Python dispatcher coverage remains to be added.
