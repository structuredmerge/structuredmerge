#!/usr/bin/env python3
"""Render a review-only Phase 10 semantic-retirement audit."""

import argparse
import json
from pathlib import Path


DISPOSITION_ACTIONS = {
    "local_consumer_migrated": "Candidate for explicit authority review; retain legacy regression evidence and do not delete yet.",
    "retain_until_consumer_migration": "Retain until every listed consumer is migrated and installed gates pass.",
    "retain_regression_only": "Retain as regression evidence; no product/default promotion is authorized.",
}


def render(inventory: dict) -> str:
    groups = inventory["groups"]
    lines = [
        "# Semantic Retirement Audit",
        "",
        "This is a review-only Phase 10 work list generated from",
        "`legacy-operation-migration.json`. It authorizes no deletion, default",
        "change, publication or parser promotion.",
        "",
        f"Inventory schema: `{inventory['schema']}`; groups: **{len(groups)}**.",
        "",
        "| Group | Disposition | Consumers | Required action |",
        "| --- | --- | ---: | --- |",
    ]
    for group in groups:
        disposition = group["disposition"]
        action = DISPOSITION_ACTIONS.get(
            disposition, "Stop and obtain an explicit disposition before changing code."
        )
        lines.append(
            f"| `{group['id']}` | `{disposition}` | {len(group.get('consumers', []))} | {action} |"
        )
    lines.extend(["", "## Group review records", ""])
    for group in groups:
        lines.extend([
            f"### `{group['id']}`",
            "",
            f"- Methods: `{', '.join(group['methods'])}`",
            f"- Consumers: {', '.join(f'`{consumer}`' for consumer in group.get('consumers', [])) or 'none recorded'}",
            f"- Replacement: {group['replacement']}",
            f"- Disposition: `{group['disposition']}`",
            f"- Gap / gate: {group['gap']}",
            "",
        ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text())
    args.output.write_text(render(inventory))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
