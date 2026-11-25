"""The scheduled pin-bump workflow must stay runnable and policy-compliant."""
import pathlib
import re

import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/pin-bump.yml"


def load():
    return yaml.safe_load(WORKFLOW.read_text())


def test_runs_on_a_schedule_and_on_demand():
    triggers = load()[True]  # PyYAML parses the bare key `on` as boolean True
    assert triggers["schedule"][0]["cron"]
    assert "workflow_dispatch" in triggers


def test_every_action_is_pinned_to_a_commit_sha():
    for match in re.finditer(r"uses:\s*(\S+)", WORKFLOW.read_text()):
        assert re.search(r"@[0-9a-f]{40}$", match.group(1)), match.group(1)


def test_script_it_runs_exists_and_is_executable():
    script = ROOT / "workspace-scripts/bump_pins.sh"
    assert script.exists() and script.stat().st_mode & 0o111
    assert "bump_pins.sh" in WORKFLOW.read_text()
