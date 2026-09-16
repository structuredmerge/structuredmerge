"""Negative archive gates; temporary data stays inside the kernel workspace."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("artifact", ROOT / "workspace-scripts/check_core_python_artifact.py")
ARTIFACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ARTIFACT)


class ArchiveValidationTest(unittest.TestCase):
    def check(self, *, license_bytes=None, package="structuredmerge-core", extra=None):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="wheel-audit-test-") as directory:
            wheel = Path(directory) / "test.whl"
            with zipfile.ZipFile(wheel, "w") as archive:
                archive.writestr("core.dist-info/METADATA", "Metadata-Version: 2.4\n"
                    f"Name: {package}\nVersion: 0.2.0\n"
                    "License-Expression: AGPL-3.0-only OR PolyForm-Small-Business-1.0.0\n")
                if license_bytes is not None:
                    archive.writestr("core.dist-info/licenses/LICENSE", license_bytes)
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


if __name__ == "__main__":
    unittest.main()
