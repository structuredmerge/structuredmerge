# Typed Bash engine migration

`bash_merge::typed::owners` and `merge3` adapt validated TreeHaver parser results
to the existing Bash owner analyzer and shared source-preserving owner merger.
They do not load a parser, create a registry, search source text for owners or
introduce a second merge policy. The original Rust entry points remain intact.

The existing scope is retained: top-level functions, variable assignments and
literal `test_expect_success` calls (including literal prerequisites). Duplicate
identities, unsupported top-level constructs and dynamic test titles fail closed.
Bodies remain whole owners; this is not full Bash semantic authority or nested
body merging. Existing comment/gap handling and conflict classifications remain
the engine's policy, not new host-side decisions.

Input roles and distinct source IDs are mandatory. Clean output, including
whole-source/no-op selections, requires a fresh validated Output parse with exact
bytes, a non-input source ID and the same backend descriptor as ours. Existing
decision/classification and source-byte-segment evidence is retained. A failed
verification never exposes clean output or retention evidence. Conflict results
are not reparsed or rendered into invented marker output.

TreeHaver's `ParsedResult::normalized_nodes` supplies a shared internal Rust view
for existing analyzers. It checks successful source-bound facts and derives each
fragment directly from validated native byte spans while preserving node IDs,
topology, field names, types and semantic roles. Native extensions remain in the
original parse document; the view does not replace or flatten that carrier.
Typed JSON now reuses this helper, removing duplicate conversion code.

Verification: eight existing Bash tests and five new typed tests pass, along with
JSON, TreeHaver, core and Git regressions and strict all-target Clippy. Typed cases
compare existing/typed Bash results, verify exact source-segment bytes/digests,
require fresh no-op verification, and reject unsupported syntax, wrong roles,
changed verification bytes/identity/backend and failed output parsing.
Logs: `tmp/bash-typed-{tests,regressions,clippy,generation,audits}.log`.

## Common operation profile

`kernel.bash.owners.v1`, provider `kernel.bash`, family/dialect `bash`, now executes
analyze, diff2, merge2 and merge3 through the existing common native-owner orchestration.
Explicit parser selection, cancellation and resource bounds remain TreeHaver's
responsibility. No extra registry or binding signature is added. Unsupported
dialects, enrichments, fallbacks and marker/label options fail closed.

Analysis captures native owner node IDs during the original family projection,
not by searching spans or text afterward. Portable analysis carries exact owner
spans/digests, embedded native facts, layout gaps, adjacency and controller records.
The default/exact-source-owners policy does not claim semantic comment attachment:
comments are retained in the embedded parse and source gaps, with comment analysis
declared not requested. Explicit comments/tokens enrichment is rejected. Embedded
analysis validation recomputes native owner/layout decisions and permits passive
compatible fields. Native comment records are validated according to the backend's
declared capability, not discarded because an optional enrichment wasn't requested.

Diff2 retains Rust owner classification/order/layout evidence and adds a complete
document-byte summary whenever sources differ. This summary overlaps owner changes
and is not an edit script or semantic-equivalence claim. Merge3 retains shared
classification, canonical conflicts and byte-segment evidence, with fresh output
verification even for source-selection shortcuts. Canonical decision references
are checked by the trusted executor; a standalone data-only result validator must
not invent the executor's authorization evidence.

Five common Bash tests pass, covering native references and tampering, passive
fields, comment/layout retention, trivia-only diffs, independent/no-op merge3,
conflicts, rejected syntax/policies/selection, exported execution and cancellation.
Broader Bash/core tests and strict Clippy pass. Logs:
`tmp/bash-common-{tests,regressions,clippy,generation,audits}.log` (the final fifth
exported-facade test is included in the regression run).

Next: broader analysis-policy and result-verifier
conformance, shared generated fixtures, rebuilt installed artifacts and actual
Ruby consumer migration. No Bash legacy export is marked migrated by this Rust
work. Broader authority, hosted, platform and publication gates remain open.
No parser-default switch, prototype expansion or Alef push.

## Directional merge2

`template-into-current` / `source-preserving` uses the shared directional owner
classifier and verifier with a Bash-owned insertion planner. Shared identities
keep current owners, including current body edits. Incoming-only owners are
inserted before the next shared anchor, or before the current footer. Every
current byte remains in order. No fabricated base or three-way shortcut is used.
This intentionally does not reproduce the prototype's destination-as-base wrapper,
which selected the whole template even when current had its own edits.

Insertion ranges use native owner/comment facts and newline framing: trailing
inline comments remain with the preceding owner, while later leading gaps follow
the next owner. Document headers (including shebangs) and footers are not imported
from incoming. Current headers/footers remain exact. Empty/comment-only inputs are
recognized from native root children and supported as typed endpoints; this is an
intentional extension beyond the older entry point's no-owner rejection.

Additions with reordered shared anchors, multiple owners on one physical line,
unowned nontrivia separators or absent insertion newline boundaries fail closed.
No newline is synthesized and misrepresented as retained source. With no additions,
current is preserved even without a final newline. Output is freshly reparsed and
checked against classified owner order/fingerprints. Complete directional source
segments record exact bytes/digests and prove ordered retention of all current
bytes. This is a declared whole-owner insertion policy, not broad Bash/Ruby parity.

Seven common Bash tests cover all four exported operations and cancellation,
including directional current precedence, empty inputs, native comments, Unicode,
headers/footers, multi-owner additions, no-op verification and rejected placement.
Bash/core regressions, strict Clippy, reviewed API baselines and 14 audits pass.
Logs: `tmp/bash-merge2-{tests,regressions,clippy,generation,audits}.log`.
Installed-artifact/shared-fixture and Ruby consumer gates remain next, not passed.

## Shared installed binding verification (2026-09-17)

Fixtures revision `8b52ce8` adds eleven shared cases across analyze, diff2, merge2
and merge3. Test-only adapters register explicit Bash parsers and construct typed
requests; no matching/rendering or expected-output logic is embedded in adapters.
Alef generates four test files per language. Cases assert native owner references,
rejected unsupported syntax, owner/trivia-only diff, current preference, comments,
empty-current insertion, rejected anchor order, independent merges, canonical
conflict presence and fresh no-op verification with exact Unicode bytes.

Rebuilt isolated artifacts pass Python **35 runtime + 46 generated** tests and
Ruby **31 runtime + 43 generated** tests. Both execute the same eleven Bash cases.
Wheel SHA-256: `0a345cc06b2dcd65954da6fbeabab739be5c3604e85a6fca09d05b8a79731629`.
Gem SHA-256: `fda56ed1711ca7c70521ae4da2ffc807e39e70d7474418ffc6b93f52a7fcc976`.
Logs: `tmp/bash-shared-{generation,bindings,python-build,ruby-build,python-artifact,ruby-artifact,audits}.log`.
Reviewed API baselines and all 14 audits pass. Optional `poly fmt` is unavailable;
generation does not prove the formatter gate or independent clean reproducibility.

Next: actual Ruby Bash consumer migration and broader analysis/result-verifier
conformance. These local Linux artifact checks do not approve broad family parity,
source-gem, multi-platform, hosted/downstream or publication gates. The Bash legacy
exports remain unmigrated; no default switch, package publication or Alef push.

## Local Ruby consumer migration (2026-09-17)

Ruby commit `e326d7a0e` replaces the opt-in Bash prototype adapter with the shared
typed core transport. Family-specific dialect, parser and owner identity hooks
preserve JSON/Git behavior. Native-owner diff revisions use typed kernel spans,
not host source scans or host classifications. Full typed records, canonical
conflicts and actual verification remain portable Ruby values. Legacy analysis
shapes are intentionally replaced by kernel owner identities and native spans;
merge2 is current-preferred, not the old fake-base template selection.

The adapter accepts Git path context, empty labels and default width 7, but never
renders markers; custom marker requests fail. Native Ruby defaults are unchanged.
The artifact bundle has no prototype dependency, while released-package CI still
requires the real core package and remains a separate gate.

Against the installed gem above, all 13 provider examples pass, including native
merge3 comparisons, byte/line spans after Unicode, added/deleted owners, trivia,
canonical conflicts, invalid syntax/selectors and directional comment retention.
Full Bash: 463 examples, zero failures, two existing removal-mode pending cases.
JSON regression: 93 examples, zero failures. Git regression: 98 examples, zero
failures, three unavailable-family pending cases before strengthening its conflict
tests. The updated real-Git suite then passes 30 examples with six pending cases:
Go/Rust/TypeScript clean and conflict tests now require their actual providers.
Git status 1 alone is not conflict evidence, since a failed driver produces it too.
The available Bash path proves actual conflict classification before checking Git.

Logs: `tmp/bash-ruby-{final,focused-final}.log`,
`tmp/bash-json-merge-regression.log`, `tmp/bash-git-{final,real-final}.log`.
The migration inventory now marks the Bash consumer migrated locally. Broad
golden-master/full-language authority, hosted/released-package, full lint/coverage
and downstream gates remain open. No prototype publication or Alef push occurred.
