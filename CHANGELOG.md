# Changelog

## [Unreleased]

### Changed

- Point kernel crate metadata and new artifact provenance at structuredmerge/structuredmerge; remove kettle-rusty from this repository’s release inventory while accepting pre-split registry metadata.

- Use upstream Alef 0.89.0, preserve generated Ruby API compatibility, include dual-license texts in generated packages, generate platform linker configuration, and track the kernel Cargo dependency lock.

- Build Ruby platform extensions without forced libruby linkage and inspect packaged Linux/macOS libraries before accepting artifacts.

- Correct benchmark candidate provenance to the kernel repository and provide a runtime-only bundle for the retained Slice 1023 harness without the prototype host gem.

- Return unsupported native analysis as typed error results with role-specific rejection records and all validated input parses, preserving the older internal Rust error entry point.

- Return typed input parser-selection and provider failures from native merge operations while retaining request/resource/cancellation errors and the existing parse_sources exception API.

- Record ast-crispr and ast-template consumers as migrated on local Ruby main after revalidating their typed adapters against the current installed core gem; remaining release and downstream gates stay explicit.

- Rename the kernel CLI crate to smorg and ship the smorg executable alongside the retained smorg-rs compatibility alias; existing Git-driver defaults remain unchanged.

- Retire the prototype Ruby publication workflow and convert the kernel package job/task to non-publishing typed-core exports with integrity checks; retain independent installed tests and legacy regression sources.

- Run the installed typed-core Ruby artifact gate across the six Ruby/platform CI combinations while retaining a separately labeled legacy checkout regression matrix.

- Expand installed Python core artifact CI to Linux, macOS and Windows x64/ARM64 targets, retain Python 3.10 minimum-runtime coverage, and resolve exactly one wheel without shell-dependent wildcard expansion.

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

- Expose typed explicit UTF-8 source edits through generated Ruby and Python core bindings, validating source identity and bounded output while reusing the shared Rust renderer.

- Expose typed structural operation-profile batch reports through generated core bindings, sharing ast-crispr classification with the retained JSON report adapter.

- Expose typed match, selection, and destination profile reports through Ruby/Python core bindings, retaining ast-crispr defaults, unknown vocabulary, and optional comment-region values.

- Expose typed structural limit constraints and evaluation reports in generated Ruby/Python core APIs, retaining exactly-one defaults, empty conjunctions, and all six comparison operators.

- Expose the historical ast-crispr package boundary as a typed core report while retaining its fixture-compatible serialization and separating metadata from runtime capability claims.

- Typed template options, profile resolution, and read-only directory-plan reports in structuredmerge-core and generated Ruby/Python bindings; template application remains outside this API.

- Rust-owned exact-source structural diff primitive with deterministic owner changes, validated before/after byte regions, and independent order/layout evidence for the typed diff2 migration.

- TreeHaver-backed native structural diff orchestration with a YAML/Psych family entry point, validated before/after roles, retained parser and analysis failures, and cancellation-safe result handling.

- Typed YAML/Python native diff requests and results across generated Ruby/Python bindings, retaining request identity, exact source changes, parser evidence and fail-closed cancellation behavior without merged output.

- Add typed Rust operation-request normalization with role-keyed sources, operation policies, preserved extensions, verified local content resolution, and shared batch limits.

- Add request-correlated Rust operation-result checks for semantic roles, explicit selection, contradictory outcomes, source evidence, preservation claims, and batch identity.

- Add canonical diagnostic records to typed operation results, with causal/source validation, native-origin preservation, strict schema parsing, and explicit legacy-reason migration.

- Add canonical conflict records with verified source/output regions, ordered alternatives, independently supplied resolution authorization, and request-scoped batch evidence.

- Retain executed Rust owner-classification decisions and project edit/edit, delete/modify, and add/add conflicts into canonical source-verified records.

- Execute explicit native diff2 and merge3 profiles through the common Rust operation contract, retaining parser and decision evidence, verifying selected outputs, and rejecting unsupported requirements without fallback.

- Populate common native diff change spans from verified source bytes and digests, preserving UTF-8 byte coordinates and leaving absent revisions without fabricated ranges.

- Retain and validate native parser-node provenance for Rust-owned YAML mapping and Python declaration analysis, rejecting stale sources and missing, duplicate, misordered or out-of-owner node references.

- Share exact byte-gap layout evidence between native owner analysis and rendering, retaining source digests and explicit emission controllers without inferring comment attachment or deletion fallback.

- Execute bounded common analyze requests through TreeHaver and Rust family ownership, retaining native parse references, verified owner/layout evidence and explicit unsupported-policy failures without rendering output.

- Exercise common analyze, diff2 and merge3 execution with real LibCST syntax in the Python CI matrix, sharing the syntax projector with installed-wheel tests and preserving Rust-owned decisions.

- Add typed common-operation Rust facade entry points using registered ParserHost providers, checked inline source normalization and shared cancellation/deadline controls; unresolved source references never trigger implicit I/O.

- Expose the experimental common operation facade and DTOs through Alef; verify typed Python analyze, diff2, and merge3 with installed LibCST callbacks. Ruby typed policy input remains a documented generator gap.

- Add common typed analyze, diff2, and merge3 fixtures for installed Ruby/Psych and Python/LibCST bindings, including syntax rejection, conflicts, and byte preservation.

- Add Rust directional whole-owner classification with explicit incoming/current identities, current-preferred decisions, and exact layout evidence; merge2 facade execution remains unsupported.

- Add directional byte rendering for explicit Rust family insertion plans, preserving all current bytes and incoming/current provenance with independent byte and output-owner verification; native merge2 integration remains pending.

- Add internal TreeHaver-backed directional merge orchestration with Rust-owned planning, native output verification, explicit incoming/current identities, and cancellation checks; production merge2 facade support remains pending.

- Add Rust-owned Python directional placement using native whitespace-inclusive LibCST spans, preserving current comments and layout while inserting incoming-only declarations; common merge2 facade integration remains pending.

- Enable common typed merge2 execution for the explicit native Python declaration profile with Rust-owned current-preferred insertion, verified incoming/current provenance, stage-specific failure diagnostics, and installed-binding fixtures.

- Add explicit Ruby/Python typed API source-review snapshots with a non-mutating CI drift gate and packaged-surface checks; runtime semantic compatibility and platform ABI approval remain separate.

- Add an explicitly registered typed Rust TreeHaver language-pack provider with native node/error/comment facts, source validation and bounded projection; generated-core exports and Ruby consumer migration remain pending.

- Expose opt-in Rust language-pack parser registration and provider-neutral removal through both generated structuredmerge-core bindings, with shared TreeHaver lifecycle and installed-artifact parser tests.

- Preserve native tree-sitter extra-node flags in an opt-in versioned extension for typed Ruby TreeHaver consumer migration.

- Allow the existing nested JSON/JSONC/JSON5 merge engine to consume typed TreeHaver facts and caller-supplied output verification, including no-op results; retain native comment indexing without optional enrichment.

- Execute explicit nested JSON/JSONC/JSON5 merge2 and merge3 requests through the typed common facade with shared TreeHaver selection, canonical conflict records, mandatory output verification and replay-checked render evidence.

- Add Rust-owned JSON owner facts with parser-derived byte spans and a bounded exact-source owner comparison helper; reject ambiguous duplicate keys without claiming complete document diff coverage.

- Retain parser-node provenance through shared comment grouping and expose JSON comment coverage plus byte-exact blank-run layout evidence without changing legacy attachment behavior.

- Execute exact-source JSON diff2 through the typed common facade with nested native-span changes, whole-document trivia coverage, and independently recomputed evidence validation.

- Execute common typed JSON analysis with native owner/comment references, exact blank-gap bytes and controllers, explicit unresolved comment retention, and recomputed evidence validation.

- Generate and run shared JSON/JSONC/JSON5 operation fixtures through isolated installed Ruby and Python core bindings.

- Add Rust typed Git merge3 rendering over validated TreeHaver inputs, retaining verified clean edits and replayable conflict-marker provenance with explicit review-artifact limitations.

- Expose an explicit typed Git JSON merge3 profile through the common operation facade, validating marker options and replaying canonical conflict classifications and rendering evidence without prototype transport.

- Execute the existing Bash owner merge over validated TreeHaver facts with explicit output verification, including no-op selections; share native-node adaptation with typed JSON without parser rediscovery.

- Expose explicit common Bash owner analysis, exact-source diff2 and merge3 operations with native owner references, retained layout evidence and complete-byte diff summaries; unsupported policies fail closed.

- Implement typed directional Bash merge2 with current-preferred ownership, native-comment-aware insertion ranges, exact current-byte retention and fresh output verification; reject ambiguous placement instead of fabricating a three-way base.

- Add validated TreeHaver-based Go owner analysis and three-way merge execution, preserving native node references, the existing membership/edit safety guard, exact-source evidence, and fresh output verification.

- Execute typed Go analysis, structural diffs and three-way merges through kernel.go.owners.v1; retain the Go ownership guard as a validated whole-document canonical conflict instead of bypassing it through generic merge orchestration.

- Add current-preferred typed Go directional function insertion with native comment ranges, exact current-source retention and fresh output verification; reject mismatched package/import declarations for additions instead of silently dropping dependencies.

- Add typed Rust declaration analysis and guarded three-way merge over validated TreeHaver facts, sharing fresh output verification with typed Go execution while preserving each family ownership policy.

- Expose typed Rust declaration analysis, exact-source diff2 and guarded source-preserving merge3 through the common kernel API, with source-bound canonical membership conflicts and verified output reparsing.

- Add current-preferred typed Rust merge2 with native declaration/comment ranges, module documentation preservation, exact use-declaration compatibility, and verified source retention.

- Exercise all four typed Rust operations through shared Alef-generated Ruby/Python fixtures and isolated installed core-package tests.

- Add typed TypeScript/TSX declaration analysis and source-preserving merge3 over validated TreeHaver facts, retaining native wrapper spans and fresh output verification without host merge logic.

- Expose typed TypeScript/TSX analysis, diff2 and verified merge3 through the common kernel API with explicit grammar selection and dialect-bound analysis validation.

- Add typed TypeScript/TSX directional merge2 with current-byte retention, native declaration wrappers and comment ranges, preserved document headers, exact import compatibility and fresh output verification.

- Exercise typed TypeScript and TSX operations, including grammar-specific JSX behavior, through shared Alef-generated tests in isolated installed Ruby/Python core packages.

- Allow CI to export an allowlisted typed Ruby core platform gem and digest report without running the full installed-artifact harness, preserving ABI/API/linkage checks and explicit unrun-test states.

- Verify downloaded typed Ruby core exports before installation, rejecting digest, package, file allowlist, platform, Ruby ABI, and producer-report mismatches without claiming runtime validation.

- Document and verify typed parser snapshot retention during in-flight removal, same-ID re-registration, cancellation, and retired-host release without recreating a prototype registry API.

- Expose a typed, non-loading parser registry inventory in Rust with cached provider declarations, snapshot generation and descriptor digest, without conflating registration with availability or default authority.

- Expose the typed, non-loading parser registry inventory through generated Ruby and Python bindings, with installed-consumer tests for ownership, cached declarations, removal visibility, and no provider probes.

- Expose source-free parser selection reports through Rust and generated Ruby/Python APIs, sharing dispatch eligibility/probing and preserving unavailable, unprobed, rejection, and cancellation states without parsing or granting merge authority.

- Add generation-checked atomic parser replacement to TreeHaver, preserving in-flight snapshots and releasing retired providers outside registry locks.

- Expose generation-checked atomic ParserHost replacement through Rust and generated Ruby/Python bindings, preserving in-flight snapshots and rejecting stale or unknown registrations.

- Expose a typed, source-free operation profile catalog across Rust, Ruby and Python with declared operation sets, dialects and syntax limits; availability remains unprobed and default authority remains false.

- Dispatch smorg NAME to smorg-NAME on PATH with OS-native arguments and no implicit Unix shell fallback; add explicit benchmark-provider-merge3 while retaining positional benchmark compatibility in smorg-rs.

- Add an installed-executable real Git merge gate over every canonical JSON driver case; it exposes the unresolved delete/edit review-rendering gap without skipping that fixture.

- Run the installed smorg and smorg-rs real-Git gate in Linux CI with pinned canonical fixtures and retained failure evidence, separate from publication and default approval.

- Add an installed Python typed-core JSON adapter for the retained Slice 1023 benchmark, with cold-file and persistent-session protocols and explicit unsupported family coverage.

- Extend the installed typed-core benchmark adapter to Bash, Go, Rust and TypeScript/TSX owner-profile merge3 without old CLI fallback; retain explicit unsupported operation coverage.

- Exercise LibCST-backed Python merge2 and merge3 through the installed typed-core benchmark adapter, reusing the conformance parser projection without a prototype package.

- Add an isolated installed Ruby/Psych typed benchmark adapter with explicit YAML merge3 scope, startup isolation, and transport regression checks.

- Prepare an allowlisted typed Ruby source archive through Alef in an isolated committed snapshot, recording provenance without claiming installation or publication readiness.

- Verify installed Python typed-core function parameter signatures and defaults against generated declarations through both native and public facade exports.

- Generate Python package documentation and Ruby/Python typed-core consumer test applications.

- Exercise generated Ruby and Python test applications with staged native-provider support in isolated pre-publication artifact gates; report registry installation separately.

- Add a typed Rust capability manifest that separates profile scope, snapshot-bound parser eligibility observations, and explicit default authority without parsing or merging source.

- Expose typed capability manifests in generated Ruby/Python bindings with installed-runtime tests for declarations, eligibility, probe faults, limits, cancellation and snapshot retention during host retirement.

- Run shared capability manifest and inventory fixtures through Alef-generated Ruby/Python e2e and test-app suites.

- Add the ast-merge merge-provider registry foundation with explicit workflow/backend declarations, bounded metadata, stable snapshot digests, and generation-checked replacement and retirement; registration alone grants no selection or default authority.

- Add snapshot-bound merge-provider declaration filtering and deterministic ranking through TreeHaver parser negotiation; provider constraints apply conjunctively to both parser observations and dispatch without changing existing explicit kernel profiles.

- Add a Rust typed WorkflowHost batch boundary over the merge registry, with TreeHaver-prepared source facts, explicit host execution ownership, bounded envelopes, cancellation and common-result validation; generated binding exposure and release gates remain pending.

- Expose typed WorkflowHost registration, inventory, prepared native-parser batches and bounded host-owned execution to Ruby and Python, with shared cancellation controls and no default-authority promotion.

- Extend installed Ruby/Python WorkflowHost lifecycle gates with GC retention/release, reentrant registry changes, stale generations, overlapping host threads and caller context, late-result cancellation, and bounded registered/retired/drained runtime-exit checks.

- Allow isolated Python core artifact gates to run all boundary and generated suites against an independently installed LibCST provider wheel, with separate provenance from the retained conformance adapter.

- Expose human and JSON version identity for smorg and smorg-rs using the linked typed kernel, with strict version arguments and no parser or grammar loading.

- Add an explicit cached-only TreeHaver language-pack provider mode that refuses implicit grammar acquisition, with isolated cold/corrupt-cache and pinned warm-grammar tests.

- Expose explicit cached-only language-pack parser registration in the typed Rust facade and generated Ruby/Python bindings, preserving legacy registration and shared registry lifetime semantics.

- Add an explicit typed merge-driver lane with cached-only parser selection, kernel-owned merge results, bounded inputs, and staged conflict/output policies; retain legacy entry points during migration.

- Add bounded real-Git verification for explicit typed CLI merges, including conflict policies, cold grammars, parser failures, quoted paths, index preservation, and guaranteed disposable-repository cleanup.

- Route explicit diff-driver selections through typed kernel diff2 with cached-only grammars, bounded sources, JSON/file reports, kernel-derived change exits, and real-Git external-diff verification.

- Add bounded, read-only CLI artifact-manifest integrity checks as an availability prerequisite, keeping signatures, build provenance, provider validity, and runtime availability explicitly unverified.

- Embed Cargo build target/profile/features and explicitly declared source identity in CLI version JSON without runtime checkout discovery or provenance/availability claims.

- Add bounded, opt-in local CLI candidate manifest assembly from embedded identity, measured executable bytes, and explicit declarations; preserve unsigned and unverified status.

- Expose the linked typed-kernel compiled provider inventory in CLI version JSON; preserve exact workflow/parser descriptors in candidate assembly and reject declaration drift without implying runtime availability.

- Implement read-only conflicts diff JSON through ast-merge-git with exact source digests and ours/base/theirs byte ranges; reject malformed markers and bound input/region sizes.

- Unify compiled kernel and host workflow executors in the merge registry, protect compiled IDs from host mutation, and execute explicit compiled batches with kernel ownership and bounded typed results.

- Expose bounded source-free workflow/parser selection reports and a controlled variant across the Rust, Ruby and Python facades, sharing execution query validation and immutable snapshot traces without claiming preflight authority.

- Add opt-in detached SSH authentication for CLI candidate manifests with explicit digest-pinned trust inputs, bounded verification, and no implied runtime or publication authority.

- Add strict declared-asset byte verification and per-provider asset evidence to CLI manifest integrity and signature checks without inferring linkage or runtime availability.

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

- Check cancellation and deadlines before classifying native parser callback faults or contained panics, consistently with parser probing.

- Include top-level error diagnostics for native syntax rejection, unsupported analysis, and input parser service failures while retaining detailed typed origin evidence.

- Export Python source descriptors and line-ending DTOs with their native identity so public SourceInput construction works without private-module types.

- Reject native merge family analysis that changes verified source bytes or supplies invalid owner ranges before classification.

- Validate embedded native analysis results against request sources and reconstructed Rust family decisions, rejecting inconsistent parser, owner, layout and attachment evidence while preserving compatible fields.

- Regenerate Ruby and Python bindings with Rust enum-key hashing and without invented defaults for required DTOs.

- Accept typed Ruby common-operation policy variants and exercise analyze, diff2, merge3, cancellation and wrong-payload rejection through installed Psych callbacks.

- Return native Ruby canonical/migration conflict and diagnostic record objects with explicit typed factories and readers; add installed Ruby/Python record-boundary regression coverage.

- Preserve typed Ruby OperationPolicy values on request getters using native from_* factories; replace experimental Data variants, verify all four policy payload round trips, and remove invented Python wire discriminators.

- Render absent conflict alternatives as empty review sides, append explicitly placed review blocks for deleted ours owners, preserve source bytes, and count marker positions by actual newlines; enable the canonical delete/edit Git case.

- Regenerate the typed Ruby core dispatcher with bounded interrupt-aware waits so an idle registered parser does not hang Ruby 3.2 interpreter shutdown.

- Build the retained historical Ruby regression extension explicitly before checkout tests; keep this test-only step separate from typed-core artifact production and publication.

- Reject successful typed JSON merge results with forged input parse evidence, unrelated output request IDs, or contradictory output parser-selection records.

- Reject typed JSON diff and merge evidence that mixes registry snapshot generations or digests across input parsing and output verification.

- Emit the retained benchmark diagnostic prefix for typed-core parser rejections, allowing expected malformed-input failures to satisfy the existing negative-input contract without changing the corpus or gate.

- Validate successful common native merge parse evidence against input/output bytes, selected candidates and one registry snapshot, rejecting forged or missing output parse records.

- Validate successful native merge source partitions against input and output bytes, require preservation claims, and reject inconsistent retained-source projections.

- Derive the typed Rust release dependency closure from Cargo metadata, validate it in CI and before release commands, and provide a side-effect-free release inventory listing.

- Reject missing merge-driver option values and unknown required promotion statuses before output writes, preserving current files on malformed CLI and attribute policies.

- Reject merge-driver report destinations that alias inputs or output, including hard links, symlinks, and new output paths, before any source or output writes.

- Clean disposable Python core artifact environments on success, failure, and handled interruption; retain reports, refuse low-disk runs, and disable pip download caching.

- Clean Ruby typed-core artifact installations and source-package snapshots on completion or failure while preserving reports and explicit exports.

- Stage merge-driver output and reports before replacement so report failures and partial staging writes preserve current bytes; report output-commit uncertainty explicitly.

- Correct generated Python LineEndings and ParseOptions constructor declarations: omission uses Rust defaults, while explicit None is rejected. Audit installed constructor and method signatures through native and public exports.

- Describe Ruby unit-enum values with RBS symbol aliases instead of nonexistent classes, and verify declared classes, readers, methods, and source-role values against isolated installed core artifacts.

- Declare frozen API review snapshots and retained legacy regression sources outside active Alef generation ownership, preserving independent API drift checks.

- Correct generated Python struct constructor enum annotations and verify explicit enum inputs, optional None, and rejection of implicit string/int coercion in installed artifacts.

- Generated Python capability assertions compare profile identifiers and parser-language strings exactly rather than case-folding indexed fields.

- Common-operation diagnostics classify source and parser-result resource limits consistently as resource_limit, while preserving opaque provider-native error codes and failure evidence.

- Keep test-only Psych native byte-span projection consistent across BOM column conventions in Psych 5.3.1 and 5.5.0 without preprocessing source bytes.

- Reject ambiguous merge source forms, duplicate options and excess positional arguments before writes; validate diff path options and support read-only per-command help and option terminators.

- Write canonical typed CLI error reports for selection/source rejection and kernel failures without fabricating operation results; preserve no-write behavior for malformed invocations and unsafe report paths.

- Bound real-Git gate subprocess capture, file writes, deadlines, and live disk reserve; disable child core dumps and clean legacy disposable repositories while retaining compact CI evidence.

- Make CLI Git installation scope-aware and ownership-safe, preserve user configuration, and report partial failures without claiming complete driver setup.

- Normalize Git external-diff absent sides without opening devices and compare validated zero-byte JSON documents through kernel owner semantics, with real-Git added/deleted regression coverage.

- Pin negotiated compiled-workflow parsers before dispatch and output verification, fail without alternate probing when availability changes, and preserve policy-selection provenance and semantic/parser dialect separation.

- Require complete compiled parser/workflow declarations for non-development CLI manifest assembly, and expose strict shared inventory checks in integrity and signature verification.
