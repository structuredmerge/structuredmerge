"""The scheduled pin-bump workflow must stay runnable and policy-compliant.

Plain text assertions: the CI jobs that run these tests do not ship PyYAML.
"""

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/pin-bump.yml"


def test_runs_on_a_schedule_and_on_demand():
    text = WORKFLOW.read_text()
    assert re.search(
        r"^on:\n  schedule:\n    - cron: \"[^\"]+\"\n  workflow_dispatch:", text, re.M
    )


def test_every_action_is_pinned_to_a_commit_sha():
    for match in re.finditer(r"uses:\s*(\S+)", WORKFLOW.read_text()):
        assert re.search(r"@[0-9a-f]{40}$", match.group(1)), match.group(1)


def test_script_it_runs_exists_and_is_executable():
    script = ROOT / "workspace-scripts/bump_pins.sh"
    assert script.exists() and script.stat().st_mode & 0o111
    assert "bump_pins.sh" in WORKFLOW.read_text()


def test_every_workflow_action_is_pinned_to_a_commit_sha():
    """The repository rejects any action not pinned to a full-length commit SHA."""
    for path in sorted((ROOT / ".github").rglob("*.yml")):
        for match in re.finditer(r"^\s*(?:-\s*)?uses:\s*(\S+)", path.read_text(), re.M):
            reference = match.group(1)
            if reference.startswith("./"):
                continue
            assert re.search(r"@[0-9a-f]{40}$", reference), (
                f"{path.relative_to(ROOT)}: {reference}"
            )
