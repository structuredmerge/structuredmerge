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

Next: common operation profile, portable analysis/diff/merge evidence, remaining
operation policies, shared generated fixtures, rebuilt installed artifacts and
the actual Ruby consumer migration. No Bash legacy export is marked migrated by
this Rust groundwork. Broader authority, hosted, platform and publication gates
remain open. No parser-default switch, prototype expansion or Alef push.
