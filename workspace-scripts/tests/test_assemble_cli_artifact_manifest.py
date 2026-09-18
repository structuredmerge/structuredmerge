import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "workspace-scripts"))
import assemble_cli_artifact_manifest as assembler
import check_cli_artifact_manifest as checker
sys.path.pop(0)


class AssembleManifestTest(unittest.TestCase):
    def setUp(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "tmp")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.declarations = {
            "schema": "structuredmerge.cli-artifact-declarations/v1", "profile": "standalone",
            "schema_contracts": {key: "test/v1" for key in ("kernel", "operation", "provider", "diagnostic", "preservation")},
            "built_in_provider_descriptors": [], "host_bridge_protocols": [], "grammar_assets": [],
            "grammar_installation_policy": {"implicit_installation": False, "explicit_preparation_supported": False},
            "network_policy": {"operation_time_acquisition": False}, "platform_requirements": [],
        }
        self.observed = {"schema": "structuredmerge.cli-version/v1", "executable": "smorg", "package": "smorg",
            "version": "0.2.0", "kernel_version": "0.2.0", "cli_contract": "structuredmerge.cli/v1",
            "build": {"schema": "structuredmerge.cli-build/v1", "target": "test-target", "host": "test-host",
                "cargo_profile": "debug", "cargo_opt_level": "0", "cargo_debug": "false",
                "cargo_feature_flags": [], "target_features": [], "provenance_verified": False,
                "source": {"revision": None, "state": "unknown", "origin": "unspecified", "verified": False}}}
        self.declaration_file = self.root / "declarations.json"
        self.declaration_file.write_text(json.dumps(self.declarations))
        self.binary = self.root / "fixture-cli"
        self.binary.write_text("#!" + sys.executable + "\nimport json\nprint(json.dumps(" + repr(self.observed) + "))\n")
        self.binary.chmod(0o700)

    def assemble(self, development=False):
        return assembler.assemble(self.declarations, self.observed, "test-artifact", "sha256:" + "a" * 64, development)

    def test_source_policy_is_explicit_and_never_reads_checkout(self):
        with self.assertRaisesRegex(checker.Rejected, "allow-development-build"):
            self.assemble()
        candidate = self.assemble(development=True)
        self.assertEqual(candidate["build_revision"], "unknown")
        source = self.observed["build"]["source"]
        source.update(revision="a" * 40, state="dirty", origin="build-environment")
        with self.assertRaises(checker.Rejected):
            self.assemble()
        source["state"] = "clean"
        self.assertEqual(self.assemble()["build_revision"], "a" * 40)
        self.assertFalse(self.assemble()["build_provenance_verified"])

    def test_identity_overrides_and_invalid_contracts_reject(self):
        for name in ("artifact_digest", "artifact_version", "build_revision", "target", "compiled_features"):
            declarations = copy.deepcopy(self.declarations)
            declarations[name] = "override"
            with self.assertRaises(checker.Rejected):
                assembler.validate_declarations(declarations)
        self.declarations["network_policy"]["operation_time_acquisition"] = True
        with self.assertRaises(checker.Rejected):
            self.assemble(development=True)

    def test_claimed_trust_wrong_package_and_inconsistent_source_reject(self):
        for mutate in [lambda o: o.update(package="other"),
                       lambda o: o["build"].update(provenance_verified=True),
                       lambda o: o["build"]["source"].update(verified=True),
                       lambda o: o["build"]["source"].update(state="clean"),
                       lambda o: o["build"].update(target="")]:
            observed = copy.deepcopy(self.observed)
            mutate(observed)
            with self.assertRaises(checker.Rejected):
                assembler.assemble(self.declarations, observed, "test", "sha256:" + "a" * 64, True)

    @unittest.skipUnless(os.name == "posix", "POSIX observation gate")
    def test_real_process_assembly_integrity_roundtrip_and_cleanup(self):
        candidate = assembler.collect(self.binary, self.declaration_file, "test", True, self.root)
        path = self.root / "candidate.json"
        path.write_text(json.dumps(candidate))
        checked = checker.check(path, self.binary, "test-target", "standalone")
        self.assertTrue(checked["passed"])
        self.assertEqual(candidate["artifact_digest"], checker.digest_file(self.binary)[0])
        for field in ("signature_verified", "build_provenance_verified", "provider_descriptors_verified",
                      "runtime_availability_checked", "publication_authorized"):
            self.assertFalse(candidate[field])
        self.assertFalse(list(self.root.glob("cli-manifest-observation-*")))

    @unittest.skipUnless(os.name == "posix", "POSIX observation gate")
    def test_failure_output_flood_and_timeout_clean_copies(self):
        for program in ["raise SystemExit(2)", "print('not-json')", "print('x' * 200000)", "import time; time.sleep(1)"]:
            self.binary.write_text("#!" + sys.executable + "\n" + program + "\n")
            original = assembler.version
            def quick(*args, **kwargs):
                return original(*args, **kwargs, timeout=0.1)
            with patch.object(assembler, "version", side_effect=quick):
                with self.assertRaises((checker.Rejected, ValueError)):
                    assembler.collect(self.binary, self.declaration_file, "test", True, self.root)
            self.assertFalse(list(self.root.glob("cli-manifest-observation-*")))
            self.assertFalse(list(self.root.rglob("core")))

    def test_invalid_declarations_do_not_execute_artifact(self):
        self.declarations["artifact_digest"] = "override"
        self.declaration_file.write_text(json.dumps(self.declarations))
        with patch.object(assembler, "version") as version:
            with self.assertRaises(checker.Rejected):
                assembler.collect(self.binary, self.declaration_file, "test", True, self.root)
            version.assert_not_called()

    @unittest.skipUnless(os.name == "posix", "POSIX observation gate")
    def test_artifact_mutation_during_observation_rejects_and_cleans(self):
        original = assembler.version
        def mutate(*args, **kwargs):
            observed = original(*args, **kwargs)
            self.binary.write_bytes(b"changed")
            return observed
        with patch.object(assembler, "version", side_effect=mutate):
            with self.assertRaisesRegex(checker.Rejected, "original artifact changed"):
                assembler.collect(self.binary, self.declaration_file, "test", True, self.root)
        self.assertFalse(list(self.root.glob("cli-manifest-observation-*")))
