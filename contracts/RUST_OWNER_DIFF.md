# Rust-owned structural diff primitive

`ast_merge::owner_diff::diff_owner_documents` matches family-analyzed owner
identities and classifies added, deleted and edited owners in Rust. This is a
building block for the planned typed analyze/diff2 migration, not a replacement
for the complete Slice 1025 operation/result envelope or an exported binding API.

Inputs are validated `SourceDocument`s with distinct IDs and exact before/after
roles plus corresponding `SourcePreservingOwnerDocument`s. The primitive rejects
stale source bytes, duplicate/empty owner identities, overlapping or invalid
UTF-8 byte ranges and invalid line-range shape before returning any changes.
It reuses the existing owner validation used by the shared merge engine.

Matching uses owner identity, not source-text searches or caller-provided
fingerprints. Equality requires equal exact owner bytes and structural paths.
Changes are ordered by before-source ownership order, followed by after-only
owners. IDs are contiguous, result-local and deterministic for the same inputs.
Each present alternative has its source identity, semantic role, byte range and
SHA-256; absent alternatives have no region. No output is rendered or synthesized.

Unowned regions (including BOM, comments and trailing layout) and complete
before/after ownership order are returned separately. An empty owner-change list
does not mean the documents are identical. The primitive does not yet classify
moves, semantic equivalence, or layout changes as portable change records. It
does not trust host-provided merge decisions; family analysis must establish the
ownership inputs from parsed syntax before calling it.

The Ruby shared merge adapter still uses its old diff implementation. Migrating
it requires typed family analysis with exact source spans, TreeHaver dispatch,
operation envelopes, portable diagnostics, and installed-boundary regression
tests. This primitive does not satisfy those remaining gates or justify changing
the adapter's require name alone.

## Native parser orchestration

`ast_merge::typed_diff::diff_native_sources_with_evidence` now connects the
primitive to TreeHaver's parse service and a Rust family analyzer. It validates
the exact before/after role set, source descriptors, language and resource
limits before parsing, retains input parses in semantic role order, and rejects
cross-backend batches. Native syntax failures precede family analysis failures;
both retain the input roles and validated sources. Stale family analysis is
rejected before classification. Cancellation/deadlines are checked before
dispatch, after parsing, after each analysis and after diff classification.

`yaml_merge::typed::diff_mapping_sources` selects the existing conservative
mapping analyzer. A real Ruby/Psych process integration test proves one parse
batch, Rust-owned edit/add/delete classification, parser selection failure,
source/role/limit rejection before callbacks, syntax and ownership failures,
and discarded late successes/errors after cancellation. Run with:

```sh
cargo test -p yaml-merge --test typed_psych_merge -- --ignored
```

These are native parser process tests, not generated binding tests. The shared
internal failure type is currently `NativeMergeError`; it is reused rather than
inventing a parallel diagnostic vocabulary. Typed core projection, portable
operation/result envelopes, Python-family dispatch, and migration of the Ruby
family adapters remain unfinished. No diff capability was added to the binding
manifest or advertised as a completed public provider operation.
