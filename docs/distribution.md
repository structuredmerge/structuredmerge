# Distribution Architecture

StructuredMerge distributes one Rust merge kernel to several host runtimes
without flattening every parser into the same package. Parser selection remains
TreeHaver's responsibility in every runtime, and an unavailable explicitly
selected provider is an error.

## Package Roles

The distribution has four roles. A package may implement more than one role,
but the dependency direction remains fixed.

| Role | Rust shape | Responsibility |
| --- | --- | --- |
| Kernel | `tree-haver`, `ast-merge`, and the format substrate crates | Normalized parser contracts, cross-format merge mechanics, format-specific policy, diagnostics, and source-preserving rendering. |
| In-process provider | TSLP and native Rust provider crates | Register concrete parser capabilities with TreeHaver and implement provider extensions without changing substrate semantics. |
| Application bundle | `smorg-rs` and the generated native extension | Select a reviewed set of kernel and provider crates at compile time and expose capability discovery plus merge operations. |
| Host adapter | Thin code in the generated host package, such as Ruby's `WorkflowProvider` | Transport coarse operation envelopes to a host-native provider. It does not parse, select a substitute parser, or reimplement merge behavior. |

The format substrate is the shared implementation point. A parser-specific
provider depends on its format substrate. A bundle depends on the providers it
ships. A host adapter depends on the generated bridge contract and accepts an
already selected host provider. Dependencies must never point in the opposite
direction.

## Rust Artifacts

The Cargo workspace remains modular for embedders. The production application
facade will collect the operation envelope, provider registry, capability
manifest, and selected provider bundles. `smorg-rs` and generated native
extensions must use that facade rather than assemble different registries.

Provider inclusion is compile-time and auditable. The default application
bundle may include TSLP and reviewed Rust-native providers. Smaller bundles may
omit providers, but they must report the resulting capability set and fail when
a caller requests an omitted provider.

TSLP grammar loading may remain on demand. Capability discovery distinguishes
"compiled into this artifact" from "loaded in this process" so a cold process
does not under-report grammars merely because their parsers have not yet been
used.

## Ruby Artifact

The current `structuredmerge_host_prototype` gem is the generated integration
artifact while the ABI is still experimental. Its production successor will be
named `structuredmerge-rust` and expose the Rust application facade, not a
second Ruby implementation of merge behavior.

The prototype is distributed only as precompiled platform gems. Its private
host core and workspace path dependencies make the development source gem
non-installable outside this repository. A source gem may be added only after
the production facade is independently publishable and an isolated source-gem
install proves the complete Cargo graph is available. Development `rake build`
output is not a release artifact.

The generated Magnus layer and hand-maintained Ruby host adapter live in the
same gem. The adapter accepts providers from the existing Ruby package family;
it does not depend on every native parser gem or auto-register them. Parser and
format gems retain ownership of their TreeHaver registrations. Applications
explicitly require the providers they intend to make available.

Ruby-native parsers cross the boundary through coarse `analyze`, `diff2`,
`merge2`, or `merge3` operations with source segments and versioned extension
payloads. Node-by-node callbacks and parser-specific fallback branches are not
part of the ABI.

## Versioned Boundaries

Three versions are independent and must be reported in artifact provenance:

1. package version for the Rust crate, executable, or host package;
2. operation and result schema versions; and
3. provider capability and extension-schema versions.

A package release may preserve the ABI while adding a provider. A schema change
may require a new protocol version without renaming a package. Provider-native
extensions may evolve without forcing unrelated providers to adopt a lossy
common AST.

Generated code is release input. `alef verify --exit-code`, host binding tests,
and an isolated install of each platform package must pass from a clean
checkout before publication.

Until Alef's Ruby platform-metadata fix is released, CI installs the exact fork
revision recorded in `workspace-scripts/alef-source-revision`. The same revision
is included in artifact provenance. Return to the released installer after the
upstream fix ships; do not leave an unrecorded branch or floating Git reference
in the build chain.

## Capability Discovery

Every application artifact exposes a deterministic capability manifest. Each
entry identifies the operation, format family, dialect, merge provider,
TreeHaver backend, support status, package version, and required extension
schemas. Selection is always explicit at the operation boundary.

Registration order, require order, and parser load order do not select a
provider. No provider may silently substitute itself after an explicit
selection fails. Automatic policy, when requested by the caller, is resolved by
TreeHaver from declared metadata and included in the result trace.

## Non-Goals

- The Rust bundle does not replace Ruby-native parsers before evidence shows it
  can reproduce their behavior.
- Alef does not define merge semantics or parser preference.
- A host package does not become a monolithic dependency on every parser.
- Generated wrappers do not create alternate parse or merge entry points.
- Additional language bindings wait until the Ruby-driven API and ABI gates are
  stable.
