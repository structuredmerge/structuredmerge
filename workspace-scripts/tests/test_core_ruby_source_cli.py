import pathlib
import subprocess
import shutil
import tempfile
import unittest


class RubySourceArgumentsTest(unittest.TestCase):
    def test_source_package_detaches_extension_from_checkout_workspace(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        script = (root / "workspace-scripts/prepare_core_ruby_source.rb").read_text()
        assert "Standalone root for the packaged Ruby extension crate" in script
        assert "unless manifest_text.match?(/^\\[workspace\\]\\s*$/)" in script

    def test_help_and_invalid_arguments_do_not_prepare(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        script = root / "workspace-scripts/prepare_core_ruby_source.rb"
        for arguments, expected in [
            (["--help"], 0),
            ([], 1),
            (["--alef"], 1),
            (["--unknown"], 1),
            (["--output", "unused"], 1),
        ]:
            with self.subTest(arguments=arguments):
                result = subprocess.run(["ruby", str(script), *arguments], cwd=root, text=True, capture_output=True)
                assert result.returncode == expected, result.stderr
                assert "usage:" in result.stdout + result.stderr
                assert "Successfully built" not in result.stdout

    def test_existing_output_is_rejected_before_preparation(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(dir=root / "tmp") as directory:
            sentinel = pathlib.Path(directory) / "sentinel"
            sentinel.write_text("preserve")
            result = subprocess.run(
                [
                    "ruby",
                    str(root / "workspace-scripts/prepare_core_ruby_source.rb"),
                    "--alef",
                    shutil.which("ruby"),
                    "--output",
                    directory,
                ],
                cwd=root,
                text=True,
                capture_output=True,
            )
            assert result.returncode == 1
            assert "output already exists" in result.stderr
            assert sentinel.read_text() == "preserve"
