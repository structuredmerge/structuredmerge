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

This is an engine building block, not a common operation-result implementation.
Canonical conflict projection for the Go-specific guard, typed analyze/diff2 and
directional merge2 orchestration, shared installed binding fixtures and consumer
migration remain open. In particular, merely routing Go through the common generic
owner path would bypass the family guard and is not an acceptable integration.
No legacy consumer is marked migrated by this change; no default or publication
authority is transferred.

## Validation

Five typed tests cover actual native references and unowned headers, legacy
merge3 parity, fresh no-op output parsing, membership additions/deletions combined
with edits, rejected syntax/roles, wrong output identity/bytes/backend and parser
service failure. Shared projector regressions retain the document-only result and
check native reference identity. Test counts alone do not establish broad parity.
