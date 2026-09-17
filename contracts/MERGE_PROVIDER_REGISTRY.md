# Merge-provider registration foundation

`ast_merge::provider_registry` supplies the merge-behavior registry required by
Slice 1026, separate from TreeHaver's parser registry. The corresponding spec is
`MERGE_PROVIDER_REGISTRY_CONTRACT.md` in the spec repository.

`MergeProviderRegistry<P>` accepts a validated declaration and an `Arc<P>` handle;
the typed facade can use its executor trait without making `ast-merge` depend on
the facade or discarded prototype. The registry never calls the provider.
Descriptors retain explicit workflow/backend role, profile/operation scope,
parser requirements, delegation policy, preservation declarations and package
identity. Registration does not establish actual support, authority or liveness.

Snapshots cache normalized declarations and retain executor handles. Exact-ID
lookup is not a selector: callers must negotiate the request before execution.
Replacement/removal/clear require the observed generation. Retired executors
are destroyed outside registry locks, after the last snapshot/handle releases
them. Descriptor digests are independent of registration order and set order;
identical-descriptor replacement still advances the generation.

The initial tests cover deterministic inventories, metadata round trips, cached
identity, replacement/removal/re-registration, old-snapshot lifetime, stale and
unknown mutations, concurrent replacements, re-entrant destructor access,
descriptor/capacity bounds, and trait-object handles without serialization or
clone requirements. Full `ast-merge` tests and strict Clippy pass; see
`tmp/merge-registry-full-tests.log` (562 tests, no failures or ignored tests).

## Source-free two-stage selection

`ast_merge::provider_selection::negotiate_merge_provider` filters these
declarations by explicit provider ID or workflow role, family, operation,
dialect, merge profile, capability and preservation requirements. It passes
provider parser constraints to `TreeHaverParseService`, rejecting candidates
without an eligible parser before ranking by explicit match, descending
priority and stable ID. Every registered merge candidate remains in the trace;
declaration-rejected candidates are not probed. The report retains both
snapshot generations/digests and each attempted parser selection report.

TreeHaver's `with_constraints` adds conjunctive constraints. It cannot replace
or weaken application restrictions. Backend-ID/family allow/deny constraints,
normalized contracts and required capabilities apply to both source-free
observation and actual parse dispatch. Request preference still precedes
language-profile preference, priority and stable backend ID.

Parser language/dialect requirements are checked against the explicit parser
query, not guessed from the merge-provider ID. Versioned parser-profile
requirements currently fail closed: TreeHaver has language preference lists,
not a versioned parser-profile catalog. This is a recorded missing capability,
not proof of full Slice 1026 negotiation. Source-free selection does not call
merge executors or establish source-specific support, runtime availability,
execution ownership or default authority.

New tests cover backend non-hijacking, registration-order independence,
explicit-provider/backend failure without substitution, merge and preservation
filters, parser-contract rejection before provider ranking, conjunctive
constraints, preference order, callback faults, cancellation, and reentrant
removal from both registries. A TreeHaver test verifies the same constraints
govern `parse_batch` dispatch with an instrumented provider, not only reports.
`tmp/negotiation-tests.log` records
638 passed tests and five existing opt-in tests ignored, plus strict Clippy.

This is not yet connected to the public typed-core facade or Alef bindings.
Next connect the typed executor/WorkflowHost boundary to this selector, adding
request/result validation, explicit execution ownership, host availability,
allowed delegation and the portable capability envelope. Versioned parser
profiles remain open. Existing explicit kernel profiles are unchanged. No new
package, prototype release dependency, registry-mode acceptance claim, or
default promotion is introduced.
