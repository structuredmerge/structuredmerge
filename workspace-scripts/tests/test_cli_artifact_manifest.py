import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "workspace-scripts/check_cli_artifact_manifest.py"
spec = importlib.util.spec_from_file_location("cli_artifact_manifest", SCRIPT)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class ArtifactManifestTest(unittest.TestCase):
    def test_required_fields_match_shared_slice_1032_policy(self):
        policy = json.loads((ROOT.parent / "fixtures/diagnostics/slice-1032-cli-artifact-provider-policy/contract.json").read_text())
        self.assertEqual(checker.REQUIRED, set(policy["artifact_manifest"]["required_fields"]))

    def setUp(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "tmp")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binary = self.root / "binary"
        self.binary.write_bytes(b"opaque test artifact; never executed")
        self.asset = self.root / "grammar"
        self.asset.write_bytes(b"opaque test asset; never loaded")
        self.path = self.root / "manifest.json"
        self.manifest = {
            "schema": checker.SCHEMA, "artifact_id": "smorg.test", "artifact_version": "0.2.0",
            "build_revision": "test-fixture-not-a-build-attestation", "target": "test-target", "profile": "standalone",
            "artifact_digest": checker.digest_file(self.binary)[0],
            "schema_contracts": {key: "test/v1" for key in ("kernel", "operation", "provider", "diagnostic", "preservation")},
            "built_in_provider_descriptors": [{"id": "parser.test", "kind": "parser", "origin": "in_process",
                "contract": "test/v1", "descriptor": {"id": "parser.test"}, "asset_requirements": ["json"]}],
            "host_bridge_protocols": ["bridge.test/v1"],
            "grammar_assets": [{"id": "json", "source": "external", "digest": checker.digest_file(self.asset)[0],
                                "locator": "https://must-not-be-fetched.invalid/grammar"}],
            "grammar_installation_policy": {"implicit_installation": False, "explicit_preparation_supported": False},
            "network_policy": {"operation_time_acquisition": False},
            "compiled_features": [], "platform_requirements": [],
        }

    def write(self, manifest=None):
        self.path.write_text(json.dumps(manifest or self.manifest))

    def check(self, **kwargs):
        return checker.check(self.path, self.binary, "test-target", "standalone", **kwargs)

    def test_exact_byte_check_does_not_confer_trust_or_availability(self):
        self.write()
        result = self.check(assets={"json": self.asset}, expected_manifest_digest=checker.digest_file(self.path)[0])
        self.assertTrue(result["passed"])
        self.assertTrue(result["manifest_digest_pinned"])
        self.assertEqual(result["assets"][0]["byte_integrity"], "matched")
        for field in ("signature_verified", "build_provenance_verified", "binary_target_verified",
                      "provider_descriptors_verified", "runtime_availability_checked", "publication_authorized"):
            self.assertFalse(result[field])

    def test_absent_explicit_asset_is_unchecked_not_missing_or_available(self):
        self.write()
        self.asset.unlink()
        self.assertEqual(self.check()["assets"][0]["byte_integrity"], "not_checked")

    def test_binary_asset_and_manifest_tampering_are_rejected(self):
        self.write()
        pinned = checker.digest_file(self.path)[0]
        self.binary.write_bytes(b"different artifact")
        with self.assertRaisesRegex(checker.Rejected, "artifact bytes differ"):
            self.check()
        self.binary.write_bytes(b"opaque test artifact; never executed")
        self.asset.write_bytes(b"different asset")
        with self.assertRaisesRegex(checker.Rejected, "asset bytes differ"):
            self.check(assets={"json": self.asset})
        self.path.write_text(self.path.read_text() + "\n")
        with self.assertRaisesRegex(checker.Rejected, "pinned manifest digest mismatch"):
            self.check(expected_manifest_digest=pinned)

    def test_missing_required_fields_reject(self):
        for field in checker.REQUIRED:
            manifest = copy.deepcopy(self.manifest)
            del manifest[field]
            with self.subTest(field=field), self.assertRaises(checker.Rejected):
                checker.validate(manifest)

    def test_runtime_claims_policy_bypasses_and_descriptor_confusion_reject(self):
        mutations = [lambda m: m.update(available=True),
                     lambda m: m.update(default_approved=True),
                     lambda m: m["network_policy"].update(operation_time_acquisition=True),
                     lambda m: m["grammar_installation_policy"].update(implicit_installation=True),
                     lambda m: m["built_in_provider_descriptors"][0].update(kind="bridge"),
                     lambda m: m["built_in_provider_descriptors"][0].update(origin="host"),
                     lambda m: m["built_in_provider_descriptors"][0].update(available=True),
                     lambda m: m["built_in_provider_descriptors"][0].update(asset_requirements=["missing"]),
                     lambda m: m["built_in_provider_descriptors"][0]["descriptor"].update(id="different"),
                     lambda m: m["grammar_assets"][0].update(digest=None)]
        for mutate in mutations:
            manifest = copy.deepcopy(self.manifest)
            mutate(manifest)
            with self.subTest(mutate=mutate), self.assertRaises(checker.Rejected):
                checker.validate(manifest)

    def test_duplicate_fields_and_non_json_constants_reject(self):
        self.write()
        valid = self.path.read_text()
        for raw in [valid[:-1] + ', "profile":"standalone"}', valid[:-1] + ', "extra":NaN}']:
            self.path.write_text(raw)
            with self.assertRaises(checker.Rejected):
                self.check()

    def test_platform_profile_and_undeclared_asset_mismatch_reject(self):
        self.write()
        for target, profile in [("wrong-target", "standalone"), ("test-target", "embedded_host")]:
            with self.assertRaises(checker.Rejected):
                checker.check(self.path, self.binary, target, profile)
        with self.assertRaisesRegex(checker.Rejected, "undeclared asset"):
            self.check(assets={"not-declared": self.asset})

    def test_process_failure_remains_failure_under_python_optimization(self):
        self.write()
        self.binary.write_bytes(b"tampered")
        for mode in ([], ["-O"]):
            result = subprocess.run([sys.executable, *mode, str(SCRIPT), "--manifest", str(self.path),
                "--artifact", str(self.binary), "--target", "test-target", "--profile", "standalone"],
                capture_output=True, text=True, timeout=10, check=False)
            self.assertEqual(result.returncode, 2)
            report = json.loads(result.stdout)
            self.assertEqual(report["code"], "artifact.digest_mismatch")
            self.assertFalse(report["passed"])
            self.assertEqual(result.stderr, "")

    def test_size_budgets_and_nonregular_inputs_reject(self):
        with self.assertRaises(checker.Rejected):
            checker.digest_file(self.root)
        with self.assertRaises(checker.Rejected):
            checker.digest_file(self.binary, maximum=2)
        self.write()
        with patch.object(checker, "MAX_TOTAL", self.binary.stat().st_size + 1):
            with self.assertRaises(checker.Rejected):
                self.check(assets={"json": self.asset})
        with self.path.open("wb") as stream:
            stream.truncate(checker.MAX_MANIFEST + 1)
        with self.assertRaises(checker.Rejected):
            self.check()
