# Typed Rust language-pack provider

`tree_haver::language_pack_provider::LanguagePackProvider` implements the typed
TreeHaver `ParserProvider` interface. It is a migration building block for the
existing Ruby `rust_tslp` backend, not a default parser or a replacement native
merge engine. It has no dependency on the host-prototype crate.

Construction takes an explicit provider ID and language without loading a
grammar. Registration and selection use the existing `ParserRegistry`, not a
parallel registry. Probe and parse use the language pack's configured cache and
on-demand grammar loading; callers must opt into that behavior. Tests keep the
cache under the kernel's `tmp/typed-tslp-cache` directory.

The provider projects native nodes directly, preserving missing/error flags,
byte spans, points, parent/child order and field names. Comment records refer to
native nodes; attachment remains unknown. Syntax failures retain a validated
partial tree and blocking diagnostics. Request forwarding fields are nested so
they cannot shadow output-reserved fields. No JSON-string parse facade or
host-owned matching/merge decisions are introduced.

The initial provider supports UTF-8, source spans, comments, diagnostics and
partial trees. Dialect constraints, token requests and native extensions are
not advertised. TreeHaver validates request/result identities, source bytes,
selection and graph integrity. Projection uses iterative traversal and checks
node budgets; interruption checks surround native parsing and occur during
projection. This is not preemptive interruption of an in-progress native parse
or grammar download. Projection-budget exhaustion currently retains a native
`resource.limit` provider fault, distinct from service-level resource rejection.

Five tests pass with the real JSON grammar, including ordered batches,
Unicode/CRLF spans, child fields, comments, malformed input, control/selection
rejection and projection limits. The full TreeHaver suite and strict Clippy
pass. Logs: `tmp/typed-tslp-provider-{native,regression,clippy}.log`. CI explicitly
enables the grammar-dependent tests. Hosted success is not claimed here.

## Generated core API and lifecycle

Both generated bindings now expose
`register_language_pack_parser(id, language) -> ParserProviderDescriptor` and
`unregister_parser_provider(id)`. Registration does not probe or load a grammar,
does not overwrite duplicate IDs, and does not promise availability. It adds to
the same registry as `register_parser_host`; callers should select the returned
ID explicitly. Removal affects future snapshots only, fails for unknown IDs,
and can report `StaleGeneration` on concurrent mutations. The existing
`unregister_parser_host` spelling delegates to the same removal implementation.
There is no bulk-clear or implicit replacement API.

The reviewed Ruby/Python source API baselines include these additive exports.
Fresh isolated installed artifacts pass real JSON parse, comment/field, partial
syntax-error, duplicate registration and post-removal selection tests: Python
34 tests plus 15 generated fixtures; Ruby 30 tests plus 12 generated fixtures.
Both artifact gates place grammar caches under the repository's `tmp/` by
default (the explicit environment override remains supported). Core regression,
strict Clippy, and 14 artifact/inventory audits pass. Logs:
`tmp/typed-tslp-{python,ruby}-artifact.log` and
`tmp/typed-tslp-core-{tests,regression,clippy}.log`.

Still required: Ruby normalized-tree compatibility projection and consumer
migration. Its legacy `extra?` node surface also needs an explicit native-fact
mapping; the current typed node DTO does not carry that tree-sitter flag.
Parser/grammar version reporting is still runtime/unknown rather
than an exact grammar-build identity. Cross-platform ABI, full language coverage,
release/default authority and complete lifecycle contracts remain separate gates.
