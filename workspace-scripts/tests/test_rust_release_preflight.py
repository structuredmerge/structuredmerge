"""Keep release metadata and credentials checks ahead of expensive matrices."""
import json
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]


class RustReleasePreflightTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        result = subprocess.run(
            [
                "ruby", "-ryaml", "-rjson", "-e",
                "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))",
                str(ROOT / ".github/workflows/publish.yml"),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
        cls.jobs = json.loads(result.stdout)["jobs"]

    def test_release_metadata_and_package_preflight_gate_long_builds(self):
        validate = self.jobs["validate"]
        preflight = next(
            step for step in validate["steps"]
            if step.get("name") == "Preflight every Rust release package and registry name"
        )
        self.assertIn("--preflight-only", preflight["run"])
        self.assertIn("--no-push", preflight["run"])
        for name in ("build-python-wheels", "verify-ruby-platform-gems", "release-binaries"):
            with self.subTest(job=name):
                self.assertIn("validate", self.jobs[name]["needs"])

    def test_crates_io_token_is_checked_before_long_builds(self):
        auth = self.jobs["preflight-cratesio-auth"]
        self.assertEqual(auth["environment"], "crates-io")
        command = next(
            step["run"] for step in auth["steps"]
            if step.get("name") == "Verify crates.io token can authenticate"
        )
        self.assertIn("cargo owner --list tree-haver", command)
        for name in ("build-python-wheels", "verify-ruby-platform-gems", "release-binaries"):
            with self.subTest(job=name):
                self.assertIn("preflight-cratesio-auth", self.jobs[name]["needs"])

    def test_rust_release_can_resume_from_registry_state(self):
        script = (ROOT / "workspace-scripts/release_rust_crates.rb").read_text()
        self.assertIn("registry_states[crate[:name]] = released", script)
        self.assertIn("if options[:push] && registry_states.fetch(crate[:name])", script)
        self.assertIn("return if crate_released?(crate[:name], crate[:version])", script)
        self.assertIn("DUPLICATE_PUBLISH_PATTERN", script)
        self.assertIn('"crates.io publication permission"', script)
        self.assertIn("validate_crates_io_auth! if options[:push] && !options[:preflight_only]", script)


if __name__ == "__main__":
    unittest.main()
