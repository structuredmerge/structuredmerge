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
