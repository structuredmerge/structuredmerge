# CLI artifact integrity prerequisite

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
  string `asset_requirements`. The embedded descriptor's `id` must agree, and
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
  --artifact tmp/cli-build-identity-bin/smorg \
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
