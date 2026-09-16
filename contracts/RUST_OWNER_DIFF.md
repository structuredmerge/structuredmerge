# Rust-owned structural diff primitive

`ast_merge::owner_diff::diff_owner_documents` matches family-analyzed owner
identities and classifies added, deleted and edited owners in Rust. This is a
building block for the planned typed analyze/diff2 migration, not a replacement
for the complete Slice 1025 operation/result envelope. A development typed
binding boundary now exposes it as described below.

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
inventing a parallel diagnostic vocabulary. Complete portable
operation/result envelopes and migration of the Ruby family adapters remain
unfinished. No diff capability is advertised as a completed portable provider
operation or approved as a default.

## Generated typed development boundary

`diff_native_owners(NativeDiffRequest, ParseLimits)` and its `_controlled`
counterpart accept request identity, an explicit profile ID and typed parse
requests with before/after sources. The supported profiles are
`kernel.yaml.native_mapping.v1` and `kernel.python.native_declarations.v1`.
Both reuse their Rust family analyzers and the common diff engine. TreeHaver
retains parser selection; the request cannot substitute an arbitrary analyzer
or ask a host to classify changes.

`NativeDiffResult` retains request identity, profile ID, validated source
descriptors, input parse evidence, diagnostics, service failure and analysis
rejections. A successful result contains `OwnerDiff`; a rejected result has no
partial diff. Invalid requests, unsupported profiles, resource limits and
cancellation/deadline failures remain structured core errors. Failure projection
shares the existing core merge error categorization, but never executes a merge.
There is no output field, base synthesis or output verification parse.

Installed Ruby/Psych and Python/LibCST tests cover real edits/additions/deletions,
exact role/range evidence, missing alternatives, input syntax/analysis failures,
role substitution and cancellation before callbacks. These development DTOs
still do not implement the full Slice 1025 schema/policy/extensions/result
envelope. The existing `native_merge_profiles` function remains merge-entry-point
introspection, not a negotiated registry manifest for these diff operations.

Python `ParseRequest` and `ParseOptions` now reexport their native classes,
like the existing source descriptors. This keeps nested request constructors
compatible with objects created through the public namespace. Dataclass-specific
introspection/equality is not retained; these are development API identities,
not yet a stable-version compatibility promise.
