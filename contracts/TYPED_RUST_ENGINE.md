# Typed Rust declaration engine

`rust_merge::typed` consumes validated TreeHaver results. It reuses the existing
Rust family owner policy and shared named-owner projector, recording native IDs
during ownership derivation rather than recovering them from source searches.
No prototype facade, new parser registry or host merge logic is involved.

The existing supported owners are const, enum, function, module, static, struct,
trait, type and union declarations, each merged as a whole owner. Use declarations
and comments remain unowned layout. Impl blocks, standalone macros/attributes,
duplicate identities, unsupported syntax and documents without supported owners
fail closed. This is not full Rust language support, import reconciliation,
attribute/macro semantics or nested-body merge authority. The existing syn backend
is not silently promoted into a source-preserving provider without native spans.

## Execution and evidence

Go and Rust now share the validated-parse execution helper in
`ast_merge::typed_merge`, using their separate Rust-owned analysis/merge functions.
The helper requires distinct base/ours/theirs identities, their actual roles,
one selected backend and source-bound owner documents. Family merge guards run
before generic whole-source shortcuts. The Rust membership-change plus owner-edit
guard retains its original conservative conflict with no fabricated generic
classification, source-retention segments or output parse.

Every clean result is freshly parsed, including no-op/whole-source selections.
Output bytes, role, identity, selected backend and source-bound owner analysis are
verified. Failure cannot retain clean output or its proof. The shared helper is
Rust orchestration, not a runtime callback for host-owned merge decisions.

Five typed Rust tests cover all nine owner kinds and native IDs/byte spans,
Unicode and use/comment layout, legacy merge3 parity, guarded additions/deletions
including a base-equals-side shortcut, unsupported constructs and mixed parser/
role inputs, output identity/bytes/backend mismatch and verification service
failure. Go's typed regressions protect behavior during shared-helper extraction.

This foundation is not common Rust operation support or a migrated consumer.
Common analyze/diff2/merge2/merge3, canonical Rust-guard conflicts, shared fixtures,
installed Ruby/Python artifacts and actual Ruby consumer migration remain next.
Full language/golden-master/downstream authority, native parser promotion,
publication and default approval remain separate gates.
