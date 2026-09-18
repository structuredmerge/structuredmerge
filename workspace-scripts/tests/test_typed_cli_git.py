"""Gate assertions and cleanup are tested independently of any installed grammar."""
import contextlib
import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "workspace-scripts"))
import typed_cli_git as gate
sys.path.pop(0)


class TypedCliGitTest(unittest.TestCase):
    def fixture(self):
        return {"schema": "structuredmerge.cli-typed-git-cases/v1", "cases": [{
            "id": "test", "base": "{}", "ours": "{\"a\":1}", "theirs": "{\"b\":2}",
            "expected": {"git_exit": 0, "driver_exit": 0, "outcome": "changed", "json": {"a": 1, "b": 2}},
        }]}

    def test_fixture_validation_rejects_empty_duplicate_and_weak_cases(self):
        gate.validate_cases(self.fixture())
        for mutate in [lambda f: f.update(cases=[]),
                       lambda f: f["cases"].append(copy.deepcopy(f["cases"][0])),
                       lambda f: f["cases"][0]["expected"].pop("json"),
                       lambda f: f["cases"][0].update(conflict_policy="silent"),
                       lambda f: f["cases"][0].update(base="x" * 16385)]:
            fixture = self.fixture()
            mutate(fixture)
            with self.assertRaises(AssertionError):
                gate.validate_cases(fixture)

    def exercise(self, fail_case=False, fail_report=False):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as temporary:
            root = Path(temporary)
            fixture, binary, grammar = [root / name for name in ("fixture.json", "smorg", "grammar.so")]
            fixture.write_text(json.dumps(self.fixture()))
            binary.write_bytes(b"binary")
            grammar.write_bytes(b"grammar")
            stage = root / "evidence"
            stage.mkdir()
            def check(binary, grammar, case, work, evidence):
                (work / "disposable").write_bytes(b"compiler/install/repo stand-in")
                if fail_case:
                    raise RuntimeError("injected case failure")
                return {"git_exit": 0, "driver_exit": 0, "outcome": "changed"}
            real_write = Path.write_text
            real_mkdtemp = tempfile.mkdtemp
            def mkdtemp(*args, **kwargs):
                if kwargs.get("prefix") == "typed-cli-git-":
                    return str(stage)
                return real_mkdtemp(*args, **kwargs)
            def write(path, *args, **kwargs):
                if fail_report and path.name == "report.json":
                    raise OSError("injected report write failure")
                return real_write(path, *args, **kwargs)
            with patch.object(gate.tempfile, "mkdtemp", side_effect=mkdtemp), \
                    patch.object(gate, "check_case", side_effect=check), \
                    patch.object(Path, "write_text", write), contextlib.redirect_stdout(io.StringIO()):
                if fail_report:
                    with self.assertRaisesRegex(OSError, "report write failure"):
                        gate.run(binary, fixture, grammar)
                else:
                    self.assertEqual(gate.run(binary, fixture, grammar), int(fail_case))
            self.assertFalse(list(stage.glob("work-*")))
            if not fail_report:
                report = json.loads((stage / "report.json").read_text())
                self.assertEqual(report["results"][0]["passed"], not fail_case)
                self.assertFalse(report["publication_gate"])

    def test_success_cleans_disposable_work(self):
        self.exercise()

    def test_case_failure_cleans_disposable_work(self):
        self.exercise(fail_case=True)

    def test_report_failure_still_cleans_disposable_work(self):
        self.exercise(fail_report=True)
