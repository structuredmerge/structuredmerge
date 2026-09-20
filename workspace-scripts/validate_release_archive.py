#!/usr/bin/env python3
"""Safely extract and smoke-test a platform-specific smorg archive."""

import argparse
from pathlib import Path
import subprocess
import tarfile
import tempfile
import zipfile


def _safe_member(name: str) -> Path:
    path = Path(name)
    if path.is_absolute() or ".." in path.parts or len(path.parts) != 1:
        raise ValueError(f"unsafe archive member: {name}")
    return path


def validate(archive: Path, version: str, platform: str) -> None:
    windows = platform.startswith("windows")
    suffix = ".exe" if windows else ""
    expected = {f"smorg{suffix}", f"smorg-rs{suffix}"}
    with tempfile.TemporaryDirectory(prefix="smorg-archive-check-") as directory:
        destination = Path(directory)
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as source:
                members = [_safe_member(name) for name in source.namelist()]
                if set(map(str, members)) != expected:
                    raise ValueError("archive does not contain exactly both CLI executables")
                for member in members:
                    target = destination / member
                    target.write_bytes(source.read(member.as_posix()))
        elif archive.name.endswith(".tar.gz"):
            with tarfile.open(archive, "r:gz") as source:
                members = [_safe_member(member.name) for member in source.getmembers()]
                if set(map(str, members)) != expected:
                    raise ValueError("archive does not contain exactly both CLI executables")
                for member in source.getmembers():
                    target = destination / _safe_member(member.name)
                    extracted = source.extractfile(member)
                    if extracted is None:
                        raise ValueError(f"archive member is not a regular file: {member.name}")
                    target.write_bytes(extracted.read())
        else:
            raise ValueError(f"unsupported archive format: {archive.name}")
        for name in sorted(expected):
            executable = destination / name
            if not windows:
                executable.chmod(0o755)
            result = subprocess.run([str(executable), "--version"], capture_output=True, text=True, check=True)
            if version not in result.stdout and version not in result.stderr:
                raise ValueError(f"{name} did not report release version {version}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--platform", required=True)
    args = parser.parse_args()
    validate(args.archive, args.version, args.platform)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
