#!/usr/bin/env python3
"""Authenticate candidate manifest bytes with explicitly pinned SSH trust inputs.

No default release keys, private-key access, artifact execution or publication.
This local POSIX tool does not establish build provenance or runtime availability.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import tempfile

from check_cli_artifact_manifest import MAX_MANIFEST, Rejected, check, require, sha256
from typed_cli_git import RESERVE, command

ROOT = Path(__file__).resolve().parents[1]
NAMESPACE = "cli-artifact-manifest@structuredmerge.org"
SCHEMA = "structuredmerge.cli-artifact-authentication/v1"
MAX_TRUST = 1024 * 1024


def digest(raw):
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def read_bounded(path, maximum):
    require(path.is_file() and path.stat().st_size <= maximum,
            "authentication input is not a bounded regular file", "artifact.authentication_invalid")
    with path.open("rb") as stream:
        raw = stream.read(maximum + 1)
    require(len(raw) <= maximum, "authentication input grew beyond budget", "artifact.authentication_invalid")
    return raw


def pinned(path, expected):
    raw = read_bounded(path, MAX_TRUST)
    require(sha256(expected) and digest(raw) == expected,
            "trust input digest mismatch", "artifact.trust_mismatch")
    return raw


def authenticate(manifest, artifact, target, profile, signature, allowed_signers,
                 allowed_signers_digest, principal, assets=None, revoked_keys=None,
                 revoked_keys_digest=None):
    require(os.name == "posix", "SSH authentication currently requires POSIX resource limits",
            "artifact.authentication_unsupported")
    require(isinstance(principal, str) and 0 < len(principal) <= 1024
            and not any(c.isspace() or ord(c) < 32 or ord(c) == 127 for c in principal),
            "an explicit signer principal is required", "artifact.authentication_invalid")
    require((revoked_keys is None) == (revoked_keys_digest is None),
            "revoked keys and their digest must be supplied together", "artifact.authentication_invalid")
    raw = read_bounded(manifest, MAX_MANIFEST)
    signature_raw = read_bounded(signature, MAX_TRUST)
    trust = pinned(allowed_signers, allowed_signers_digest)
    revoked = None if revoked_keys is None else pinned(revoked_keys, revoked_keys_digest)
    verifier = shutil.which("ssh-keygen")
    require(verifier is not None, "ssh-keygen is unavailable", "artifact.authentication_unsupported")
    require(shutil.disk_usage(ROOT).free >= RESERVE + 4 * MAX_TRUST,
            "disk reserve reached", "artifact.authentication_failed")
    (ROOT / "tmp").mkdir(exist_ok=True)
    # Only bounded snapshots are passed to OpenSSH. The later integrity check
    # must observe exactly these signed manifest bytes, not a replaced document.
    with tempfile.TemporaryDirectory(prefix="manifest-auth-", dir=ROOT / "tmp") as temporary:
        work = Path(temporary)
        (work / "signature").write_bytes(signature_raw)
        (work / "allowed-signers").write_bytes(trust)
        argv = [verifier, "-Y", "verify", "-f", str(work / "allowed-signers"),
                "-I", principal, "-n", NAMESPACE, "-s", str(work / "signature")]
        if revoked is not None:
            (work / "revoked-keys").write_bytes(revoked)
            argv.extend(["-r", str(work / "revoked-keys")])
        # Verification needs public keys only, no agent, HOME, provider library,
        # network acquisition, or inherited SSH configuration overrides.
        env = {"PATH": os.defpath, "LC_ALL": "C", "TMPDIR": str(work)}
        try:
            result = command(argv, work, env, data=raw, timeout=10)
        except (AssertionError, OSError) as error:
            raise Rejected("artifact.authentication_failed", "bounded signature verifier failed") from error
        require(result.returncode == 0, "signature is not authorized by the supplied trust policy",
                "artifact.signature_invalid")
    integrity = check(manifest, artifact, target, profile, assets, digest(raw))
    return {"schema": SCHEMA, "passed": True,
            "scope": "caller-trusted-signature-and-explicit-byte-integrity",
            "manifest_digest": digest(raw), "signature_digest": digest(signature_raw),
            "signature_verified": True, "signature_format": "openssh-sshsig",
            "namespace": NAMESPACE, "authorized_principal": principal,
            "allowed_signers_digest": digest(trust),
            "revoked_keys_digest": None if revoked is None else digest(revoked),
            "revocation_checked": revoked is not None,
            "integrity": integrity, "build_provenance_verified": False,
            "runtime_availability_checked": False, "publication_authorized": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("manifest", "artifact", "signature", "allowed-signers"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("target", "allowed-signers-digest", "principal"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--profile", choices=("standalone", "embedded_host", "explicit_sidecar"), required=True)
    parser.add_argument("--revoked-keys", type=Path)
    parser.add_argument("--revoked-keys-digest")
    parser.add_argument("--asset", nargs=2, action="append", default=[], metavar=("ID", "FILE"))
    args = parser.parse_args()
    try:
        require(len({identity for identity, _ in args.asset}) == len(args.asset), "duplicate asset input")
        result = authenticate(args.manifest, args.artifact, args.target, args.profile,
            args.signature, args.allowed_signers, args.allowed_signers_digest, args.principal,
            {identity: Path(path) for identity, path in args.asset}, args.revoked_keys, args.revoked_keys_digest)
    except (Rejected, OSError, ValueError, KeyError, TypeError, RecursionError) as error:
        result = {"schema": SCHEMA, "passed": False,
                  "code": getattr(error, "code", "artifact.authentication_invalid"), "message": str(error),
                  "signature_verified": False, "runtime_availability_checked": False,
                  "publication_authorized": False}
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
