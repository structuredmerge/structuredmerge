# Typed WorkflowHost implementation status

`structuredmerge_core::workflow` implements the initial Rust boundary specified
by the spec repository's `TYPED_WORKFLOW_HOST_CONTRACT.md`. It uses
`ast_merge::provider_registry` and `provider_selection`; no prototype dependency
or additional registry implementation is introduced.

The facade exports registration, generation-checked replacement/removal,
inventory and explicit-provider batch execution, with a controlled variant.
Hosts receive existing typed operation requests and TreeHaver-validated parse
facts, then return existing typed operation results in one coarse batch. The
kernel marks execution host-owned and not approved as default.

Tests use instrumented Rust providers and an analysis callback that computes
its result from supplied parse facts. They prove request/source correlation,
coarse batching, opaque request forwarding, cumulative limits, malformed-batch
rejection before probes, prepared-payload limits, returned identity/cardinality
validation, no retry, fault redaction, reserved-field shadowing rejection,
cancellation precedence and in-flight registry retirement. They are boundary
tests, not native parser or merge semantic parity evidence.

Verification is recorded in `tmp/workflow-final-tests.log`: core and ast-merge
tests and strict Clippy. Existing native opt-in tests are not claimed by this
run.

Alef now generates the Ruby/Python registry functions, workflow DTOs and trait
bridges. The callback receives a native `OperationControl` whose cloned Rust
handle shares the caller's cancellation state; it is not a serialized control
message. Python provider descriptors and parser requirements are native
re-exports, so their public and callback/result identities agree. Three local-only
Alef fixes supply native opaque callback arguments and resolve shared `host`
parameter names by trait type before name fallback, and retain serializable
nested request records containing data enums.

Installed-artifact tests exercise LibCST/Psych analysis callbacks over two
TreeHaver-prepared operations, Unicode/BOM source identity, typed results, one
coarse host callback and cancellation from inside that callback. The analysis
is a native-fact node count, not a migrated merge implementation. Broad runtime
lifecycle/stress checks and independently packaged native workflow providers
remain open. Build and installed-consumer evidence is retained under
`tmp/workflow-bindings-*.log`. Completed isolated runs pass 49 Python boundary
tests, 113 generated e2e tests and 114 test-app tests; Ruby passes 42 boundary
examples and 110 examples in each generated suite. Python uses 3.14.2/LibCST
1.9.0; Ruby uses 4.0.6. Reports are retained at
`tmp/core-python-artifact-cqc1qgob/report.json` and
`tmp/core-ruby-artifact-20260917-2383726-b0ukqd/report.json`.
The successful Python rebuild is `tmp/workflow-bindings-python-build.log`;
the earlier combined build log retains the diagnosed, now-fixed generator error.
Both API baselines, all 40 workspace-script tests and local Alef verification
pass. Verification's presence-only/declared-ownership entries are not content
checks. Local Alef commits `a82b77e`, `8e40b19` and `16b8ed0` remain unpushed;
464 trait-bridge and 239 PyO3 generator tests pass.
The test-only Psych projector probes whether the installed parser counts a BOM
in character columns; it always parses the original document bytes. This keeps
the isolated default Psych 5.3.1 and local Psych 5.5.0 byte ranges consistent.

## Installed lifecycle evidence

`tmp/workflow-lifecycle-python.log` and `tmp/workflow-lifecycle-ruby.log` extend
the same installed artifacts' coverage to:

- Eight registration/GC/retirement cycles per runtime, including Ruby compaction.
- Reentrant replacement with stale-generation rejection and unchanged in-flight
  provider identity, results and selection generation; the next call uses the
  replacement. Previously returned inventories remain unchanged.
- Self-unregistration during the callback, preserving the captured batch but
  preventing a later dispatch, with no retry.
- Sixteen overlapping calls on four host-created threads, preserving Python
  context variables and Ruby thread-local values on the calling thread.
- Cross-thread cancellation and registry retirement while a callback is paused;
  the late successful host result is discarded after cooperative release.
- Nine fresh subprocess exits per runtime: registered, retired and
  cancelled-and-drained hosts, each repeated three times with a 20-second bound.

The completed runs pass 55 Python boundary tests and 48 Ruby boundary examples;
generated e2e/test-app counts remain 113/114 and 110/110 respectively. All 40
workspace-script tests and both API baselines pass. Reports are
`tmp/core-python-artifact-c7p7fo86/report.json` and
`tmp/core-ruby-artifact-20260917-2391920-lj0xd1/report.json`. No binding rebuild or
API change was needed; artifact digests match the preceding binding slice.
Temporary installed consumers were removed, leaving only their small reports.

This is Linux CPython 3.14.2/MRI 4.0.6 evidence for synchronous host-thread calls
and cooperative draining. It does not establish abrupt VM teardown with active
callbacks, foreign Rust-thread dispatch, free-threaded Python, other Ruby
implementations, or the remaining platform/runtime matrix.

The initial error path returns `CoreError`; full portable failure/selection
envelopes remain open. Host availability, versioned parser-profile support,
family-default workflow execution, allowed delegation, complete portable batch
conformance and broad binding-runtime guarantees also remain open. No existing
kernel profile or native default changes, and no new package is published.

## Unified compiled/host executor registry

The facade now seeds the same merge registry with eight compiled executor handles
derived from the common-operation catalog. Host handles continue to use the same
generation, descriptor normalization, snapshot and lifetime machinery. Inventory
is no longer an empty-or-host-only list; consumers must select by provider ID.
Initialization registers no parsers and probes/loads no grammar. Compiled IDs
reject registration, replacement and retirement through all host mutation APIs,
without changing the registry generation.

Explicit compiled batches validate the entire request and profile-owned parser
queries before negotiation, capture the existing provider/parser snapshots, then
call the native operation engines. They preserve typed kernel results, check
negotiated identity and common result validation, enforce cumulative source and
response budgets, and share cancellation/deadline control. They report `kernel`
ownership and `approved_as_default: false`. Host batches retain the coarse
callback and `host` ownership. The common single-operation facade resolves known
compiled profiles through this registry too; unsupported profiles retain their
portable error results, with no implicit host dispatch or fallback.

Alef generates the added Python `WorkflowExecutionOwner.KERNEL` and Ruby
`:kernel` variant. Existing `HOST`/`:host` values and function signatures are
unchanged. The generated surface was reviewed and API baselines were refreshed
with the owning tool. Older retained bindings cannot represent the new owner;
use the newly verified artifacts rather than mixing a stale native extension
with updated declarations.

Local verification:

- All 15 core unit tests pass, including all four JSON operations in one
  compiled batch, protected inventory, invalid/cold queries, budgets, and
  existing host lifecycle/fault tests: `tmp/workflow-registry-tests.log`.
- All 128 CLI tests and 74 tooling tests pass:
  `tmp/workflow-registry-cli-tests.log`, `tmp/workflow-registry-tooling.log`.
- Both real-Git merge and modified/added/deleted diff gates pass:
  `tmp/typed-cli-git-_7dsmlh9/report.json`,
  `tmp/typed-cli-git-d2iqb5st/report.json`.
- CPython 3.14.2/LibCST 1.9.0 installed wheel: 57 boundary tests, 113 generated
  tests and 114 test-app tests pass. Report:
  `tmp/core-python-artifact-gefjl_5t/report.json`.
- MRI 4.0.6 installed gem: 50 boundary examples and 110/110 generated/test-app
  examples pass, with linkage and type checks. Report:
  `tmp/core-ruby-artifact-20260918-2567582-jw50mt/report.json`.
- Both runtimes verify all three compiled-ID mutation protections. Existing
  host replacement, retirement, GC, concurrency and cancellation checks still
  pass with the compiled entries present. Installed tests validate the new
  inventory/lifecycle behavior; the four-operation compiled-batch proof is Rust.
- Local Alef verification, both API baselines and the 20-crate release inventory
  pass. Alef's presence-only entries remain weaker than content verification.

Current retained artifacts and SHA-256:

- `tmp/workflow-registry-bin/smorg`:
  `e9681cdbd7042f19c8ac5822549626980337db3df3450ebaa497032eee83b4e2`.
- `tmp/workflow-registry-bin/smorg-rs`:
  `5dc24fc3048cd2f294131f64c301851ef39a6d6376bfcaf85750c61bf4241f92`.
- `tmp/workflow-registry-wheels/structuredmerge_core-0.2.0-cp310-abi3-manylinux_2_34_x86_64.whl`:
  `1cfe495a8b683e2fa62a878c79e784238d50eabbc91bff3cb19e08b49b5e27f3`.
- `packages/ruby/lib/structuredmerge_core_rb.so`:
  `89a59382cf080ffd06c2d2fc56546cd3b7a7e6b1d394085073121b50ab8c2703`.

One heavy build ran at a time, with incremental/debug data disabled, no core
dumps, and live 30 GiB free-space/8 GiB target guards. The 4.9 GiB target, 39 MiB
temporary build environment, superseded CLI pair and old cached-binding wheel
are removed after checks. Disposable installed consumers are removed by their
runners; current binaries/wheel/extension and small evidence remain.

Registry presence still does not prove parser availability, authenticated assets,
default authority or platform support. Negotiation is not a liveness lease;
changed parser identity fails closed but preflight/stale-snapshot envelopes remain
open. Common single-operation host dispatch, family defaults, delegation,
broader compiled-provider batch coverage and other runtime/platform gates remain.
Nothing is published or pushed upstream; the local-only Alef constraint remains.
