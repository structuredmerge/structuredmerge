# Runtime grammar identity prerequisite

## Audited state

The kernel locks `tree-sitter-language-pack` 1.17.0, crates.io checksum
`8074a6f562c62733c6e1c837affd0b14f8e749b079b416344e3e4d8176b6d5d0`.
This audit concerns that dependency, not an assertion about upstream HEAD or a
newer release. No dependency source, lockfile or parser implementation was edited.

Follow-up read-only upstream inspection on 2026-09-18 checked main at
[`e147e2d04bbce6eebb92eee66ecd948c94f26722`](https://github.com/xberg-io/tree-sitter-language-pack/tree/e147e2d04bbce6eebb92eee66ecd948c94f26722).
Its public `crates/ts-pack-core/src/lib.rs` and `src/registry.rs` interfaces still
return languages/parsers without a loaded-library identity receipt. The loader
still retains `HashMap<PathBuf, libloading::Library>` and caches languages by
name. That source includes a newer load-time ABI compatibility check, but an ABI
check is not byte identity. GitHub's latest release endpoint reported v1.20.0,
published 2026-09-14; this observation is not a claim that v1.20.0 was installed
or tested. A dependency bump is therefore not justified as a fix for this gap
by the inspected main interface. No clone, download of grammar assets, dependency
update, upstream issue or pull request was performed.

In that package's `src/registry.rs`:

- `LanguageRegistry::get_language` returns an already-cached dynamic language
  before examining library directories. Static language loaders are separate.
- `DynamicLoader::load_from_path` caches a `Language` by normalized language name.
  Its library key is the canonicalized path, with an original-path fallback.
- `language_from_process_library` retains `libloading::Library` objects in the
  process-wide `LOADED_LIBRARIES` map. An existing path key reuses its library;
  it does not reload current bytes at that path.
- The map intentionally has no removal operation: language function pointers
  must remain valid after the originating registry is dropped.
- The public APIs return languages/parsers and availability information, not
  a loaded-library identity receipt or a digest tied to the retained handle.

The crate includes tests named
`test_dynamic_library_is_loaded_once_across_registries` and
`should_keep_loaded_libraries_present_after_the_owning_registry_is_dropped`.
Their source corroborates the intended lifetime invariant; this audit did not
run those upstream tests and does not claim their conditional grammar branches
were exercised locally.

An additional read-only check of the released `v1.20.0` `registry.rs`
(2026-09-19) finds the same boundary: `LOADED_LIBRARIES` remains a
process-wide `HashMap<PathBuf, libloading::Library>`, and the public registry
still returns languages/parsers rather than a receipt carrying loaded-byte
identity. The release's ABI compatibility checks do not establish content
identity. The locked dependency must therefore not be bumped as a speculative
fix; an upstream loader-owned extension remains the required dependency action.

TreeHaver's `LanguagePackProvider::parser` correctly calls the public
`has_parser` / `get_parser` path for cached-only operations. That proves local
loadability without implicit acquisition. Neither call exposes the missing
loaded-byte evidence. Do not replace this path with direct library loading,
cache scanning, a private registry inspection, or a different parser.

## Why current file hashes cannot fill the gap

A process can load grammar A, then retain A after its pathname is removed or
replaced with grammar B. Rehashing that pathname describes B (or an absent
file), not the code behind the retained language. Reconfiguring search paths
also does not prove that a previously loaded language was replaced. A grammar
ABI version, language name, pointer value or successful probe is not a content
digest. Reading a configured cache directory is not loader provenance.

Consequently the manifest tools' explicit asset hashes and the typed selector's
successful probes must remain separate observations. Combining two individually
true observations does not prove they describe the same loaded object. The full
Slice 1032 availability/preflight gate cannot claim verified loaded assets from
these observations alone.

## Required loader-owned capability

The next implementation must obtain evidence from TSLP's supported loading
interface, rather than maintain a second loader in StructuredMerge. API spelling
and the upstream implementation are not decided by this document. It needs:

1. An immutable receipt tied to the same retained language/library handle used
   by parsing, including normalized grammar identity, origin kind, and digest
   algorithm/value where verified byte identity can actually be established.
2. Explicit unknown/unverified evidence when that guarantee cannot be made.
   A path or caller assertion must never be upgraded to verified byte identity.
3. Capture at the loader ownership boundary, including process-wide library
   reuse across registries. A receipt must not be reconstructed by rehashing a
   current pathname after loading, even when the path key is unchanged.
4. A loading/verification strategy that binds the checked bytes to the actual
   loaded object. Hash-before/load/hash-after is not a portable race-proof
   strategy under concurrent replacement or in-place mutation. Supported
   filesystem/threat assumptions and platform limits must be explicit.
5. Separate build-attested identity for statically linked grammars. Do not invent
   a shared-library path or equate a grammar source digest with linked code.
6. The normal cached-only/acquisition policy and lifetime guarantees unchanged.
   Receipt collection must not download, start a runtime, evict live libraries,
   or choose an alternative parser. Acquisition remains explicit preparation.

TreeHaver can then propagate the receipt through the selected provider handle.
Availability must compare that receipt with authenticated asset declarations;
preflight must pin it and execution must verify that the same selected handle
and asset evidence remain applicable. A report alone is not an execution lease.

## Acceptance cases before closing the gate

- Load A, atomically replace the pathname with different B, and demonstrate that
  evidence for an existing language continues to identify A, never B.
- Remove the loaded library and change cache/search configuration; retained
  handles keep their original identity or explicit unverified state.
- Use multiple registries at one canonical path and path aliases; metadata
  follows the process-retained library rather than each caller's current file.
- Exercise concurrent first loads and replacement under the stated threat
  model. A mismatch fails closed without an alternative grammar or line merge.
- Distinguish static, dynamic, and unknown identity; prove that language ABI and
  provider/package versions cannot substitute for verified asset identity.
- Preserve cold-cache no-acquisition behavior and public TSLP loading semantics.
- Compare a signed asset declaration for B against a retained A; reject the
  mismatch before parsing. Reject stale preflight evidence rather than reselect.
- Run per supported platform, including Windows library lifetime/removal rules.
  Bound any verification copies, retain only process-needed objects, and clean
  disposable storage on success, errors, cancellation and subprocess exit.

This is a concrete dependency prerequisite, not completion of runtime asset
verification and not a blocker for unrelated plan work. No upstream issue/PR,
fork, dependency patch, package publication, or release-key choice was made.
