"""Post-publish PyPI verification: backoff schedule, propagation waits, integrity."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("pypi_gate", ROOT / "workspace-scripts/check_pypi_published.py")
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)

FILES = {"pkg-1.0.0.tar.gz": "a" * 64, "pkg-1.0.0-cp311-abi3-manylinux_2_28_x86_64.whl": "b" * 64}


def release(files):
    return {"urls": [{"filename": name, "digests": {"sha256": digest}} for name, digest in files.items()]}


class FakeClock:
    def __init__(self):
        self.sleeps = []

    def __call__(self, seconds):
        self.sleeps.append(seconds)


def schedule(clock, **env):
    return GATE.Schedule(env={k: str(v) for k, v in env.items()}, sleep=clock)


class ScheduleTest(unittest.TestCase):
    def test_backoff_grows_by_1_6_and_is_capped_like_kettle_dev(self):
        s = schedule(FakeClock())
        self.assertEqual([s.delay(n) for n in range(1, 9)], [10, 16, 26, 41, 66, 105, 120, 120])

    def test_defaults_and_environment_overrides(self):
        s = schedule(FakeClock())
        self.assertEqual((s.attempts, s.initial_delay, s.interval, s.max_interval), (15, 5, 10, 120))
        s = schedule(
            FakeClock(),
            STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS=3,
            STRUCTUREDMERGE_REGISTRY_PROBE_INITIAL_DELAY=1,
            STRUCTUREDMERGE_REGISTRY_PROBE_INTERVAL=2,
            STRUCTUREDMERGE_REGISTRY_PROBE_MAX_INTERVAL=3,
        )
        self.assertEqual((s.attempts, s.initial_delay, s.interval, s.max_interval), (3, 1, 2, 3))

    def test_invalid_override_falls_back_to_default(self):
        s = schedule(FakeClock(), STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS="nope")
        self.assertEqual(s.attempts, 15)

    def test_retry_sleeps_initial_delay_then_backs_off_between_attempts_only(self):
        clock = FakeClock()
        calls = []

        def step():
            calls.append(1)
            if len(calls) < 3:
                raise GATE.RetryError("lagging")
            return "ok"

        self.assertEqual(schedule(clock).retry("probe", step), "ok")
        self.assertEqual(clock.sleeps, [5, 10, 16])

    def test_retry_gives_up_after_the_configured_attempts_without_a_trailing_sleep(self):
        clock = FakeClock()

        def step():
            raise GATE.RetryError("lagging")

        with pytest.raises(GATE.FailureError, match="after 3 attempt"):
            schedule(clock, STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS=3).retry("probe", step)
        self.assertEqual(clock.sleeps, [5, 10, 16])


class ListedTest(unittest.TestCase):
    def check(self, responses, clock=None, **env):
        clock = clock or FakeClock()
        queue = list(responses)

        def fetch(*_args):
            item = queue.pop(0) if len(queue) > 1 else queue[0]
            if isinstance(item, Exception):
                raise item
            return item

        return GATE.check_listed("https://pypi.test", "pkg", "1.0.0", FILES, schedule(clock, **env), fetch=fetch), clock

    def test_waits_through_404_then_partial_listing_then_succeeds(self):
        ok, clock = self.check([None, release({"pkg-1.0.0.tar.gz": "a" * 64}), release(FILES)])
        self.assertTrue(ok)
        self.assertEqual(len(clock.sleeps), 3)

    def test_transient_registry_errors_are_retried(self):
        ok, _ = self.check([GATE.RetryError("HTTP 503"), release(FILES)])
        self.assertTrue(ok)

    def test_never_visible_says_the_publish_itself_may_have_failed(self):
        with pytest.raises(GATE.FailureError, match="publish itself may have failed"):
            self.check([None], STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS=2)

    def test_listed_but_files_missing_after_the_window_is_not_blamed_on_the_publish(self):
        with pytest.raises(GATE.FailureError) as caught:
            self.check([release({"pkg-1.0.0.tar.gz": "a" * 64})], STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS=2)
        assert "not yet listed" in str(caught.value)
        assert "publish itself" not in str(caught.value)

    def test_digest_mismatch_fails_immediately_without_retrying(self):
        bad = dict(FILES, **{"pkg-1.0.0.tar.gz": "c" * 64})
        clock = FakeClock()
        with pytest.raises(GATE.FailureError, match="sha256 differs.*tar.gz"):
            self.check([release(bad)], clock=clock)
        self.assertEqual(clock.sleeps, [5])


class ExpectedFilesTest(unittest.TestCase):
    def dist(self, names):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        for name in names:
            Path(directory.name, name).write_bytes(name.encode())
        return directory.name

    def test_requires_an_sdist_and_a_wheel(self):
        with pytest.raises(GATE.FailureError, match="no sdist"):
            GATE.expected_files(self.dist(["pkg-1.0.0-py3-none-any.whl"]))
        with pytest.raises(GATE.FailureError, match="no wheels"):
            GATE.expected_files(self.dist(["pkg-1.0.0.tar.gz"]))

    def test_hashes_artifacts_and_ignores_other_files(self):
        files = GATE.expected_files(self.dist(["pkg-1.0.0.tar.gz", "pkg-1.0.0-py3-none-any.whl", "notes.txt"]))
        self.assertEqual(sorted(files), ["pkg-1.0.0-py3-none-any.whl", "pkg-1.0.0.tar.gz"])
        self.assertEqual(len(files["pkg-1.0.0.tar.gz"]), 64)


class PipRetryTest(unittest.TestCase):
    def test_pip_failures_are_retried_until_the_cdn_serves_the_file(self):
        results = [1, 1, 0]
        calls = []

        class Completed:
            def __init__(self, code):
                self.returncode, self.stdout, self.stderr = code, "", "No matching distribution"

        def run(*_args, **_kwargs):
            calls.append(1)
            return Completed(results.pop(0))

        clock = FakeClock()
        GATE.run_pip(["pip"], schedule(clock), "install", run=run)
        self.assertEqual(len(calls), 3)
        self.assertEqual(clock.sleeps, [5, 10, 16])


if __name__ == "__main__":
    unittest.main()
