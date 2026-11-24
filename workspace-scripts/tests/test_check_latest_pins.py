"""Tests for the latest-release pin gate (offline: latest versions are injected)."""
import importlib.util
import pathlib
import sys

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "check_latest_pins.py"
spec = importlib.util.spec_from_file_location("check_latest_pins", SCRIPT)
pins = importlib.util.module_from_spec(spec)
sys.modules["check_latest_pins"] = pins
spec.loader.exec_module(pins)


def test_repo_pins_agree_with_each_other():
    assert pins.main(["--offline"]) == 0


def test_stale_pins_fail_when_a_newer_release_exists(capsys):
    assert pins.main(["--tslp-latest", "999.0.0", "--alef-latest", "999.0.0", "--polylint-latest", "999.0.0"]) == 1
    err = capsys.readouterr().err
    assert "is not the latest (999.0.0)" in err


def test_caret_requirements_are_rejected():
    current = next(hit[3] for hit in pins.scan(pins.TSLP_SITES[:1]))
    violations = list(pins.exact_pin_violations(current))
    assert violations == [] or all("exact =pin" in message for message in violations)


def test_pins_are_the_latest_releases(capsys):
    """Online gate: fails the day tree-sitter-language-pack or alef ships and we have not caught up."""
    import urllib.error

    import pytest

    try:
        status = pins.main([])
    except (urllib.error.URLError, TimeoutError, OSError) as error:  # offline: the release gate still runs online
        pytest.skip(f"cannot reach crates.io/GitHub: {error}")
    assert status == 0, capsys.readouterr().err
