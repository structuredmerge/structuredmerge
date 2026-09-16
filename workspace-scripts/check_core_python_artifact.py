#!/usr/bin/env python3
"""Validate a development wheel and run native merge tests outside its checkout.

This is a local runtime gate, not publication approval or the full platform matrix.
"""
import email.parser
import ast
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import venv
import zipfile


def inspect_wheel(root, wheel):
    with zipfile.ZipFile(wheel) as archive:
        names = archive.namelist()
        metadata_files = [name for name in names if name.endswith(".dist-info/METADATA")]
        if len(metadata_files) != 1:
            raise ValueError("expected one wheel metadata record")
        metadata = email.parser.BytesParser().parsebytes(archive.read(metadata_files[0]))
        if metadata["Name"] != "structuredmerge-core":
            raise ValueError("unexpected wheel package")
        if metadata["License-Expression"] != "AGPL-3.0-only OR PolyForm-Small-Business-1.0.0":
            raise ValueError("missing dual-license metadata")
        license_files = [name for name in names if name.endswith("/licenses/LICENSE")]
        if len(license_files) != 1 or archive.read(license_files[0]) != (root / "LICENSE").read_bytes():
            raise ValueError("wheel must contain the complete authoritative combined license text")
        if any("prototype" in name or ".data/scripts/" in name for name in names):
            raise ValueError("wheel includes legacy prototype files or executables")
        for name in names:
            if name.endswith(".dist-info/entry_points.txt") and b"[console_scripts]" in archive.read(name):
                raise ValueError("binding wheel must not install console scripts")
        stub = "structuredmerge_core/_native.pyi"
        if stub not in names or "structuredmerge_core/py.typed" not in names:
            raise ValueError("wheel must contain native type declarations and py.typed")
        ast.parse(archive.read(stub).decode("utf-8"))
    return metadata, license_files


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: check_core_python_artifact.py WHEEL")
    root = Path(__file__).resolve().parent.parent
    wheel = Path(sys.argv[1]).resolve()
    metadata, license_files = inspect_wheel(root, wheel)
    (root / "tmp").mkdir(exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix="core-python-artifact-", dir=root / "tmp"))
    environment = stage / "venv"
    venv.EnvBuilder(with_pip=True).create(environment)
    python = environment / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    consumer = stage / "consumer"
    consumer.mkdir()
    for name in ("test_parser_host.py", "libcst_facts.py", "native_merge_fixture.py"):
        shutil.copyfile(root / "packages/python/tests" / name, consumer / name)
    shutil.copytree(root / "e2e/python/tests", consumer / "generated")
    env = {key: value for key, value in os.environ.items() if key not in ("PYTHONPATH", "PYTHONHOME")}
    subprocess.run([str(python), "-m", "pip", "install", str(wheel), "libcst==1.9.0", "pytest>=7.4"],
        cwd=consumer, env=env, check=True)
    subprocess.run([str(python), "-m", "unittest", "discover", "-s", ".", "-v"],
        cwd=consumer, env=env, check=True)
    subprocess.run([str(python), "-m", "pytest", "generated", "-v"],
        cwd=consumer, env=env, check=True)
    report = {
        "artifact": str(wheel), "sha256": hashlib.sha256(wheel.read_bytes()).hexdigest(),
        "package": metadata["Name"], "version": metadata["Version"],
        "python": sys.version, "libcst": "1.9.0", "license_files": license_files,
        "installed_merge_tests": "passed", "publication_gate": False,
        "generated_e2e_tests": "passed",
    }
    (stage / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report))
    print(f"Consumer and report: {stage}")


if __name__ == "__main__":
    main()
