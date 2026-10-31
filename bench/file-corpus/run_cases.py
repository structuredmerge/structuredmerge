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


def run_case(case: dict, smorg: str, timeout: int, max_bytes: int) -> dict:
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
            sm = subprocess.run(
                [smorg, "merge-driver", "--ancestor", str(base), "--current", str(left),
                 "--other", str(right), "--path-name", case["path_name"],
                 "--output", str(candidate), "--report", str(report)],
                capture_output=True, timeout=timeout, check=False,
            )
            sm_seconds = time.monotonic() - sm_started
            sm_output = candidate.read_bytes() if candidate.exists() else b""

            git_started = time.monotonic()
            git = subprocess.run(
                ["git", "merge-file", "--stdout", "--diff3", str(left), str(base), str(right)],
                capture_output=True, timeout=timeout, check=False,
            )
            git_seconds = time.monotonic() - git_started
            expected_bytes = expected.read_bytes()
            sm_markers = b"<<<<<<<" in sm_output or b"=======" in sm_output
            git_markers = b"<<<<<<<" in git.stdout or b"=======" in git.stdout
            return {
                "id": case["id"], "repository": case["repository"], "extension": case["extension"],
                "path_name": case["path_name"], "status": "completed",
                "smorg_exit": sm.returncode, "smorg_seconds": round(sm_seconds, 6),
                "smorg_markers": sm_markers,
                "smorg_exact_expected": sm.returncode == 0 and sm_output == expected_bytes,
                "smorg_normalized_expected": sm.returncode == 0 and normalize(sm_output) == normalize(expected_bytes),
                "smorg_output_sha256": hashlib.sha256(sm_output).hexdigest(),
                "git_exit": git.returncode, "git_seconds": round(git_seconds, 6),
                "git_markers": git_markers,
                "git_exact_expected": git.returncode == 0 and git.stdout == expected_bytes,
                "git_normalized_expected": git.returncode == 0 and normalize(git.stdout) == normalize(expected_bytes),
                "total_seconds": round(time.monotonic() - started, 6),
            }
    except subprocess.TimeoutExpired:
        return {"id": case["id"], "repository": case["repository"], "extension": case["extension"], "status": "timeout", "seconds": round(time.monotonic() - started, 6)}
    except (OSError, UnicodeError, ValueError) as error:
        return {"id": case["id"], "repository": case["repository"], "extension": case["extension"], "status": "error", "error_type": type(error).__name__, "seconds": round(time.monotonic() - started, 6)}


def repair_partial_jsonl(path: Path) -> set[str]:
    if not path.exists():
        return set()
    data = path.read_bytes()
    last_newline = data.rfind(b"\n")
    if last_newline + 1 != len(data):
        with path.open("r+b") as stream:
            stream.truncate(last_newline + 1)
        data = data[: last_newline + 1]
    done = set()
    for line in data.splitlines():
        try:
            done.add(json.loads(line)["id"])
        except (json.JSONDecodeError, KeyError):
            continue
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
    args = parser.parse_args()
    if args.shards < 1 or not 0 <= args.shard_index < args.shards:
        parser.error("shard-index must be in [0, shards)")
    if args.workers < 1:
        parser.error("workers must be positive")
    if args.timeout < 1 or args.max_bytes < 1:
        parser.error("timeout and max-bytes must be positive")

    done = repair_partial_jsonl(args.results)
    cases = []
    with args.manifest.open(encoding="utf-8") as stream:
        for line in stream:
            case = json.loads(line)
            if case["shard"] == args.shard_index and case["id"] not in done:
                cases.append(case)
    args.results.parent.mkdir(parents=True, exist_ok=True)

    with args.results.open("a", encoding="utf-8") as output:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futures = [pool.submit(run_case, case, args.smorg, args.timeout, args.max_bytes) for case in cases]
            for future in as_completed(futures):
                output.write(json.dumps(future.result(), ensure_ascii=False) + "\n")
                output.flush()
                os.fsync(output.fileno())
    print(f"Completed shard {args.shard_index}/{args.shards}: {len(cases)} cases")


if __name__ == "__main__":
    main()
