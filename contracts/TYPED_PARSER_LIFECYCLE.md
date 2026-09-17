# Typed parser lifecycle

The core uses TreeHaver's registry, not a second host registry. Registration owns
the parser through an `Arc`, caches its validated descriptor and rejects duplicate
IDs. Host descriptor callbacks execute before acquiring the registry lock.
Native language-pack registration does not load a grammar; probe/parse may do so.

Every operation selects from its retained registry snapshot. Removing a provider
changes future snapshots only. An in-flight operation keeps its provider alive
until the operation releases the snapshot, including when cancellation is
reported after an outstanding callback returns. Cancellation does not forcibly
interrupt arbitrary native/foreign callbacks or cancel a later operation.

Re-registering a removed ID creates a new registration. It cannot retarget an
in-flight operation to that provider. Unregister followed by register is **not
atomic replacement**: selection in the gap fails, and each mutation remains
subject to registry generation checks. Unknown IDs and stale generations fail;
callers must not silently turn these errors into successful cleanup.

`crates/structuredmerge-core/tests/parser_host.rs` exercises removal during a
blocked callback, failed selection in the gap, registration of a new provider
under the same ID, retained old identity/results, eventual release of the retired
host, and selection of the new provider. It repeats the lifecycle with cancellation
while the old callback is outstanding. Channels synchronize events without sleeps;
bounded receives prevent an absent callback/release signal from waiting forever.

This is Rust typed-facade evidence, not proof of Ruby/Python GC, foreign-thread
entry, runtime shutdown or every binding's callback affinity. Those gates remain
open. No public API was added to reproduce the abandoned prototype.

## Installed runtime process exit

The Ruby and Python artifact suites each run nine fresh child processes: three
repetitions each with a registered idle provider, an unregistered provider, and
an operation cancelled while its callback is outstanding. Every child first
executes real typed native parsing. The outstanding-callback case synchronizes
entry, unregisters the provider, cancels the operation, releases the callback,
joins the worker and verifies cancellation discarded the late result before exit.
GC runs before the child prints its completion marker and exits normally. The
parent checks both the marker and successful process termination, with a bounded
timeout and cleanup, so reaching the last assertion alone cannot hide an exit hang.

These checks establish the scoped normal-process-exit path in the tested installed
MRI/CPython artifacts. They do not prove interpreter embedding/finalization while
foreign threads remain active, forced interruption of callbacks, subinterpreter
support, exhaustive leak freedom or every platform/runtime combination. Callers
remain responsible for draining outstanding operations before graceful shutdown;
cancellation is not a join. No `start_host_runtime`/`shutdown_host_runtime` facade
or process-wide destructive registry cleanup was introduced.

## Legacy operation disposition

Rust callers can now use `parser_registry_inventory()` on the core, or
`ParserRegistrySnapshot::inventory()` on a retained TreeHaver snapshot. The owned
`structuredmerge.parser-registry-inventory/v1` record contains the generation,
descriptor digest and cached descriptors ordered by provider ID. It does not
invoke descriptor, probe or parse callbacks; changing a returned record cannot
mutate the registry. The digest identifies declarations, not provider code or
availability. Removing and re-registering identical declarations restores the
digest but advances the generation. Generations are local to a registry instance,
not globally unique identities or timestamps.

The same inventory DTO and observation function are exposed by the generated
Ruby and Python bindings. Installed-consumer tests check cached declarations,
owned-copy isolation and removal visibility with hosts that raise if probed.
The complete merge capability/authority manifest remains unfinished. Never infer
from inventory alone that a listed parser is available,
loadable, semantically supported for a merge operation or approved as default.

| Legacy operation | Typed direction and remaining work |
| --- | --- |
| `register_parser_host` | Typed `ParserHost` descriptor/probe/parse batches in the existing registry. |
| `register_tslp_parser_host` | Explicit `register_language_pack_parser`; registration is non-loading. |
| `unregister_parser_host` | Compatibility spelling of `unregister_parser_provider`, with snapshot retention. |
| `parse_with_parser`, `parse_normalized_with_tslp` | Typed `parse_sources`, with explicit selection and validated source-bound results; migrated TreeHaver consumer. |
| `replace_parser_host` | Typed Rust/Ruby/Python replacement requires the observed generation and returns its commit generation. Remove/re-register and the older signature are not equivalent. |
| `registered_parser_hosts` | Typed declaration inventory is exposed in Rust/Ruby/Python; request-specific probes use selection reports, while full merge capability/authority reporting remains pending. |
| `probe_with_parser` | Source-free typed selection reports expose the same eligibility/probes as dispatch, without a legacy JSON wrapper. |
| `clear_parser_hosts` | No product need established by the local consumer inventory. Retain legacy regression evidence; prefer explicit removal of owned IDs and require an ownership/concurrency contract before adding process-wide destructive cleanup. |

The inventory names observed local consumers, not an exhaustive external usage
search. These dispositions do not authorize deleting legacy regression coverage
or publishing a compatibility host product.

## Atomic replacement primitive

`ParserRegistry::replace(provider, expected_generation)` replaces only an existing
provider with the same declared ID. It validates and caches the new descriptor
outside the lock, then checks the observed generation and target ID under one
write lock. Invalid descriptors, stale generations and unknown IDs leave the
registration unchanged. Successful replacement advances generation once, with no
selection gap. Retained snapshots keep the original provider and descriptor.

The retired provider is destroyed after releasing the registry lock. Destructors
can therefore re-enter the registry. The returned generation identifies the
replacement's commit point, not a guarantee that no subsequent mutation occurred:
even retirement callbacks can advance the registry before the caller receives it.
Tests verify both old/new snapshot dispatch and a retired-provider destructor
that performs a registry write.

The core's `replace_parser_host(host, expected_generation)` exposes this primitive
through Rust and generated Ruby/Python bindings. Use the generation from a recent
registry inventory/selection report; do not silently retry stale updates against
a newly observed generation without reconsidering the intended replacement.
Descriptor callbacks run outside registry locks and can fail before mutation.
The existing native or host registration named by that descriptor must exist.
The generation number is required, not nullable. The return value is the commit
generation, with the re-entrant-mutation caveat above.

Installed binding tests replace a provider from inside its active parse callback,
verify the old operation completes through the old host, and verify the next call
uses the replacement. They also assert stale/unknown-ID errors and rejection of
nil/None generations. Rust lifecycle tests additionally cover cancellation while
the replaced callback remains outstanding. These tests do not establish full
runtime shutdown or arbitrary foreign-thread entry guarantees.

## Request-specific selection reports

`ParserSelectionRequest` contains language, optional dialect, parser selection
and parse options; it needs no source bytes, fabricated document or request ID.
`parser_selection_report` and its controlled variant return the same
`SelectionReport` used by parse dispatch, through the same internal selector.
The report records snapshot generation/digest, candidate ordering, rejections,
probe states and the eligible winner. Missing/unavailable explicit backends do not
fall back. No eligible provider is a report with no winner, while invalid requests,
cancellation and deadlines remain errors rather than partial successful reports.

Unlike inventory, selection reporting **can load or download grammars** through
provider probes. Candidates rejected by language, dialect, explicit selection or
capability checks remain unprobed (`available`/`loadable` are absent, not false).
Probe faults/panics retain the dispatch report's rejection behavior. No parse
callback runs. `ParseLimits` supplies execution timeout here; document byte/node,
batch and diagnostic limits do not apply to this source-free query. Cancellation
is checked around callbacks and does not forcibly interrupt foreign code.

A report is an observation, not a parser lease or a promise of later success.
Later registration changes or availability changes may alter a subsequent parse;
dispatch selects again and reports its own evidence. Eligibility does not prove
that any particular source parses, that a merge profile supports it, or that the
provider is approved as the default. Full merge capability/authority reporting
remains separate.

## Operation profile declarations

`operation_profile_catalog()` returns a versioned, ID-sorted catalog of the eight
profiles implemented by `execute_operation`. Each declaration records provider
identity, family, accepted explicit dialects, operation kinds, parser contract,
required native extension (if any), syntax scope and limitations. An absent
dialect selector is accepted in addition to the listed explicit values. The
operation-kind table is shared with dispatch: YAML omits merge2 and the Git JSON
profile exposes merge3 only. Request policy and source checks still apply.

This is static scope introspection: it neither probes registered providers nor
loads grammars, and `parser_available` is always unknown (`None`/`nil`/`null`),
never a guessed availability result. All profiles remain experimental and
unapproved as defaults. Tests assert that listing leaves registry generation
unchanged and does not invoke host probes or parse callbacks.

The existing `native_merge_profiles()` remains the two explicit native merge3
entry points; it is not renamed or expanded into this common-operation catalog.
Neither listing is a full capability/authority manifest. Registry inventory
describes registrations, selection reports observe request-specific parser
eligibility, and execution supplies actual source/policy support evidence. A
catalog declaration alone proves none of those later outcomes and does not
authorize replacing a native consumer or changing its default provider.
