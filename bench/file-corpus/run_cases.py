#!/usr/bin/env python3
"""Resumable StructuredMerge/Git comparison over a JSONL case manifest."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def normalize(data: bytes) -> bytes:
    return b"\n".join(line.rstrip() for line in data.splitlines()) + (b"\n" if data else b"")


def save_artifact(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


def save_case_artifacts(case: dict, artifacts_dir: Path, files: dict[str, bytes | None]) -> dict:
    key = hashlib.sha256(case["id"].encode("utf-8")).hexdigest()
    saved = {}
    for name, data in files.items():
        if data is None:
            saved[f"{name}_path"] = None
            continue
        relative = Path(key[:2]) / key / f"{name}.bin"
        save_artifact(artifacts_dir / relative, data)
        saved[f"{name}_path"] = relative.as_posix()
        saved[f"{name}_sha256"] = hashlib.sha256(data).hexdigest()
        saved[f"{name}_bytes"] = len(data)
    saved["artifact_key"] = key
    return saved


def run_case(case: dict, smorg: str, timeout: int, max_bytes: int, artifacts_dir: Path) -> dict:
    started = time.monotonic()
    base, left, right, expected = [Path(case[k]) for k in ("base", "left", "right", "expected")]
    try:
        for path in (base, left, right, expected):
            if path.stat().st_size > max_bytes:
                return {"id": case["id"], "repository": case["repository"], "extension": case["extension"], "status": "oversize", "seconds": 0.0}
        with tempfile.TemporaryDirectory(prefix="smorg-case-") as temp:
            directory = Path(temp)
            candidate = directory / "candidate"
            report = directory / "report.json"
            sm_started = time.monotonic()
            sm_timeout = False
            sm_stdout = b""
            sm_stderr = b""
            sm_exit = None
            try:
                sm = subprocess.run(
                    [smorg, "merge-driver", "--ancestor", str(base), "--current", str(left),
                     "--other", str(right), "--path-name", case["path_name"],
                     "--output", str(candidate), "--report", str(report)],
                    capture_output=True, timeout=timeout, check=False,
                )
                sm_stdout, sm_stderr, sm_exit = sm.stdout, sm.stderr, sm.returncode
            except subprocess.TimeoutExpired as error:
                sm_timeout = True
                sm_stdout = error.stdout or b""
                sm_stderr = error.stderr or b""
            sm_seconds = time.monotonic() - sm_started
            sm_output = candidate.read_bytes() if candidate.exists() else sm_stdout

            git_started = time.monotonic()
            git_timeout = False
            git_stdout = b""
            git_stderr = b""
            git_exit = None
            try:
                git = subprocess.run(
                    ["git", "merge-file", "--stdout", "--diff3", str(left), str(base), str(right)],
                    capture_output=True, timeout=timeout, check=False,
                )
                git_stdout, git_stderr, git_exit = git.stdout, git.stderr, git.returncode
            except subprocess.TimeoutExpired as error:
                git_timeout = True
                git_stdout = error.stdout or b""
                git_stderr = error.stderr or b""
            git_seconds = time.monotonic() - git_started
            expected_bytes = expected.read_bytes()
            sm_markers = b"<<<<<<<" in sm_output or b"=======" in sm_output
            git_markers = b"<<<<<<<" in git_stdout or b"=======" in git_stdout
            files = {
                "structuredmerge_output": sm_output,
                "structuredmerge_report": report.read_bytes() if report.exists() else None,
                "structuredmerge_stderr": sm_stderr,
                "git_output": git_stdout,
                "git_stderr": git_stderr,
            }
            artifacts = save_case_artifacts(case, artifacts_dir, files)
            return {
                "id": case["id"], "repository": case["repository"], "extension": case["extension"],
                "path_name": case["path_name"],
                "status": "timeout" if sm_timeout or git_timeout else "completed",
                "smorg_exit": sm_exit, "smorg_timed_out": sm_timeout,
                "smorg_seconds": round(sm_seconds, 6),
                "smorg_markers": sm_markers,
                "smorg_exact_expected": sm_exit == 0 and sm_output == expected_bytes,
                "smorg_normalized_expected": sm_exit == 0 and normalize(sm_output) == normalize(expected_bytes),
                "smorg_output_sha256": hashlib.sha256(sm_output).hexdigest(),
                "git_exit": git_exit, "git_timed_out": git_timeout,
                "git_seconds": round(git_seconds, 6),
                "git_markers": git_markers,
                "git_exact_expected": git_exit == 0 and git_stdout == expected_bytes,
                "git_normalized_expected": git_exit == 0 and normalize(git_stdout) == normalize(expected_bytes),
                "artifacts": artifacts,
                "total_seconds": round(time.monotonic() - started, 6),
            }
    except (OSError, UnicodeError, ValueError) as error:
        return {"id": case["id"], "repository": case["repository"], "extension": case["extension"], "status": "error", "error_type": type(error).__name__, "seconds": round(time.monotonic() - started, 6)}


def repair_partial_jsonl(path: Path, artifacts_dir: Path) -> set[str]:
    if not path.exists():
        return set()
    data = path.read_bytes()
    last_newline = data.rfind(b"\n")
    if last_newline + 1 != len(data):
        with path.open("r+b") as stream:
            stream.truncate(last_newline + 1)
        data = data[: last_newline + 1]
    done = set()
    valid_lines = []
    legacy_lines = []
    artifacts_root = artifacts_dir.resolve()
    for line in data.splitlines():
        try:
            row = json.loads(line)
        except (json.JSONDecodeError, KeyError):
            continue
        if row.get("status") == "oversize":
            valid_lines.append(line + b"\n")
            done.add(row["id"])
            continue
        artifacts = row.get("artifacts", {})
        output_paths = [artifacts.get("structuredmerge_output_path"), artifacts.get("git_output_path")]
        try:
            resolved = [(artifacts_root / relative).resolve() for relative in output_paths if relative]
            if len(resolved) == 2 and all(path.is_relative_to(artifacts_root) and path.is_file() for path in resolved):
                valid_lines.append(line + b"\n")
                done.add(row["id"])
            else:
                legacy_lines.append(line + b"\n")
        except (OSError, TypeError, ValueError):
            legacy_lines.append(line + b"\n")
    if legacy_lines:
        legacy_path = path.with_name(path.name + ".legacy.jsonl")
        with legacy_path.open("ab") as stream:
            stream.writelines(legacy_lines)
    if valid_lines:
        temporary = path.with_name(path.name + ".tmp")
        temporary.write_bytes(b"".join(valid_lines))
        os.replace(temporary, path)
    elif path.exists():
        path.unlink()
    return done


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--shard-index", type=int, required=True)
    parser.add_argument("--shards", type=int, required=True)
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--timeout", type=int, default=30)
    parser.add_argument("--max-bytes", type=int, default=8 * 1024 * 1024)
    parser.add_argument("--smorg", default="smorg-rs")
    parser.add_argument("--artifacts-dir", type=Path)
    args = parser.parse_args()
    if args.shards < 1 or not 0 <= args.shard_index < args.shards:
        parser.error("shard-index must be in [0, shards)")
    if args.workers < 1:
        parser.error("workers must be positive")
    if args.timeout < 1 or args.max_bytes < 1:
        parser.error("timeout and max-bytes must be positive")

    artifacts_dir = args.artifacts_dir or args.results.with_name(args.results.stem + "-artifacts")
    artifacts_dir.mkdir(parents=True, exist_ok=True)
    done = repair_partial_jsonl(args.results, artifacts_dir)
    cases = []
    with args.manifest.open(encoding="utf-8") as stream:
        for line in stream:
            case = json.loads(line)
            if case["shard"] == args.shard_index and case["id"] not in done:
                cases.append(case)
    args.results.parent.mkdir(parents=True, exist_ok=True)

    with args.results.open("a", encoding="utf-8") as output:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futures = [pool.submit(run_case, case, args.smorg, args.timeout, args.max_bytes, artifacts_dir) for case in cases]
            for future in as_completed(futures):
                output.write(json.dumps(future.result(), ensure_ascii=False) + "\n")
                output.flush()
                os.fsync(output.fileno())
    print(f"Completed shard {args.shard_index}/{args.shards}: {len(cases)} cases")


if __name__ == "__main__":
    main()
