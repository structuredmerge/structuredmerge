# Typed TypeScript/TSX declaration engine

`typescript_merge::typed` consumes validated TreeHaver results and reuses the
existing family owner policy. Native node identities and complete wrapper spans
are retained during projection; no host source searches, new parser registry,
prototype adapter or host merge decisions are introduced.

The existing kinds are class, enum, function, function signature, interface,
internal module, type alias and variable declarator. Supported single-owner
export/ambient/lexical wrappers retain their entire source range. Imports and
comments remain unowned layout; declaration bodies, including JSX in TSX, are
opaque. Native name-field fragments provide identities; a destructuring pattern
such as `{ a }` is one opaque variable identity, not separate binding ownership.

This preserves existing limitations: nested wrappers such as `export const`,
multi-declarator lexical statements, duplicate identities/overload sets, unsupported
top-level expressions and ownerless documents fail closed. `declare namespace`
works through its ambient wrapper; plain `namespace` is an unsupported expression
wrapper with the current grammar/projector. This is not full TypeScript semantic
analysis, import resolution, overload merging or compiler-provider promotion.

## Execution and evidence

The shared `merge_parsed_sources` helper enforces distinct semantic source roles,
matching backend descriptors and source-bound owner analysis. TypeScript uses the
existing generic owner engine, not the Go/Rust membership guard. Every clean
result, including unchanged/whole-source selections, gets a fresh output parse
with matching bytes, output identity/role and backend. Verification failure cannot
retain clean output or source-retention proof.

Four typed tests cover all eight kinds and whole-wrapper native IDs/spans, UTF-8,
TypeScript/TSX legacy merge3 parity, JSX, independent edits, conflicts, additions,
no-op/whole-source output verification, unsupported wrappers, opaque binding-pattern
identity, wrong/mixed parsers, and output identity/bytes/backend/service failures.
These checks do not authenticate arbitrary foreign parser claims.

Common TypeScript operations with explicit dialect selection, directional merge2,
shared fixtures, installed bindings and Ruby consumer migration remain next.
Full-language/golden-master/downstream, native compiler provider, platform/ABI,
publication and default-authority gates remain separate.
