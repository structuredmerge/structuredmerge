# Changelog

## [Unreleased]

### Changed

- Point kernel crate metadata and new artifact provenance at structuredmerge/structuredmerge; remove kettle-rusty from this repository’s release inventory while accepting pre-split registry metadata.

- Use upstream Alef 0.89.0, preserve generated Ruby API compatibility, include dual-license texts in generated packages, generate platform linker configuration, and track the kernel Cargo dependency lock.

- Build Ruby platform extensions without forced libruby linkage and inspect packaged Linux/macOS libraries before accepting artifacts.

- Correct benchmark candidate provenance to the kernel repository and provide a runtime-only bundle for the retained Slice 1023 harness without the prototype host gem.

- Return unsupported native analysis as typed error results with role-specific rejection records and all validated input parses, preserving the older internal Rust error entry point.

- Return typed input parser-selection and provider failures from native merge operations while retaining request/resource/cancellation errors and the existing parse_sources exception API.

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

- Expose Cargo dependency requirements, sources, optionality, and target selectors in kettle-rusty facts.

- Discover workflow names, trigger keys, and job names alongside kettle-rusty workflow paths.

- Honor explicit kettle.yml template entries and disabled profiles in kettle-rusty packaged-template planning.

- Include source fragments in JSON owner analysis so host adapters can recover line provenance.

- Honor explicit local and profiled template roots in kettle-rusty, failing closed when a selected source is missing.

- Validate every Ruby host release artifact against its provenance digest, platform metadata, and cold-path evidence before publication.

- Expose the Rust ast-merge-git JSON merge3 operation through the generated Ruby host.

- Expose the opt-in ast-crispr profile-report bridge through the generated Ruby host.

- Expose the opt-in ast-template session-report bridge through the generated Ruby host.

- Expose explicit UTF-8 source-edit projection through the ast-crispr Ruby host bridge.

- Expose read-only ast-template directory plan reports through the generated Ruby host.

- Add a parser and source-preserving merge cold/warm benchmark for the compiled Ruby host.

- Expose opt-in Bash analysis and source-preserving merge operations through the generated Ruby host.

- Include stable top-level Bash variable assignments in the opt-in Rust owner projection.

- Extend the Bash Rust owner projection to literal-title test_expect_success calls.

- Refresh GitHub Actions references for the Rust test and Ruby host release workflows.

- Begin the structuredmerge-core typed facade with explicit operation roles and a shared TreeHaver source map that verifies exact bytes, SHA-256, encoding, line endings, ranges, and input limits without depending on the prototype facade.

- Add typed TreeHaver parser records, validated source-backed trees, and in-process provider selection with immutable registry snapshots and coarse batch dispatch; retain parser faults without hidden fallback.

- Add a typed native-Psych block-mapping path through TreeHaver into Rust-owned YAML entry analysis and source-preserving three-way merge, with an explicit native integration CI gate; generated host binding integration remains pending.

- Begin generated Ruby and Python structuredmerge-core bindings with typed parser-host batches, TreeHaver dispatch, and native Psych/LibCST callback tests; upstream generator fixes and full installed merge gates remain required before release.

- Expose the Rust-owned YAML mapping merge through the typed core facade and generated bindings, retaining shared conflict alternatives and native syntax rejection details; the complete portable result contract remains in progress.

- Add Rust-owned Python declaration merging from native LibCST facts through generated bindings, sharing TreeHaver parse and verification orchestration with YAML; installed-wheel tests cover exact source bytes, conflicts, syntax failures, and fail-closed unsupported syntax.

- Add an isolated core-only Ruby platform gem gate with explicit contents, ABI-scoped installation, linkage inspection, and real Psych/Rust merge tests in a fresh bundle; publication and source-gem gates remain open.

- Emit and independently verify exact source/output byte partitions from the shared owner renderer, including segment digests and source identities through Ruby and Python bindings; policy-specific preservation claims and dispositions remain in progress.

- Expose all validated native input parse results, diagnostics, and parser-selection provenance on typed merge results in both generated bindings.

- Retain native output-verification parse evidence through both bindings, including rejection diagnostics without exposing unverified merged output.

- Expose structured parser-service failure evidence for output verification, retaining provider identity and native fault details without returning unverified output.

- Generate Ruby RBS and Python native type declarations from the shared Alef configuration, ship them in isolated artifacts, and validate them in artifact tests.

- Expose scoped native merge profile descriptions and correlate merge results with the invoked profile, explicitly separating experimental support from parser availability and default approval.

- Generate Ruby and Python native-profile conformance tests from shared fixtures and run them against isolated installed core artifacts.

- Exercise native Python merges through generated fixture suites against the installed core wheel, using a test-only LibCST provider adapter.

- Run Alef-generated native YAML merge fixtures against the isolated installed core gem using the shared Psych test provider.

- Verify installed Ruby and Python parser callback retention, GC release, reentrant in-flight unregister, and re-registration lifecycle behavior.

- Verify overlapping installed-binding parser callbacks retain distinct batch results and old provider snapshots across unregister and re-registration.

- Expose optional monotonic operation deadlines through typed Ruby and Python parse/merge limits, rejecting late successful input and verification results.

- Expose shared one-way operation cancellation controls and controlled parse/native-merge calls through Alef-generated Ruby and Python bindings.

### Fixed

- Preserve Go package clauses in source-preserving two-way merges.

- Expand Rust source-preserving merge3 ownership beyond functions to named top-level items.

- Preserve Rust declaration kinds in source-aware host analysis and diff owner paths.

- Support conservative one-sided top-level owner additions and deletions during source-preserving three-way merges while retaining fail-closed layout checks.

- Build the TreeHaver consumer gem from its package directory in CI release and isolated-install workflows.

- Preserve TypeScript destination bytes in Rust two-way merges when no owners or imports are added.

- Report ambiguous managed-block markers without overwriting destination content in kettle-rusty.

- Apply package-scoped kettle-rusty recipes inside the selected single-member workspace package.

- Target README style planning and application at the selected workspace member.

- Resolve workspace-member SECURITY.md from the selected package when rendering README style.

- Preserve differing existing packaged templates and report ownership ambiguity in kettle-rusty.

- Retain malformed GitHub workflow paths in kettle-rusty facts with non-fatal parse diagnostics.

- Keep the Ruby host transport benchmark identity provider compatible with the workflow-host callback contract.

- Reject unsafe or glob-like Cargo workspace member paths in kettle-rusty discovery.

- Include the complete combined license text in Python wheels and validate wheel contents plus real LibCST/Rust merges in a fresh installed consumer environment.

- Give typed parser and native-merge service failures stable distinct core error codes, including consistent resource-limit classification and preserved provider fault text.

- Retain all validated input source descriptors on typed native syntax rejection and select syntax failures by semantic role before family analysis, independent of request order.

- Reject duplicate source IDs across typed parser batches before probes or parsing, including distinct requests with identical bytes or different merge roles.

- Retain independently validated merge input source descriptors on parser selection and callback failures while withholding incomplete parse results and output.
