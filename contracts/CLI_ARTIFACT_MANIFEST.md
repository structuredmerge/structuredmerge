# CLI artifact integrity prerequisite

## Caller-trusted signature verification

`workspace-scripts/authenticate_cli_artifact_manifest.py` composes detached
OpenSSH signature verification with the candidate integrity checker below. It
authenticates the exact raw manifest bytes, not a reserialized representation.
This is an opt-in local POSIX build tool, not a choice of project release keys,
final distribution signature format, or runtime authorization policy.

```sh
python3 workspace-scripts/authenticate_cli_artifact_manifest.py \
  --manifest candidate.json --artifact dist/smorg \
  --target x86_64-unknown-linux-gnu --profile standalone \
  --signature candidate.json.sig \
  --allowed-signers trusted-allowed-signers \
  --allowed-signers-digest sha256:TRUST_FILE_SHA256 \
  --principal release-signer@example.invalid \
  --revoked-keys trusted-revocations \
  --revoked-keys-digest sha256:REVOCATIONS_SHA256 \
  --asset json path/to/explicitly-supplied-json-grammar
```

Trust inputs and their digest pins must come from an independently trusted
channel. Taking keys, principal and pins from the artifact publisher's untrusted
download would authenticate that supplied key, not establish release trust. No
keys are discovered from the manifest, checkout, user SSH agent or network.
OpenSSH's allowed-signers rules authorize the explicit principal, including any
namespace restrictions, certificate principals and validity periods. The tool
always requires namespace `cli-artifact-manifest@structuredmerge.org`; ordinary
`file` or Git signatures cannot substitute. The trusted installed `ssh-keygen`
executable and local clock are part of the verifier's trusted environment.

Revocation input is optional but, when supplied, requires its own digest pin.
Omission is explicitly reported as `revocation_checked: false`; it does not
assert non-revocation. Authorization freshness, key rotation, release identity,
rollback prevention, build attestation and the project's trust bootstrap remain
separate release-policy requirements. No private key is used by this tool.

Signature, allowed-signers and revocation inputs are each bounded to 1 MiB and
copied into disposable repository-local scratch. The verifier reuses the bounded
process observer: 10-second timeout, capped capture/file writes, disabled cores,
process-group cleanup and a live 20 GiB disk reserve. Scratch is removed on
success or failure. The integrity check is pinned to the signed raw bytes and
rejects manifest replacement between verification and byte checking. As with
the integrity checker, input paths assume trusted local regular files on a
stable filesystem; this is not an adversarial filesystem sandbox or a later
execution lease.

The separate `structuredmerge.cli-artifact-authentication/v1` report identifies
the authorized principal, namespace and signature/trust/manifest digests. Only
its signature flag becomes true. Its nested integrity report retains its own
scope and false signature flag because that checker does not authenticate.
Neither signature success nor an authorized signature over a development
candidate proves build provenance, complete descriptors, available grammars,
runtime health, default approval or publication authority. A signed malformed
manifest or mismatched artifact/explicit asset still fails closed. Exit 0 means
this scoped authentication passed; failures exit 2 with stable error codes.

Verification: all 83 tooling tests pass in `tmp/manifest-auth-tooling.log`.
Nine new tests use disposable Ed25519 keys and real OpenSSH verification, cover
wrong key/principal/namespace, expiry, revocation, changed manifest/artifact/asset,
invalid signed declarations, digest pins, verifier/budget failures, scratch
cleanup and optimized Python CLI behavior. An explicit retained CLI artifact
also round-trips through candidate assembly and authentication with a temporary
test key; this is not a release signature. Test keys, copied artifacts and
verification scratch are removed. No compiler or package installation is run.

## Candidate integrity checker

### Declared asset byte evidence

Both integrity and authentication commands accept `--require-all-assets`.
This requires a caller-supplied `--asset ID FILE` for every declared asset and
matching digests under the existing per-file and total byte budgets. Omission
fails with `artifact.asset_evidence_incomplete`, not `grammar.asset_missing`:
the checker has no evidence that an omitted file is absent from the machine.
The default inspection mode continues to report omitted files as unchecked.
Asset locators are never followed, grammars never loaded, and nothing downloaded.

The integrity report's `asset_evidence` records declaration/check counts,
sorted unchecked asset IDs, and per-provider declared/unchecked requirements.
Provider records are sorted by kind and stable ID. Their byte-integrity states
are `matched`, `not_checked`, or `no_declared_requirements`; the last state must
not be interpreted as proof of an asset-free provider. The boolean
`all_declared_bytes_verified` is scoped to the declarations and is vacuously
true when there are none; the counts remain explicit.

`requirement_completeness_verified`, `linkage_verified`, and
`runtime_loading_verified` remain false. Hashing a caller-supplied library cannot
prove that it is the copy bundled, linked, cached or loaded by a runtime. Nor
can it prove that operator declarations include every required grammar. The
same restriction applies to all three asset source labels. These byte checks
compose with signature and complete-inventory checks without becoming parser
availability or permission to execute.

Verification: 92 tooling tests pass in `tmp/manifest-assets-tooling.log`, with
explicit retained CLI and cached JSON grammar inputs. New tests exercise strict
omission rejection, per-kind provider evidence, empty requirements, every source
label, optimized-Python CLI behavior, and signed manifests with missing or
matched asset inputs. The real cached JSON library is hashed and authenticated
as a declared file without loading it. Disposable keys and verification scratch
are removed; no compiler output, grammar acquisition or installation is created.

### Compiled declaration completeness

Candidate assembly now requires complete typed provider declarations outside
`--allow-development-build`. In development mode, pass
`--require-complete-inventory` to require the same coverage. The integrity and
authentication commands also accept that flag; a valid signature does not waive
the requirement. Missing inventory or omitted compiled providers fail with
`artifact.inventory_incomplete`.

The shared comparison keys providers by `(kind, stable ID)`, compares each full
descriptor against the embedded compiled inventory, rejects invented or altered
descriptors and duplicate compiled identities, and checks recorded coverage
counts. Integrity reports include `compiled_inventory_coverage` with deterministic
`undeclared_providers`, counts, scope and a completeness flag. An absent inventory
is reported as null unless strict mode is requested. Partial development
manifests remain explicitly inspectable; they are not silently promoted.

This proves declaration consistency and coverage of the reported
`typed-common-operation-kernel` inventory, not semantic validation of every
descriptor field, binary-to-inventory provenance, or exhaustive coverage of
legacy CLI/benchmark implementations. Empty asset requirement declarations do
not prove that no grammar is needed. Host compatibility entries never count as
built-in provider declarations, and parser/workflow identities remain distinct.
`provider_descriptors_verified` and runtime/publication flags stay false.

Verification: 86 tooling tests pass in `tmp/manifest-inventory-tooling.log`.
Real retained CLI observation covers all eight workflows and seven parsers,
rejects workflow-only declarations in strict mode, and round-trips the complete
declarations through integrity checking and disposable-key authentication.
Tests also reject missing inventories, falsified counts, descriptor drift and
same-ID cross-kind substitution. Temporary keys, executable copies and signature
scratch are cleaned; this work creates no compiler target or installed package.

Slice 1032 requires a verified immutable artifact manifest before runtime
availability can be claimed. The source-free capability manifest has no build,
signature or asset-integrity evidence; it cannot serve that purpose. Existing
prototype Ruby provenance scripts are not a typed CLI artifact path and are not
reused or revived here.

`workspace-scripts/check_cli_artifact_manifest.py` is a read-only build/release
tool for **candidate shape and explicit byte integrity**, not an availability
implementation or publication gate. It does not execute the binary, load a
grammar, consult the current checkout as artifact evidence, follow an asset
locator, inspect caches, start a host, or contact a network.

```sh
python3 workspace-scripts/check_cli_artifact_manifest.py \
  --manifest candidate.json --artifact dist/smorg \
  --target x86_64-unknown-linux-gnu --profile standalone \
  --manifest-digest sha256:MANIFEST_SHA256 \
  --asset json path/to/explicitly-supplied-json-grammar
```

The manifest digest pin and asset inputs are optional. This command only writes
JSON to stdout; redirect evidence into the repository's `tmp/` directory.
Successful candidate/byte checks exit 0; rejected input exits 2 with a stable
failure code. No executable/release manifest is generated from guessed build
metadata. Release automation must eventually supply compiler/build-attested
declarations and authenticate the manifest independently.

## Candidate format

The local tooling format is `structuredmerge.cli-artifact-manifest/v1`. It is an
initial build-tool input, not the completed typed runtime availability wire API.
It requires every artifact declaration field listed by the shared Slice 1032
fixture; a test checks that list for drift:

- Nonempty strings: `artifact_id`, `artifact_version`, `build_revision`, `target`.
- `artifact_digest`: lowercase `sha256:` followed by 64 hexadecimal digits.
- `profile`: `standalone`, `embedded_host`, or `explicit_sidecar`.
- `schema_contracts`: string-valued map containing `kernel`, `operation`,
  `provider`, `diagnostic` and `preservation`. Version/ABI compatibility is not
  inferred from the presence of these declarations.
- `built_in_provider_descriptors`: entries with nonempty `id`, `kind` (`parser`
  or `workflow`), `origin: in_process`, `contract`, `descriptor`, and unique
  string `asset_requirements`. The embedded descriptor's `id` (parser) or
  `provider_id` (workflow) must agree, and
  every required asset must be declared. Full kind-specific descriptor/contract
  validation remains a separate gate; parser and workflow declarations are not
  treated as interchangeable.
- `host_bridge_protocols`: unique strings, compatibility declarations only.
  Neither installed host packages nor future host registrations are built-ins.
- `grammar_assets`: unique nonempty `id`, `source` (`linked`, `bundled`, or
  `external`), and an explicit SHA-256 `digest`. Unknown asset bytes cannot claim
  integrity in this candidate format. Additional locator metadata is opaque and
  never opened automatically.
- `grammar_installation_policy`: `implicit_installation: false` and a boolean
  `explicit_preparation_supported` declaration.
- `network_policy`: `operation_time_acquisition: false`.
- `compiled_features` and `platform_requirements`: unique string lists.

Top-level availability/registry/selection/default claims and provider-level
available/selected/approved flags are rejected. This prevents a build declaration
from masquerading as a runtime observation, but is not full descriptor semantic
validation. Duplicate JSON fields and non-JSON constants reject; no last-wins
parsing. Manifests are capped at 1 MiB, artifacts/assets at 512 MiB each and
1 GiB total; hashing streams 64 KiB chunks and rejects growth beyond the cap. The caller supplies
trusted local regular-file inputs on a stable filesystem. This is not an
adversarial concurrent-filesystem lock or a later execution lease.

## Meaning of a passing check

The report uses `structuredmerge.cli-artifact-integrity-check/v1` and explicitly
scopes `passed` to `candidate-shape-and-explicit-byte-integrity`. It verifies:

1. Candidate structure and declaration consistency.
2. Equality between declared and caller-expected target/profile (not executable
   architecture/ABI detection).
3. Artifact bytes against the declared digest, and raw manifest bytes against
   the caller's digest pin when supplied. A digest pin is not a signature.
4. Only explicitly supplied asset files against their declared digests. Omitted
   assets are `not_checked`, not `asset_missing`, corrupt, or available. A matched
   external file does not prove that a linked/bundled copy is actually present.

Every successful report still sets `signature_verified`,
`build_provenance_verified`, `binary_target_verified`,
`provider_descriptors_verified`, `runtime_availability_checked`, and
`publication_authorized` to false. Runtime code must not select providers or
approve defaults based on this report. Asset loading/probing still belongs to
TreeHaver and TSLP's public interface, not file hashes or cache discovery.

## Verification and remaining work

Ten unit tests cover the shared required-field list, byte/manifest/asset
tampering, duplicate fields and NaN, target/profile mismatches, missing fields,
unregistered host/runtime claim confusion, implicit acquisition, undeclared
assets, explicit not-checked assets, size limits, and failure behavior under
Python optimization. Temporary test inputs are removed on teardown. These are
synthetic integrity fixtures, not proof that a release artifact has a valid
manifest, signature or provider inventory.

Local verification: all 55 tooling tests pass; the log is
`tmp/cli-artifact-manifest-tooling-final.log`. No compiler target, executable copy
or asset installation was created. Temporary fixture inputs are removed by the
test harness, leaving only small logs; measured free space remains 134 GiB.

Next: compiler/build-owned manifest emission, authenticated manifest policy,
kind-specific typed descriptor checks, verified asset/linkage evidence, immutable
runtime snapshots and bounded probes, and pinned preflight execution. Only then
connect the complete evidence to `languages --json`; the existing discovery gate
remains 19/20, deliberately not made green with a declaration-only list.

## Embedded build identity

The CLI now includes optional `structuredmerge.cli-build/v1` metadata under
`build` in `--version --json`. Cargo's build script generates Rust constants in
its target-local `OUT_DIR`; there are no new build dependencies, Git subprocesses,
timestamps, source-tree scans or runtime checkout reads. The constants record
Cargo's target/host, profile, optimization/debug settings, package feature
environment flags and sorted target features. They are build-system inputs, not
a complete effective rustc-argument or transitive dependency-feature inventory.

Build automation may explicitly supply `SMORG_BUILD_REVISION` (a full lowercase
40- or 64-character Git object ID) and `SMORG_BUILD_SOURCE_STATE` (`unknown`,
`clean`, or `dirty`). Clean/dirty requires an explicit revision. The build script
validates syntax, not the existence or correctness of the claimed source commit.
Absent inputs remain null/unknown with origin `unspecified`; explicit inputs have
origin `build-environment`. `source.verified` and `provenance_verified` always
remain false. The current checkout is deliberately not used to fill missing
identity in exported, packaged or registry builds.

Cargo watches both source-identity variables and the build helper. Local testing
built once with the observed pre-change revision and a declared dirty state,
then rebuilt in the same target with those inputs removed. The second build
correctly regenerated null/unknown instead of reusing stale values. Changing
environment variables when running either executable cannot alter the embedded
identity; version tests run without external tools and cause no grammar writes.

Verification:

- All 109 CLI tests pass in both builds, including the three explicitly enabled
  warm grammar tests. Three build-rendering tests cover absent/invalid identity,
  complete Git IDs, Cargo fields, deterministic feature ordering and escaping.
- All 55 tooling tests pass; publication dependency inventory remains 20 crates.
- Both retained binaries pass six real-Git merge cases and the external-diff
  check. Reports: `tmp/typed-cli-git-lyodr7mm/report.json` and
  `tmp/typed-cli-git-0lz38vpi/report.json`.
- Portable discovery stays 19/20: fixtures
  `tmp/cli-conformance-_t3pjbkm/report.json` and
  `tmp/cli-conformance-49sokgkv/report.json`.
- Logs: `tmp/cli-build-identity-tests.log`,
  `tmp/cli-build-identity-default-tests.log`, `tmp/cli-build-identity-tooling.log`.
- Version observations: `tmp/cli-build-identity-declared.json` and
  `tmp/cli-build-identity-default.json`. The retained binary pair is the latter,
  with explicitly unknown source identity, not a source-attested release.
- `tmp/cli-build-identity-bin/smorg` SHA-256:
  `e5495ec9fc6b7e36c05f4028e65ab4d532031cb467658f9dd85fa2541e9ac35f`.
- `tmp/cli-build-identity-bin/smorg-rs` SHA-256:
  `28a7c1e727b2d4d41fbefb320405708c1032de678dc808ec769976dcf0782bbb`.

The bounded compiler target and superseded `typed-diff-bin` pair are removed after
verification. Current binaries and small evidence remain. This implements the
build-identity input, not complete manifest emission/authentication, asset linkage,
provider availability, preflight pinning or default authority. No package was
published and no upstream/local-only Alef policy changed.

## Local candidate assembly

`workspace-scripts/assemble_cli_artifact_manifest.py` combines the supplied
executable's embedded version/build identity and measured SHA-256 with explicit
operator declarations. It requires `--execute-local-artifact`: this runs a trusted
local program, not an untrusted-program sandbox. POSIX resource limits currently
make this observation tool POSIX-only.

```sh
python3 workspace-scripts/assemble_cli_artifact_manifest.py \
  --artifact tmp/workflow-query-bin/smorg \
  --declarations tmp/cli-manifest-assembly-declarations.json \
  --artifact-id smorg.local-candidate \
  --execute-local-artifact --allow-development-build \
  > tmp/cli-manifest-assembly-smorg.json
```

Declarations use `structuredmerge.cli-artifact-declarations/v1`, with exactly
`profile`, `schema_contracts`, `built_in_provider_descriptors`,
`host_bridge_protocols`, `grammar_assets`, `grammar_installation_policy`,
`network_policy`, and `platform_requirements`. They are validated before execution
and cannot override measured digest or embedded identity. Their raw-byte digest
is retained. Dirty or unknown source identity requires the explicit development
flag; even declared-clean identity is not authenticated provenance.

Observation copies at most 512 MiB into repository-local disposable scratch,
checks bytes before and after execution, limits stdout/stderr to 64 KiB each,
disables core dumps, enforces a ten-second deadline and 20 GiB free-space reserve,
and terminates the process group. Copies, captures and isolated grammar directories
are removed on success or failure. Empty PATH and isolated grammar directories
are not a network/security sandbox. The original executable is rehashed after
observation; no current-checkout identity is substituted.

Every output is explicitly an unsigned candidate with provenance, descriptor,
runtime availability and publication verification false. Cargo feature flags
describe only the CLI package environment, not all transitive build features.
Provider/asset declarations are neither compiler-derived nor proven complete.

Verification: all 62 tooling tests pass, including seven assembly tests covering
identity policy, override rejection, round-trip integrity, changed bytes, failed
commands, output flooding, timeout and scratch cleanup. Log:
`tmp/cli-manifest-assembly-tooling.log`. Both retained CLI names assemble and pass
the integrity checker; evidence is `tmp/cli-manifest-assembly-{smorg,smorg-rs}.json`
and matching `-check.json` files. Those local examples deliberately use empty
provider/asset declarations: they test assembly, not complete release inventories.
No compiler ran; no observation directories remain; free space remains 134 GiB.
Authenticated manifests, compiler-owned provider inventory, runtime availability
and preflight remain open. Discovery remains 19/20, not newly verified here.

## Compiled typed-kernel provider inventory

The linked kernel now owns `artifact_inventory::compiled_provider_inventory()`.
It returns the existing typed `MergeProviderDescriptor` and
`ParserProviderDescriptor` contracts, plus the operation-profile catalog. The
eight kernel workflow/backend descriptors derive from that catalog; parser
language resolution shares the capability-query helper. The seven cached-only
parser descriptors come directly from TreeHaver's non-loading constructors.
No registry mutation, provider callback, grammar probe, cache read or host-package
discovery occurs. Python/YAML workflows retain their native-extension requirement,
but no Psych/LibCST host descriptor or substituted TSLP parser is fabricated.
Capabilities name declared operations; no unconditional preservation guarantee or
default authority is inferred. Source/policy-specific acceptance remains execution's
responsibility.

Both executable names include this optional `compiled_providers` object in
version JSON. Its schema is `structuredmerge.compiled-provider-inventory/v1` and
scope is `typed-common-operation-kernel`, deliberately not all linked legacy or
benchmark implementations. The Rust build-tool API is not yet included in the
generated host facade. Generated Ruby/Python API surfaces are unchanged; Alef's
own regeneration refreshes its source-input provenance after the core changes.

Candidate assembly preserves this inventory and rejects operator descriptors
that differ from its exact kind/identity match. Coverage records declared versus
compiled counts, so empty local declarations cannot masquerade as a complete
inventory. Older version producers without the optional object retain null
inventory/coverage, not inferred evidence. Identity validation now respects the
workflow contract's `provider_id` field instead of incorrectly requiring parser
`id`. This does not authenticate executable claims or verify asset requirements,
linkage, grammar digests, contract compatibility, native extensions or liveness;
all existing trust/publication flags remain false.

Local verification uses one bounded compiler target, the inventory unit test,
109 CLI tests (including three warm-grammar cases), real Git merge/diff on both
names, candidate/integrity round trips and tooling tests. Current binaries live
in `tmp/compiled-provider-bin/`; logs and candidate evidence use
`tmp/compiled-provider-*`. The compiler target is removed after verification.
Authenticated manifests, registry integration, verified assets, runtime
availability and pinned preflight remain open; this does not implement `languages`.

Verification evidence:

- All 74 tooling tests pass with the real `smorg` binary explicitly supplied;
  the nine assembly tests also pass with `smorg-rs`. Real-artifact tests carry all
  eight workflow descriptors through candidate integrity checking and reject a
  changed package-version declaration; parser assets are not fabricated.
- Alef verification and both unchanged API review baselines pass. The typed
  publication closure remains 20 crates; no dependency was added.
- Real Git reports: `tmp/typed-cli-git-zg9mi072/report.json` and
  `tmp/typed-cli-git-im1sl9km/report.json`.
- Portable discovery remains 19/20 for each name, recorded in fixture reports
  `tmp/cli-conformance-7fy2935n/report.json` and
  `tmp/cli-conformance-y1fsg2_y/report.json` (under the fixtures repository).
- Current `smorg` SHA-256:
  `37050074bf7de37af8288ab7960cdf159e693daa879ca13e956cb8231ba64ff4`.
- Current `smorg-rs` SHA-256:
  `06316a21586a4b032dc17415de7931b9df9a4215b019e1b514e23cf328bbbb2e`.

The 2.5 GiB compiler target and superseded 47 MiB `cli-build-identity-bin` pair
are removed after verification. Current binaries and small evidence remain;
earlier build-identity hashes above are historical, not the retained pair.

Subsequent conflict-review work superseded `compiled-provider-bin` as well.
Current retained executable hashes and verification are recorded in
`contracts/CLI_CONFLICT_REVIEW.md`; the inventory and candidate formats above
remain unchanged by that command implementation.

Owned Git-install work subsequently superseded the conflict-review binary pair.
See `contracts/CLI_GIT_INSTALL.md` for current retained binary hashes and checks.

Git absent-side diff work subsequently superseded that pair; current hashes and
checks are in `contracts/CLI_GIT_ABSENT_SIDES.md`. Manifest formats are unchanged.

Unified workflow-registry work subsequently superseded that pair; current binary
hashes and checks are in `contracts/TYPED_WORKFLOW_HOST.md`. Runtime registry
integration does not authenticate the compiled declarations or manifest.

Parser-pinning verification subsequently superseded those binaries; the latest
artifact hashes are in the parser-pinning section of `TYPED_WORKFLOW_HOST.md`.

Source-free workflow-query work superseded those artifacts; current hashes are in
the source-free observation section of `TYPED_WORKFLOW_HOST.md`. These typed
observations do not replace authenticated manifest/runtime availability evidence.
