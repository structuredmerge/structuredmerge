# Common-operation binding integration status

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
