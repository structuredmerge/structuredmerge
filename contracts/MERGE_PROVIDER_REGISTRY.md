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

This is not yet connected to the public typed-core facade or Alef bindings.
Next implement the typed executor/WorkflowHost boundary, request/result
validation, explicit execution ownership, merge-provider selection and
TreeHaver negotiation using this foundation. Existing explicit kernel profiles
are unchanged. No new package, prototype release dependency, registry-mode
acceptance claim, or default promotion is introduced.
