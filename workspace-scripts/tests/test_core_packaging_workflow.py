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

    def test_tooling_audits_have_fixtures_ruby_and_both_installed_artifacts(self):
        job = self.jobs["installed-kernel-cli"]
        steps = job["steps"]
        commands = [step.get("run", "") for step in steps]
        fixture_index = next(i for i, step in enumerate(steps)
                             if step.get("with", {}).get("repository") == "structuredmerge/structuredmerge-fixtures")
        move_index = commands.index("mv shared-fixtures ../fixtures")
        ruby_index = next(i for i, step in enumerate(steps) if step.get("uses", "").startswith("ruby/setup-ruby@"))
        build_index = next(i for i, command in enumerate(commands) if "cargo install --path" in command)
        audits = [(i, step) for i, step in enumerate(steps) if "unittest discover" in step.get("run", "")]
        self.assertEqual(len(audits), 3)
        self.assertEqual({step["env"]["SMORG_TEST_ARTIFACT"].rsplit("/", 1)[1] for _, step in audits}, {"smorg", "smorg-rs"})
        self.assertLess(fixture_index, move_index)
        for index, step in audits:
            self.assertLess(move_index, index)
            self.assertLess(ruby_index, index)
            self.assertLess(build_index, index)
            self.assertNotIn("if", step)
        self.assertIn("command -v ssh-keygen", audits[0][1]["run"])
        self.assertNotIn("unittest discover", json.dumps(self.jobs["typed-core-python-artifact"]))
        self.assertEqual(job["env"]["CARGO_BUILD_JOBS"], "1")
        self.assertEqual(job["env"]["CARGO_INCREMENTAL"], "0")
        self.assertEqual(job["env"]["CARGO_PROFILE_DEV_DEBUG"], "0")
        cleanup = steps[-1]
        self.assertEqual(cleanup["if"], "always()")
        self.assertIn("rm -r -- tmp/cli-target", cleanup["run"])
        self.assertIn("rm -r -- tmp/cli-install", cleanup["run"])
        self.assertIn("rm -r -- tmp/typed-tslp-cache", cleanup["run"])
        self.assertNotIn("rm -r -- tmp\n", cleanup["run"])
        self.assertNotIn("df -h", cleanup["run"])

    def test_typed_git_ci_uses_published_fixtures_and_exact_prepared_grammar(self):
        steps = self.jobs["installed-kernel-cli"]["steps"]
        fixtures = next(step for step in steps
                        if step.get("with", {}).get("repository") == "structuredmerge/structuredmerge-fixtures")
        self.assertEqual(fixtures["with"]["ref"], "be4bf25d59bceac6a4adcc27dea081a2bab4c798")
        grammar = "tmp/typed-tslp-cache/tree-sitter-language-pack/v1.17.0/libs/libtree_sitter_json.so"
        legacy = [i for i, step in enumerate(steps) if "slice-951-git-driver-json-integration" in step.get("run", "")]
        typed = [(i, step) for i, step in enumerate(steps) if "--typed --fixtures" in step.get("run", "")]
        self.assertEqual(len(legacy), 2)
        provision = next(i for i, step in enumerate(steps)
                         if "prepare_cli_git_grammar.py" in step.get("run", ""))
        self.assertLess(provision, min(legacy))
        self.assertIn("--cache tmp/typed-tslp-cache", steps[provision]["run"])
        self.assertEqual(len(typed), 2)
        for index, step in typed:
            self.assertLess(max(legacy), index)
            self.assertIn("--grammar-library " + grammar, step["run"])
            self.assertIn("../fixtures/conformance/cli-v1/typed-git.json", step["run"])
        auth_index, auth = next((i, step) for i, step in enumerate(steps) if "SMORG_TEST_GRAMMAR" in step.get("env", {}))
        self.assertLess(max(legacy), auth_index)
        self.assertIn('test -f "$SMORG_TEST_GRAMMAR"', auth["run"])
        self.assertTrue(auth["env"]["SMORG_TEST_GRAMMAR"].endswith(grammar))
        self.assertIn('name = "tree-sitter-language-pack"\nversion = "1.17.0"', (self.root / "Cargo.lock").read_text())
        upload = next(step for step in steps if step.get("uses", "").startswith("actions/upload-artifact@"))
        self.assertIn("tmp/typed-cli-git-*/report.json", upload["with"]["path"])
        self.assertIn("tmp/cli-grammar-auth.log", upload["with"]["path"])

    def test_every_fixture_consumer_uses_the_reviewed_published_revision(self):
        consumers = {}
        for name, job in self.jobs.items():
            for step in job.get("steps", []):
                checkout = step.get("with", {})
                if checkout.get("repository") == "structuredmerge/structuredmerge-fixtures":
                    consumers[name] = checkout["ref"]
        self.assertEqual(set(consumers), {"installed-kernel-cli", "check", "ruby-bindings"})
        for name, revision in consumers.items():
            with self.subTest(job=name):
                self.assertEqual(revision, "be4bf25d59bceac6a4adcc27dea081a2bab4c798")

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

    def test_python_binding_build_pins_reproducible_source_epoch(self):
        steps = self.jobs["typed-core-python-artifact"]["steps"]
        build = next(step for step in steps if "maturin build" in step.get("run", ""))
        self.assertEqual(build["shell"], "bash")
        self.assertIn('source_date_epoch="$(git show -s --format=%ct "$GITHUB_SHA")"', build["run"])
        self.assertIn('SOURCE_DATE_EPOCH=$source_date_epoch', build["run"])

    def test_export_does_not_replace_installed_runtime_gate(self):
        steps = self.jobs["typed-core-ruby-artifact"]["steps"]
        gate = next(step["run"] for step in steps if "check_core_ruby_artifact.rb" in step.get("run", ""))
        self.assertNotIn("--package-only", gate)
        ruby = next(step for step in self.jobs["ruby-package"]["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@"))
        self.assertEqual(ruby["with"]["ruby-version"], "4.0")

    def test_windows_ruby_target_is_installed_for_the_active_toolchain(self):
        for name in ("typed-core-ruby-artifact", "ruby-bindings"):
            with self.subTest(job=name):
                job = self.jobs[name]
                steps = job["steps"]
                setup = next(i for i, step in enumerate(steps)
                             if step.get("uses", "").startswith("dtolnay/rust-toolchain@"))
                abi_index, abi = next((i, step) for i, step in enumerate(steps)
                                     if "CARGO_BUILD_TARGET=" in step.get("run", ""))
                self.assertEqual(abi["if"], "runner.os == 'Windows'")
                self.assertEqual(abi["shell"], "bash")
                # No +stable/--toolchain override: honor rust-toolchain.toml,
                # just as the subsequent rb-sys cargo subprocess does.
                self.assertIn("rustup target add x86_64-pc-windows-gnu", abi["run"].splitlines())
                self.assertNotIn("RUSTUP_TOOLCHAIN", job.get("env", {}))
                self.assertNotIn("working-directory", abi)
                self.assertLess(setup, abi_index)
                builds = [i for i, step in enumerate(steps)
                          if "rake compile" in step.get("run", "")
                          or "build_legacy_ruby_regression.rb" in step.get("run", "")]
                self.assertTrue(builds)
                for build in builds:
                    self.assertLess(abi_index, build)
                    self.assertNotIn("if", steps[build])

    def test_windows_binding_checkouts_preserve_reviewed_source_bytes(self):
        for name in ("typed-core-python-artifact", "typed-core-ruby-artifact", "ruby-bindings"):
            with self.subTest(job=name):
                steps = self.jobs[name]["steps"]
                checkout = next(i for i, step in enumerate(steps)
                                if step.get("uses", "").startswith("actions/checkout@"))
                preservation = [(i, step) for i, step in enumerate(steps)
                                if step.get("run") == "git config --global core.autocrlf false"]
                self.assertEqual(len(preservation), 1)
                index, step = preservation[0]
                self.assertEqual(step["if"], "runner.os == 'Windows'")
                self.assertLess(index, checkout)

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
        windows = next(step for step in job["steps"]
                       if "CARGO_BUILD_TARGET=" in step.get("run", ""))
        self.assertEqual(windows["if"], "runner.os == 'Windows'")
        self.assertIn("CARGO_BUILD_TARGET=x86_64-pc-windows-gnu", windows["run"])
        self.assertIn("RUST_TARGET=x86_64-pc-windows-gnu", windows["run"])
        commands = "\n".join(step.get("run", "") for step in legacy["steps"])
        self.assertIn("bundle exec rake spec", commands)
        self.assertIn("check_ruby_api.rb", commands)
        build = next(i for i, step in enumerate(legacy["steps"])
                     if "build_legacy_ruby_regression.rb" in step.get("run", ""))
        tests = next(i for i, step in enumerate(legacy["steps"])
                     if step.get("run") == "bundle exec rake spec")
        self.assertLess(build, tests)
        for name in ("typed-core-ruby-artifact", "ruby-package"):
            self.assertNotIn("build_legacy_ruby_regression.rb", json.dumps(self.jobs[name]))

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
