#!/usr/bin/env python3
"""Check a candidate CLI artifact manifest and explicit local bytes, not availability.

No executable is run, no asset locator is followed, and no provider is probed.
Signature/build provenance verification and runtime selection are separate gates.
"""
import argparse
import hashlib
import json
from pathlib import Path

SCHEMA = "structuredmerge.cli-artifact-manifest/v1"
REQUIRED = {
    "artifact_id", "artifact_version", "build_revision", "artifact_digest", "target", "profile",
    "schema_contracts", "built_in_provider_descriptors", "host_bridge_protocols", "grammar_assets",
    "grammar_installation_policy", "network_policy", "compiled_features", "platform_requirements",
}
MAX_MANIFEST = 1024 * 1024
MAX_ARTIFACT = 512 * 1024 * 1024
MAX_TOTAL = 1024 * 1024 * 1024


class Rejected(ValueError):
    def __init__(self, code, message):
        self.code = code
        super().__init__(message)


def require(condition, message, code="artifact.manifest_invalid"):
    if not condition:
        raise Rejected(code, message)


def digest_file(path, maximum=MAX_ARTIFACT):
    require(path.is_file(), "artifact must be a regular file")
    size = path.stat().st_size
    require(size <= maximum, "artifact exceeds verification size budget")
    digest = hashlib.sha256()
    count = 0
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(65536), b""):
            count += len(chunk)
            require(count <= maximum, "artifact grew beyond verification size budget")
            digest.update(chunk)
    require(size == count, "artifact changed size during verification")
    return "sha256:" + digest.hexdigest(), count


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate manifest field")
        result[key] = value
    return result


def invalid_constant(value):
    raise Rejected("artifact.manifest_invalid", f"non-JSON numeric constant: {value}")


def sha256(value):
    return (isinstance(value, str) and value.startswith("sha256:") and len(value) == 71
            and all(char in "0123456789abcdef" for char in value[7:]))


def strings(values):
    return (isinstance(values, list) and all(isinstance(v, str) and v for v in values)
            and len(values) == len(set(values)))


def validate(manifest):
    require(isinstance(manifest, dict) and manifest.get("schema") == SCHEMA, "unsupported manifest schema")
    require(REQUIRED <= manifest.keys(), "missing Slice 1032 artifact declaration fields")
    # Runtime observations do not belong in an immutable build declaration.
    require(not {"availability", "available", "registry_generation", "selected_provider", "default_approved"} & manifest.keys(),
            "runtime/default claims are not artifact declarations")
    for field in ("artifact_id", "artifact_version", "build_revision", "target"):
        require(isinstance(manifest[field], str) and manifest[field], f"invalid {field}")
    require(sha256(manifest["artifact_digest"]), "expected lowercase sha256 artifact digest")
    require(manifest["profile"] in ("standalone", "embedded_host", "explicit_sidecar"),
            "unsupported artifact profile", "artifact.profile_unsupported")
    contracts = manifest["schema_contracts"]
    require(isinstance(contracts, dict) and {"kernel", "operation", "provider", "diagnostic", "preservation"} <= contracts.keys(),
            "incomplete schema contracts")
    require(all(isinstance(v, str) and v for v in contracts.values()), "invalid schema contract")
    require(strings(manifest["compiled_features"]), "invalid compiled feature declarations")
    require(strings(manifest["platform_requirements"]), "invalid platform requirements")
    require(strings(manifest["host_bridge_protocols"]), "invalid bridge protocol declarations")
    require(manifest["network_policy"] == {"operation_time_acquisition": False}, "implicit operation network acquisition forbidden")
    policy = manifest["grammar_installation_policy"]
    require(isinstance(policy, dict) and policy.get("implicit_installation") is False
            and type(policy.get("explicit_preparation_supported")) is bool, "invalid grammar installation policy")
    assets = manifest["grammar_assets"]
    require(isinstance(assets, list) and len(assets) <= 1024, "invalid grammar asset declarations")
    ids = set()
    for asset in assets:
        require(isinstance(asset, dict), "invalid grammar asset")
        identity = asset.get("id")
        require(isinstance(identity, str) and identity and identity not in ids, "invalid/duplicate asset id")
        ids.add(identity)
        require(asset.get("source") in ("linked", "bundled", "external"), "invalid asset source")
        require(sha256(asset.get("digest")), "asset digest is required; unknown assets cannot claim integrity")
    providers = manifest["built_in_provider_descriptors"]
    require(isinstance(providers, list) and len(providers) <= 1024, "invalid provider declarations")
    identities = set()
    for provider in providers:
        require(isinstance(provider, dict), "invalid provider entry")
        identity, kind = provider.get("id"), provider.get("kind")
        require(isinstance(identity, str) and identity and kind in ("parser", "workflow"), "invalid provider identity/kind")
        require((kind, identity) not in identities, "duplicate provider declaration")
        identities.add((kind, identity))
        require(provider.get("origin") == "in_process", "host registrations are not built-in providers")
        require(isinstance(provider.get("contract"), str) and provider["contract"], "provider contract required")
        require(isinstance(provider.get("descriptor"), dict) and provider["descriptor"].get("id") == identity,
                "descriptor identity mismatch")
        require(strings(provider.get("asset_requirements")), "invalid asset requirements")
        require(set(provider["asset_requirements"]) <= ids, "provider references undeclared asset")
        require(not {"available", "selected", "approved_as_default"} & provider.keys(), "provider runtime/authority claim in manifest")


def check(manifest_path, artifact, expected_target, expected_profile, assets=None, expected_manifest_digest=None):
    require(manifest_path.is_file() and manifest_path.stat().st_size <= MAX_MANIFEST, "manifest exceeds size budget or is not a file")
    with manifest_path.open("rb") as stream:
        raw = stream.read(MAX_MANIFEST + 1)
    require(len(raw) <= MAX_MANIFEST, "manifest grew beyond size budget")
    manifest_digest = "sha256:" + hashlib.sha256(raw).hexdigest()
    if expected_manifest_digest is not None:
        require(sha256(expected_manifest_digest) and expected_manifest_digest == manifest_digest,
                "pinned manifest digest mismatch", "artifact.digest_mismatch")
    manifest = json.loads(raw, object_pairs_hook=unique_object, parse_constant=invalid_constant)
    validate(manifest)
    require(manifest["target"] == expected_target, "declared target differs from expected target", "artifact.platform_unsupported")
    require(manifest["profile"] == expected_profile, "declared profile differs from expected profile", "artifact.profile_unsupported")
    actual, size = digest_file(artifact)
    require(actual == manifest["artifact_digest"], "artifact bytes differ from manifest", "artifact.digest_mismatch")
    assets = assets or {}
    declared = {entry["id"]: entry for entry in manifest["grammar_assets"]}
    require(assets.keys() <= declared.keys(), "verification input names an undeclared asset")
    checks = []
    remaining = MAX_TOTAL - size
    for identity, entry in sorted(declared.items()):
        item = {"id": identity, "source": entry["source"], "digest": entry["digest"], "byte_integrity": "not_checked"}
        if identity in assets:
            digest, count = digest_file(assets[identity], min(MAX_ARTIFACT, remaining))
            remaining -= count
            require(digest == entry["digest"], "asset bytes differ from manifest", "grammar.asset_corrupt")
            item.update(byte_integrity="matched", byte_length=count)
        checks.append(item)
    return {"schema": "structuredmerge.cli-artifact-integrity-check/v1", "passed": True,
            "scope": "candidate-shape-and-explicit-byte-integrity",
            "manifest_digest": manifest_digest, "artifact_digest": actual, "artifact_byte_length": size,
            "declared_target": manifest["target"], "declared_profile": manifest["profile"], "assets": checks,
            "manifest_digest_pinned": expected_manifest_digest is not None,
            "signature_verified": False, "build_provenance_verified": False,
            "binary_target_verified": False, "provider_descriptors_verified": False,
            "runtime_availability_checked": False, "publication_authorized": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--profile", choices=("standalone", "embedded_host", "explicit_sidecar"), required=True)
    parser.add_argument("--manifest-digest")
    parser.add_argument("--asset", nargs=2, action="append", default=[], metavar=("ID", "FILE"))
    args = parser.parse_args()
    try:
        require(len({identity for identity, _ in args.asset}) == len(args.asset), "duplicate asset input")
        result = check(args.manifest, args.artifact, args.target, args.profile,
                       {identity: Path(path) for identity, path in args.asset}, args.manifest_digest)
    except (Rejected, OSError, ValueError, KeyError, TypeError, RecursionError) as error:
        result = {"schema": "structuredmerge.cli-artifact-integrity-check/v1", "passed": False,
                  "scope": "candidate-shape-and-explicit-byte-integrity",
                  "code": getattr(error, "code", "artifact.manifest_invalid"), "message": str(error),
                  "runtime_availability_checked": False, "publication_authorized": False}
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
