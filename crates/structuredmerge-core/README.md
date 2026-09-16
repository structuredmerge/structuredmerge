# StructuredMerge core

The typed facade for the shared Rust kernel and its generated language bindings.
It has no dependency on `structuredmerge-host-prototype-core`.

Implementation starts with the source/operation boundary from specification
Slices 722 and 1024–1027:

- immutable, validated source storage is owned by TreeHaver;
- source bytes, digests, encoding and line-ending evidence are preserved;
- semantic roles distinguish analyze, diff2, directional merge2 and merge3;
- merge-provider and parser-backend selections are separate typed values.

`OperationInputs` validates inputs before provider dispatch. It is not yet the
complete operation envelope: policy, extensions, result evidence, parser
selection, and merge execution follow in subsequent implementation slices.
The facade now exposes typed parser-host registration and `parse_sources`.
TreeHaver owns the registry, source validation, selection, dispatch, and result
validation. Callback batches contain generated records, not whole-operation
JSON strings. Native syntax failures retain their parse diagnostics.

Ruby and Python bindings are generated from the active `alef.toml`. Initial
Ruby/Psych and installed Python-wheel/LibCST callback smoke tests pass with
targeted local Alef fixes. Upstream Alef 0.89.0 cannot yet reproduce compiling
bindings for this surface: callback-only return DTO conversions and PyO3
trait-parameter construction/error conversion require upstream fixes. This is
development evidence, not an upstream-generation or publication gate pass.

The development generator is upstream v0.89.0 plus local commit `acd0469`
in the separate `tmp/alef-typed-bridge` checkout at workspace level. Its
`target/debug/alef generate --skip-compile --clean` produced this checkpoint;
generated sources were not hand-edited. `alef.toml` still names the official
release, not a fork dependency. Reproducible generation with the declared
upstream pin is therefore an **open gate**, not a passing check. Replace this
development checkpoint with output from a released upstream fix before release.

The facade also exposes `merge_yaml_mapping`, a concrete projection of the
existing YAML block-mapping operation. Generated Ruby/Psych tests exercise real
Rust-owned analysis, matching, conflicts, and rendering. Shared result fields
are retained, and native syntax rejection carries a typed parse result with its
source role. This profile-specific entry point is not the complete Slice 1025
provider-result envelope; service and unsupported-profile failures still use
the preliminary `CoreError` bridge.

`merge_python_declarations` now derives whole top-level Python owners in Rust
from native LibCST syntax facts. It shares parse/verification orchestration and
the owner merge engine with YAML. Installed-wheel tests cover assignments,
function/class bodies, conflicts, syntax failure, exact-byte retention, and
unsupported forms. See [`python-merge`](../python-merge/README.md) for the bounded
profile and remaining gates. Both operations return `NativeMergeResult`.

Successful native merges now include verified input/output source descriptors
and typed `RetainedSourceSegment` records for every output byte. Installed Ruby
and Python tests independently check each cited source slice and SHA-256. See
the [byte-evidence contract](../ast-merge/BYTE_EVIDENCE.md) for scope: exact output
partitions are implemented; protected-region dispositions and complete
policy-specific preservation envelopes remain open.

The complete operation and typed failure/preservation envelopes,
runtime lifecycle stress tests, the full Ruby artifact matrix, broader
Python public DTO ergonomics, and registry publication remain unfinished.
Legacy Ruby packaging files still coexist with the new package. The generated
source gem's broad glob remains unsafe for publication. The development
platform-artifact gate stages an explicit core-only file list, includes both
licenses, scopes the binary to the current Ruby ABI, checks linkage, and runs
all five Psych/merge examples from an isolated installed gem and fresh bundle:

```sh
cd packages/ruby
bundle exec rake compile
bundle exec ruby ../../workspace-scripts/check_core_ruby_artifact.rb
```

The gate leaves its gem, isolated consumer, and digest report in repository
`tmp/core-ruby-artifact-*`. It neither publishes nor establishes source-gem,
cross-platform, or upstream-generator reproducibility. CI now defines the
same Linux/Ruby 4.0 check; hosted execution remains unverified.

The Python artifact gate likewise audits the wheel's package identity, combined
license text, and absence of prototype files/executables, then runs the LibCST
merge suite in a fresh virtual environment and copied test directory:

```sh
python workspace-scripts/check_core_python_artifact.py PATH_TO_WHEEL
```

Use the generated Python package's Maturin configuration to build the wheel.
The gate leaves its consumer and digest report under `tmp/core-python-artifact-*`.
Linux CI defines Python 3.10 and 3.14 runs; only local Python 3.14 execution has
been verified so far. The license and typing-marker files are generated by
`alef scaffold --lang python`, not hand-authored package copies.

The next gates include the full artifact matrix and complete
operation/preservation contracts for both runtimes. Passing an encoded
operation through a host-owned merge does not meet that requirement. Existing
kernel mechanics should be reused; new parallel merge algorithms and
independent parser-selection registries are not intended.

Run the boundary tests with:

```sh
cargo test -p structuredmerge-core -p tree-haver --locked
```

Parser-service failures from both `parse_sources` and native merge calls use
stable `CoreError.code` values: `request.invalid`, `source.invalid`,
`selection.no_parser`, `resource.limit`, `execution.cancelled`,
`execution.deadline_exceeded`, `parser.provider_fault`, `parser.provider_panic`,
`parser.invalid_batch`, and `parser.invalid_result`. Input-byte and returned-node
limits share `resource.limit`. Native provider codes never replace these core
codes; their text remains in the human-readable message. Do not parse the
message as a structured contract.

This projection is not the complete Slice 1028 diagnostic envelope. Structured
origin/native-code fields, selection reports on failure, cause references, and
runtime-specific exception attributes remain to be implemented. Cancellation
codes here do not imply that the generated API already exposes cancellation
handles. Registration errors retain their existing separate codes.

Native syntax rejection retains all three validated input source descriptors,
ordered by semantic role, while leaving output descriptors and segments empty.
All input syntax outcomes are checked before family analysis; the primary
rejected parse is selected in base/ours/theirs order, independent of request
order. `input_parses` retains all three validated parse results, including native
diagnostics, syntax facts, provider descriptors, and selection snapshot digests,
on clean, conflicted, rendering-failed, and native-syntax-rejected results.
`rejected_parse` remains a compatibility shorthand for the first failing input.
Native warnings and errors belong to each parse result; they are not recoded as
merge diagnostics. Provider-service and unsupported-analysis exceptions still
need the full portable failure envelope.

`output_parse` separately retains the actual native reparse of rendered bytes,
including its parser identity, source digest, syntax facts, and diagnostics.
An explicit native rejection prevents owner analysis and clean output; its
parse evidence remains available even though `output`, `output_source`, and
`source_segments` are absent. `output_parse` is absent when no output reparse
occurred, including whole-source selection of an already validated input.
Absence is not a successful verification claim. `verification_failure` retains
callback exceptions and malformed verification results as `ParserFailure`, with
the stable core code, backend ID, and separate native code/message where
provided. Source validation and selection failures also have slots for source
identity and the selection report. No retry or parser substitution occurs.
These records are not the full Slice 1028 envelope: stage/request references,
causes, and portable category/origin nesting still need implementation. Input
service failures currently continue to raise `CoreError`.
