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
        ruby = next(step for step in self.jobs["ruby-package"]["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@"))
        self.assertEqual(ruby["with"]["ruby-version"], "4.0")

    def test_installed_core_matrix_preserves_separate_legacy_coverage(self):
        job = self.jobs["typed-core-ruby-artifact"]
        legacy = self.jobs["ruby-bindings"]
        matrix = job["strategy"]["matrix"]["include"]
        self.assertEqual(matrix, legacy["strategy"]["matrix"]["include"])
        self.assertEqual(len(matrix), 6)
        self.assertEqual({row["ruby"] for row in matrix}, {"3.2", "4.0"})
        self.assertEqual({row["platform"] for row in matrix}, {
            "x86_64-linux", "aarch64-linux", "arm64-darwin",
            "x86_64-darwin", "x64-mingw-ucrt",
        })
        self.assertFalse(job["strategy"]["fail-fast"])
        self.assertEqual(job["runs-on"], "${{ matrix.runner }}")
        ruby = next(step for step in job["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@"))
        self.assertEqual(ruby["with"]["ruby-version"], "${{ matrix.ruby }}")
        windows = next(step for step in job["steps"] if step.get("if") == "runner.os == 'Windows'")
        self.assertIn("CARGO_BUILD_TARGET=x86_64-pc-windows-gnu", windows["run"])
        self.assertIn("RUST_TARGET=x86_64-pc-windows-gnu", windows["run"])
        commands = "\n".join(step.get("run", "") for step in legacy["steps"])
        self.assertIn("bundle exec rake spec", commands)
        self.assertIn("check_ruby_api.rb", commands)

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

    def test_python_installed_matrix_covers_planned_architectures(self):
        job = self.jobs["typed-core-python-artifact"]
        matrix = job["strategy"]["matrix"]["include"]
        self.assertEqual(len(matrix), 7)
        self.assertEqual({row["platform"] for row in matrix}, {
            "linux-x64", "linux-arm64", "macos-x64", "macos-arm64",
            "windows-x64", "windows-arm64",
        })
        self.assertEqual({row["python"] for row in matrix}, {"3.10", "3.14"})
        self.assertEqual(job["runs-on"], "${{ matrix.runner }}")
        self.assertFalse(job["strategy"]["fail-fast"])
        self.assertEqual(job["env"]["STRUCTUREDMERGE_NATIVE_PYTHON"], "python")
        gate = next(step for step in job["steps"] if "check_core_python_artifact.py" in step.get("run", ""))
        self.assertNotIn("if", gate)
        self.assertEqual(gate["run"], "python workspace-scripts/check_core_python_artifact.py tmp/core-wheels")
