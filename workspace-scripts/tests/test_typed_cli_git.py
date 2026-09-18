"""Gate assertions and cleanup are tested independently of any installed grammar."""
import contextlib
import copy
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "workspace-scripts"))
import typed_cli_git as gate
import check_installed_cli_git as legacy
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

    def test_legacy_case_and_report_failures_clean_repositories(self):
        for fail_report in (False, True):
            with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as temporary:
                root = Path(temporary)
                binary, fixture, stage = root / "smorg", root / "fixtures.json", root / "evidence"
                binary.write_bytes(b"fixture binary")
                fixture.write_text(json.dumps({"cases": [{"case_id": "injected"}]}))
                stage.mkdir()
                original = Path.write_text
                def write(path, *args, **kwargs):
                    if fail_report and path.name == "report.json":
                        raise OSError("injected report failure")
                    return original(path, *args, **kwargs)
                with patch.object(sys, "argv", ["gate", str(binary), "--fixtures", str(fixture)]), \
                        patch.object(gate, "command", side_effect=RuntimeError("injected command failure")), \
                        patch.object(legacy.tempfile, "mkdtemp", return_value=str(stage)), \
                        patch.object(Path, "write_text", write), contextlib.redirect_stdout(io.StringIO()):
                    if fail_report:
                        with self.assertRaisesRegex(OSError, "report failure"):
                            legacy.main()
                    else:
                        self.assertEqual(legacy.main(), 1)
                self.assertFalse(list(stage.glob("work-*")))


@unittest.skipUnless(os.name == "posix", "POSIX process resource limits")
class CommandBudgetTest(unittest.TestCase):
    def setUp(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "tmp")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def run_program(self, program, **kwargs):
        return gate.command([sys.executable, "-c", program], self.root, dict(os.environ), **kwargs)

    def assert_clean(self):
        self.assertEqual(list(self.root.iterdir()), [])
        self.assertFalse(list(self.root.parent.glob("command-*")))

    def test_capture_stdin_exit_and_cleanup(self):
        result = self.run_program("import sys; print(sys.stdin.read()); print('error', file=sys.stderr); sys.exit(7)", data=b"input")
        self.assertEqual((result.returncode, result.stdout, result.stderr), (7, b"input\n", b"error\n"))
        self.assert_clean()

    def test_output_floods_reject_and_cleanup_without_core(self):
        for channel in ("stdout", "stderr"):
            with self.assertRaisesRegex(AssertionError, "capture budget"):
                self.run_program(f"import sys; sys.{channel}.write('x' * {gate.FILE_LIMIT * 2})")
            self.assert_clean()

    def test_timeout_and_launch_failure_cleanup(self):
        with self.assertRaisesRegex(AssertionError, "deadline"):
            self.run_program("import time; time.sleep(30)", timeout=0.1)
        self.assert_clean()
        with self.assertRaises(OSError):
            gate.command([str(self.root / "missing")], self.root, dict(os.environ))
        self.assert_clean()

    def test_live_disk_reserve_cancels_command(self):
        with patch.object(gate.shutil, "disk_usage", side_effect=[
                SimpleNamespace(free=gate.RESERVE + 1), SimpleNamespace(free=gate.RESERVE - 1)]):
            with self.assertRaisesRegex(AssertionError, "disk reserve"):
                self.run_program("import time; time.sleep(30)")
        self.assert_clean()

    def test_low_space_and_oversized_input_do_not_launch(self):
        with patch.object(gate.subprocess, "Popen") as launch:
            with self.assertRaisesRegex(AssertionError, "command input"):
                self.run_program("", data=b"x" * (gate.CAPTURE_LIMIT + 1))
            with patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=0)):
                with self.assertRaisesRegex(AssertionError, "disk reserve"):
                    self.run_program("")
            launch.assert_not_called()
        self.assert_clean()

    def test_child_core_and_file_limits_are_enforced(self):
        result = self.run_program("import resource; print(resource.getrlimit(resource.RLIMIT_CORE)); print(resource.getrlimit(resource.RLIMIT_FSIZE))")
        self.assertEqual(result.stdout.decode().splitlines(), ["(0, 0)", f"({gate.FILE_LIMIT}, {gate.FILE_LIMIT})"])
        self.assert_clean()

    def test_descendant_is_retired_after_leader_exits(self):
        # A surviving descendant would leave its delayed marker after command()
        # returns. It inherits the leader's process group, as Git drivers do.
        child = "import time; time.sleep(0.2); open('survived', 'w').write('leaked')"
        result = self.run_program(f"import subprocess, sys; subprocess.Popen([sys.executable, '-c', {child!r}])")
        self.assertEqual(result.returncode, 0)
        time.sleep(0.35)
        self.assert_clean()

    def test_capture_files_are_not_staged_by_git_add(self):
        for args in (["init", "--template=", "-b", "test"], ["add", "."]):
            result = gate.command(["git", *args], self.root, dict(os.environ))
            self.assertEqual(result.returncode, 0, result.stderr)
        result = gate.command(["git", "status", "--porcelain"], self.root, dict(os.environ))
        self.assertEqual(result.stdout, b"")
        self.assertFalse(list(self.root.parent.glob("command-*")))
