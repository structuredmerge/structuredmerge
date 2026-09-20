from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import render_semantic_retirement_audit as audit


class SemanticRetirementAuditTest(unittest.TestCase):
    root = Path(__file__).resolve().parents[2]

    def test_render_covers_every_inventory_group_without_authorizing_deletion(self):
        import json
        inventory = json.loads((self.root / "contracts/legacy-operation-migration.json").read_text())
        rendered = audit.render(inventory)
        for group in inventory["groups"]:
            self.assertIn(f"`{group['id']}`", rendered)
        self.assertIn("authorizes no deletion", rendered)
        self.assertIn("retain legacy regression evidence", rendered)

    def test_unknown_disposition_fails_closed_in_rendered_action(self):
        rendered = audit.render({
            "schema": "test",
            "groups": [{
                "id": "unknown", "methods": ["m"], "disposition": "new",
                "replacement": "none", "gap": "review", "consumers": [],
            }],
        })
        self.assertIn("Stop and obtain an explicit disposition", rendered)


if __name__ == "__main__":
    unittest.main()
