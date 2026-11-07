#!/usr/bin/env python3
"""Transport checks; requires the isolated Ruby artifact environment variables."""

import base64
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "workspace-scripts/typed_ruby_benchmark"


class RubyBenchmarkTest(unittest.TestCase):
    """Coordinate RubyBenchmarkTest behavior."""

    def run_driver(self, args, **kwargs):
        # A parent Bundler 4 environment must not leak into the candidate Ruby.
        """Run driver."""
        env = dict(kwargs.pop("env", os.environ))
        env.update(
            BUNDLER_SETUP="/nonexistent/parent/bundler/setup",
            RUBYOPT="-r/nonexistent/parent/setup",
            RUBYLIB="/nonexistent",
            BUNDLE_GEMFILE="/nonexistent/parent/Gemfile",
        )
        return subprocess.run(
            [str(DRIVER), *args], capture_output=True, text=True, timeout=30, env=env, check=False, **kwargs
        )

    def test_session_native_merges_and_recovery(self):
        """Handle test session native merges and recovery."""
        cases = [
            (["a: 1\nb: 1\n", "a: 2\nb: 1\n", "a: 1\nb: 2\n"], 0, b"a: 2\nb: 2\n"),
            (["a: 1\n", "a: [\n", "a: 2\n"], 2, b""),
            (["a: 1\n", "a: 2\n", "a: 3\n"], 1, b""),
            (["a: 1\n"] * 3, 0, b"a: 1\n"),
        ]
        requests = [
            {
                "schema_version": "structuredmerge.benchmark.adapter-request/v1",
                "request_id": str(index),
                "operation": "merge3",
                "selector": {"family": "yaml", "dialect": "yaml"},
                "sources": {
                    role: base64.b64encode(text.encode()).decode()
                    for role, text in zip(["base", "ours", "theirs"], texts, strict=False)
                },
            }
            for index, (texts, _, _) in enumerate(cases)
        ]
        result = self.run_driver(["benchmark-provider-session"], input="\n".join(map(json.dumps, requests)) + "\n")
        if not (result.returncode == 0):
            raise AssertionError(result.stderr)
        responses = [json.loads(line) for line in result.stdout.splitlines()]
        if not (len(responses) == len(cases)):
            raise AssertionError("validation failed")
        if not (len({response["process_id"] for response in responses}) == 1):
            raise AssertionError("validation failed")
        for index, (case, response) in enumerate(zip(cases, responses, strict=False)):
            if not (response["request_id"] == str(index)):
                raise AssertionError("validation failed")
            if not (response["status"] == case[1]):
                raise AssertionError(response)
            if not (base64.b64decode(response["output_base64"]) == case[2]):
                raise AssertionError("validation failed")
            if not (response["result"]["provider_id"] == "kernel.yaml"):
                raise AssertionError("validation failed")
            if not (response["result"]["profile_id"] == "kernel.yaml.native_mapping.v1"):
                raise AssertionError("validation failed")
        if not (responses[1]["result"]["diagnostics"][0]["category"] == "parse_error"):
            raise AssertionError("validation failed")

    def test_cold_error_preserves_ours_and_clean_merge_writes_kernel_output(self):
        """Handle test cold error preserves ours and clean merge writes kernel output."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            env = dict(os.environ, AST_MERGE_FAMILY="yaml", AST_MERGE_DIALECT="yaml")
            for role, text in {"base": "a: 1\n", "ours": "a: [\n", "theirs": "a: 2\n"}.items():
                (path / role).write_bytes(text.encode())
            args = ["base", "ours", "theirs", "file.yaml", "7"]
            result = self.run_driver(args, cwd=path, env=env)
            if not (result.returncode == 2):
                raise AssertionError(result.stderr)
            if not (result.stderr.startswith("typed-core: parse_error:")):
                raise AssertionError(result.stderr)
            if not ((path / "ours").read_bytes() == b"a: [\n"):
                raise AssertionError("validation failed")
            (path / "ours").write_bytes(b"a: 1\n")
            result = self.run_driver(args, cwd=path, env=env)
            if not (result.returncode == 0):
                raise AssertionError(result.stderr)
            if not ((path / "ours").read_bytes() == b"a: 2\n"):
                raise AssertionError("validation failed")

    def test_unrecognized_family_does_not_fall_back(self):
        """Handle test unrecognized family does not fall back."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            for role in ["base", "ours", "theirs"]:
                (path / role).write_bytes(b"{}")
            result = self.run_driver(
                ["base", "ours", "theirs", "file.json", "7"],
                cwd=path,
                env=dict(os.environ, AST_MERGE_FAMILY="json", AST_MERGE_DIALECT="json"),
            )
            if not (result.returncode == 2):
                raise AssertionError("validation failed")
            if "unsupported" not in result.stderr:
                raise AssertionError("validation failed")
            if not ((path / "ours").read_bytes() == b"{}"):
                raise AssertionError("validation failed")

    def test_missing_installed_gem_is_error_not_conflict(self):
        """Handle test missing installed gem is error not conflict."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            result = self.run_driver(
                ["benchmark-provider-session"],
                input="",
                env=dict(os.environ, STRUCTUREDMERGE_BENCHMARK_GEM_HOME=directory),
            )
            if not (result.returncode == 2):
                raise AssertionError(result.stderr)
            if "benchmark setup:" not in result.stderr:
                raise AssertionError("validation failed")


if __name__ == "__main__":
    unittest.main()
