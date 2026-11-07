from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import generate_release_evidence as evidence
import pytest


class ReleaseEvidenceTest(unittest.TestCase):
    def setUp(self):
        self.inventory = {
            "version": "0.2.1",
            "assets": {
                platform: {"name": f"smorg-0.2.1-{platform}.tar.gz", "sha256": "a" * 64}
                for platform in (
                    "linux-x86_64",
                    "linux-aarch64",
                    "macos-x86_64",
                    "macos-arm64",
                    "windows-x86_64",
                    "windows-arm64",
                )
            },
        }

    def test_evidence_is_closed_and_resumable(self):
        first = evidence.render("0.2.1", "v0.2.1", "a" * 40, self.inventory)
        second = evidence.render("0.2.1", "v0.2.1", "a" * 40, self.inventory)
        assert first == second
        assert not first["publication_authorized"]
        assert first["registries"]["crates-io"]["state"] == "pending"
        assert first["rollback"]["partial_release_requires_manual_review"]

    def test_version_revision_and_platform_mismatches_fail_closed(self):
        with pytest.raises(ValueError):
            evidence.render("0.2.1", "v0.2.2", "a" * 40, self.inventory)
        with pytest.raises(ValueError):
            evidence.render("0.2.1", "v0.2.1", "A" * 40, self.inventory)
        broken = dict(self.inventory, assets={"linux-x86_64": {}})
        with pytest.raises(ValueError):
            evidence.render("0.2.1", "v0.2.1", "a" * 40, broken)


if __name__ == "__main__":
    unittest.main()
