"""Inventory coverage only: this deliberately does not certify migration parity."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class LegacyOperationMigrationTest(unittest.TestCase):
    def test_every_recorded_export_has_exactly_one_disposition(self):
        baseline = json.loads((ROOT / "contracts/ruby-api-v1.json").read_text())
        migration = json.loads((ROOT / "contracts/legacy-operation-migration.json").read_text())
        expected = set(baseline["facade_methods"]) | set(baseline["native_method_arities"])
        mapped = [method for group in migration["groups"] for method in group["methods"]]
        self.assertEqual(len(mapped), len(set(mapped)), "duplicate dispositions")
        self.assertEqual(set(mapped), expected, "unmapped or stale exports")
        ids = [group["id"] for group in migration["groups"]]
        self.assertEqual(len(ids), len(set(ids)))
        for group in migration["groups"]:
            with self.subTest(group=group["id"]):
                self.assertIn(group["disposition"], (
                    "retain_until_consumer_migration", "retain_regression_only", "local_consumer_migrated"))
                if group["disposition"] == "local_consumer_migrated":
                    self.assertTrue(group.get("consumer_revision"))
                for field in ("methods", "consumers", "replacement", "gap"):
                    self.assertTrue(group[field], field)


if __name__ == "__main__":
    unittest.main()
