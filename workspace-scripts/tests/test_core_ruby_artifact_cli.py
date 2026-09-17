"""Read-only argument checks for the Ruby development artifact builder."""
import pathlib
import subprocess
import unittest


class RubyArtifactArgumentsTest(unittest.TestCase):
    def test_help_and_invalid_arguments_do_not_build(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        script = root / "workspace-scripts/check_core_ruby_artifact.rb"
        for arguments, expected in [(["--help"], 0), (["--package-only"], 1),
                                    (["--package-only", ""], 1), (["--unknown"], 1),
                                    (["--package-only", "unused", "extra"], 1)]:
            with self.subTest(arguments=arguments):
                result = subprocess.run(["ruby", str(script), *arguments], cwd=root,
                                        text=True, capture_output=True, check=False)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertIn("usage:", result.stdout + result.stderr)
                self.assertNotIn("Successfully built", result.stdout)
