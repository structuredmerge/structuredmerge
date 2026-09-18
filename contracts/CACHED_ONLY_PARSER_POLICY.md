# Cached-only TreeHaver language-pack provider

`LanguagePackProvider::new_cached_only(id, language)` is an explicit Rust
provider construction path for operations that must not implicitly acquire
grammars. It preserves the existing descriptor and adds
`metadata.grammar_policy = "cached-only"`. Registration, TreeHaver selection,
typed parse validation and byte/node budgets remain unchanged.

The legacy `new` constructor retains configured cache/download behavior. The
generated facade's existing `register_language_pack_parser` still uses that
legacy constructor. No binding API or CLI default was changed here. CLI routing
must deliberately choose the constrained path before claiming no acquisition.

## Mechanism and dependency evidence

The locked TSLP 1.17.0 public `has_parser` registers configured local cache
directories and tries actual local loading without downloading. Success retains
the loaded Language in the process registry. Its public `get_parser` then uses
that same retained Language; `configure` and `clean_cache` do not evict loaded
Languages. The provider uses these public APIs, never a private cache-path scan,
grammar-name catalog, direct native parser, or alternate fallback.

Both probe and parse use the same guard. If no local grammar is usable, they
return `parser.local_unavailable` before calling the acquisition-capable
`get_parser`. That code covers absent, corrupt or otherwise unloadable local
grammars; the Boolean TSLP probe does not distinguish their causes. It must not
be relabeled unconditionally as `asset_missing` in a richer availability report.
Construction does not configure, load or download anything.

This mechanism is verified against the version in Cargo.lock. TSLP upgrades must
rerun cold/corrupt/warm tests and re-audit loaded-Language retention; registry
eviction or a change in public probing semantics would invalidate this reasoning.

## Local verification

`cargo test -p tree-haver --test language_pack_provider --locked` passes its
three enabled tests (including the isolated-child entry point); six explicit
native/asset tests remain ignored by that default command. The new parent test
runs fresh subprocesses for both empty and corrupt JSON grammar directories,
isolating TSLP's process-global registry and configured paths. Both reject probe
and direct provider parsing with `parser.local_unavailable`, leave the designated
cache empty, and make no observed connection to the test HTTP/HTTPS proxy.
Children have five-second deadlines and scratch directories are removed.

The explicit warm test also passes, using a copied preinstalled JSON grammar:

```sh
TREE_HAVER_CACHED_JSON_LIBRARY=/absolute/path/to/libtree_sitter_json.so \
  cargo test -p tree-haver --test language_pack_provider --locked \
  cached_only_warm_grammar_survives_cache_file_removal -- --ignored --exact
```

It probes, removes only its own copied grammar file, then parses Unicode/CRLF
JSON through `TreeHaverParseService`, preserving descriptor bytes and selected
backend evidence. This exercises the loaded-grammar retention that prevents a
probe/use race from turning into acquisition. The original cache is untouched.
Grammar SHA-256:
`8e44debe3f89057328a3db45fb5cbb98ddb41f4bcfca82a2a1aa268301a579d4`.
Logs: `tmp/cached-only-tests.log` and `tmp/cached-only-warm.log`.

The target reached approximately 2.3 GiB under an 8 GiB target / 30 GiB free-space
watchdog and is removed after testing, together with scratch Cargo metadata.
The tempfile dev dependency does not change the 20-crate runtime publication
closure; the owning inventory checker passes without a changed inventory.

## Limits and remaining work

This prevents the language pack's implicit grammar acquisition; it is not an OS
network sandbox, validation of arbitrary native library code, grammar-signature
verification, a provider availability manifest, or a preflight lease. Already
loaded grammars and all TSLP-registered local directories remain eligible; this
is not confinement to one filesystem directory. The proxy check complements the
audited call path, not proof of hard network isolation.

Evidence is local Linux with locked TSLP 1.17.0. Other platforms (including library
removal semantics on Windows), concurrent reconfiguration, asset provenance,
full artifact/runtime reports, generated binding exposure, typed CLI integration,
publication and default authority remain separate gates. The legacy auto-download
path remains explicitly outside the cached-only policy.
