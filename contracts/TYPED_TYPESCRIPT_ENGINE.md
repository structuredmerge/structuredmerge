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

## Common operations and dialect selection

`kernel.typescript.owners.v1` selects `kernel.typescript` for analyze, diff2, merge2
and merge3. Family identity stays `typescript`; omitted/`typescript` dialect selects
the TypeScript grammar and `tsx` selects the TSX grammar. Parser selection,
execution and output verification use that requested grammar. An explicitly
selected TypeScript-only backend cannot satisfy TSX. Embedded analysis validation
checks the requested grammar against parser evidence and reruns owner projection.
Unknown dialects and a `tsx` family alias are rejected, not silently normalized.

Diff2 includes exact whole-owner changes and a complete-source summary for changed
import/comment/layout bytes; overlapping summaries are not edit instructions.
Merge3 retains the native generic engine's decisions and canonical owner conflicts,
including its membership behavior. Clean results require a fresh matching output
parse, including no-ops. Custom marker policies fail closed.

Eight common tests cover both dialects, native merge3 parity and canonical conflicts,
native identity/dialect tampering, owner/import/comment diffs, JSX analysis/diff/
merge and output reparsing, wrong/missing backend selection, cancellation, default
dialect selection, unsupported policies and directional merges. These are source-level common
API tests, not installed package or foreign-parser authentication gates.

## Directional merge2

The Rust-owned TypeScript planner implements current-preferred insertion with
explicit incoming/current roles, retaining every current byte and shared owner.
Incoming-only declarations keep their full native wrappers and are inserted before
the next shared anchor, or before the current footer. The exact ordered native
import fragments must match when adding owners, including interleaved imports.
Imports are never copied as declaration trivia or semantically reconciled.

Comments before the first declaration/import are document headers: current headers
stay before inserted statements and incoming headers are not transferred. An
ownerless comment-only current document is all header. After imports/owners,
native leading comments travel with additions and inline comments stay with the
preceding full line. This positional contract does not interpret compiler
directives or claim semantic JSDoc attachment. Byte scans locate newline framing
only; native nodes define syntax, imports, comments and owners.

Empty/import-only documents are directional endpoints without widening analyze or
merge3. With no additions, current remains exact even with differing imports or
missing final newline. With additions, reordered anchors, same-line declarations
and absent newline boundaries fail closed. The existing unsupported wrappers and
ambiguous owners remain rejected. The shared executor reparses using the selected
grammar and verifies owner order/fingerprints and exact source retention; a valid
parse alone is not sufficient. No base or separators are fabricated.

Tests cover both dialects, all eight owner kinds, JSX, headers, comments, imports,
empty endpoints, UTF-8/CRLF partitions, rejected placement/dependencies and forged
ownership before the no-addition shortcut. This is not compilation, import/name
resolution, directive interpretation or parity with legacy directional precedence.

## Shared fixtures and installed bindings

Twenty-eight canonical TypeScript/TSX fixtures now exercise all four operations
through Alef-generated Ruby/Python tests. Each grammar runs native analysis,
unsupported-wrapper rejection, owner/import diff, directional current preference,
document headers, comments, interleaved imports, import mismatch rejection and
independent/conflicting/unchanged merge3. Additional JSX cases verify TSX analysis
and merges and TypeScript rejection. Helpers select the requested grammar and
transport typed requests; they contain no ownership or merge decisions.

Rebuilt isolated Linux x86_64 packages pass: Python 3.14.2 / LibCST 1.9.0 runs 35
runtime tests and 101 generated cases; Ruby 4.0.6 / ABI 4.0.0 runs 31 runtime tests
and 98 generated cases. Package contents, reviewed API surfaces, Ruby linkage and
RBS validation pass. Eight new test files were generated with the local Alef
bridge; optional `poly fmt` was unavailable, so that formatting gate is not claimed.

Actual Ruby TypeScript-provider migration remains next. These development artifacts
do not establish source-gem distribution, the full platform matrix or publication
readiness. No prototype files/logic were added and no Alef changes were pushed.
Full-language/golden-master/downstream, native compiler provider, platform/ABI,
publication and default-authority gates remain separate.
