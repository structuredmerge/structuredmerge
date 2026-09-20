import hashlib
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

import sys
sys.path.insert(0, str(Path(__file__).parents[1]))
import package_release_archive as archives


class ReleaseArchiveTest(unittest.TestCase):
    def binaries(self, directory: Path) -> None:
        for name in archives.BINARIES:
            path = directory / name
            path.write_bytes(name.encode())
            path.chmod(0o755)

    def windows_binaries(self, directory: Path) -> None:
        for name in archives.BINARIES:
            (directory / f"{name}.exe").write_bytes(name.encode())

    def test_tar_archive_is_normalized_and_contains_both_executables(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            binary_dir = root / "bin"
            binary_dir.mkdir()
            self.binaries(binary_dir)
            old = os.environ.get("SOURCE_DATE_EPOCH")
            os.environ["SOURCE_DATE_EPOCH"] = "123"
            try:
                archive = archives.package("0.2.1", "linux-x86_64", binary_dir, root / "out")
                repeat = archives.package("0.2.1", "linux-x86_64", binary_dir, root / "repeat")
            finally:
                if old is None:
                    os.environ.pop("SOURCE_DATE_EPOCH", None)
                else:
                    os.environ["SOURCE_DATE_EPOCH"] = old
            with tarfile.open(archive, "r:gz") as stream:
                members = stream.getmembers()
                self.assertEqual([member.name for member in members], ["smorg", "smorg-rs"])
                self.assertTrue(all(member.uid == member.gid == 0 for member in members))
                self.assertTrue(all(member.mtime == 123 for member in members))
            self.assertEqual(hashlib.sha256(archive.read_bytes()).digest(), hashlib.sha256(repeat.read_bytes()).digest())

    def test_windows_archive_is_repeatable(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            binary_dir = root / "bin"
            binary_dir.mkdir()
            self.windows_binaries(binary_dir)
            first = archives.package("0.2.1", "windows-x86_64", binary_dir, root / "one")
            second = archives.package("0.2.1", "windows-x86_64", binary_dir, root / "two")
            self.assertEqual(hashlib.sha256(first.read_bytes()).digest(), hashlib.sha256(second.read_bytes()).digest())
            with zipfile.ZipFile(first) as stream:
                self.assertEqual(stream.namelist(), ["smorg.exe", "smorg-rs.exe"])

    def test_missing_binary_fails_closed(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaisesRegex(ValueError, "missing release executable"):
                archives.package("0.2.1", "linux-x86_64", Path(root), Path(root) / "out")


if __name__ == "__main__":
    unittest.main()
