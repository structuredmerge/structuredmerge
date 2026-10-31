"""The post-release RubyGems gate must validate an exact isolated installation."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


class CoreRubyRegistryInstallTest(unittest.TestCase):
    root = Path(__file__).resolve().parents[2]

    @classmethod
    def setUpClass(cls):
        workflow = cls.root / ".github/workflows/publish.yml"
        result = subprocess.run(
            ["ruby", "-ryaml", "-rjson", "-e", "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))", str(workflow)],
            check=True, capture_output=True, text=True, timeout=10,
        )
        cls.jobs = json.loads(result.stdout)["jobs"]

    def test_registry_job_runs_only_after_real_rubygems_publication(self):
        job = self.jobs["verify-ruby-registry-install"]
        self.assertEqual(job["needs"], ["prepare", "publish-ruby"])
        condition = job["if"]
        self.assertIn("publish_enabled == 'true'", condition)
        self.assertIn("dry_run != 'true'", condition)
        self.assertIn("publish-ruby.result == 'success'", condition)
        run = next(step["run"] for step in job["steps"] if "check_core_ruby_registry_install.rb" in step.get("run", ""))
        self.assertIn('--version "${{ needs.prepare.outputs.version }}"', run)
        self.assertNotIn("continue-on-error", json.dumps(job))

    def test_verifier_enforces_isolation_and_propagates_subprocess_failures(self):
        script = (self.root / "workspace-scripts/check_core_ruby_registry_install.rb").read_text()
        for required in (
            '"GEM_HOME" => gem_home', '"GEM_PATH" => gem_home', '"RUBYLIB" => nil',
            '"RUBYOPT" => nil', 'unsetenv_others: true', 'expected_version',
            'native extension escaped installed core gem', 'expected one installed structuredmerge-core gem directory',
            'installed core signatures are missing', 'raise "registry install gate failed',
        ):
            with self.subTest(required=required):
                self.assertIn(required, script)

    def test_cli_rejects_missing_or_invalid_versions_without_building(self):
        script = self.root / "workspace-scripts/check_core_ruby_registry_install.rb"
        for arguments, expected in ((["--help"], 0), ([], 1), (["--version", ""], 1),
                                    (["--version", "not-a-version"], 1)):
            with self.subTest(arguments=arguments):
                result = subprocess.run(["ruby", str(script), *arguments], cwd=self.root,
                                        text=True, capture_output=True, check=False, timeout=10)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertIn("usage:", result.stdout + result.stderr)

    def test_local_archive_must_have_the_exact_requested_version(self):
        script = self.root / "workspace-scripts/check_core_ruby_registry_install.rb"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "README.md").write_text("test gem\n")
            build = subprocess.run(
                ["ruby", "-rrubygems/package", "-e", "spec = Gem::Specification.new { |s| s.name = 'structuredmerge-core'; s.version = '0.2.0'; s.summary = 'test'; s.authors = ['test']; s.files = ['README.md'] }; puts Gem::Package.build(spec)"],
                cwd=root, capture_output=True, text=True, check=True, timeout=10,
            )
            archive, = root.glob("*.gem")
            result = subprocess.run(
                ["ruby", str(script), "--version", "0.2.1", "--gem", str(archive)],
                cwd=self.root, text=True, capture_output=True, check=False, timeout=10,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("local archive version does not match", result.stderr)
