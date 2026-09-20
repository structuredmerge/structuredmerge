from pathlib import Path
import tempfile
import unittest
import zipfile

import sys
sys.path.insert(0, str(Path(__file__).parents[1]))
import package_release_archive as packager
import validate_release_archive as validator


class ReleaseArchiveValidationTest(unittest.TestCase):
    def test_tar_archive_runs_both_installed_executables(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            binaries = root / "bin"; binaries.mkdir()
            for name in packager.BINARIES:
                path = binaries / name
                path.write_text(f"#!/bin/sh\nprintf '%s\\n' '{name} 0.2.1'\n")
                path.chmod(0o755)
            archive = packager.package("0.2.1", "linux-x86_64", binaries, root / "out")
            validator.validate(archive, "0.2.1", "linux-x86_64")

    def test_zip_path_traversal_fails_closed(self):
        with tempfile.TemporaryDirectory() as root:
            archive = Path(root) / "bad.zip"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr("../smorg", b"bad")
                output.writestr("smorg-rs", b"bad")
            with self.assertRaisesRegex(ValueError, "unsafe archive member"):
                validator.validate(archive, "0.2.1", "linux-x86_64")


if __name__ == "__main__":
    unittest.main()
