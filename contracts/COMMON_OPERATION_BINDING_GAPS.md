# Common-operation binding integration status

## Typed policy round trips and declaration correction

Local Alef `92912f3` extends native payload enums to eligible tagged policies.
Ruby `OperationPolicy` is now a native class with `from_analyze`, `from_diff2`,
`from_merge2`, `from_merge3` and nullable typed readers, matching Python's API.
Request getters no longer turn the policy into a Hash. This replaces the
experimental, unreleased Ruby `OperationPolicyAnalyze`-style Data wrappers;
callers now use the native factories. The generated RBS describes the actual
class and methods rather than an empty class for a runtime module.

Python stubs no longer invent `type` discriminator dictionaries for untagged
or externally tagged enums. Explicitly tagged wire-shape helpers remain; this
does not establish complete wire-schema typing for every enum shape.

Generator checks pass: 134 Magnus tests and 235 PyO3 tests. Both native packages
build. Installed Ruby checks pass 29 examples plus six fixtures; Python passes
32 tests plus six fixtures. Every policy factory is exercised through a request
getter, preserving `false`, nil/None, empty vectors, labels, marker size and
passive JSON-leaf metadata. Common analyze/diff2/merge3 execution reuses the
returned policy. This is not evidence that merge2 execution is implemented.

Logs: `tmp/native-policy-roundtrip-{ruby,python}-build.log` and
`tmp/native-policy-roundtrip-{ruby,python}-artifact.log`. Package SHA-256:

- Ruby: `fff71a0d28d805b972d57f30764594228f045b61efcd7e4a91ef5823a0f7869f`.
- Python: `28869a73de40c01e4b94006b822a33c30d64fbca656f73c288bd8369a53edd96`.

This supersedes the tagged-policy output gap recorded in earlier stages below.
Other enum shapes, complete schema/domain round trips, conformance, consumer
migration, platform matrices, upstream-only generation and release/default
authority remain open. Alef changes remain local; no publication occurred.

## Native canonical record boundary

Local Alef `7ba8335` represents eligible untagged, single-named-payload record
enums as native Ruby objects. `ConflictRecord` and `DiagnosticRecord` now retain
the actual variant returned by Rust instead of becoming anonymous hashes.
`from_canonical`/`from_migration` factories and nullable typed payload readers
match the generated RBS. Implicit allocation and hash-to-variant guessing are
not supported on this typed path. Other enum shapes retain their existing
behavior and still need separate verification.

All 134 Magnus generator tests pass. The Ruby native build and isolated gem
tests pass (28 examples plus six generated fixtures). The unchanged Python wheel
passes 31 installed tests plus six fixtures. Both runtimes exercise real
edit/edit conflicts and parse errors, check canonical payload types, source
identities and decision IDs, round-trip payloads through native variant
factories, distinguish explicit migration records, and reject wrong payload
types. This verifies binding-native round trips, not every core conversion or
domain invariant on caller-created records.

Evidence: `tmp/native-record-ruby-build.log`,
`tmp/native-record-ruby-artifact.log`, and
`tmp/native-record-python-final-artifact.log`. Ruby platform gem SHA-256:
`6cc676236d54daa76df00cffcbf3c2a923ed31ffd499aac80d1580f6866b79a2`.
The Python wheel SHA remains the value recorded below. Tagged policy output,
complete record/schema round trips, full conformance, consumer migration,
platform matrices and upstream-only/release gates remain open.

## Current status: additive common exports enabled locally

Alef `6849273` generates typed Python tuple-variant factories (`from_analyze`,
`from_diff2`, `from_merge3`, etc.) without replacing the payload getters. Runtime
unit-enum hashing uses the discriminant, consistent with existing integer
equality. Factory discovery, DTO coercion discovery and stubs share the Python
variant selection. Generator tests pass: 234 PyO3 tests and 179 shared-generator
tests. These are targeted checks, not a full Alef suite or platform matrix.

The single active `alef.toml` now includes the common facade and DTOs. Both native
targets compile. Installed Python tests construct requests entirely from typed
DTOs and enum-keyed maps, execute analyze/diff2/merge3 with real LibCST callbacks,
and verify exact merged output plus output reparsing. The declaration checker
now distinguishes type-only `TypedDict` shapes from runtime classes. No JSON
operation tunnel or hand-edited generated code was introduced.
All 30 installed Python tests and six generated fixtures pass, including
wrong-policy-type rejection and cancellation before native callbacks.

**Ruby typed policy input now passes installed execution tests.** Local Alef
`2511d2e` extracts native DTO payloads directly from generated tagged-newtype
variants. `c526b0b` fixes their Ruby Data accessors, which previously called a
nonexistent superclass reader. There is no caller-side JSON workaround.
All 133 Magnus generator tests pass. The installed Ruby gem passes 27 examples
plus six generated fixtures, including typed common analyze/diff2/merge3 with
real Psych callbacks, exact merge output and output reparsing, and rejection of
wrong payload types and cancellation before callbacks.

Ruby native build log: `tmp/typed-ruby-policy-native-build.log`; installed proof:
`tmp/typed-ruby-policy-final-artifact.log`. Platform gem SHA-256:
`408bd22d1764975be94d0541d474d4dc9b79e192e34d695c8f75b4bdf3de803c`.
This fixes typed newtype input, not complete enum output or canonical-record
round trips. Other data-enum representations still need dedicated verification.

Logs under kernel `tmp/`: `typed-policy-{python,ruby}-build.log`,
`typed-policy-python-final-artifact.log`, `typed-policy-ruby-artifact.log`, and
`typed-policy-ruby-probe.log`. Python wheel SHA-256:
`0c5488ead559af872a2f7b3e849a0f8d4ed93e1daed88368c7ab181d731a36fd`.
These additive exports remain experimental: consumer migration, complete
sum-type/canonical-record round trips, broader policies/providers, full platform
coverage, upstream-only generation and release/default authority remain open.
All Alef fixes remain local. No package was published or default changed.

The sections below preserve the earlier trials and their evidence; their
restored-export status is superseded by this section.

The Rust facade now exposes `execute_operation` and
`execute_operation_controlled`, accepting the existing common `OperationRequest`
and returning `OperationResult`. They normalize inline content/bytes, use the
registered `ParserHost` through TreeHaver and apply one execution context across
normalization and execution. References are rejected without opening files.
Rust-only closure/resolver and evidence-validation methods are not binding APIs.

## Generator trial

A local Alef Ruby/Python generation trial included the common request, policy,
result, diagnostic and conflict DTOs plus the two functions. It generated output
but neither native target compiled. The active `alef.toml` and generated bindings
were restored using Alef to the preceding working export set. No generated code
was hand-patched; no prototype or JSON operation fallback was added.

Resolved configuration issues during the trial:

- Exported DTO map fields now use explicit map types instead of the `Metadata`
  alias, which Alef treated as an opaque object. This is the identical Rust type
  and serde contract, not a second transport representation.
- Exclude Rust-only validation methods and their internal dependency types from
  the candidate binding surface. They otherwise pulled closures, source maps and
  registry execution types into generated code.

Compilation issues observed in the initial trial (resolved in the local retry below):

- Enum-keyed maps (`SourceRole` to operation source/span) need usable generated
  key equality/hashing and conversion of keys in both directions. Generated
  conversion left core enum keys in maps requiring binding enum keys.
- Nested `Vec<BTreeMap<String, Value>>` conversion does not recursively convert
  the binding's map/value representation.
- Generated defaults assume required data-enum fields have `Default` in Python;
  Ruby untagged-record lowering also assumed defaults for required diagnostic
  and conflict records. Invalid fabricated defaults are not an acceptable fix.

The Python policy constructor also currently serializes dictionaries/kwargs via
Python `json.dumps`; typed variant construction must be checked before claiming
an ergonomic typed input boundary. Successful compilation will not by itself
close constructor, sum-type round-trip, canonical-record, or installed-call gates.

Reproduction evidence is retained locally under kernel `tmp/`:
`common-bindings-config.patch`, `common-bindings-generation.log`,
`common-bindings-python-build.log`, and `common-bindings-ruby-build.log`.
The patch is the candidate configuration, not an approved second configuration
or a release artifact. Fixes must live in the generator, with regression tests;
common contracts must not be weakened to fit generated code. Alef changes remain
local under the maintainer's instruction. Upstream-only generation and release
readiness are still unmet.

## Local conversion follow-up

Local Alef commit `ef37145` adds named-key conversion for maps with named values
and element-wise conversion for vectors of string-keyed JSON metadata maps,
including optional fields. All 102 conversion tests pass, with regressions for
both directions and the default configuration wrappers. Test-only fixture and
path-comparison corrections also restore compilation of Alef's library tests.

This is generator-unit evidence, not a successful repeat of the common binding
compile trial. Generated enum key equality/hashing, required data-enum defaults,
typed constructors and installed-call gates remain open. The active export set
is unchanged. The Alef commit remains local; no push or upstream PR was made.

## Compiling common exports and runtime follow-up

Local Alef `5b23c49` adds Rust equality/hashing to Python/Ruby unit-enum wrappers,
stops deriving defaults for Python structs that lack a core default, and stops
inventing Ruby data-enum defaults. Together with `ef37145`, both common-export
native builds now pass. The generator regression run passes 1,796 tests with
five ignored; PHP tests are excluded because their pinned Rust 1.98.1 toolchain
installation fails locally. This is not a full Alef test-suite pass.

The candidate Ruby gem passes its existing 25 installed examples and six
generated fixtures. The candidate Python wheel runs 27 installed tests, with
one declaration-check error: it treats stub-only `TypedDict` variant declarations
as runtime exports. That test must distinguish type-only declarations without
weakening checks for real runtime classes.

Direct probes against the installed candidate wheel establish additional gaps:

- `hash(_native.SourceRole.SOURCE)` raises `TypeError`: Rust `Hash` is not a
  Python `__hash__` implementation. Fixing Python hashing must preserve equality
  consistency, including the existing enum/integer equality behavior.
- Passing an `AnalyzePolicy` DTO to `OperationPolicy(operation="analyze",
  policy=policy)` raises `TypeError` in `json.dumps`. Generated tuple-variant
  factories must accept typed payloads rather than requiring a dictionary/JSON
  workaround. The current generated API has getters, but no such factories.

The candidate export configuration was restored to the previous set through
Alef. No common installed-call or consumer-adoption gate is closed by these
builds. Local logs: `common-bindings-verified-{python,ruby}-build.log` and
`common-bindings-installed-{python,ruby}.log` under kernel `tmp/`. Candidate
Python environment: `tmp/core-python-artifact-o55deh31/venv`. All Alef changes
remain local; publication and upstream-only generation remain open.

The restored active exports were regenerated, rebuilt and installed in isolated
consumers: Python passes 27 tests plus six fixtures; Ruby passes 25 examples plus
six fixtures. Logs: `tmp/common-restored-{python,ruby}-artifact.log`. These checks
cover the existing exports, not the excluded common-operation surface.
