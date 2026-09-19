"""Provisioning owns cold-cache scratch without relaxing Git operation limits."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "workspace-scripts"))
import prepare_cli_git_grammar as gate
import check_core_python_source as bounded
import typed_cli_git
sys.path.pop(0)


class CliGitProvisionTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "tmp")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "tmp").mkdir()
        self.binary = self.root / "smorg"
        self.binary.write_bytes(b"test binary")
        self.destination = self.root / "tmp/cache"

    def exercise(self, fail=False, wrong_output=False):
        def run(argv, cwd, env, evidence, label, work, timeout, work_budget):
            self.assertEqual(work_budget, 256 * 1024**2)
            self.assertEqual(timeout, 120)
            self.assertEqual(list((work / "cache").iterdir()), [])
            self.assertEqual(env["TREE_HAVER_LANGUAGE_PACK_CACHE_DIR"], str(work / "cache"))
            self.assertNotIn("TREE_SITTER_LANGUAGE_PACK_LIBS_DIR", env)
            self.assertIn("--strict", argv)
            (work / "cache/manifest.json").write_text("{}")
            if fail:
                raise RuntimeError("injected provisioning failure")
            (work / "ours").write_text(json.dumps({} if wrong_output else {"base": 0, "ours": 1, "theirs": 2}))
            (work / "driver.json").write_text('{"exit_code": 0}')
        with patch.object(gate, "ROOT", self.root), patch.object(gate, "run", side_effect=run), \
                patch.dict(os.environ, {"TREE_SITTER_LANGUAGE_PACK_LIBS_DIR": "/unused"}):
            if fail or wrong_output:
                with self.assertRaises((RuntimeError, ValueError)):
                    gate.prepare(self.binary, self.destination)
            else:
                self.assertTrue(gate.prepare(self.binary, self.destination)["passed"])
        self.assertFalse(list((self.root / "tmp").glob("cli-provision-work-*")))
        reports = list((self.root / "tmp").glob("cli-provision-evidence-*/report.json"))
        self.assertEqual(len(reports), 1)
        self.assertEqual(json.loads(reports[0].read_text())["passed"], not (fail or wrong_output))
        self.assertEqual(self.destination.exists(), not (fail or wrong_output))
        self.assertEqual(typed_cli_git.FILE_LIMIT, 8 * 1024**2)

    def test_success_retains_cache_but_not_work(self):
        self.exercise()

    def test_failed_provisioning_removes_partial_cache(self):
        self.exercise(fail=True)

    def test_wrong_merge_result_is_not_provisioning_success(self):
        self.exercise(wrong_output=True)

    def test_existing_or_outside_cache_is_never_overwritten(self):
        self.destination.mkdir()
        marker = self.destination / "owned-by-user"
        marker.write_bytes(b"preserve")
        with patch.object(gate, "ROOT", self.root), patch.object(gate, "run") as run:
            for destination in (self.destination, self.root / "outside"):
                with self.assertRaises(ValueError):
                    gate.prepare(self.binary, destination)
            run.assert_not_called()
        self.assertEqual(marker.read_bytes(), b"preserve")

    def test_explicit_work_budget_is_enforced_even_after_fast_exit(self):
        with patch.object(bounded, "RESERVE", 0):
            with self.assertRaisesRegex(RuntimeError, "resource budget"):
                bounded.run([sys.executable, "-c", "open('over-budget', 'wb').write(b'x' * 4096)"],
                            self.root, dict(os.environ), self.root, "budget", self.root,
                            work_budget=1024, timeout=10)
