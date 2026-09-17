"""Packaging must produce the typed core, never release the retired prototype."""
import json
from pathlib import Path
import subprocess
import unittest


class CorePackagingWorkflowTest(unittest.TestCase):
    root = Path(__file__).resolve().parents[2]

    @classmethod
    def setUpClass(cls):
        result = subprocess.run([
            "ruby", "-ryaml", "-rjson", "-e",
            "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))",
            str(cls.root / ".github/workflows/current.yml"),
        ], check=True, capture_output=True, text=True, timeout=10)
        cls.jobs = json.loads(result.stdout)["jobs"]

    def test_export_compiles_then_packages_verifies_and_uploads_only_core(self):
        steps = self.jobs["ruby-package"]["steps"]
        commands = [step.get("run", "") for step in steps]
        compile_index = commands.index("bundle exec rake compile")
        export_index = next(i for i, command in enumerate(commands) if "--package-only" in command)
        verify_index = next(i for i, command in enumerate(commands) if "verify_core_ruby_export.rb" in command)
        upload_index = next(i for i, step in enumerate(steps) if step.get("uses", "").startswith("actions/upload-artifact@"))
        self.assertLess(compile_index, export_index)
        self.assertLess(export_index, verify_index)
        self.assertLess(verify_index, upload_index)
        self.assertIn("check_core_ruby_artifact.rb", commands[export_index])
        self.assertEqual(steps[export_index]["working-directory"], "packages/ruby")
        artifact = steps[upload_index]["with"]
        self.assertEqual(artifact["name"], "structuredmerge-core-development-x86_64-linux")
        self.assertIn("core-ruby-artifact.json", artifact["path"])
        self.assertNotIn("prototype", json.dumps(steps))
        self.assertNotIn("alef publish", "\n".join(commands))

    def test_export_does_not_replace_installed_runtime_gate(self):
        steps = self.jobs["typed-core-ruby-artifact"]["steps"]
        gate = next(step["run"] for step in steps if "check_core_ruby_artifact.rb" in step.get("run", ""))
        self.assertNotIn("--package-only", gate)
        for job in ("typed-core-ruby-artifact", "ruby-package"):
            ruby = next(step for step in self.jobs[job]["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@"))
            self.assertEqual(ruby["with"]["ruby-version"], "4.0")

    def test_retired_publication_is_removed_but_regression_sources_remain(self):
        self.assertFalse((self.root / ".github/workflows/release-ruby-host.yml").exists())
        for path in (self.root / ".github/workflows").glob("*.yml"):
            source = path.read_text()
            self.assertNotIn("structuredmerge_host_prototype-v", source)
            self.assertNotIn("structuredmerge-host-prototype-", source)
        for path in ("packages/ruby/spec/structuredmerge_host_prototype_spec.rb",
                     "crates/structuredmerge-host-prototype-core/src/lib.rs",
                     "contracts/legacy-operation-migration.json"):
            self.assertTrue((self.root / path).is_file())
