#!/usr/bin/env python3
"""Build an existing sdist in isolation and verify its installed wheel (POSIX).

Requires Python 3.11+, maturin in this interpreter and cached Cargo dependencies. This is a
local debug source-build gate, not publication or cross-platform approval.
Only pass trusted development sources: Cargo executes their build scripts.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
RESERVE = 30 * 1024**3
BUILD_BUDGET = 8 * 1024**3
ARCHIVE_BUDGET = 64 * 1024**2
LOG_BUDGET = 1024**2


def extract_source(archive, destination):
    """Reject links, traversal, duplicate entries and oversized source exports."""
    if archive.stat().st_size > ARCHIVE_BUDGET:
        raise ValueError("compressed source archive exceeds 64 MiB")
    with tarfile.open(archive, "r:gz") as source:
        members, names, roots, size = [], set(), set(), 0
        for member in source:
            path = PurePosixPath(member.name)
            if (path.is_absolute() or ".." in path.parts or "\\" in member.name
                    or not path.parts or path.as_posix() in names
                    or not (member.isfile() or member.isdir())):
                raise ValueError("unsafe or duplicate source archive entry")
            names.add(path.as_posix())
            roots.add(path.parts[0])
            size += member.size
            members.append(member)
            if size > ARCHIVE_BUDGET or len(members) > 10000:
                raise ValueError("expanded source archive exceeds budget")
        if len(roots) != 1:
            raise ValueError("source archive must have exactly one root")
        # Only ordinary files/directories with validated relative paths survive.
        # Do not restore archive owners, modes or timestamps.
        for member in members:
            path = destination / member.name
            if member.isdir():
                path.mkdir(parents=True, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as incoming, path.open("xb") as outgoing:
                    shutil.copyfileobj(incoming, outgoing)
    root = destination / roots.pop()
    for name in ("pyproject.toml", "Cargo.toml", "Cargo.lock", "LICENSE", "PKG-INFO"):
        if not (root / name).is_file():
            raise ValueError(f"source archive missing {name}")
    if (root / "LICENSE").read_bytes() != (ROOT / "LICENSE").read_bytes():
        raise ValueError("source license differs from authoritative license")
    return root


def check_metadata(metadata, source):
    """Cargo parses manifests; every local package/path must stay in the export."""
    local = []
    for package in metadata["packages"]:
        if package["source"] is not None:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        if not manifest.is_relative_to(source.resolve()):
            raise ValueError("source package escapes archive")
        if "prototype" in package["name"]:
            raise ValueError("prototype package in source archive")
        local.append(package["name"])
        for dependency in package["dependencies"]:
            if dependency.get("path") and not Path(dependency["path"]).resolve().is_relative_to(source.resolve()):
                raise ValueError("source dependency escapes archive")
    if not {"structuredmerge-core", "structuredmerge-core-py"}.issubset(local):
        raise ValueError("typed facade and Python binding must be in source archive")
    return sorted(local)


def child_limits():
    import resource
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_FSIZE, (512 * 1024**2, 512 * 1024**2))


def verify_lock_pruning(before, after):
    """Cargo may prune the exported workspace, never introduce/change a pin."""
    def packages(document):
        return {(p["name"], p["version"], p.get("source")): p
                for p in tomllib.loads(document)["package"]}
    old, new = packages(before), packages(after)
    def edges(package, index):
        result = set()
        for dependency in package.get("dependencies", []):
            parts = dependency.split(" ", 2)
            matches = [key for key in index if key[0] == parts[0]
                       and (len(parts) < 2 or key[1] == parts[1])
                       and (len(parts) < 3 or key[2] == parts[2].removeprefix("(").removesuffix(")"))]
            if len(matches) != 1:
                raise ValueError("ambiguous or missing Cargo lock dependency")
            result.add(matches[0])
        return result
    for key, package in new.items():
        if key not in old or {k: v for k, v in package.items() if k != "dependencies"} != {
                k: v for k, v in old.get(key, {}).items() if k != "dependencies"}:
            raise ValueError("source preparation changed a locked package pin")
        if not edges(package, new).issubset(edges(old[key], old)):
            raise ValueError("source preparation introduced a dependency edge")
    return {"removed_packages": len(old.keys() - new.keys()), "retained_packages": len(new),
            "new_or_changed_pins": 0}


def run(argv, cwd, env, evidence, label, work, timeout=180, work_budget=None):
    """Bound logs, disk and wall time; retire descendants even after leader exit."""
    if work_budget is None:
        work_budget = BUILD_BUDGET
    if shutil.disk_usage(work).free < RESERVE:
        raise RuntimeError("source gate requires 30 GiB free")
    paths = [evidence / (label + suffix) for suffix in (".stdout", ".stderr")]
    with paths[0].open("wb") as stdout, paths[1].open("wb") as stderr:
        process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
            stdout=stdout, stderr=stderr, start_new_session=True, preexec_fn=child_limits)
        start = time.monotonic()
        def check_budget():
            size = 0
            for path in work.rglob("*"):
                try:
                    if path.is_file():
                        size += path.stat().st_size
                except FileNotFoundError:
                    # Cargo atomically replaces/removes intermediates.
                    continue
            if (shutil.disk_usage(work).free < RESERVE or size > work_budget
                    or time.monotonic() - start > timeout
                    or any(path.stat().st_size > LOG_BUDGET for path in paths)):
                raise RuntimeError(f"{label}: source gate resource budget exceeded")
        try:
            while process.poll() is None:
                check_budget()
                time.sleep(1)
        finally:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
        check_budget()
    if process.returncode or any(path.stat().st_size > LOG_BUDGET for path in paths):
        raise RuntimeError(f"{label} failed; inspect {evidence}")
    return paths[0].read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sdist", type=Path)
    parser.add_argument("--prepare", action="store_true",
        help="let Cargo prune the exported lock, repack with maturin, then test that exact new sdist")
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("this bounded source-build gate requires POSIX")
    scratch = ROOT / "tmp"
    scratch.mkdir(exist_ok=True)
    if shutil.disk_usage(scratch).free < RESERVE + BUILD_BUDGET:
        raise RuntimeError("source gate requires 38 GiB free (30 GiB floor + 8 GiB budget)")
    archive = args.sdist.resolve()
    evidence = Path(tempfile.mkdtemp(prefix="core-python-source-", dir=scratch))
    report = {"schema": "structuredmerge.python-source-gate/v1", "status": "failed",
        "source": str(archive), "publication_gate": False, "registry_install": "not_run",
        "profile": "dev", "python": sys.version}
    print(f"Source evidence: {evidence}", flush=True)
    try:
        with tempfile.TemporaryDirectory(prefix="work-", dir=evidence) as directory:
            work = Path(directory)
            source = extract_source(archive, work)
            with archive.open("rb") as stream:
                report["source_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
            env = {key: value for key, value in os.environ.items()
                   if key not in ("PYTHONPATH", "PYTHONHOME", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER")}
            env.update(TMPDIR=str(work), PYTHONDONTWRITEBYTECODE="1", CARGO_INCREMENTAL="0",
                CARGO_BUILD_JOBS="1", CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0",
                CARGO_TARGET_DIR=str(work / "target"))
            def command(argv, label, timeout=180):
                return run(argv, source, env, evidence, label, work, timeout)
            report["maturin"] = command([sys.executable, "-m", "maturin", "--version"], "maturin").decode().strip()
            report["cargo"] = command(["cargo", "--version"], "cargo-version").decode().strip()
            report["rustc"] = command(["rustc", "--version"], "rustc-version").decode().strip()
            if args.prepare:
                original_lock = (source / "Cargo.lock").read_text()
                command(["cargo", "metadata", "--format-version", "1", "--offline"], "prepare-lock")
                report["lock_pruning"] = verify_lock_pruning(original_lock, (source / "Cargo.lock").read_text())
                prepared = evidence / "prepared"
                command([sys.executable, "-m", "maturin", "sdist", "--out", str(prepared)], "prepare-sdist")
                archives = list(prepared.glob("*.tar.gz"))
                if len(archives) != 1:
                    raise ValueError("preparation must produce exactly one source archive")
                # Build a fresh extraction of the retained final artifact, never
                # the mutable preparation tree or a wheel from the checkout.
                source = extract_source(archives[0], work / "final")
                report["prepared_source"] = str(archives[0])
                with archives[0].open("rb") as stream:
                    report["prepared_source_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
            metadata = json.loads(command(["cargo", "metadata", "--format-version", "1",
                "--no-deps", "--locked", "--offline"], "metadata"))
            report["local_packages"] = check_metadata(metadata, source)
            report["build_lock_sha256"] = hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest()
            command([sys.executable, "-m", "maturin", "build", "--locked", "--offline",
                "--profile", "dev", "-j", "1", "--out", str(work / "wheels")], "build", 900)
            wheels = list((work / "wheels").glob("*.whl"))
            if len(wheels) != 1:
                raise ValueError("source build must produce exactly one wheel")
            with wheels[0].open("rb") as wheel:
                report["wheel_sha256"] = hashlib.file_digest(wheel, "sha256").hexdigest()
            command([sys.executable, str(ROOT / "workspace-scripts/check_core_python_artifact.py"),
                str(wheels[0])], "installed", 300)
            report["status"] = "passed"
            report["installed_wheel_gate"] = "passed"
    except BaseException as error:
        report.update(error=type(error).__name__, message=str(error))
        raise
    finally:
        (evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"Source gate {report['status']}; disposable export and target removed", flush=True)


if __name__ == "__main__":
    main()
