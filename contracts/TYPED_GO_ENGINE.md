# Typed Go owner engine

`go_merge::typed` consumes validated TreeHaver `ParsedResult` values without
registering a separate parser or calling the prototype facade. The existing
named-owner projector now also returns native references captured while it
establishes each owner. Its original document-only API delegates to that result;
ownership policy is unchanged for other families using the shared projector.

## Supported boundary

The current source-preserving Go engine owns top-level function declarations.
Package/import declarations and comments remain unowned source layout; this is
not import reconciliation or semantic comment attachment. Other declarations,
duplicate function identities, invalid trees and documents without supported
owners reject analysis. This preserves the existing conservative engine boundary,
not full Go-language authority or a substitute for Go-native parser providers.

Typed analysis checks parser language support, successful source-bound normalized
facts, native owner references, exact spans and source bytes. Native node IDs are
recorded during projection, not recovered by source searches or guessed offsets.

## Three-way execution

`merge3` requires distinct base/ours/theirs source identities and actual roles.
It invokes the existing Go family guard **before** generic owner merge shortcuts:
owner membership changes together with an existing owner edit return the original
`unmanaged_source_change` conflict. That path has no fabricated owner-classifier
decision, output parse or source-retention segments.

Remaining cases use the shared source-preserving owner engine. Clean output must
have a fresh parse, including exact/no-op/whole-source selections. Output role,
identity, exact bytes, selected backend and supported ownership are checked.
Failed output verification cannot retain clean output or its source proof.

The engine building block is now connected to common operations as described
below. Directional merge2, shared installed binding fixtures and consumer migration
remain open. No legacy consumer is marked migrated; no default or publication
authority is transferred.

## Validation

Five typed tests cover actual native references and unowned headers, legacy
merge3 parity, fresh no-op output parsing, membership additions/deletions combined
with edits, rejected syntax/roles, wrong output identity/bytes/backend and parser
service failure. Shared projector regressions retain the document-only result and
check native reference identity. Test counts alone do not establish broad parity.

## Common operation profile

`kernel.go.owners.v1`, provider `kernel.go`, supports analyze, diff2 and merge3
through the existing typed facade, parser registry, limits and cancellation.
Selection permits an omitted dialect or explicit `go`. Unsupported constraints,
marker options and merge2 reject; no directional base is fabricated.

Analysis projects native owner IDs/spans, embedded parse facts and exact layout.
Embedded validation reconstructs family ownership and rejects altered owner
references while preserving passive fields. Comments remain in native facts and
layout; requested semantic comment/token enrichment is not silently discarded.
Diff2 uses Rust owner decisions plus a whole-document byte summary so package,
import or comment-only changes remain visible. Summaries overlap owner changes
and are not edit scripts or import-merge semantics.

Shared native merge orchestration now accepts a Rust family engine alongside its
analyzer. Existing callers retain the generic default; Go selects its guarded
engine. This is not an FFI callback for host-owned merge decisions or a new registry.
The guard conflict projector checks source-bound parses, recomputes the family
predicate, and checks the actual conflict result before projecting canonical
`ownership` conflict code `go.membership_with_owner_edit`. All three alternatives
carry exact full-document ranges/digests. Localization is whole-document, not a
fabricated owner range. No owner decision IDs or render fragments are invented.
Common `classification_reached` is true for this family conflict decision;
`owner_classification` remains null, correctly distinguishing the generic classifier.

Seven common-profile tests exercise analysis validation, owner/document diffs,
clean/no-op/ordinary-conflict merge3, the family guard including a base-equals-side
shortcut hazard, forged guard messages/sources/provider identity, unsupported
requests, cancellation and exported facade selection. Broader data-only semantic
result verification, complete Go conformance and installed Ruby/Python evidence
remain separate gates; executor projection checks are not foreign-result authentication.
