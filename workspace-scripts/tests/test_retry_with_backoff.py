"""retry_with_backoff: retry only while the registry lags; fail fast otherwise."""
import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("retry_gate", ROOT / "workspace-scripts/retry_with_backoff.py")
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)
PYPI = GATE.sys.modules["check_pypi_published"]


class Done:
    def __init__(self, code, out="", err=""):
        self.returncode, self.stdout, self.stderr = code, out, err


def schedule(sleeps, **env):
    return PYPI.Schedule(env={k: str(v) for k, v in env.items()}, sleep=sleeps.append)


class RetryTest(unittest.TestCase):
    def run_cmd(self, results, sleeps, **env):
        queue = list(results)
        calls = []

        def run(command, **_kwargs):
            calls.append(command)
            return queue.pop(0)

        code = GATE.run_with_backoff(["cargo", "install", "x"], r"could not find|no matching package",
                                     schedule(sleeps, **env), run=run)
        return code, calls

    def test_retries_while_the_registry_lags_then_succeeds(self):
        sleeps = []
        code, calls = self.run_cmd([Done(101, err="error: could not find `x` in registry"),
                                    Done(101, err="no matching package named `x`"), Done(0, out="Installed")], sleeps)
        self.assertEqual((code, len(calls)), (0, 3))
        self.assertEqual(sleeps, [5, 10, 16])

    def test_other_failures_are_not_retried_and_keep_their_exit_code(self):
        sleeps = []
        code, calls = self.run_cmd([Done(101, err="error[E0432]: unresolved import")], sleeps)
        self.assertEqual((code, len(calls)), (101, 1))
        self.assertEqual(sleeps, [5])

    def test_gives_up_after_the_configured_attempts_with_failure(self):
        sleeps = []
        code, calls = self.run_cmd([Done(101, err="could not find `x`")] * 3, sleeps,
                                   STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS=3)
        self.assertEqual((code, len(calls)), (1, 3))


if __name__ == "__main__":
    unittest.main()
