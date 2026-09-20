"""Consumer rejects incompatible or mismatched exports before installation."""
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile
import unittest


class RubyExportTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = pathlib.Path(__file__).resolve().parents[2]
        (cls.root / "tmp").mkdir(exist_ok=True)
        cls.scratch = tempfile.TemporaryDirectory(prefix="ruby-export-test-", dir=cls.root / "tmp")
        cls.directory = pathlib.Path(cls.scratch.name)
        # Build a harmless archive: deliberately no executable native code. This
        # verifies metadata only; passing must never imply a runtime test passed.
        build = r'''
require "rubygems/package"
require "rbconfig"
require "fileutils"
require "json"
abi = RbConfig::CONFIG.fetch("ruby_version")
files = ["AGPL-3.0-only.md", "PolyForm-Small-Business-1.0.0.md", "README.md",
  "lib/structuredmerge_core.rb", "lib/structuredmerge_core/native.rb",
  "lib/structuredmerge_core/version.rb", "sig/types.rbs",
  "lib/structuredmerge_core_rb/#{abi}/structuredmerge_core_rb.#{RbConfig::CONFIG.fetch('DLEXT')}"]
files.each { |file| FileUtils.mkdir_p(File.dirname(file)); File.write(file, "test fixture\n") }
major, minor = RUBY_VERSION.split(".").map(&:to_i)
spec = Gem::Specification.new do |s|
  s.name = "structuredmerge-core"
  s.version = "0.2.0"
  s.summary = "Metadata verification fixture"
  s.authors = ["StructuredMerge"]
  s.files = files
  s.platform = Gem::Platform.local
  s.required_ruby_version = Gem::Requirement.new(">= #{major}.#{minor}.0", "< #{major}.#{minor + 1}.0")
end
artifact = Gem::Package.build(spec)
puts JSON.generate(artifact: artifact, package: spec.name, version: spec.version.to_s,
  platform: spec.platform.to_s, ruby: RUBY_VERSION, ruby_abi: abi,
  required_ruby_version: spec.required_ruby_version.to_s, files: files.sort)
'''
        result = subprocess.run(["ruby", "-e", build], cwd=cls.directory,
                                capture_output=True, text=True, check=True)
        cls.report = json.loads(result.stdout.splitlines()[-1])
        cls.artifact = cls.directory / cls.report["artifact"]
        cls.report.update(sha256=hashlib.sha256(cls.artifact.read_bytes()).hexdigest(),
                          mode="package-only", installed_merge_tests="not_run",
                          generated_e2e_tests="not_run", type_declarations="not_validated",
                          publication_gate=False, source_gem_gate=False,
                          linkage_check="passed", api_review_baseline="ruby source surface matched")

    @classmethod
    def tearDownClass(cls):
        cls.scratch.cleanup()

    def verify(self, report, artifact=None):
        report_path = self.directory / "report.json"
        report_path.write_text(json.dumps(report))
        return subprocess.run(["ruby", str(self.root / "workspace-scripts/verify_core_ruby_export.rb"),
                               str(artifact or self.artifact), str(report_path)],
                              capture_output=True, text=True, check=False)

    def test_matching_export_is_not_a_runtime_claim(self):
        result = self.verify(self.report)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["runtime_tests"], "not_run")

    def test_mismatches_fail_closed(self):
        for key, value in {
            "artifact": "other.gem", "sha256": "0" * 64, "package": "prototype",
            "version": "99.0", "platform": "ruby", "ruby_abi": "0.0.0",
            "ruby": "0.0.1", "required_ruby_version": ">= 0", "files": [],
            "installed_merge_tests": "passed", "publication_gate": True,
            "source_gem_gate": True, "linkage_check": "not_run", "mode": "full",
        }.items():
            with self.subTest(key=key):
                result = self.verify(dict(self.report, **{key: value}))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("core export verification failed:", result.stderr)

    def test_missing_fields_fail_closed(self):
        for key in self.report:
            with self.subTest(key=key):
                report = self.report.copy()
                del report[key]
                self.assertNotEqual(self.verify(report).returncode, 0)

    def test_matching_report_cannot_authorize_wrong_archive(self):
        changes = {
            "package": 'spec.name = "unrelated_host_package"',
            "platform": 'spec.platform = Gem::Platform::RUBY',
            "ruby_requirement": 'spec.required_ruby_version = Gem::Requirement.new(">= 0")',
            "extra_file": 'File.write("extra.rb", ""); spec.files += ["extra.rb"]',
            "build_step": 'File.write("extconf.rb", ""); spec.extensions = ["extconf.rb"]',
            "executable": 'FileUtils.mkdir_p("bin"); File.write("bin/extra", "#!/usr/bin/env ruby\\n"); spec.executables = ["extra"]',
        }
        for name, change in changes.items():
            with self.subTest(name=name):
                directory = self.directory / name
                directory.mkdir()
                for file in self.report["files"]:
                    target = directory / file
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(self.directory / file, target)
                build = '''
require "rubygems/package"
require "json"
require "fileutils"
spec = Gem::Package.new(ARGV.fetch(0)).spec
''' + change + '''
artifact = Gem::Package.build(spec)
puts JSON.generate(artifact: artifact, package: spec.name, version: spec.version.to_s,
  platform: spec.platform.to_s, required_ruby_version: spec.required_ruby_version.to_s,
  files: Gem::Package.new(artifact).contents.sort)
'''
                result = subprocess.run(["ruby", "-e", build, str(self.artifact)],
                                        cwd=directory, text=True, capture_output=True, check=True)
                metadata = json.loads(result.stdout.splitlines()[-1])
                artifact = directory / metadata["artifact"]
                report = dict(self.report, **metadata)
                report["sha256"] = hashlib.sha256(artifact.read_bytes()).hexdigest()
                result = self.verify(report, artifact)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("core export verification failed:", result.stderr)
