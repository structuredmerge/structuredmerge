import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import unittest
from unittest.mock import patch

import test_cli_artifact_manifest as candidate_tests

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "workspace-scripts"))
import authenticate_cli_artifact_manifest as auth
import assemble_cli_artifact_manifest as assembler
sys.path.pop(0)


@unittest.skipUnless(os.name == "posix" and shutil.which("ssh-keygen"), "requires POSIX OpenSSH")
class ManifestAuthenticationTest(unittest.TestCase):
    def setUp(self):
        candidate_tests.ArtifactManifestTest.setUp(self)
        self.path.write_text(json.dumps(self.manifest))
        self.key = self.root / "test-key"
        self.invoke("-q", "-t", "ed25519", "-N", "", "-f", str(self.key))
        self.trust = self.root / "allowed-signers"
        self.trust.write_text("test@example.invalid " + self.key.with_suffix(".pub").read_text())
        self.sign()

    def invoke(self, *args):
        result = subprocess.run(["ssh-keygen", *args], stdin=subprocess.DEVNULL,
                                capture_output=True, timeout=10, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)

    def sign(self, namespace=auth.NAMESPACE):
        self.signature = Path(str(self.path) + ".sig")
        self.signature.unlink(missing_ok=True)
        self.invoke("-Y", "sign", "-f", str(self.key), "-n", namespace, str(self.path))

    def authenticate(self, **overrides):
        args = dict(manifest=self.path, artifact=self.binary, target="test-target", profile="standalone",
                    signature=self.signature, allowed_signers=self.trust,
                    allowed_signers_digest=auth.digest(self.trust.read_bytes()), principal="test@example.invalid")
        args.update(overrides)
        before = set((ROOT / "tmp").iterdir())
        try:
            return auth.authenticate(**args)
        finally:
            self.assertEqual(before, set((ROOT / "tmp").iterdir()), "authentication scratch leaked")

    def test_real_signature_binds_manifest_and_artifact_without_authorizing_runtime(self):
        result = self.authenticate(assets={"json": self.asset})
        self.assertTrue(result["passed"])
        self.assertTrue(result["signature_verified"])
        self.assertFalse(result["revocation_checked"])
        self.assertEqual(result["authorized_principal"], "test@example.invalid")
        self.assertEqual(result["manifest_digest"], result["integrity"]["manifest_digest"])
        self.assertEqual(result["integrity"]["assets"][0]["byte_integrity"], "matched")
        for name in ("build_provenance_verified", "runtime_availability_checked", "publication_authorized"):
            self.assertFalse(result[name])

    def test_wrong_principal_key_namespace_and_manifest_reject(self):
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate(principal="other@example.invalid")
        trust = self.trust.read_bytes()
        other = self.root / "other-key"
        self.invoke("-q", "-t", "ed25519", "-N", "", "-f", str(other))
        self.trust.write_text("test@example.invalid " + other.with_suffix(".pub").read_text())
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate()
        self.trust.write_bytes(trust)
        self.sign("file")
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate()
        self.sign()
        self.path.write_bytes(self.path.read_bytes() + b"\n")
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate()

    def test_trust_pin_revocation_and_expiry_reject(self):
        with self.assertRaisesRegex(auth.Rejected, "trust input digest mismatch"):
            self.authenticate(allowed_signers_digest="sha256:" + "0" * 64)
        revoked = self.root / "revoked"
        revoked.write_bytes(self.key.with_suffix(".pub").read_bytes())
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate(revoked_keys=revoked, revoked_keys_digest=auth.digest(revoked.read_bytes()))
        revoked.write_bytes(b"")
        result = self.authenticate(revoked_keys=revoked, revoked_keys_digest=auth.digest(b""))
        self.assertTrue(result["revocation_checked"])
        with self.assertRaisesRegex(auth.Rejected, "supplied together"):
            self.authenticate(revoked_keys=revoked)
        self.trust.write_text('test@example.invalid valid-before="20000101Z" ' + self.key.with_suffix(".pub").read_text())
        with self.assertRaisesRegex(auth.Rejected, "not authorized"):
            self.authenticate()

    def test_signed_manifest_does_not_excuse_artifact_or_asset_tampering(self):
        self.binary.write_bytes(b"tampered")
        with self.assertRaisesRegex(auth.Rejected, "artifact bytes differ"):
            self.authenticate()
        self.binary.write_bytes(b"opaque test artifact; never executed")
        self.asset.write_bytes(b"tampered")
        with self.assertRaisesRegex(auth.Rejected, "asset bytes differ"):
            self.authenticate(assets={"json": self.asset})

    def test_replacement_after_verification_rejects_instead_of_authenticating_other_bytes(self):
        original = auth.command

        def replace(*args, **kwargs):
            result = original(*args, **kwargs)
            self.assertEqual(result.returncode, 0)
            self.path.write_bytes(self.path.read_bytes() + b"\n")
            return result

        with patch.object(auth, "command", side_effect=replace):
            with self.assertRaisesRegex(auth.Rejected, "pinned manifest digest mismatch"):
                self.authenticate()

    def test_limits_and_verifier_failure_clean_scratch(self):
        with patch.object(auth, "MAX_TRUST", 1):
            with self.assertRaisesRegex(auth.Rejected, "bounded regular file"):
                self.authenticate()
        with patch.object(auth, "command", side_effect=AssertionError("deadline")):
            with self.assertRaisesRegex(auth.Rejected, "bounded signature verifier failed"):
                self.authenticate()
        with patch.object(auth.shutil, "which", return_value=None):
            with self.assertRaisesRegex(auth.Rejected, "unavailable"):
                self.authenticate()
        with patch.object(auth, "RESERVE", 10**30):
            with self.assertRaisesRegex(auth.Rejected, "disk reserve reached"):
                self.authenticate()

    def test_authorized_signature_does_not_excuse_invalid_manifest(self):
        self.manifest["network_policy"]["operation_time_acquisition"] = True
        self.path.write_text(json.dumps(self.manifest))
        self.sign()
        with self.assertRaisesRegex(auth.Rejected, "network acquisition forbidden"):
            self.authenticate()

    @unittest.skipUnless(os.environ.get("SMORG_TEST_ARTIFACT"), "requires explicitly supplied built CLI")
    def test_real_cli_candidate_authentication_roundtrip(self):
        binary = Path(os.environ["SMORG_TEST_ARTIFACT"])
        declarations = {key: self.manifest[key] for key in assembler.DECLARED}
        declarations.update(schema="structuredmerge.cli-artifact-declarations/v1",
                            built_in_provider_descriptors=[], grammar_assets=[], host_bridge_protocols=[])
        declaration_file = self.root / "declarations.json"
        declaration_file.write_text(json.dumps(declarations))
        candidate = assembler.collect(binary, declaration_file, "local-signature-test", True, self.root)
        self.path.write_text(json.dumps(candidate))
        self.sign()
        # This key authenticates only a local test candidate. Partial descriptor
        # declarations and unknown source provenance must not become release truth.
        result = self.authenticate(artifact=binary, target=candidate["target"])
        self.assertTrue(result["signature_verified"])
        self.assertEqual(result["integrity"]["artifact_digest"], candidate["artifact_digest"])
        self.assertFalse(result["build_provenance_verified"])
        self.assertFalse(result["publication_authorized"])

    def test_cli_optimized_success_and_failure(self):
        args = [sys.executable, "-O", str(ROOT / "workspace-scripts/authenticate_cli_artifact_manifest.py"),
                "--manifest", str(self.path), "--artifact", str(self.binary), "--signature", str(self.signature),
                "--allowed-signers", str(self.trust), "--allowed-signers-digest", auth.digest(self.trust.read_bytes()),
                "--principal", "test@example.invalid", "--target", "test-target", "--profile", "standalone"]
        for tamper in (False, True):
            if tamper:
                self.signature.write_bytes(b"invalid signature")
            result = subprocess.run(args, capture_output=True, timeout=15, check=False)
            self.assertEqual(result.returncode, 2 if tamper else 0)
            self.assertEqual(result.stderr, b"")
            report = json.loads(result.stdout)
            self.assertEqual(report["signature_verified"], not tamper)
            self.assertFalse(report["publication_authorized"])
