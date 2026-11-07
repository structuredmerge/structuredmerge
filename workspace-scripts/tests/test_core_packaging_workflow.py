"""Packaging must produce the typed core, never release the retired prototype."""

import json
from pathlib import Path
import subprocess
import unittest


class CorePackagingWorkflowTest(unittest.TestCase):
    root = Path(__file__).resolve().parents[2]

    @classmethod
    def setUpClass(cls):
        result = subprocess.run(
            [
                "ruby",
                "-ryaml",
                "-rjson",
                "-e",
                "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))",
                str(cls.root / ".github/workflows/current.yml"),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
        cls.jobs = json.loads(result.stdout)["jobs"]

    def test_tooling_audits_have_fixtures_ruby_and_both_installed_artifacts(self):
        job = self.jobs["installed-kernel-cli"]
        steps = job["steps"]
        commands = [step.get("run", "") for step in steps]
        fixture_index = next(
            i
            for i, step in enumerate(steps)
            if step.get("with", {}).get("repository") == "structuredmerge/structuredmerge-fixtures"
        )
        move_index = commands.index("mv shared-fixtures ../fixtures")
        ruby_index = next(i for i, step in enumerate(steps) if step.get("uses", "").startswith("ruby/setup-ruby@"))
        build_index = next(i for i, command in enumerate(commands) if "cargo install --path" in command)
        audits = [(i, step) for i, step in enumerate(steps) if "unittest discover" in step.get("run", "")]
        assert len(audits) == 3
        assert {step["env"]["SMORG_TEST_ARTIFACT"].rsplit("/", 1)[1] for _, step in audits} == {"smorg", "smorg-rs"}
        assert fixture_index < move_index
        for index, step in audits:
            assert move_index < index
            assert ruby_index < index
            assert build_index < index
            assert "if" not in step
        assert "command -v ssh-keygen" in audits[0][1]["run"]
        assert "unittest discover" not in json.dumps(self.jobs["typed-core-python-artifact"])
        assert job["env"]["CARGO_BUILD_JOBS"] == "1"
        assert job["env"]["CARGO_INCREMENTAL"] == "0"
        assert job["env"]["CARGO_PROFILE_DEV_DEBUG"] == "0"
        cleanup = steps[-1]
        assert cleanup["if"] == "always()"
        assert "rm -r -- tmp/cli-target" in cleanup["run"]
        assert "rm -r -- tmp/cli-install" in cleanup["run"]
        assert "rm -r -- tmp/typed-tslp-cache" in cleanup["run"]
        assert "rm -r -- tmp\n" not in cleanup["run"]
        assert "df -h" not in cleanup["run"]

    def test_typed_git_ci_uses_published_fixtures_and_exact_prepared_grammar(self):
        steps = self.jobs["installed-kernel-cli"]["steps"]
        fixtures = next(
            step
            for step in steps
            if step.get("with", {}).get("repository") == "structuredmerge/structuredmerge-fixtures"
        )
        assert fixtures["with"]["ref"] == "9c6e4bb0625f28207d1f40e5d103936eed56154a"
        grammar = "tmp/typed-tslp-cache/tree-sitter-language-pack/v1.17.0/libs/libtree_sitter_json.so"
        legacy = [i for i, step in enumerate(steps) if "slice-951-git-driver-json-integration" in step.get("run", "")]
        typed = [(i, step) for i, step in enumerate(steps) if "--typed --fixtures" in step.get("run", "")]
        assert len(legacy) == 2
        provision = next(i for i, step in enumerate(steps) if "prepare_cli_git_grammar.py" in step.get("run", ""))
        assert provision < min(legacy)
        assert "--cache tmp/typed-tslp-cache" in steps[provision]["run"]
        assert len(typed) == 2
        for index, step in typed:
            assert max(legacy) < index
            assert "--grammar-library " + grammar in step["run"]
            assert "../fixtures/conformance/cli-v1/typed-git.json" in step["run"]
        auth_index, auth = next(
            (i, step) for i, step in enumerate(steps) if "SMORG_TEST_GRAMMAR" in step.get("env", {})
        )
        assert max(legacy) < auth_index
        assert 'test -f "$SMORG_TEST_GRAMMAR"' in auth["run"]
        assert auth["env"]["SMORG_TEST_GRAMMAR"].endswith(grammar)
        assert 'name = "tree-sitter-language-pack"\nversion = "1.17.0"' in (self.root / "Cargo.lock").read_text()
        upload = next(step for step in steps if step.get("uses", "").startswith("actions/upload-artifact@"))
        assert "tmp/typed-cli-git-*/report.json" in upload["with"]["path"]
        assert "tmp/cli-grammar-auth.log" in upload["with"]["path"]

    def test_every_fixture_consumer_uses_the_reviewed_published_revision(self):
        consumers = {}
        for name, job in self.jobs.items():
            for step in job.get("steps", []):
                checkout = step.get("with", {})
                if checkout.get("repository") == "structuredmerge/structuredmerge-fixtures":
                    consumers[name] = checkout["ref"]
        assert set(consumers) == {"installed-kernel-cli", "check", "alef"}
        for name, revision in consumers.items():
            with self.subTest(job=name):
                assert revision == "9c6e4bb0625f28207d1f40e5d103936eed56154a"

    def test_export_compiles_then_packages_verifies_and_uploads_only_core(self):
        steps = self.jobs["ruby-package"]["steps"]
        commands = [step.get("run", "") for step in steps]
        compile_index = commands.index("bundle exec rake compile")
        export_index = next(i for i, command in enumerate(commands) if "--package-only" in command)
        verify_index = next(i for i, command in enumerate(commands) if "verify_core_ruby_export.rb" in command)
        upload_index = next(
            i for i, step in enumerate(steps) if step.get("uses", "").startswith("actions/upload-artifact@")
        )
        assert compile_index < export_index
        assert export_index < verify_index
        assert verify_index < upload_index
        assert "check_core_ruby_artifact.rb" in commands[export_index]
        assert steps[export_index]["working-directory"] == "packages/ruby"
        artifact = steps[upload_index]["with"]
        assert artifact["name"] == "structuredmerge-core-development-x86_64-linux"
        assert "core-ruby-artifact.json" in artifact["path"]
        assert "prototype" not in json.dumps(steps)
        assert "alef publish" not in "\n".join(commands)

    def test_python_binding_build_pins_reproducible_source_epoch(self):
        steps = self.jobs["typed-core-python-artifact"]["steps"]
        build = next(step for step in steps if "maturin build" in step.get("run", ""))
        assert build["shell"] == "bash"
        assert 'source_date_epoch="$(git show -s --format=%ct "$GITHUB_SHA")"' in build["run"]
        assert "SOURCE_DATE_EPOCH=$source_date_epoch" in build["run"]

    def test_export_does_not_replace_installed_runtime_gate(self):
        steps = self.jobs["typed-core-ruby-artifact"]["steps"]
        gate = next(step["run"] for step in steps if "check_core_ruby_artifact.rb" in step.get("run", ""))
        assert "--package-only" not in gate
        ruby = next(
            step for step in self.jobs["ruby-package"]["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@")
        )
        assert ruby["with"]["ruby-version"] == "4.0"

    def test_windows_ruby_target_is_installed_for_the_active_toolchain(self):
        for name in ("typed-core-ruby-artifact",):
            with self.subTest(job=name):
                job = self.jobs[name]
                steps = job["steps"]
                setup = next(
                    i for i, step in enumerate(steps) if step.get("uses", "").startswith("dtolnay/rust-toolchain@")
                )
                abi_index, abi = next(
                    (i, step) for i, step in enumerate(steps) if "CARGO_BUILD_TARGET=" in step.get("run", "")
                )
                assert abi["if"] == "runner.os == 'Windows'"
                assert abi["shell"] == "bash"
                # No +stable/--toolchain override: honor rust-toolchain.toml,
                # just as the subsequent rb-sys cargo subprocess does.
                assert "rustup target add x86_64-pc-windows-gnu" in abi["run"].splitlines()
                assert "RUSTUP_TOOLCHAIN" not in job.get("env", {})
                assert "working-directory" not in abi
                assert setup < abi_index
                builds = [i for i, step in enumerate(steps) if "rake compile" in step.get("run", "")]
                assert builds
                for build in builds:
                    assert abi_index < build
                    assert "if" not in steps[build]

    def test_windows_binding_checkouts_preserve_reviewed_source_bytes(self):
        for name in ("typed-core-python-artifact", "typed-core-ruby-artifact"):
            with self.subTest(job=name):
                steps = self.jobs[name]["steps"]
                checkout = next(
                    i for i, step in enumerate(steps) if step.get("uses", "").startswith("actions/checkout@")
                )
                preservation = [
                    (i, step)
                    for i, step in enumerate(steps)
                    if step.get("run") == "git config --global core.autocrlf false"
                ]
                assert len(preservation) == 1
                index, step = preservation[0]
                assert step["if"] == "runner.os == 'Windows'"
                assert index < checkout

    def test_installed_core_matrix_covers_supported_rubies(self):
        job = self.jobs["typed-core-ruby-artifact"]
        matrix = job["strategy"]["matrix"]["include"]
        # EOL Ruby (3.2) is deliberately unsupported: rb-sys cannot bind its
        # headers and nothing ever validated the claim. Only supported,
        # tested rubies appear in the matrix.
        assert len(matrix) == 5
        assert {row["ruby"] for row in matrix} == {"4.0"}
        assert {row["platform"] for row in matrix} == {
            "x86_64-linux",
            "aarch64-linux",
            "arm64-darwin",
            "x86_64-darwin",
            "x64-mingw-ucrt",
        }
        assert not job["strategy"]["fail-fast"]
        assert job["runs-on"] == "${{ matrix.runner }}"
        ruby = next(step for step in job["steps"] if step.get("uses", "").startswith("ruby/setup-ruby@"))
        assert ruby["with"]["ruby-version"] == "${{ matrix.ruby }}"
        windows = next(step for step in job["steps"] if "CARGO_BUILD_TARGET=" in step.get("run", ""))
        assert windows["if"] == "runner.os == 'Windows'"
        assert "CARGO_BUILD_TARGET=x86_64-pc-windows-gnu" in windows["run"]
        assert "RUST_TARGET=x86_64-pc-windows-gnu" in windows["run"]

    def test_retired_publication_is_removed(self):
        assert not (self.root / ".github/workflows/release-ruby-host.yml").exists()

    def test_python_installed_matrix_covers_planned_architectures(self):
        job = self.jobs["typed-core-python-artifact"]
        matrix = job["strategy"]["matrix"]["include"]
        assert len(matrix) == 7
        assert {row["platform"] for row in matrix} == {
            "linux-x64",
            "linux-arm64",
            "macos-x64",
            "macos-arm64",
            "windows-x64",
            "windows-arm64",
        }
        assert {row["python"] for row in matrix} == {"3.11", "3.14"}
        assert job["runs-on"] == "${{ matrix.runner }}"
        assert not job["strategy"]["fail-fast"]
        assert job["env"]["STRUCTUREDMERGE_NATIVE_PYTHON"] == "python"
        gate = next(step for step in job["steps"] if "check_core_python_artifact.py" in step.get("run", ""))
        assert "if" not in gate
        assert gate["run"] == "python workspace-scripts/check_core_python_artifact.py tmp/core-wheels"
