"""Typed mode of check_installed_cli_git; local POSIX evidence, not publication."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import socket
import subprocess
import tempfile

from check_installed_cli_git import require


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def validate_cases(cases):
    require(cases["schema"] == "structuredmerge.cli-typed-git-cases/v1", "wrong fixture schema")
    require(0 < len(cases["cases"]) <= 100, "fixture suite must contain 1..100 cases")
    ids = set()
    for case in cases["cases"]:
        require(isinstance(case["id"], str) and case["id"] and case["id"] not in ids, "invalid/duplicate case id")
        ids.add(case["id"])
        require(case.get("conflict_policy", "leave-ours") in ("leave-ours", "write"), "invalid conflict policy")
        require(type(case.get("cold", False)) is bool, "cold must be boolean")
        for role in ("base", "ours", "theirs"):
            require(isinstance(case[role], str) and len(case[role].encode()) <= 16384, "source fixture exceeds 16 KiB")
        expected = case["expected"]
        require((expected["git_exit"], expected["driver_exit"], expected["outcome"]) in
                ((0, 0, "changed"), (0, 0, "clean"), (1, 1, "conflict"), (1, 2, "error")), "invalid exit/outcome expectation")
        require("json" in expected or expected.get("preserve_ours") is True or
                expected.get("provider_conflicted_output") is True, "output assertion required")


def command(argv, cwd, env, data=None):
    with subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(data, timeout=20)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
            raise RuntimeError("process exceeded 20-second gate deadline")
        finally:
            # Retire descendants even if the observed leader already exited.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    return subprocess.CompletedProcess(argv, process.returncode, stdout, stderr)


def import_history(git, case, path):
    """Create disposable fixture history without user signing/configuration."""
    stream = bytearray()
    for mark, role, parent in [(1, "base", None), (2, "ours", 1), (3, "theirs", 1)]:
        content = case[role].encode("utf-8")
        stream.extend(f"commit refs/heads/{role}\nmark :{mark}\n".encode())
        stream.extend(b"committer CLI fixture <fixture@example.invalid> 1000000000 +0000\ndata 7\nfixture\n")
        if parent:
            stream.extend(f"from :{parent}\n".encode())
        # Git fast-import accepts unquoted UTF-8 paths (including spaces).
        stream.extend(f"M 100644 inline {path}\ndata {len(content)}\n".encode())
        stream.extend(content + b"\n\n")
    git("fast-import", "--quiet", data=bytes(stream))
    git("switch", "ours")


def check_case(binary, grammar, case, work, evidence):
    repo = work / "repository"
    repo.mkdir()
    libs, cache = work / "libs", work / "cache"
    libs.mkdir()
    cache.mkdir()
    if not case.get("cold", False):
        shutil.copy2(grammar, libs / grammar.name)
    # Also exercise shell quoting of the executable's installation path.
    installed = work / "installed ' tools"
    installed.mkdir()
    executable = installed / binary.name
    shutil.copy2(binary, executable)
    path = "document '雪.txt"
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_TERMINAL_PROMPT="0", GIT_EDITOR="true", LC_ALL="C",
               TMPDIR=str(work), TREE_SITTER_LANGUAGE_PACK_LIBS_DIR=str(libs),
               TREE_HAVER_LANGUAGE_PACK_CACHE_DIR=str(cache),
               TREE_SITTER_LANGUAGE_PACK_CACHE_DIR=str(cache))
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.setblocking(False)
        proxy = f"http://127.0.0.1:{listener.getsockname()[1]}"
        for key in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
            env[key] = proxy
        env.pop("NO_PROXY", None)
        env.pop("no_proxy", None)

        def git(*arguments, check=True, data=None):
            result = command(["git", *arguments], repo, env, data)
            if check:
                require(result.returncode == 0, (arguments, result.stderr.decode(errors="replace")))
            return result

        git("init", "--template=", "-b", "unused")
        git("config", "core.hooksPath", str(work / "no-hooks"))
        git("config", "user.name", "CLI fixture")
        git("config", "user.email", "fixture@example.invalid")
        import_history(git, case, path)
        (repo / ".git/info").mkdir(exist_ok=True)
        (repo / ".git/info/attributes").write_text("*.txt merge=typed-test\n")
        driver = (shlex.quote(str(executable)) + " merge-driver --provider kernel.git.json "
                  "--backend kernel.tslp.json --profile kernel.git.json.v1 "
                  "--report .git/driver-report.json "
                  + ("--conflict-policy write " if case.get("conflict_policy") == "write" else "")
                  + '"%O" "%A" "%B" %P')
        git("config", "merge.typed-test.driver", driver)
        before_head = git("rev-parse", "HEAD").stdout
        merged = git("merge", "--no-commit", "--no-ff", "theirs", check=False)
        for name, content in [("merge.stdout", merged.stdout), ("merge.stderr", merged.stderr)]:
            (evidence / name).write_bytes(content)
        report_bytes = (repo / ".git/driver-report.json").read_bytes()
        (evidence / "driver-report.json").write_bytes(report_bytes)
        report = json.loads(report_bytes)
        expected = case["expected"]
        require(merged.returncode == expected["git_exit"], merged.stderr.decode(errors="replace"))
        require(report["schema"] == "structuredmerge.cli-report/v1", report)
        require(report["exit_code"] == expected["driver_exit"], report)
        require(report["outcome"] == expected["outcome"], report)
        require(report["output_commit_verified"] is False, report)
        require(report["cli"]["executable"] == binary.name, report)
        result = report["operation_result"]
        require(result["provider"]["provider_id"] == "kernel.git.json", result)
        require(result["operation"] == "merge3", result)
        require(result["profile"]["profile_id"] == "kernel.git.json.v1", result)
        require(result["request_forwarding"]["path_name"] == path, result)
        require(not result["fallbacks"], "unexpected fallback")
        if expected["outcome"] == "conflict":
            require(result["ok"] is False and any(
                conflict.get("resolution", {}).get("status") == "unresolved"
                for conflict in result["conflicts"]), "missing typed unresolved-conflict evidence")
        actual = (repo / path).read_bytes()
        (evidence / "actual-source").write_bytes(actual)
        if "json" in expected:
            require(json.loads(actual) == expected["json"], actual)
            require(result["ok"] is True and not result["conflicts"], result)
        if expected.get("preserve_ours"):
            require(actual == case["ours"].encode(), actual)
        if expected.get("provider_conflicted_output"):
            require(actual == result["conflicted_output"].encode(), actual)
            require(b"<<<<<<<" in actual and b">>>>>>>" in actual, actual)
        if expected["outcome"] == "error":
            require(result["ok"] is False and not result["conflicts"], result)
        index = git("ls-files", "-u", "--", path).stdout
        (evidence / "unmerged-index").write_bytes(index)
        if expected["git_exit"] == 0:
            require(not index, index)
            require(git("show", f":0:{path}").stdout == actual, "clean index differs from worktree")
        else:
            require(len(index.splitlines()) == 3, index)
            for stage, role in enumerate(("base", "ours", "theirs"), 1):
                require(git("show", f":{stage}:{path}").stdout == case[role].encode(), role)
        require(git("rev-parse", "HEAD").stdout == before_head, "no-commit changed HEAD")
        for role in ("base", "ours", "theirs"):
            require(git("show", f"{role}:{path}").stdout == case[role].encode(), role)
        diff_verified = False
        if expected["git_exit"] == 0:
            # Use Git's actual external-diff protocol, not synthetic positional
            # arguments. Git controls its temporary source files and hashes.
            (repo / ".git/info/attributes").write_text("*.txt merge=typed-test diff=typed-test\n")
            git("config", "diff.typed-test.command", shlex.quote(str(executable)) +
                " diff-driver --provider kernel.json --backend kernel.tslp.json "
                "--profile kernel.json.nested.v1 --json")
            compared = git("diff", "--ext-diff", "base", "ours", "--", path)
            (evidence / "diff-report.json").write_bytes(compared.stdout)
            diff = json.loads(compared.stdout)
            require(diff["command"] == "diff-driver" and diff["outcome"] == "changed", diff)
            require(diff["operation_result"]["operation"] == "diff2", diff)
            require(diff["operation_result"]["provider"]["provider_id"] == "kernel.json", diff)
            require(diff["operation_result"]["request_forwarding"]["path_name"] == path, diff)
            require(diff["operation_result"]["diff"]["change_ids"], diff)
            require((repo / path).read_bytes() == actual, "external diff changed worktree")
            require(git("show", f":0:{path}").stdout == actual, "external diff changed index")
            require(git("rev-parse", "HEAD").stdout == before_head, "external diff changed HEAD")
            diff_verified = True
        require(not list(cache.iterdir()), "grammar acquisition cache was modified")
        try:
            connection, _ = listener.accept()
        except BlockingIOError:
            pass
        else:
            connection.close()
            raise AssertionError("unexpected proxy connection")
        require(not list(repo.glob(".smorg-write-*")), "staging debris")
        return {"git_exit": merged.returncode, "driver_exit": report["exit_code"], "outcome": report["outcome"],
                "git_diff_verified": diff_verified}


def run(binary, fixture, grammar):
    require(os.name == "posix", "typed Git gate currently requires POSIX process groups")
    binary, fixture, grammar = [path.resolve(strict=True) for path in (binary, fixture, grammar)]
    require(fixture.stat().st_size <= 1024**2, "fixture file exceeds 1 MiB")
    for artifact in (binary, grammar):
        require(artifact.is_file() and artifact.stat().st_size <= 128 * 1024**2,
                "artifact must be a regular file of at most 128 MiB")
    cases = json.loads(fixture.read_text())
    validate_cases(cases)
    root = Path(__file__).resolve().parent.parent / "tmp"
    root.mkdir(exist_ok=True)
    require(shutil.disk_usage(root).free >= 21 * 1024**3, "requires 20 GiB reserve plus 1 GiB job budget")
    stage = Path(tempfile.mkdtemp(prefix="typed-cli-git-", dir=root))
    report = {"schema": "structuredmerge.typed-cli-git-gate/v1", "binary": str(binary),
              "binary_sha256": digest(binary), "fixture_sha256": digest(fixture),
              "grammar_sha256": digest(grammar), "results": [], "publication_gate": False,
              "full_cli_conformance": False, "default_approved": False}
    report["git_version"] = subprocess.run(["git", "--version"], capture_output=True,
                                           text=True, check=True, timeout=10).stdout.strip()
    try:
        for index, case in enumerate(cases["cases"]):
            evidence = stage / str(index)
            evidence.mkdir()
            record = {"case": case["id"], "passed": False}
            try:
                require(shutil.disk_usage(root).free >= 21 * 1024**3, "disk reserve reached")
                with tempfile.TemporaryDirectory(prefix="work-", dir=stage) as work:
                    record.update(check_case(binary, grammar, case, Path(work), evidence))
                record["passed"] = True
            except Exception as error:
                record["error"] = str(error)
            report["results"].append(record)
    finally:
        # Disposable repos/install copies are gone even if report writing fails.
        (stage / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    print(f"Evidence: {stage}")
    return 0 if all(case["passed"] for case in report["results"]) else 1
