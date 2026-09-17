"""Negative archive gates; temporary data stays inside the kernel workspace."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("artifact", ROOT / "workspace-scripts/check_core_python_artifact.py")
ARTIFACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ARTIFACT)


class WorkspaceLifecycleTest(unittest.TestCase):
    def test_main_failure_records_error_and_cleans_partial_install(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="artifact-main-test-") as directory:
            root = Path(directory)
            script = root / "workspace-scripts/check_core_python_artifact.py"
            script.parent.mkdir()
            wheel = root / "test.whl"
            wheel.touch()

            def fail_install(root, wheel, metadata, licenses, stage):
                (stage / "venv").mkdir()
                (stage / "venv/partial").write_text("incomplete installation")
                raise RuntimeError("injected install failure")

            with patch.object(ARTIFACT, "__file__", str(script)), \
                    patch.object(ARTIFACT.sys, "argv", [str(script), str(wheel)]), \
                    patch.object(ARTIFACT, "inspect_wheel", return_value=({}, [])), \
                    patch.object(ARTIFACT, "run_checks", side_effect=fail_install), \
                    patch.object(ARTIFACT.shutil, "disk_usage") as usage:
                usage.return_value.free = 22 * 1024**3
                with self.assertRaisesRegex(RuntimeError, "injected install failure"):
                    ARTIFACT.main()
            stages = list((root / "tmp").iterdir())
            self.assertEqual(len(stages), 1)
            self.assertEqual(list(stages[0].iterdir()), [stages[0] / "report.json"])
            report = json.loads((stages[0] / "report.json").read_text())
            self.assertEqual(report["status"], "failed")
            self.assertEqual(report["error"], "RuntimeError")
            self.assertFalse(report["publication_gate"])

    def test_cleanup_on_success_failure_and_interruption_retains_report(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        for error in (None, RuntimeError("install failed"), KeyboardInterrupt()):
            with self.subTest(error=error), tempfile.TemporaryDirectory(
                dir=ROOT / "tmp", prefix="artifact-lifecycle-test-"
            ) as directory:
                root = Path(directory)
                with patch.object(ARTIFACT.shutil, "disk_usage") as usage:
                    usage.return_value.free = 22 * 1024**3
                    try:
                        with ARTIFACT.artifact_workspace(root) as stage:
                            for name in ("venv", "consumer"):
                                (stage / name).mkdir()
                                (stage / name / "partial").write_text("disposable")
                            (stage / "report.json").write_text("evidence")
                            if error is not None:
                                raise error
                    except (RuntimeError, KeyboardInterrupt) as caught:
                        self.assertIs(caught, error)
                self.assertEqual(list(stage.iterdir()), [stage / "report.json"])
                self.assertEqual((stage / "report.json").read_text(), "evidence")

    def test_low_disk_refuses_before_creating_environment(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="artifact-budget-test-") as directory:
            root = Path(directory)
            with patch.object(ARTIFACT.shutil, "disk_usage") as usage:
                usage.return_value.free = 22 * 1024**3 - 1
                with self.assertRaisesRegex(RuntimeError, "22 GiB free"):
                    with ARTIFACT.artifact_workspace(root):
                        self.fail("low-disk runner must not start")
            self.assertEqual(list((root / "tmp").iterdir()), [])


class ArchiveValidationTest(unittest.TestCase):
    def test_resolves_one_wheel_without_shell_expansion(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="wheel-path-test-") as directory:
            path = Path(directory)
            with self.assertRaisesRegex(ValueError, "exactly one wheel"):
                ARTIFACT.resolve_wheel(path)
            wheel = path / "core.whl"
            wheel.touch()
            self.assertEqual(ARTIFACT.resolve_wheel(path), wheel.resolve())
            self.assertEqual(ARTIFACT.resolve_wheel(wheel), wheel.resolve())
            with self.assertRaisesRegex(ValueError, "expected a wheel file"):
                ARTIFACT.resolve_wheel(path / "*.whl")
            (path / "other.whl").touch()
            with self.assertRaisesRegex(ValueError, "exactly one wheel"):
                ARTIFACT.resolve_wheel(path)

    def check(self, *, license_bytes=None, package="structuredmerge-core", extra=None, typing_files=True):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="wheel-audit-test-") as directory:
            wheel = Path(directory) / "test.whl"
            with zipfile.ZipFile(wheel, "w") as archive:
                archive.writestr("core.dist-info/METADATA", "Metadata-Version: 2.4\n"
                    f"Name: {package}\nVersion: 0.2.0\n"
                    "License-Expression: AGPL-3.0-only OR PolyForm-Small-Business-1.0.0\n")
                if license_bytes is not None:
                    archive.writestr("core.dist-info/licenses/LICENSE", license_bytes)
                baseline = json.loads((ROOT / "contracts/typed-api/python/manifest.json").read_text())
                for name in baseline["files"]:
                    if name in (extra or {}) or not typing_files and name.endswith((".pyi", "py.typed")):
                        continue
                    archive.writestr(name, (ROOT / "packages/python" / name).read_bytes())
                for name, data in (extra or {}).items():
                    archive.writestr(name, data)
            return ARTIFACT.inspect_wheel(ROOT, wheel)

    def test_rejects_missing_and_truncated_license(self):
        for data in (None, b"license omitted"):
            with self.subTest(data=data), self.assertRaisesRegex(ValueError, "complete authoritative"):
                self.check(license_bytes=data)

    def test_rejects_wrong_package(self):
        with self.assertRaisesRegex(ValueError, "unexpected wheel package"):
            self.check(package="wrong-package")

    def test_rejects_prototype_and_executable_payloads(self):
        for extra in ({"prototype.py": ""}, {"core.data/scripts/smorg": ""},
                {"core.dist-info/entry_points.txt": "[console_scripts]\nsmorg=core:main\n"}):
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.check(license_bytes=(ROOT / "LICENSE").read_bytes(), extra=extra)

    def test_accepts_exact_license_and_package_identity(self):
        metadata, licenses = self.check(license_bytes=(ROOT / "LICENSE").read_bytes())
        self.assertEqual(metadata["Name"], "structuredmerge-core")
        self.assertEqual(licenses, ["core.dist-info/licenses/LICENSE"])

    def test_rejects_missing_type_declarations(self):
        with self.assertRaisesRegex(ValueError, "native type declarations"):
            self.check(license_bytes=(ROOT / "LICENSE").read_bytes(), typing_files=False)

    def test_rejects_valid_but_changed_api_source(self):
        for name, data in (("structuredmerge_core/__init__.py", "__all__ = []\n"),
                           ("structuredmerge_core/_native.pyi", "def wrong() -> None: ...\n")):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "API differs from reviewed baseline"):
                self.check(license_bytes=(ROOT / "LICENSE").read_bytes(), extra={name: data})

    def test_rejects_unreviewed_api_module(self):
        with self.assertRaisesRegex(ValueError, "API file set differs"):
            self.check(license_bytes=(ROOT / "LICENSE").read_bytes(), extra={"structuredmerge_core/extra.py": ""})


if __name__ == "__main__":
    unittest.main()
