"""Read-only checks for the installed real-Git gate's failure semantics."""
from pathlib import Path
import subprocess
import sys
import unittest


class InstalledCliGitGateTest(unittest.TestCase):
    root = Path(__file__).resolve().parents[2]
    script = root / "workspace-scripts/check_installed_cli_git.py"

    def test_usage_requires_explicit_binary_and_fixture(self):
        for arguments, status in [(["--help"], 0), ([], 2), (["missing-binary"], 2)]:
            with self.subTest(arguments=arguments):
                result = subprocess.run([sys.executable, str(self.script), *arguments],
                                        capture_output=True, text=True, timeout=10, check=False)
                self.assertEqual(result.returncode, status)
                self.assertIn("usage:", result.stdout + result.stderr)

    def test_failed_require_is_not_removed_by_python_optimization(self):
        program = """
import importlib.util, sys
spec = importlib.util.spec_from_file_location('git_gate', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.require(False, 'intentional gate failure')
"""
        for mode in ([], ["-O"]):
            result = subprocess.run([sys.executable, *mode, "-c", program, str(self.script)],
                                    capture_output=True, text=True, timeout=10, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("intentional gate failure", result.stderr)
