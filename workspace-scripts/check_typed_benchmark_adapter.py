#!/usr/bin/env python3
"""Run with the installed core's Python; test transport without replacing the benchmark."""
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "workspace-scripts/typed_core_benchmark.py"


class TypedBenchmarkProtocolTest(unittest.TestCase):
    def run_driver(self, args, **kwargs):
        return subprocess.run([sys.executable, str(DRIVER), *args],
            capture_output=True, text=True, timeout=30, **kwargs)

    def request(self, operation, texts, identity):
        roles = {"merge2": ["incoming", "current"], "merge3": ["base", "ours", "theirs"]}[operation]
        return {"schema_version": "structuredmerge.benchmark.adapter-request/v1",
                "request_id": identity, "operation": operation,
                "selector": {"family": "json", "dialect": "json"},
                "sources": {role: base64.b64encode(text.encode()).decode()
                            for role, text in zip(roles, texts)}}

    def test_session_recovers_after_invalid_input_and_reuses_process(self):
        good = self.request("merge2", ['{"add":1}', '{"keep":2}'], "good")
        invalid = self.request("merge2", ["{}", "{}"], "invalid")
        invalid["sources"]["incoming"] = "!"
        conflict = self.request("merge3", ['{"x":0}', '{"x":1}', '{"x":2}'], "conflict")
        requests = [invalid, good, conflict, good]
        process = self.run_driver(["benchmark-provider-session"], input="\n".join(map(json.dumps, requests)) + "\n")
        self.assertEqual(process.returncode, 0, process.stderr)
        results = [json.loads(line) for line in process.stdout.splitlines()]
        self.assertEqual([r["status"] for r in results], [2, 0, 1, 0])
        self.assertEqual([r["request_id"] for r in results], ["invalid", "good", "conflict", "good"])
        self.assertEqual(len({r["process_id"] for r in results}), 1)
        self.assertEqual(json.loads(base64.b64decode(results[1]["output_base64"])), {"add": 1, "keep": 2})
        self.assertIn(b"<<<<<<<", base64.b64decode(results[2]["output_base64"]))
        self.assertEqual(results[1]["result"]["provider_id"], "kernel.json")
        self.assertEqual(results[2]["result"]["provider_id"], "kernel.git.json")

    def test_cold_malformed_input_preserves_ours_and_reports_parser_category(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            for role, text in {"base": '{}', "ours": '{"x":', "theirs": '{"x":1}'}.items():
                (path / role).write_bytes(text.encode())
            env = dict(os.environ, AST_MERGE_FAMILY="json", AST_MERGE_DIALECT="json")
            result = self.run_driver(["base", "ours", "theirs", "file.json", "7"], cwd=path, env=env)
            self.assertEqual(result.returncode, 2)
            self.assertIn("parse_error:", result.stderr)
            self.assertEqual((path / "ours").read_bytes(), b'{"x":')

    def test_cold_merge2_returns_output_without_modifying_inputs(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "tmp") as directory:
            path = Path(directory)
            (path / "incoming").write_bytes(b'{"add":1}')
            (path / "current").write_bytes(b'{"keep":2}\r\n')
            env = dict(os.environ, AST_MERGE_FAMILY="json", AST_MERGE_DIALECT="json")
            result = self.run_driver(["benchmark-provider-merge2", "incoming", "current", "file.json"], cwd=path, env=env)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(json.loads(result.stdout)["output"]), {"add": 1, "keep": 2})
            self.assertEqual((path / "current").read_bytes(), b'{"keep":2}\r\n')
            self.assertEqual((path / "incoming").read_bytes(), b'{"add":1}')


if __name__ == "__main__":
    unittest.main()
