import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import sys

sys.path.insert(0, str(Path(__file__).parents[1]))
import generate_release_manifests as manifests


class ReleaseManifestTest(unittest.TestCase):
    def make_assets(self, directory: Path, version: str = "0.2.1") -> None:
        for platform in manifests.PLATFORMS:
            suffix = ".zip" if platform.startswith("windows") else ".tar.gz"
            (directory / f"smorg-{version}-{platform}{suffix}").write_bytes(
                f"{platform}\n".encode()
            )

    def test_generates_hash_pinned_homebrew_and_scoop_manifests(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            assets = root / "assets"
            output = root / "output"
            assets.mkdir()
            self.make_assets(assets)
            manifests.generate_manifests("0.2.1", assets, output)
            formula = (output / "smorg.rb").read_text()
            scoop = json.loads((output / "smorg.json").read_text())
            expected = hashlib.sha256((assets / "smorg-0.2.1-windows-x86_64.zip").read_bytes()).hexdigest()
            self.assertIn(expected, scoop["architecture"]["64bit"]["hash"])
            self.assertIn("bin \"smorg\"", formula)
            self.assertIn("smorg-0.2.1-macos-arm64.tar.gz", formula)

    def test_missing_platform_asset_fails_closed(self):
        with tempfile.TemporaryDirectory() as root:
            assets = Path(root)
            self.make_assets(assets)
            (assets / "smorg-0.2.1-windows-arm64.zip").unlink()
            with self.assertRaises(ValueError):
                manifests.collect_assets(assets, "0.2.1")
if __name__ == "__main__":
    unittest.main()
