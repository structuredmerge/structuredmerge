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

`contracts/ruby-api-v1.json` is the compatibility baseline for the generated
Ruby facade, native Magnus module, and Ruby provider adapter. The gate checks
public constants, facade methods, native method arities, provider method
arities, and the platform extension basename. An intentional API change must
update the contract and changelog in the same commit. Alef generation is also
run clean twice in CI; either tracked drift or a new untracked generated file
fails the reproducibility gate.

The platform-package job also records artifact-level cold paths. It installs
the gem into a fresh `GEM_HOME`, then uses a fresh Ruby process for each require,
native TSLP JSON parser load, and first JSON merge sample. The report gates
successful installation, exact parser byte preservation, and the expected
clean merge result. Timings are evidence, not pass/fail thresholds, because
runner, filesystem, network, and package caches are not controlled.

Hosted Alef generation remains opt-in for cost control. Released Alef 0.94.0
supports the current `send_sync_types` configuration, so local and hosted
verification use the released tool. Do not add an unrecorded branch or
floating Git reference to the hosted build chain.

## Capability Discovery

Every application artifact exposes a deterministic capability manifest. Each
entry identifies the operation, format family, dialect, merge provider,
TreeHaver backend, support status, package version, and required extension
schemas. Selection is always explicit at the operation boundary.

The generated Ruby bundle returns the manifest as JSON:

```ruby
require "json"
require "structuredmerge_host_prototype"

manifest = JSON.parse(StructuredmergeHostPrototype.capability_manifest)
json_merge3 = manifest.fetch("operations").find do |capability|
  capability.fetch("family") == "json" && capability.fetch("operation") == "merge3"
end
raise "JSON merge3 is unavailable" unless json_merge3
```

`operations` contains only operations compiled into that artifact.
`parser_provider_factories` describes providers the artifact can register on
demand; it does not imply that every grammar has reviewed merge behavior.
`registered_providers` is the current process registry and includes the exact
version and validated descriptor captured when each parser or workflow host was
registered. Entries and registry snapshots are sorted by stable identifiers.

Support statuses are claims, not preference weights:

- `production` means the operation passed its declared Ruby-GM differential,
  preservation, malformed-input, and false-auto-merge gates.
- `experimental` means callers must opt into a reviewed but incomplete merge
  contract.
- `unsupported` records a known unavailable combination; callers must not
  substitute another provider implicitly.
- `host_runtime_only` exposes parsing or transport to the host without claiming
  a merge operation for that language.

Registration order, require order, and parser load order do not select a
provider. No provider may silently substitute itself after an explicit
selection fails. Automatic policy, when requested by the caller, is resolved by
TreeHaver from declared metadata and included in the result trace.

The manifest's `selection_policy` records this contract for machines. Registry
operations require an exact provider ID; an unavailable ID is an error even if
another compatible-looking provider is registered. Compiled operations such as
`rust.json.tslp.merge3` are bound to the provider named by their manifest entry
and do not perform runtime provider selection. Git-driver conflict-output modes
do not alter parser selection and still return an unresolved-conflict status.

## Non-Goals

- The Rust bundle does not replace Ruby-native parsers before evidence shows it
  can reproduce their behavior.
- Alef does not define merge semantics or parser preference.
- A host package does not become a monolithic dependency on every parser.
- Generated wrappers do not create alternate parse or merge entry points.
- Additional language bindings wait until the Ruby-driven API and ABI gates are
  stable.
