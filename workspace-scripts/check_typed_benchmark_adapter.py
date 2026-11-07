#!/usr/bin/env python3
"""Run with the installed core's Python; test transport without replacing the benchmark."""

import base64
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "workspace-scripts/typed_core_benchmark.py"


class TypedBenchmarkProtocolTest(unittest.TestCase):
    """Coordinate TypedBenchmarkProtocolTest behavior."""

    def run_driver(self, args, **kwargs):
        """Run driver."""
        return subprocess.run(
            [sys.executable, str(DRIVER), *args], capture_output=True, text=True, timeout=30, check=False, **kwargs
        )

    def request(self, operation, texts, identity):
        """Handle request."""
        roles = {"merge2": ["incoming", "current"], "merge3": ["base", "ours", "theirs"]}[operation]
        return {
            "schema_version": "structuredmerge.benchmark.adapter-request/v1",
            "request_id": identity,
            "operation": operation,
            "selector": {"family": "json", "dialect": "json"},
            "sources": {
                role: base64.b64encode(text.encode()).decode() for role, text in zip(roles, texts, strict=False)
            },
        }

    def test_session_recovers_after_invalid_input_and_reuses_process(self):
        """Handle test session recovers after invalid input and reuses process."""
        good = self.request("merge2", ['{"add":1}', '{"keep":2}'], "good")
        invalid = self.request("merge2", ["{}", "{}"], "invalid")
        invalid["sources"]["incoming"] = "!"
        conflict = self.request("merge3", ['{"x":0}', '{"x":1}', '{"x":2}'], "conflict")
        requests = [invalid, good, conflict, good]
        process = self.run_driver(["benchmark-provider-session"], input="\n".join(map(json.dumps, requests)) + "\n")
        if not (process.returncode == 0):
            raise AssertionError(process.stderr)
        results = [json.loads(line) for line in process.stdout.splitlines()]
        if not ([r["status"] for r in results] == [2, 0, 1, 0]):
            raise AssertionError("validation failed")
        if not ([r["request_id"] for r in results] == ["invalid", "good", "conflict", "good"]):
            raise AssertionError("validation failed")
        if not (len({r["process_id"] for r in results}) == 1):
            raise AssertionError("validation failed")
        if not (json.loads(base64.b64decode(results[1]["output_base64"])) == {"add": 1, "keep": 2}):
            raise AssertionError("validation failed")
        if b"<<<<<<<" not in base64.b64decode(results[2]["output_base64"]):
            raise AssertionError("validation failed")
        if not (results[1]["result"]["provider_id"] == "kernel.json"):
            raise AssertionError("validation failed")
        if not (results[2]["result"]["provider_id"] == "kernel.git.json"):
            raise AssertionError("validation failed")

    def test_cold_malformed_input_preserves_ours_and_reports_parser_category(self):
        """Handle test cold malformed input preserves ours and reports parser category."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            for role, text in {"base": "{}", "ours": '{"x":', "theirs": '{"x":1}'}.items():
                (path / role).write_bytes(text.encode())
            env = dict(os.environ, AST_MERGE_FAMILY="json", AST_MERGE_DIALECT="json")
            result = self.run_driver(["base", "ours", "theirs", "file.json", "7"], cwd=path, env=env)
            if not (result.returncode == 2):
                raise AssertionError("validation failed")
            if not (result.stderr.startswith("typed-core: parse_error:")):
                raise AssertionError(result.stderr)
            if not ((path / "ours").read_bytes() == b'{"x":'):
                raise AssertionError("validation failed")

    def test_cold_merge2_returns_output_without_modifying_inputs(self):
        """Handle test cold merge2 returns output without modifying inputs."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            (path / "incoming").write_bytes(b'{"add":1}')
            (path / "current").write_bytes(b'{"keep":2}\r\n')
            env = dict(os.environ, AST_MERGE_FAMILY="json", AST_MERGE_DIALECT="json")
            result = self.run_driver(
                ["benchmark-provider-merge2", "incoming", "current", "file.json"], cwd=path, env=env
            )
            if not (result.returncode == 0):
                raise AssertionError(result.stderr)
            if not (json.loads(json.loads(result.stdout)["output"]) == {"add": 1, "keep": 2}):
                raise AssertionError("validation failed")
            if not ((path / "current").read_bytes() == b'{"keep":2}\r\n'):
                raise AssertionError("validation failed")
            if not ((path / "incoming").read_bytes() == b'{"add":1}'):
                raise AssertionError("validation failed")

    def test_unsupported_family_is_not_relabelled_as_a_parser_rejection(self):
        """Handle test unsupported family is not relabelled as a parser rejection."""
        request = self.request("merge2", ["{}", "{}"], "unsupported")
        request["selector"]["family"] = "yaml"
        process = self.run_driver(["benchmark-provider-session"], input=json.dumps(request) + "\n")
        if not (process.returncode == 0):
            raise AssertionError(process.stderr)
        response = json.loads(process.stdout)
        if not (response["status"] == 2):
            raise AssertionError("validation failed")
        if not (response["result"] == {}):
            raise AssertionError("validation failed")
        if not (response["output_base64"] == ""):
            raise AssertionError("validation failed")
        if not ("parse_error:" not in response["stderr"]):
            raise AssertionError("validation failed")

    def test_owner_profiles_and_tsx_keep_native_provider_identity(self):
        """Handle test owner profiles and tsx keep native provider identity."""
        cases = [
            ("bash", "bash", "a() { echo one; }\n"),
            ("go", "go", "package main\nfunc a() int { return 1 }\n"),
            ("rust", "rust", "fn a() -> i32 { 1 }\n"),
            ("typescript", "typescript", "function a(): number { return 1; }\n"),
            ("typescript", "tsx", "function View() { return <div />; }\n"),
        ]
        requests = []
        for family, dialect, source in cases:
            request = self.request("merge3", [source] * 3, dialect)
            request["selector"] = {"family": family, "dialect": dialect}
            requests.append(request)
        process = self.run_driver(["benchmark-provider-session"], input="\n".join(map(json.dumps, requests)) + "\n")
        if not (process.returncode == 0):
            raise AssertionError(process.stderr)
        responses = [json.loads(line) for line in process.stdout.splitlines()]
        if not (len(responses) == len(cases)):
            raise AssertionError("validation failed")
        for (family, dialect, source), response in zip(cases, responses, strict=False):
            with self.subTest(dialect=dialect):
                if not (response["status"] == 0):
                    raise AssertionError(response)
                if not (response["result"]["provider_id"] == "kernel." + family):
                    raise AssertionError("validation failed")
                if not (response["result"]["profile_id"] == "kernel." + family + ".owners.v1"):
                    raise AssertionError("validation failed")
                if not (base64.b64decode(response["output_base64"]) == source.encode()):
                    raise AssertionError("validation failed")

    def test_libcst_session_executes_both_merges_and_recovers_from_parse_failure(self):
        """Handle test libcst session executes both merges and recovers from parse failure."""
        cases = [
            ("merge3", ["a = 1\nb = 1\n", "a = 2\nb = 1\n", "a = 1\nb = 2\n"], 0, b"a = 2\nb = 2\n"),
            ("merge3", ["a = 1\n", "a =\n", "a = 2\n"], 2, b""),
            ("merge3", ["a = 1\n", "a = 2\n", "a = 3\n"], 1, b""),
            ("merge2", ["a = 1\nb = 2\n", "a = 9 # keep\n"], 0, b"a = 9 # keep\nb = 2\n"),
        ]
        requests = []
        for index, (operation, texts, _, _) in enumerate(cases):
            request = self.request(operation, texts, str(index))
            request["selector"] = {"family": "python", "dialect": "python"}
            requests.append(request)
        process = self.run_driver(["benchmark-provider-session"], input="\n".join(map(json.dumps, requests)) + "\n")
        if not (process.returncode == 0):
            raise AssertionError(process.stderr)
        responses = [json.loads(line) for line in process.stdout.splitlines()]
        if not (len(responses) == len(cases)):
            raise AssertionError("validation failed")
        if not (len({response["process_id"] for response in responses}) == 1):
            raise AssertionError("validation failed")
        for case, response in zip(cases, responses, strict=False):
            if not (response["status"] == case[2]):
                raise AssertionError(response)
            if not (base64.b64decode(response["output_base64"]) == case[3]):
                raise AssertionError("validation failed")
            if not (response["result"]["provider_id"] == "kernel.python"):
                raise AssertionError("validation failed")
            if not (response["result"]["profile_id"] == "kernel.python.native_declarations.v1"):
                raise AssertionError("validation failed")
        if not (responses[1]["result"]["diagnostics"][0]["category"] == "parse_error"):
            raise AssertionError("validation failed")

    def test_cold_libcst_parse_error_preserves_ours(self):
        """Handle test cold libcst parse error preserves ours."""
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            for role, text in {"base": "a = 1\n", "ours": "a =\n", "theirs": "a = 2\n"}.items():
                (path / role).write_bytes(text.encode())
            env = dict(os.environ, AST_MERGE_FAMILY="python", AST_MERGE_DIALECT="python")
            result = self.run_driver(["base", "ours", "theirs", "file.py", "7"], cwd=path, env=env)
            if not (result.returncode == 2):
                raise AssertionError(result.stderr)
            if not (result.stderr.startswith("typed-core: parse_error:")):
                raise AssertionError(result.stderr)
            if not ((path / "ours").read_bytes() == b"a =\n"):
                raise AssertionError("validation failed")


if __name__ == "__main__":
    unittest.main()
