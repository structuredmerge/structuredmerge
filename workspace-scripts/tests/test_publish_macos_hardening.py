"""publish.yml: macOS runners must drop the eMudhra roots before any download."""
import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = (ROOT / "workspace-scripts/harden_macos_ca_bundle.sh").read_text().rstrip("\n")
STEP_NAME = "Remove eMudhra roots from the macOS CA bundle"


def load_workflow():
    completed = subprocess.run(
        ["ruby", "-ryaml", "-rjson", "-e", "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))",
         str(ROOT / ".github/workflows/publish.yml")],
        capture_output=True, text=True, check=True)
    return json.loads(completed.stdout)


def macos_jobs(workflow):
    for name, job in workflow["jobs"].items():
        runners = [str(entry.get("runner", "")) for entry in job.get("strategy", {}).get("matrix", {}).get("include", [])]
        runners.append(str(job.get("runs-on", "")))
        if any(runner.startswith("macos") for runner in runners):
            yield name, job


class PublishMacosHardeningTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = load_workflow()
        cls.macos = dict(macos_jobs(cls.workflow))

    def test_the_release_still_builds_on_macos_for_wheels_and_cli_archives(self):
        self.assertEqual(sorted(self.macos), ["build-python-wheels", "release-binaries"])

    def test_every_macos_job_removes_the_roots_before_any_other_step(self):
        for name, job in self.macos.items():
            with self.subTest(job=name):
                first = job["steps"][0]
                self.assertEqual(first["name"], STEP_NAME)
                self.assertEqual(first["if"], "runner.os == 'macOS'")
                self.assertNotIn("uses", first, "must be inline: nothing may be downloaded before it runs")

    def test_the_inlined_step_is_exactly_the_tested_script(self):
        for name, job in self.macos.items():
            with self.subTest(job=name):
                self.assertEqual(job["steps"][0]["run"].rstrip("\n"), SCRIPT)

    def test_no_macos_ruby_platform_gem_verification_in_the_release(self):
        labels = [m["label"] for m in self.workflow["jobs"]["verify-ruby-platform-gems"]["strategy"]["matrix"]["include"]]
        self.assertFalse([label for label in labels if "darwin" in label], labels)

    def test_publish_python_runs_in_dry_runs_where_the_sdist_is_skipped(self):
        condition = self.workflow["jobs"]["publish-python"]["if"]
        self.assertIn("!cancelled()", condition)
        self.assertIn("needs.check-registries.result == 'success'", condition)
        self.assertIn("needs.build-python-wheels.result == 'success'", condition)
        self.assertIn("needs.prepare.outputs.dry_run == 'true' ||\n    needs.build-python-sdist.result == 'success'", condition)


if __name__ == "__main__":
    unittest.main()
