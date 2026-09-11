# Changelog

## [Unreleased]

### Added

- Publish and enforce exact provider selection: unavailable explicit provider
  IDs fail closed even when another parser or workflow host is registered, and
  the capability manifest declares that implicit provider fallback is disabled.
- Expose a deterministic, versioned capability manifest from the generated
  bundle so callers can distinguish compiled merge operations, on-demand
  parser factories, and currently registered host providers before selection.
- Measure packaged Ruby cold install, cold start, native TSLP parser load, and
  first merge paths in CI, retaining correctness-checked structured evidence
  without imposing noisy runner-dependent timing thresholds.
- Classify `.json5` template targets as the JSON family with the JSON5 dialect,
  follow fixture-owned synthetic action pins in YAML synchronization tests, and
  recognize the portable benchmark contract's new merge2 gold case.
- Add a checked-in Ruby API/native ABI contract and require both local and
  isolated packaged bindings to match it; reproduce Alef's generated tree
  twice from clean inputs to reject generated drift and untracked output.
- Exercise every declared Ruby native platform in CI, build and install the
  Linux platform gem in isolation, and emit a content-addressed artifact
  provenance manifest, including the exact temporary Alef fork revision used
  for packaging until its platform-metadata fix is released.
- Define the production package boundaries for the Rust kernel, compile-time
  provider bundles, generated Ruby artifact, and host-native provider adapters.
- Promote the Alef Ruby binding configuration to the canonical `alef.toml`,
  complete its generated README and API-reference surface, and gate both Alef
  freshness and Ruby 3.2/4.0 binding tests in CI.
- Add a shared source-preserving top-level declaration merge kernel.
- Add fail-closed, source-preserving TypeScript and TSX three-way merging through
  TreeHaver's normalized TSLP parser interface.
- Expose the TypeScript three-way provider through the benchmark adapter without
  claiming source-preserving two-way support.
- Add a validated TreeHaver normalized-tree index so format providers share
  fail-closed node lookup, root validation, child traversal, and descendant search.
- Share parser diagnostic conversion and role-attributed three-way parse failure
  construction from `ast-merge` instead of redefining them in format providers.
- Route Go analysis through TreeHaver's normalized TSLP parser and add a
  fail-closed, source-preserving top-level function three-way merge provider.
- Route Rust analysis through TreeHaver's normalized TSLP parser and add a
  fail-closed, source-preserving top-level function three-way merge provider;
  keep the spanless native `syn` backend explicitly unsupported for merging.
- Add a thin Bash provider that projects top-level function ownership from
  TreeHaver's normalized TSLP AST and reuses the shared source-preserving
  declaration kernel for fail-closed three-way merges.
- Add a shared normalized-tree projection for strict, uniquely named top-level
  owners so language providers can reuse one source-range and identity contract.
- Add an experimental generic TreeHaver/TSLP provider for languages without a
  dedicated substrate, limited to exact-layout three-way merges of uniquely
  named top-level owners and explicit fail-closed behavior everywhere else.
- Advertise Python as the first reviewed generic-provider combination through
  the Rust benchmark adapter without claiming generic two-way support.
- Route the Bash provider through the shared normalized named-owner projection
  without broadening its top-level-function-only merge contract.
- Route Go's three-way function ownership through the same shared projection
  while retaining Go-specific import and declaration behavior in its substrate.
- Route Rust-language three-way function ownership through the shared
  projection while preserving its separate TSLP and native analysis contracts.
- Route TypeScript and TSX three-way declaration ownership through the shared
  projection, including the existing export and ambient wrapper policy.

- Add the kettle-rusty plan/apply CLI with human-readable and JSON reports.

- Expose packaged Rust project template inventory planning and application through kettle-rusty.

- Expose TypeScript analysis and source-preserving merge3 through the experimental Ruby host boundary.

- Expose an opt-in TypeScript RustHostProvider for host-backed analyze, merge2, and merge3 operations.

- Expose kettle-rusty README style planning and application through the CLI.

- Include sorted Cargo runtime and development dependency names in kettle-rusty discovery facts.

- Regenerate the kettle-rusty README with the current StructuredMerge family and backend compatibility inventory.

- Discover sorted GitHub workflow paths in kettle-rusty Rust project facts.

- Discover Cargo package facts from a single explicit workspace member in kettle-rusty.

### Fixed

- Preserve Go package clauses in source-preserving two-way merges.

- Expand Rust source-preserving merge3 ownership beyond functions to named top-level items.

- Preserve Rust declaration kinds in source-aware host analysis and diff owner paths.

- Support conservative one-sided top-level owner additions and deletions during source-preserving three-way merges while retaining fail-closed layout checks.

- Build the TreeHaver consumer gem from its package directory in CI release and isolated-install workflows.

- Preserve TypeScript destination bytes in Rust two-way merges when no owners or imports are added.

- Report ambiguous managed-block markers without overwriting destination content in kettle-rusty.
