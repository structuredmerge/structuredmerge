"""Exercise Ruby cleanup in real processes, without building or installing gems."""
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "workspace-scripts/artifact_workspace.rb"


class RubyArtifactWorkspaceTest(unittest.TestCase):
    def test_cleanup_on_success_exit_abort_exception_and_interrupt(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        for action, success in [("nil", True), ("exit 0", True), ("abort 'failed'", False),
                                ("raise 'failed'", False), ("raise Interrupt", False)]:
            with self.subTest(action=action), tempfile.TemporaryDirectory(
                dir=ROOT / "tmp", prefix="ruby-cleanup-test-"
            ) as directory:
                root = Path(directory)
                code = '''
                  root, action = ARGV
                  File.write(File.join(root, "export.gem"), "export")
                  ArtifactWorkspace.open(root: root, prefix: "probe-",
                    disposable: %w[package consumer gems workspace source.tar]) do |stage|
                    %w[package consumer gems workspace].each do |name|
                      FileUtils.mkdir_p(File.join(stage, name))
                      File.write(File.join(stage, name, "partial"), "discard")
                    end
                    File.write(File.join(stage, "source.tar"), "discard")
                    File.write(File.join(stage, "prepare.log"), "retain")
                    eval(action)
                    File.write(File.join(stage, "report.json"), "retain")
                  end
                '''
                result = subprocess.run(["ruby", "-r", str(HELPER), "-e", code, directory, action],
                                        text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode == 0, success, result.stderr)
                stage, = (root / "tmp").iterdir()
                self.assertEqual((stage / "prepare.log").read_text(), "retain")
                self.assertEqual((root / "export.gem").read_text(), "export")
                for name in ("package", "consumer", "gems", "workspace", "source.tar"):
                    self.assertFalse((stage / name).exists())
                self.assertEqual((stage / "failure.json").exists(), not success)

    def test_rejects_parent_traversal_before_allocating_scratch(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="ruby-cleanup-test-") as directory:
            code = 'ArtifactWorkspace.open(root: ARGV[0], prefix: "probe-", disposable: ["../outside"]) {}'
            result = subprocess.run(["ruby", "-r", str(HELPER), "-e", code, directory],
                                    text=True, capture_output=True, timeout=15)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("direct child names", result.stderr)
            self.assertEqual(list(Path(directory).iterdir()), [])


if __name__ == "__main__":
    unittest.main()
