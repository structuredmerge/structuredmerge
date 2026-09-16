# JSON family typed-parser migration

`json_merge::typed` supplies analysis, directional merge2 and base-aware merge3
over validated TreeHaver `ParsedResult` values. This is the existing nested
JSON/JSONC/JSON5 engine, not a new top-level-only profile or a host implementation.
The old source-taking entry points remain wrappers around the same analyzer,
planner and renderer while repository consumers migrate.

The family adapts typed node facts internally to its existing analyzer's node
view. Text comes only from validated byte spans; field names come from child
edges. This view is neither a public JSON-string transport nor a second parser
registry. The typed path does not load grammars or call the old parser entry
point. Dialect validation, nested matching, conflict decisions, comments/layout,
current-preferred arrays and source-preserving edits remain Rust-owned.

Merge2 requires distinct Incoming/Current source IDs and never synthesizes a
base. Merge3 requires distinct Base/Ours/Theirs sources. Every successful render,
including selected-input/no-op outcomes, invokes a caller-supplied output parser.
The result must carry Output role, a non-input source ID, exact rendered bytes,
the destination/ours parser descriptor and the planned semantic value. Failed
verification returns no output. The orchestrator must use the same TreeHaver
snapshot and propagate resource/cancellation controls; this family callback
interface does not itself prove registry generation or preempt recursive work.

Five native typed integration tests cover nested independent edits/conflicts,
directional current preservation, arrays, Unicode/CRLF, JSONC/JSON5 comments,
dialect rejection, unsuccessful parses, swapped roles, and output verification
failure/byte/backend/role rejection. Analysis and commented merges match the
existing engine. All JSON tests (24 unit, 14 fixture, 5 typed), five Git adapter
tests, TreeHaver regression, six opt-in provider tests and strict Clippy pass.
Logs: `tmp/typed-json-{family-regression,engine-tests,provider-regression,engine-clippy}.log`.

These tests uncovered and fixed a TreeHaver provider contract issue: native
Comment-role nodes require comment index records even when optional comment
enrichment is not requested. Enrichment controls do not suppress those facts.

Next: orchestrate this path from the common typed operation facade, add canonical
analysis/change/conflict/preservation projections, generate binding fixtures and
migrate the Ruby family/Git consumers. Their prototype calls and Ruby-owned
diff decisions are not considered migrated by this prerequisite. No public core
API signature, parser default or publication authority changed here.
