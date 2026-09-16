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
