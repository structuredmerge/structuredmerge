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

    def test_prepared_gem_owns_a_cargo_workspace_root(self):
        script = (self.root / "workspace-scripts/prepare_core_ruby_source.rb").read_text()
        self.assertIn("if 'workspace' not in manifest", script)
        self.assertIn("path.write_text(source)", script)
        self.assertIn("prepared native extension must own an empty workspace root", script)

    def test_verifier_enforces_isolation_and_propagates_subprocess_failures(self):
        script = (self.root / "workspace-scripts/check_core_ruby_registry_install.rb").read_text()
        for required in (
            '"GEM_HOME" => gem_home', '"GEM_PATH" => gem_home', '"RUBYLIB" => nil',
            '"RUBYOPT" => nil', 'unsetenv_others: true', 'expected_version',
            'CoreRubyRegistryInstallContract.verify!', 'expected one installed structuredmerge-core gem directory',
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

    def test_install_contract_covers_missing_wrong_version_and_path_leaks(self):
        contract = self.root / "workspace-scripts/core_ruby_registry_install_contract.rb"
        ruby = r"""
          require "json"
          require "core_ruby_registry_install_contract"
          input = JSON.parse(ARGV.fetch(0))
          spec = if input.fetch("installed")
                   Struct.new(:name, :version, :full_gem_path).new(
                     "structuredmerge-core", input.fetch("installed_version"), input.fetch("gem_path")
                   )
                 end
          begin
            CoreRubyRegistryInstallContract.verify!(
              spec: spec, expected_version: input.fetch("expected_version"),
              gem_home: input.fetch("gem_home"), native_paths: input.fetch("native_paths")
            )
          rescue CoreRubyRegistryInstallContract::VerificationError => error
            warn error.message
            exit 1
          end
        """
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "gems"
            gem_path = home / "gems/structuredmerge-core-0.2.1"
            native = gem_path / "lib/structuredmerge_core_rb/extension.so"
            outside = root / "outside/extension.so"
            native.parent.mkdir(parents=True)
            outside.parent.mkdir(parents=True)
            native.touch()
            outside.touch()

            cases = (
                ("exact version and paths", {"installed": True, "installed_version": "0.2.1", "expected_version": "0.2.1", "gem_path": str(gem_path), "gem_home": str(home), "native_paths": [str(native)]}, 0, ""),
                ("missing package", {"installed": False, "expected_version": "0.2.1", "gem_home": str(home), "native_paths": []}, 1, "is not installed"),
                ("wrong version", {"installed": True, "installed_version": "0.2.0", "expected_version": "0.2.1", "gem_path": str(gem_path), "gem_home": str(home), "native_paths": [str(native)]}, 1, "wrong installed version"),
                ("wrong gem path", {"installed": True, "installed_version": "0.2.1", "expected_version": "0.2.1", "gem_path": str(root / "outside"), "gem_home": str(home), "native_paths": [str(outside)]}, 1, "escaped isolated GEM_HOME"),
                ("wrong native path", {"installed": True, "installed_version": "0.2.1", "expected_version": "0.2.1", "gem_path": str(gem_path), "gem_home": str(home), "native_paths": [str(outside)]}, 1, "native extension escaped"),
            )
            for label, payload, expected_code, expected_message in cases:
                with self.subTest(case=label):
                    result = subprocess.run(
                        ["ruby", "-I", str(contract.parent), "-e", ruby, json.dumps(payload)],
                        cwd=self.root, text=True, capture_output=True, check=False, timeout=10,
                    )
                    self.assertEqual(result.returncode, expected_code, result.stderr)
                    self.assertIn(expected_message, result.stderr + result.stdout)
