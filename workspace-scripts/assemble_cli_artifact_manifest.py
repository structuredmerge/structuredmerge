#!/usr/bin/env python3
"""Assemble an unsigned candidate by executing an explicitly trusted local CLI.

Copies the supplied executable into bounded disposable scratch, runs only
--version --json, and combines embedded identity with explicit declarations.
No signing, provider discovery, asset loading, publication, or default approval.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time

from check_cli_artifact_manifest import (MAX_ARTIFACT, MAX_MANIFEST, Rejected,
    digest_file, invalid_constant, require, strings, unique_object, validate)

ROOT = Path(__file__).resolve().parent.parent
RESERVE = 20 * 1024**3
CAPTURE_LIMIT = 64 * 1024
DECLARED = {"profile", "schema_contracts", "built_in_provider_descriptors", "host_bridge_protocols",
            "grammar_assets", "grammar_installation_policy", "network_policy", "platform_requirements"}


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique_object, parse_constant=invalid_constant)


def child_limits():
    import resource
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_FSIZE, (CAPTURE_LIMIT, CAPTURE_LIMIT))


def version(artifact, work, expected_digest, timeout=10):
    (work / "artifact").mkdir()
    executable = work / "artifact" / artifact.name
    # Streaming copy is bounded even if the original grows during collection.
    with artifact.open("rb") as incoming, executable.open("xb") as outgoing:
        copied = 0
        for chunk in iter(lambda: incoming.read(65536), b""):
            copied += len(chunk)
            require(copied <= MAX_ARTIFACT, "artifact grew beyond copy budget")
            outgoing.write(chunk)
    executable.chmod(0o700)
    require(digest_file(executable)[0] == expected_digest, "artifact changed while copying", "artifact.digest_mismatch")
    env = dict(os.environ)
    env.update(PATH=str(work), TMPDIR=str(work), TREE_HAVER_LANGUAGE_PACK_CACHE_DIR=str(work / "cache"),
               TREE_SITTER_LANGUAGE_PACK_CACHE_DIR=str(work / "cache"),
               TREE_SITTER_LANGUAGE_PACK_LIBS_DIR=str(work / "libs"))
    (work / "cache").mkdir()
    (work / "libs").mkdir()
    out, err = work / "version.stdout", work / "version.stderr"
    with out.open("wb") as stdout, err.open("wb") as stderr:
        process = subprocess.Popen([str(executable), "--version", "--json"], cwd=work, env=env,
            stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, start_new_session=True,
            preexec_fn=child_limits)
        started = time.monotonic()
        try:
            while process.poll() is None:
                require(time.monotonic() - started < timeout, "version command exceeded deadline")
                require(shutil.disk_usage(work).free >= RESERVE, "disk reserve reached")
                time.sleep(0.01)
        finally:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
    require(out.stat().st_size < CAPTURE_LIMIT and err.stat().st_size < CAPTURE_LIMIT, "version output exceeds budget")
    require(process.returncode == 0 and err.stat().st_size == 0, "version command failed or emitted diagnostics")
    require(not list((work / "cache").iterdir()) and not list((work / "libs").iterdir()), "version command modified grammar state")
    require(digest_file(executable)[0] == expected_digest, "copied executable changed during observation", "artifact.digest_mismatch")
    return decode(out.read_bytes())


def validate_declarations(declarations):
    require(isinstance(declarations, dict) and declarations.get("schema") == "structuredmerge.cli-artifact-declarations/v1",
            "unsupported declaration schema")
    require(set(declarations) == DECLARED | {"schema"}, "declarations must not override measured identity/build fields")
    # Check operator declarations before running a program. Placeholder identity
    # is internal validation input only and is never emitted as an artifact.
    provisional = {key: declarations[key] for key in DECLARED}
    provisional.update(schema="structuredmerge.cli-artifact-manifest/v1", artifact_id="shape-check",
        artifact_version="unrecorded", build_revision="unrecorded", artifact_digest="sha256:" + "0" * 64,
        target="unrecorded", compiled_features=[])
    validate(provisional)


def assemble(declarations, observed, artifact_id, artifact_digest, development=False):
    validate_declarations(declarations)
    require(isinstance(observed, dict) and observed.get("schema") == "structuredmerge.cli-version/v1"
            and observed.get("cli_contract") == "structuredmerge.cli/v1", "unsupported version identity")
    require(observed.get("package") == "smorg" and observed.get("executable") in ("smorg", "smorg-rs"), "not a kernel CLI identity")
    require(isinstance(observed.get("version"), str) and observed["version"]
            and isinstance(observed.get("kernel_version"), str) and observed["kernel_version"], "missing version identity")
    build = observed.get("build")
    require(isinstance(build, dict) and build.get("schema") == "structuredmerge.cli-build/v1"
            and build.get("provenance_verified") is False, "unsupported build metadata")
    for key in ("target", "host", "cargo_profile", "cargo_opt_level", "cargo_debug"):
        require(isinstance(build.get(key), str) and build[key], f"missing build field {key}")
    require(strings(build.get("cargo_feature_flags")) and strings(build.get("target_features")), "invalid feature metadata")
    source = build.get("source")
    require(isinstance(source, dict) and source.get("verified") is False
            and source.get("state") in ("unknown", "clean", "dirty"), "invalid source identity")
    revision = source.get("revision")
    if revision is None:
        require(source["state"] == "unknown" and source.get("origin") == "unspecified", "unrecorded source cannot claim cleanliness")
    else:
        require(isinstance(revision, str) and len(revision) in (40, 64)
                and all(c in "0123456789abcdef" for c in revision)
                and source.get("origin") == "build-environment", "invalid source revision/origin")
    require(development or (revision is not None and source["state"] == "clean"),
            "dirty/unrecorded source requires --allow-development-build; never infer it from this checkout")
    inventory = observed.get("compiled_providers")
    coverage = None
    if inventory is not None:
        require(isinstance(inventory, dict) and inventory.get("schema") == "structuredmerge.compiled-provider-inventory/v1"
                and inventory.get("scope") == "typed-common-operation-kernel"
                and inventory.get("kernel_version") == observed["kernel_version"]
                and inventory.get("runtime_availability_checked") is False, "invalid compiled provider inventory")
        descriptors = {}
        for kind, field, identity_field in (("parser", "parsers", "id"), ("workflow", "workflows", "provider_id")):
            entries = inventory.get(field)
            require(isinstance(entries, list) and len(entries) <= 1024, "invalid compiled provider list")
            for descriptor in entries:
                require(isinstance(descriptor, dict), "invalid compiled descriptor")
                identity = descriptor.get(identity_field)
                require(isinstance(identity, str) and identity and (kind, identity) not in descriptors,
                        "invalid/duplicate compiled provider identity")
                descriptors[kind, identity] = descriptor
        for provider in declarations["built_in_provider_descriptors"]:
            require(descriptors.get((provider["kind"], provider["id"])) == provider["descriptor"],
                    "provider declaration differs from compiled descriptor")
        coverage = {"declared": len(declarations["built_in_provider_descriptors"]), "compiled": len(descriptors)}
    result = {key: declarations[key] for key in DECLARED}
    result.update(schema="structuredmerge.cli-artifact-manifest/v1", artifact_id=artifact_id,
        artifact_version=observed["version"], build_revision=revision or "unknown", artifact_digest=artifact_digest,
        target=build["target"], compiled_features=build["cargo_feature_flags"], build_identity=build,
        linked_kernel_version=observed["kernel_version"], executable=observed["executable"],
        compiled_feature_scope="cli-package-cargo-environment-only", candidate_only=True,
        signature_verified=False, build_provenance_verified=False, provider_descriptors_verified=False,
        runtime_availability_checked=False, publication_authorized=False)
    result["compiled_provider_inventory"] = inventory
    result["compiled_descriptor_coverage"] = coverage
    validate(result)
    return result


def collect(artifact, declarations_path, artifact_id, development=False, scratch=None):
    require(os.name == "posix", "local executable observation currently requires POSIX resource limits")
    artifact = artifact.resolve(strict=True)
    require(declarations_path.is_file() and declarations_path.stat().st_size <= MAX_MANIFEST, "declarations exceed size budget")
    with declarations_path.open("rb") as stream:
        raw = stream.read(MAX_MANIFEST + 1)
    require(len(raw) <= MAX_MANIFEST, "declarations grew beyond size budget")
    declarations = decode(raw)
    validate_declarations(declarations)
    artifact_digest, _ = digest_file(artifact)
    scratch = scratch or ROOT / "tmp"
    scratch.mkdir(exist_ok=True)
    require(shutil.disk_usage(scratch).free >= RESERVE + 1024**3, "requires 20 GiB reserve plus 1 GiB job budget")
    with tempfile.TemporaryDirectory(prefix="cli-manifest-observation-", dir=scratch) as directory:
        observed = version(artifact, Path(directory), artifact_digest)
        require(digest_file(artifact)[0] == artifact_digest, "original artifact changed during observation", "artifact.digest_mismatch")
        candidate = assemble(declarations, observed, artifact_id, artifact_digest, development)
    candidate["declarations_digest"] = "sha256:" + hashlib.sha256(raw).hexdigest()
    return candidate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--declarations", type=Path, required=True)
    parser.add_argument("--artifact-id", required=True)
    parser.add_argument("--execute-local-artifact", action="store_true", help="explicitly trust this local artifact to execute --version")
    parser.add_argument("--allow-development-build", action="store_true")
    args = parser.parse_args()
    if not args.execute_local_artifact:
        parser.error("--execute-local-artifact is required; this command runs the supplied trusted program")
    try:
        candidate = collect(args.artifact, args.declarations, args.artifact_id, args.allow_development_build)
        print(json.dumps(candidate, sort_keys=True, indent=2))
        return 0
    except (Rejected, OSError, ValueError, KeyError, TypeError, RecursionError) as error:
        # Failed assembly never puts a partial candidate on stdout.
        import sys
        print(json.dumps({"code": getattr(error, "code", "artifact.manifest_invalid"), "message": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
