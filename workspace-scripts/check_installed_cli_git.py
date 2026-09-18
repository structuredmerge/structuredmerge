#!/usr/bin/env python3
"""Run canonical JSON fixtures through real Git merges and an installed CLI."""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import tempfile


def require(condition, detail):
    if not condition:
        raise AssertionError(detail)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--typed", action="store_true", help="exercise the explicit typed merge lane")
    parser.add_argument("--grammar-library", type=Path, help="existing JSON grammar; typed mode never downloads")
    args = parser.parse_args()
    if args.typed:
        if not args.grammar_library:
            parser.error("--typed requires --grammar-library")
        from typed_cli_git import run
        return run(args.binary, args.fixtures, args.grammar_library)
    if args.grammar_library:
        parser.error("--grammar-library requires --typed")
    binary = args.binary.resolve(strict=True)
    fixture = args.fixtures.resolve(strict=True)
    from typed_cli_git import command, digest, CAPTURE_LIMIT
    require(os.name == "posix", "installed Git gate currently requires POSIX resource limits")
    require(binary.is_file() and binary.stat().st_size <= 128 * 1024**2, "binary exceeds 128 MiB")
    require(fixture.is_file() and fixture.stat().st_size <= 1024**2, "fixture exceeds 1 MiB")
    cases = json.loads(fixture.read_text())["cases"]
    require(0 < len(cases) <= 100, "fixture suite must contain 1..100 cases")
    root = Path(__file__).resolve().parent.parent
    (root / "tmp").mkdir(exist_ok=True)
    require(shutil.disk_usage(root).free >= 21 * 1024**3, "requires 20 GiB reserve plus 1 GiB job budget")
    stage = Path(tempfile.mkdtemp(prefix="installed-cli-git-", dir=root / "tmp"))
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_TERMINAL_PROMPT="0", GIT_EDITOR="true", LC_ALL="C")
    env.setdefault("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", str(root / "tmp/typed-tslp-cache"))
    results = []
    for index, case in enumerate(cases):
        repo = stage / f"work-{index}"
        repo.mkdir()
        evidence = stage / str(index)
        evidence.mkdir()
        env["TMPDIR"] = str(repo)

        def git(*arguments, check=True):
            output = command(["git", *arguments], cwd=repo, env=env, timeout=30)
            if check and output.returncode:
                raise RuntimeError((arguments, output.returncode, output.stderr.decode()))
            return output

        try:
            git("init", "--template=", "-b", "main")
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
            (evidence / "merge-stdout").write_bytes(merged.stdout)
            (evidence / "merge-stderr").write_bytes(merged.stderr)
            report_path = repo / ".git/driver-report.json"
            require(report_path.stat().st_size < CAPTURE_LIMIT, "driver report exceeds 1 MiB")
            report_bytes = report_path.read_bytes()
            (evidence / "driver-report.json").write_bytes(report_bytes)
            report = json.loads(report_bytes)
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
        finally:
            # These are this invocation's disposable repositories only. Preserve
            # compact evidence separately even when assertions/report writes fail.
            shutil.rmtree(repo)
    report = {"binary": str(binary), "sha256": digest(binary),
              "fixture": str(fixture), "fixture_sha256": digest(fixture),
              "results": results, "publication_gate": False, "default_approved": False}
    (stage / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    print(f"Evidence: {stage}")
    return 0 if results and all(result["passed"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
