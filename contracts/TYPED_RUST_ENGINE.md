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

## Common operations

`kernel.rust.owners.v1` selects `kernel.rust` for analyze, diff2, merge2 and merge3.
Selection accepts only the Rust family and absent/Rust dialect. Unsupported
selectors, policies and marker options fail closed. Parsing uses the
existing TreeHaver registry; no alternate registry is involved.

Analysis retains native references and revalidates embedded owner claims. Diff2
reports exact whole-owner changes plus a complete-source summary when bytes
change, including unowned use/comment layout. These overlapping summaries are
not an edit script. Merge3 preserves the family engine's decisions and requires
fresh output verification, including unchanged and whole-source selections.

The common conflict projector shares transport validation between Go and Rust,
but calls each family's own membership predicate on source-bound native facts.
The Rust guard produces `rust.membership_with_owner_edit`, with verified full
source alternatives and a whole-document subject. It fabricates no owner,
decision ID or output localization. Common classification/base participation is
true for this family decision; generic `owner_classification` remains null.
Messages/categories alone cannot establish this evidence. General foreign-parser
authentication and full semantic validation remain separate gates.

Nine common Rust tests cover facade selection/cancellation, analysis tampering,
owner/use/comment diff, native merge3 parity and fresh output parsing, guarded
addition/deletion and shortcut cases, unsupported syntax/policies, directional
insertion and forged guard evidence. Existing Go tests cover the shared projector
regression. A family test rejects forged directional ownership even on no-op paths.

## Directional merge2

The Rust-owned planner implements `template-into-current` / `source-preserving`:
retain every current byte and shared declaration, inserting incoming-only owners
before the next shared anchor or before the current footer. Native top-level
declarations define ownership. Native comments define trivia; inner documentation
markers keep module docs at module scope, while outer docs and ordinary leading
comments travel with additions. Inline comments stay on the preceding full line.
Byte scanning only identifies newline framing, never Rust syntax.

Use declarations are module-scoped barriers, including when interleaved with
owners. Additions require the exact ordered use-declaration fragments to agree;
the planner neither transfers imports nor resolves dependencies. No-addition
requests preserve current even with differing imports or a missing final newline.
Empty/comment/use-only documents are valid directional endpoints without widening
analysis/merge3's supported-owner requirement. All nine existing owner kinds are
supported; attributes/macros/impl blocks and duplicate identities remain rejected.
Reordered shared anchors with additions, same-line declarations and missing newline
boundaries fail closed. Separators are never fabricated as source retention.

The shared executor verifies a fresh output parse, owner order/fingerprints and
the exact source partition. Tests check current-byte retention, Unicode/CRLF,
all nine declaration kinds, interleaved imports, inner/outer line/block docs,
empty endpoints, no-op import differences and unsupported placement/dependencies.
These are structural/source guarantees, not Rust compilation or semantic name
resolution guarantees, nor a claim of parity with legacy directional precedence.

## Shared fixtures and installed bindings

Fifteen canonical `polyglot/typed-core/rust_*.json` fixtures exercise all four
operations through Alef-generated Ruby/Python suites. They cover native analysis,
rejected attributes, owner/use diff, current-preferred insertion, module/owner docs,
empty/use-only endpoints, interleaved imports, mismatched-use rejection, independent
merge3, ordinary conflicts, unchanged output and membership guards including a
base-equals-side shortcut. Helpers only construct typed requests and manage the
existing parser registry; host code makes no merge decisions.

Rebuilt isolated development packages pass on Linux x86_64: Python 3.14.2 (35
runtime tests and 73 generated cases, LibCST 1.9.0), Ruby 4.0.6 / ABI 4.0.0 (31
runtime tests and 70 generated cases). Package contents, reviewed API surfaces,
Ruby linkage and RBS validation pass. The local Alef bridge generated eight new
test files; optional `poly fmt` was unavailable, so that formatting gate is not
claimed. No Alef changes were pushed and no core package was published.

## Ruby consumer migration

Ruby main `45cc6000d` migrates the opt-in `rust.rust` provider to the shared
`Ast::Merge::TypedCoreProvider`. Its `RustHostProvider` name remains for call-site
compatibility, but it no longer inherits or loads legacy host glue. All four
operations execute typed Rust decisions; the adapter transports native owner
identities/byte spans, actual verification and complete canonical conflict records.
Neutral Git framing is accepted; custom labels/marker widths fail closed.

The installed-core bundle passes all 47 Rust examples, including 18 focused
provider cases. Native merge3 comparisons cover functions, named items, comments,
membership conflicts and actual reordered declarations (the old test's quoted
newline substitutions were inert). Native merge2 prefers incoming while typed
merge2 retains current: both outcomes are explicitly asserted, not labeled parity.
Tests additionally cover UTF-8 spans, docs/import/empty-file insertion, full-document
guard evidence before shortcuts, deterministic no-op reparsing, layout diffs and
unsupported inputs. No prototype gem is loaded. Go/JSON/Bash regressions pass.
Real Git integration passes 98 examples with two TypeScript-only pending cases;
both Rust clean and conflicting driver paths now run against the installed core.

Native defaults remain unchanged. Installed development artifacts and local
consumer tests do not establish source-gem distribution or the complete platform/ABI matrix.
Full language/golden-master/downstream authority, native parser promotion,
publication and default approval remain separate gates.
