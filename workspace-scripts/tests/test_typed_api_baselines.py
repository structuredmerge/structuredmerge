import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("baseline", ROOT / "workspace-scripts/check_typed_api_baselines.py")
BASELINE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASELINE)


class TypedApiBaselineTest(unittest.TestCase):
    def setUp(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        self.stage = tempfile.TemporaryDirectory(prefix="api-baseline-test-", dir=ROOT / "tmp")
        self.addCleanup(self.stage.cleanup)
        self.root = Path(self.stage.name)
        self.files = BASELINE.surface_files(ROOT, "python")
        for name, data in self.files.items():
            path = self.root / "packages/python" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        BASELINE.record(self.root, "python", "test initial baseline")

    def test_exact_baseline_passes_without_writes(self):
        path = self.root / "contracts/typed-api/python/manifest.json"
        before = path.stat().st_mtime_ns
        self.assertEqual(BASELINE.check(self.root, "python"), len(self.files))
        self.assertEqual(path.stat().st_mtime_ns, before)

    def test_declaration_and_facade_drift_require_review(self):
        for name in ("structuredmerge_core/_native.pyi", "structuredmerge_core/__init__.py"):
            path = self.root / "packages/python" / name
            path.write_bytes(self.files[name] + b"\nchanged_api = True\n")
            with self.assertRaisesRegex(ValueError, "API review required"):
                BASELINE.check(self.root, "python")
            path.write_bytes(self.files[name])

    def test_added_module_and_missing_required_surface_fail(self):
        path = self.root / "packages/python/structuredmerge_core/extra.py"
        path.write_text("new_api = True\n")
        with self.assertRaisesRegex(ValueError, "API review required"):
            BASELINE.check(self.root, "python")
        path.unlink()
        (path.parent / "_native.pyi").unlink()
        with self.assertRaisesRegex(ValueError, "required API surface"):
            BASELINE.check(self.root, "python")

    def test_snapshot_tampering_and_unreviewed_record_fail(self):
        with self.assertRaisesRegex(ValueError, "nonempty"):
            BASELINE.record(self.root, "python", " ")
        path = self.root / "contracts/typed-api/python/snapshot/structuredmerge_core/_native.pyi"
        path.write_text("tampered\n")
        with self.assertRaisesRegex(ValueError, "content/hash mismatch"):
            BASELINE.check(self.root, "python")

    def test_removed_optional_file_keeps_review_evidence(self):
        path = self.root / "packages/python/structuredmerge_core/extra.py"
        path.write_text("new_api = True\n")
        BASELINE.record(self.root, "python", "test added module")
        path.unlink()
        BASELINE.record(self.root, "python", "test removed module")
        with self.assertRaisesRegex(ValueError, "file set differs"):
            BASELINE.check(self.root, "python")

    def test_ruby_surface_includes_loader_version_and_rbs(self):
        files = BASELINE.surface_files(ROOT, "ruby")
        for name, content in files.items():
            path = self.root / "packages/ruby" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        BASELINE.record(self.root, "ruby", "test initial Ruby baseline")
        self.assertEqual(BASELINE.check(self.root, "ruby"), len(files))
        for name in ("sig/types.rbs", "lib/structuredmerge_core/native.rb", "lib/structuredmerge_core/version.rb"):
            path = self.root / "packages/ruby" / name
            path.write_bytes(files[name] + b"\n# changed\n")
            with self.assertRaisesRegex(ValueError, "API review required"):
                BASELINE.check(self.root, "ruby")
            path.write_bytes(files[name])
        self.assertFalse(any("prototype" in name for name in files))


if __name__ == "__main__":
    unittest.main()
