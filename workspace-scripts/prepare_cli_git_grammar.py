#!/usr/bin/env python3
"""Bounded cold-cache JSON provisioning before the strict installed Git gate.

Uses the installed CLI and normal TSLP loader, not a replacement downloader.
The resulting cache is retained for the job; callers own its final cleanup.
"""
import argparse
import json
import os
from pathlib import Path
import tempfile

from check_core_python_source import run
from typed_cli_git import CAPTURE_LIMIT, digest

ROOT = Path(__file__).resolve().parents[1]
PROVISION_BUDGET = 256 * 1024**2


def prepare(binary, destination):
    binary = binary.resolve(strict=True)
    scratch_root = (ROOT / "tmp").resolve()
    scratch_root.mkdir(exist_ok=True)
    destination = destination.absolute()
    # Never overwrite a user's cache or publish outside repository-local scratch.
    if destination.exists() or destination.is_symlink():
        raise ValueError("provisioning requires a new cache destination")
    if destination.parent.resolve() != scratch_root:
        raise ValueError("cache destination must be directly inside repository tmp")
    if not binary.is_file() or binary.stat().st_size > 128 * 1024**2:
        raise ValueError("binary exceeds 128 MiB or is not a file")
    evidence = Path(tempfile.mkdtemp(prefix="cli-provision-evidence-", dir=scratch_root))
    report = {"binary": str(binary), "sha256": digest(binary), "cold_cache": True,
              "work_budget_bytes": PROVISION_BUDGET, "passed": False,
              "publication_gate": False, "default_approved": False}
    try:
        with tempfile.TemporaryDirectory(prefix="cli-provision-work-", dir=scratch_root) as temporary:
            work = Path(temporary)
            cache = work / "cache"
            cache.mkdir()
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith(("GIT_", "TREE_HAVER_", "TREE_SITTER_LANGUAGE_PACK_"))}
            env.update(TMPDIR=str(work), TREE_HAVER_LANGUAGE_PACK_CACHE_DIR=str(cache),
                       TREE_SITTER_LANGUAGE_PACK_CACHE_DIR=str(cache),
                       GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
                       GIT_TERMINAL_PROMPT="0", LC_ALL="C")
            for name, value in (("base", {"base": 0}), ("ours", {"base": 0, "ours": 1}),
                                ("theirs", {"base": 0, "theirs": 2})):
                (work / name).write_text(json.dumps(value))
            # A nontrivial strict merge forces parsing; identical inputs could
            # short-circuit without ever acquiring or loading the JSON grammar.
            run([str(binary), "merge-driver", "--strict", "--report", "driver.json",
                 "base", "ours", "theirs", "provision.json"], work, env, evidence,
                "provision", work, timeout=120, work_budget=PROVISION_BUDGET)
            for name in ("ours", "driver.json"):
                if (work / name).stat().st_size > CAPTURE_LIMIT:
                    raise ValueError("provisioning result exceeds capture budget")
            if json.loads((work / "ours").read_text()) != {"base": 0, "ours": 1, "theirs": 2}:
                raise ValueError("provisioning merge produced wrong result")
            driver = json.loads((work / "driver.json").read_text())
            if driver.get("exit_code") != 0 or not any(cache.rglob("manifest.json")):
                raise ValueError("provisioning did not verify a successful cached parse")
            (evidence / "driver.json").write_text(json.dumps(driver, indent=2) + "\n")
            report["cache_bytes"] = sum(path.stat().st_size for path in cache.rglob("*") if path.is_file())
            if destination.exists() or destination.is_symlink():
                raise ValueError("cache destination appeared during provisioning")
            cache.rename(destination)
            report.update(passed=True, cache=str(destination))
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        (evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"Provisioning evidence: {evidence}", flush=True)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--cache", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.binary, args.cache), indent=2))
