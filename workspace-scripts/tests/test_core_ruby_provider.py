"""The opt-in native-provider gate must not package or use the test projector."""
import os
from pathlib import Path
import shutil
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

    PROVIDER_BUILD = '''require "rubygems/package"
require "fileutils"
provider_dir, support_dir = ARGV
FileUtils.mkdir_p("lib/psych/merge")
File.write("lib/psych/merge/core_parser_host.rb", "# harmless provider host\\n")
provider = Gem::Specification.new do |s|
  s.name = "psych-merge"
  s.version = "99.0.1"
  s.summary = "Harmless archive test"
  s.authors = ["StructuredMerge"]
  s.files = ["lib/psych/merge/core_parser_host.rb"]
  s.add_dependency "zzq-sibling", "= 0.1.0"
end
Gem::Package.build(provider, false, false, File.join(provider_dir, "psych-merge-99.0.1.gem"))
sibling_version = ENV.fetch("ZZQ_SIBLING_VERSION", "0.1.0")
sibling = Gem::Specification.new do |s|
  s.name = "zzq-sibling"
  s.version = sibling_version
  s.summary = "Harmless archive test"
  s.authors = ["StructuredMerge"]
  s.files = []
end
Gem::Package.build(sibling, false, false, File.join(support_dir, "zzq-sibling-#{sibling_version}.gem"))
extra_name = ENV["ZZQ_EXTRA_NAME"]
if extra_name
  extra = Gem::Specification.new do |s|
    s.name = extra_name
    s.version = "0.1.0"
    s.summary = "Harmless archive test"
    s.authors = ["StructuredMerge"]
    s.files = []
  end
  Gem::Package.build(extra, false, false, File.join(support_dir, "#{extra_name}-0.1.0.gem"))
end
'''

    def _build_provider_with_support(self, directory, sibling_version="0.1.0", extra_name=None):
        provider_dir = Path(directory) / "provider"
        support_dir = Path(directory) / "support"
        provider_dir.mkdir()
        support_dir.mkdir()
        env = {"ZZQ_SIBLING_VERSION": sibling_version}
        if extra_name:
            env["ZZQ_EXTRA_NAME"] = extra_name
        subprocess.run(["ruby", "-e", self.PROVIDER_BUILD, str(provider_dir), str(support_dir)],
            cwd=directory, env={**os.environ, **env}, capture_output=True, check=True, timeout=30)
        return provider_dir / "psych-merge-99.0.1.gem", support_dir

    def test_support_dir_requires_provider_gem(self):
        result = subprocess.run(["ruby", str(SCRIPT), "--provider-support-dir", "somewhere"], cwd=ROOT,
            text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 1)
        self.assertIn("usage:", result.stdout + result.stderr)

    def test_provider_support_rejects_missing_or_empty_directory(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="ruby-provider-support-", dir=ROOT / "tmp") as directory:
            provider, support = self._build_provider_with_support(directory)
            for arguments, expected in [
                (["--provider-gem", str(provider), "--provider-support-dir", str(Path(directory) / "absent")],
                 "provider support directory is missing"),
                (["--provider-gem", str(provider), "--provider-support-dir",
                  str(self._empty_dir(Path(directory) / "empty"))],
                 "provider support directory has no gem archives"),
            ]:
                with self.subTest(expected=expected):
                    result = subprocess.run(["ruby", str(SCRIPT), *arguments], cwd=ROOT,
                        text=True, capture_output=True, timeout=30)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(expected, result.stderr)
                    self.assertNotIn("Artifact report directory:", result.stdout)

    @staticmethod
    def _empty_dir(path):
        path.mkdir(parents=True, exist_ok=True)
        return path

    def test_provider_support_rejects_version_that_cannot_satisfy_provider(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="ruby-provider-support-", dir=ROOT / "tmp") as directory:
            provider, support = self._build_provider_with_support(directory, sibling_version="0.2.0")
            result = subprocess.run(["ruby", str(SCRIPT), "--provider-gem", str(provider),
                "--provider-support-dir", str(support)], cwd=ROOT,
                text=True, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("does not satisfy", result.stderr)
            self.assertNotIn("Artifact report directory:", result.stdout)

    def test_provider_support_rejects_archives_outside_the_dependency_closure(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="ruby-provider-support-", dir=ROOT / "tmp") as directory:
            provider, support = self._build_provider_with_support(directory, extra_name="zzq-unrelated")
            result = subprocess.run(["ruby", str(SCRIPT), "--provider-gem", str(provider),
                "--provider-support-dir", str(support)], cwd=ROOT,
                text=True, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unexpected provider support archive zzq-unrelated", result.stderr)
            self.assertNotIn("Artifact report directory:", result.stdout)

    def test_provider_support_rejects_duplicate_provider_or_core_archives(self):
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="ruby-provider-support-", dir=ROOT / "tmp") as directory:
            provider, support = self._build_provider_with_support(directory)
            shutil.copyfile(provider, support / "psych-merge-99.0.1.gem")
            result = subprocess.run(["ruby", str(SCRIPT), "--provider-gem", str(provider),
                "--provider-support-dir", str(support)], cwd=ROOT,
                text=True, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("must not duplicate the provider or core", result.stderr)
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
