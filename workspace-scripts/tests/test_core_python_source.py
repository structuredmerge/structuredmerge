"""Source-package closure, archive validation and bounded cleanup regressions."""
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("source_gate", ROOT / "workspace-scripts/check_core_python_source.py")
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class SourceTest(unittest.TestCase):
    def setUp(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="source-test-", dir=ROOT / "tmp")
        self.addCleanup(self.temporary.cleanup)
        self.work = Path(self.temporary.name)

    def archive(self, extra=(), omit=None):
        archive = self.work / "source.tar.gz"
        entries = [("export/" + name, b"test") for name in
                   ("pyproject.toml", "Cargo.toml", "Cargo.lock", "PKG-INFO") if name != omit]
        if omit != "LICENSE":
            entries.append(("export/LICENSE", (ROOT / "LICENSE").read_bytes()))
        with tarfile.open(archive, "w:gz") as output:
            for name, content in entries + list(extra):
                info = tarfile.TarInfo(name)
                if isinstance(content, bytes):
                    info.size = len(content)
                    output.addfile(info, io.BytesIO(content))
                else:
                    info.type = content[0]
                    info.linkname = "../outside"
                    output.addfile(info)
        return archive

    def test_extract_preserves_bytes_and_returns_one_root(self):
        root = GATE.extract_source(self.archive(), self.work / "unpacked")
        self.assertEqual(root, self.work / "unpacked/export")
        self.assertEqual((root / "LICENSE").read_bytes(), (ROOT / "LICENSE").read_bytes())

    def test_rejects_unsafe_duplicate_and_special_entries_before_extraction(self):
        for name, content in (("../outside", b"bad"), ("/absolute", b"bad"),
                              ("export/../../outside", b"bad"), ("export\\bad", b"bad"),
                              ("export/Cargo.toml", b"duplicate"),
                              ("export/link", (tarfile.SYMTYPE,)), ("export/hard", (tarfile.LNKTYPE,)),
                              ("export/device", (tarfile.CHRTYPE,))):
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, "unsafe or duplicate"):
                    GATE.extract_source(self.archive([(name, content)]), self.work / "unpacked")
                self.assertFalse((self.work / "unpacked").exists())

    def test_rejects_multiple_roots_and_missing_manifest(self):
        with self.assertRaisesRegex(ValueError, "exactly one root"):
            GATE.extract_source(self.archive([("other/file", b"test")]), self.work / "many")
        with self.assertRaisesRegex(ValueError, "missing Cargo.lock"):
            GATE.extract_source(self.archive(omit="Cargo.lock"), self.work / "missing")

    def test_rejects_expanded_budget(self):
        archive = self.archive([("export/padding", b"x" * 100000)])
        with patch.object(GATE, "ARCHIVE_BUDGET", 50000):
            with self.assertRaisesRegex(ValueError, "expanded"):
                GATE.extract_source(archive, self.work / "unpacked")

    def metadata(self):
        return {"packages": [{"name": name, "source": None,
            "manifest_path": str(self.work / name / "Cargo.toml"), "dependencies": []}
            for name in ("structuredmerge-core", "structuredmerge-core-py")]}

    def test_cargo_metadata_must_include_both_local_roots(self):
        metadata = self.metadata()
        self.assertEqual(GATE.check_metadata(metadata, self.work),
                         ["structuredmerge-core", "structuredmerge-core-py"])
        metadata["packages"].pop()
        with self.assertRaisesRegex(ValueError, "facade and Python binding"):
            GATE.check_metadata(metadata, self.work)

    def test_rejects_external_local_manifests_dependencies_and_prototypes(self):
        metadata = self.metadata()
        metadata["packages"][0]["manifest_path"] = str(self.work.parent / "outside/Cargo.toml")
        with self.assertRaisesRegex(ValueError, "package escapes"):
            GATE.check_metadata(metadata, self.work)
        metadata = self.metadata()
        metadata["packages"][0]["dependencies"] = [{"path": str(self.work.parent / "outside")}]
        with self.assertRaisesRegex(ValueError, "dependency escapes"):
            GATE.check_metadata(metadata, self.work)
        metadata = self.metadata()
        metadata["packages"][0]["name"] = "structuredmerge-host-prototype-core"
        with self.assertRaisesRegex(ValueError, "prototype"):
            GATE.check_metadata(metadata, self.work)

    def test_low_disk_rejects_before_process_launch(self):
        with patch.object(GATE.shutil, "disk_usage") as usage, patch.object(GATE.subprocess, "Popen") as launch:
            usage.return_value.free = 0
            with self.assertRaisesRegex(RuntimeError, "30 GiB"):
                GATE.run([], self.work, {}, self.work, "test", self.work)
            launch.assert_not_called()

    def test_lock_pruning_accepts_only_existing_pins_and_edges(self):
        before = '''version = 4
[[package]]
name = "root"
version = "1.0.0"
dependencies = ["leaf 1.0.0"]
[[package]]
name = "leaf"
version = "1.0.0"
checksum = "original"
[[package]]
name = "leaf"
version = "2.0.0"
checksum = "other"
'''
        after = '''version = 4
[[package]]
name = "root"
version = "1.0.0"
dependencies = ["leaf"]
[[package]]
name = "leaf"
version = "1.0.0"
checksum = "original"
'''
        self.assertEqual(GATE.verify_lock_pruning(before, after),
            {"removed_packages": 1, "retained_packages": 2, "new_or_changed_pins": 0})
        for changed in (after.replace('checksum = "original"', 'checksum = "changed"'),
                        after.replace('name = "leaf"', 'name = "new"')):
            with self.assertRaises(ValueError):
                GATE.verify_lock_pruning(before, changed)
        with self.assertRaisesRegex(ValueError, "introduced a dependency edge"):
            GATE.verify_lock_pruning(before, after.replace('name = "leaf"\nversion = "1.0.0"\nchecksum = "original"',
                'name = "leaf"\nversion = "2.0.0"\nchecksum = "other"'))

    @unittest.skipUnless(os.name == "posix", "POSIX resource gate")
    def test_deadline_kills_process_and_waits(self):
        processes = []
        factory = subprocess.Popen
        def launch(*args, **kwargs):
            process = factory(*args, **kwargs)
            processes.append(process)
            return process
        with patch.object(GATE, "RESERVE", 0), patch.object(GATE.subprocess, "Popen", side_effect=launch):
            with self.assertRaisesRegex(RuntimeError, "resource budget"):
                GATE.run([sys.executable, "-c", "import time; time.sleep(60)"],
                         self.work, os.environ.copy(), self.work, "timeout", self.work, timeout=0)
            self.assertEqual(len(processes), 1)
            self.assertIsNotNone(processes[0].returncode)
        # The observed child is reaped; the command's capture is retained, not a
        # live process or unbounded pipe. Main owns the disposable tree lifecycle.
        self.assertTrue((self.work / "timeout.stderr").is_file())

    def test_failure_keeps_report_and_removes_source_target(self):
        root = self.work / "fake-root"
        root.mkdir()
        def fail(archive, destination):
            (destination / "target").mkdir()
            (destination / "target/partial").write_bytes(b"partial")
            raise RuntimeError("injected source failure")
        with patch.object(GATE, "ROOT", root), patch.object(GATE, "RESERVE", 0), \
                patch.object(GATE, "BUILD_BUDGET", 0), \
                patch.object(GATE, "extract_source", side_effect=fail), \
                patch.object(sys, "argv", ["source-gate", str(self.work / "archive")]):
            with self.assertRaisesRegex(RuntimeError, "injected"):
                GATE.main()
        stages = list((root / "tmp").iterdir())
        self.assertEqual(len(stages), 1)
        self.assertEqual([p.name for p in stages[0].iterdir()], ["report.json"])
        report = json.loads((stages[0] / "report.json").read_text())
        self.assertEqual(report["status"], "failed")
        self.assertFalse(report["publication_gate"])


if __name__ == "__main__":
    unittest.main()
