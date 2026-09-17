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
Request-specific availability and the complete capability/authority manifest
remain unfinished. Never infer that a listed parser is available,
loadable, semantically supported for a merge operation or approved as default.

| Legacy operation | Typed direction and remaining work |
| --- | --- |
| `register_parser_host` | Typed `ParserHost` descriptor/probe/parse batches in the existing registry. |
| `register_tslp_parser_host` | Explicit `register_language_pack_parser`; registration is non-loading. |
| `unregister_parser_host` | Compatibility spelling of `unregister_parser_provider`, with snapshot retention. |
| `parse_with_parser`, `parse_normalized_with_tslp` | Typed `parse_sources`, with explicit selection and validated source-bound results; migrated TreeHaver consumer. |
| `replace_parser_host` | Still unimplemented as an atomic typed operation; remove/re-register must not be advertised as equivalent. |
| `registered_parser_hosts` | Typed declaration inventory is exposed in Rust/Ruby/Python; full capability and request-specific availability observability remain pending. |
| `probe_with_parser` | Typed service selection already probes internally; explicit public observability remains part of the capability contract, not a legacy JSON wrapper. |
| `clear_parser_hosts` | No product need established by the local consumer inventory. Retain legacy regression evidence; prefer explicit removal of owned IDs and require an ownership/concurrency contract before adding process-wide destructive cleanup. |

The inventory names observed local consumers, not an exhaustive external usage
search. These dispositions do not authorize deleting legacy regression coverage
or publishing a compatibility host product.
