#!/usr/bin/env python3
"""Run canonical JSON fixtures through real Git merges and an installed CLI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile


def require(condition, detail):
    if not condition:
        raise AssertionError(detail)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--fixtures", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    fixture = args.fixtures.resolve(strict=True)
    root = Path(__file__).resolve().parent.parent
    (root / "tmp").mkdir(exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix="installed-cli-git-", dir=root / "tmp"))
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_TERMINAL_PROMPT="0", GIT_EDITOR="true", LC_ALL="C")
    env.setdefault("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", str(root / "tmp/typed-tslp-cache"))
    results = []
    for index, case in enumerate(json.loads(fixture.read_text())["cases"]):
        repo = stage / str(index)
        repo.mkdir()

        def git(*arguments, check=True):
            output = subprocess.run(["git", *arguments], cwd=repo, env=env,
                                    capture_output=True, timeout=30, check=False)
            if check and output.returncode:
                raise RuntimeError((arguments, output.returncode, output.stderr.decode()))
            return output

        try:
            git("init", "-b", "main")
            git("config", "user.name", "Installed CLI test")
            git("config", "user.email", "installed-cli@example.invalid")
            git("config", "core.hooksPath", str(stage / "no-hooks"))
            driver = (shlex.quote(str(binary)) + ' merge-driver --strict --report .git/driver-report.json '
                      '"%O" "%A" "%B" %P')  # Git shell-quotes %P itself.
            git("config", "merge.smorg-test.driver", driver)
            (repo / ".gitattributes").write_text("*.json merge=smorg-test smorg.language=json\n")
            path = Path(case["path_name"])
            if path.is_absolute() or ".." in path.parts:
                raise ValueError("fixture path escapes repository")
            source = repo / path
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(case["base_source"].encode())
            git("add", ".")
            git("commit", "-m", "base")
            git("checkout", "-b", "other")
            source.write_bytes(case["theirs_source"].encode())
            git("commit", "-am", "theirs")
            git("checkout", "main")
            source.write_bytes(case["ours_source"].encode())
            git("commit", "-am", "ours")
            merged = git("merge", "--no-edit", "other", check=False)
            (repo / ".git/merge-stdout").write_bytes(merged.stdout)
            (repo / ".git/merge-stderr").write_bytes(merged.stderr)
            report = json.loads((repo / ".git/driver-report.json").read_text())
            require(report["path_name"] == str(path), report)
            expected = case["expected"]
            require(merged.returncode == expected["exit_code"], merged.stderr.decode())
            require(report["exit_code"] == expected["exit_code"], report)
            actual = source.read_bytes()
            if "merged_json" in expected:
                require(json.loads(actual) == expected["merged_json"], actual)
            if "merged_source" in expected:
                require(actual == expected["merged_source"].encode(), actual)
            for needle in expected.get("conflicted_source_contains", []):
                require(needle.encode() in actual, (needle, actual))
            for needle in expected["stderr_contains"]:
                require(needle.encode() in merged.stderr, merged.stderr)
            unmerged = git("ls-files", "-u", "--", str(path)).stdout
            if expected["exit_code"] == 0:
                require(not unmerged, unmerged)
                require(not git("status", "--porcelain").stdout, "dirty clean merge")
                require(git("show", f"HEAD:{path}").stdout == actual, "HEAD differs from worktree")
            else:
                require(len(unmerged.splitlines()) == 3, unmerged)
                for number, role in enumerate(("base", "ours", "theirs"), 1):
                    require(git("show", f":{number}:{path}").stdout == case[f"{role}_source"].encode(),
                            f"unmerged index stage {number} differs from {role}")
            results.append({"case": case["case_id"], "passed": True})
        except Exception as error:
            results.append({"case": case["case_id"], "passed": False, "error": str(error)})
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "fixture": str(fixture), "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
              "results": results, "publication_gate": False, "default_approved": False}
    (stage / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    print(f"Evidence: {stage}")
    return 0 if results and all(result["passed"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
