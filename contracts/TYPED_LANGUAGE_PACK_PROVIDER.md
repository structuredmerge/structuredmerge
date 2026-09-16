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

Still required: typed core registration exports, generated binding and installed
artifact tests, Ruby normalized-tree compatibility projection and consumer
migration. Parser/grammar version reporting is still runtime/unknown rather
than an exact grammar-build identity. Cross-platform ABI, full language coverage,
release/default authority and complete lifecycle contracts remain separate gates.
