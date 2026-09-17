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
