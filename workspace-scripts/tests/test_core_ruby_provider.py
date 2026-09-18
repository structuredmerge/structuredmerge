"""The opt-in native-provider gate must not package or use the test projector."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "workspace-scripts/check_core_ruby_artifact.rb"


class RubyProviderGateTest(unittest.TestCase):
    def test_help_and_invalid_argument_combinations_do_not_package(self):
        for arguments, code in [(["--help"], 0), (["--provider-gem"], 1),
                                (["--provider-gem", "missing", "--package-only", "unused"], 1)]:
            with self.subTest(arguments=arguments):
                result = subprocess.run(["ruby", str(SCRIPT), *arguments], cwd=ROOT,
                    text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode, code, result.stderr)
                self.assertIn("usage:", result.stdout + result.stderr)
                self.assertNotIn("Artifact report directory:", result.stdout)

    def test_rejects_wrong_provider_package_before_creating_workspace(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="ruby-provider-gate-", dir=ROOT / "tmp") as directory:
            build = '''require "rubygems/package"
spec = Gem::Specification.new do |s|
  s.name = "wrong-provider"
  s.version = "0.1.0"
  s.summary = "Harmless archive test"
  s.authors = ["StructuredMerge"]
end
Gem::Package.build(spec)
'''
            subprocess.run(["ruby", "-e", build], cwd=directory, capture_output=True, check=True, timeout=15)
            result = subprocess.run(["ruby", str(SCRIPT), "--provider-gem",
                str(Path(directory) / "wrong-provider-0.1.0.gem")], cwd=ROOT,
                text=True, capture_output=True, timeout=15)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("expected a psych-merge", result.stderr)
            self.assertNotIn("Artifact report directory:", result.stdout)

    def test_installed_helper_requires_real_provider_and_never_falls_back(self):
        # The branch must attempt the installed package, not load the supplied
        # poisoned projector when the provider is absent. Run without Bundler so
        # the harmless core require can be satisfied without a native extension.
        with tempfile.TemporaryDirectory(prefix="ruby-provider-helper-", dir=ROOT / "tmp") as directory:
            script = '''$LOADED_FEATURES << "structuredmerge_core.rb"
ENV["STRUCTUREDMERGE_PSYCH_INSTALLED"] = "1"
ENV["STRUCTUREDMERGE_PSYCH_FACTS"] = ARGV.fetch(1)
module Kernel
  alias original_require require
  def require(name)
    raise LoadError, "missing installed Psych adapter" if name == "psych/merge/core_parser_host"
    original_require(name)
  end
end
begin
  load ARGV.fetch(0)
  abort "provider absence was ignored"
rescue LoadError => error
  abort error.message unless error.message == "missing installed Psych adapter"
end
'''
            result = subprocess.run(["ruby", "-e", script,
                str(ROOT / "packages/ruby/spec/native_merge_fixture.rb"), str(Path(directory) / "nonexistent")],
                cwd=directory, text=True, capture_output=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
