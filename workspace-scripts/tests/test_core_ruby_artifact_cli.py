"""Read-only argument checks for the Ruby development artifact builder."""

import pathlib
import json
import shutil
import subprocess
import tempfile
import unittest


class RubyArtifactArgumentsTest(unittest.TestCase):
    def test_missing_extension_cleans_staging_and_retains_failure(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        (root / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root / "tmp", prefix="ruby-artifact-cleanup-test-") as directory:
            checkout = pathlib.Path(directory)
            scripts = checkout / "workspace-scripts"
            scripts.mkdir()
            for name in ("check_core_ruby_artifact.rb", "artifact_workspace.rb"):
                shutil.copyfile(root / "workspace-scripts" / name, scripts / name)
            package = checkout / "packages/ruby"
            package.mkdir(parents=True)
            (package / "structuredmerge_core.gemspec").write_text(
                'Gem::Specification.new { |s| s.name = "structuredmerge-core"; s.version = "0.2.0" }\n'
            )
            result = subprocess.run(
                ["ruby", str(scripts / "check_core_ruby_artifact.rb")],
                cwd=checkout,
                text=True,
                capture_output=True,
                timeout=15,
            )
            assert result.returncode == 1, result.stderr
            assert "build the current native extension" in result.stderr
            (stage,) = (checkout / "tmp").iterdir()
            assert list(stage.iterdir()) == [stage / "failure.json"]
            report = json.loads((stage / "failure.json").read_text())
            assert report["status"] == "failed"
            assert not report["publication_gate"]

    def test_help_and_invalid_arguments_do_not_build(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        script = root / "workspace-scripts/check_core_ruby_artifact.rb"
        for arguments, expected in [
            (["--help"], 0),
            (["--package-only"], 1),
            (["--package-only", ""], 1),
            (["--unknown"], 1),
            (["--package-only", "unused", "extra"], 1),
        ]:
            with self.subTest(arguments=arguments):
                result = subprocess.run(
                    ["ruby", str(script), *arguments], cwd=root, text=True, capture_output=True, check=False
                )
                assert result.returncode == expected, result.stderr
                assert "usage:" in result.stdout + result.stderr
                assert "Successfully built" not in result.stdout
