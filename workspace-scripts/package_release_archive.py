#!/usr/bin/env python3
"""Create a deterministic smorg release archive from the two CLI binaries."""

import argparse
import gzip
import io
import os
from pathlib import Path
import tarfile
import zipfile


BINARIES = ("smorg", "smorg-rs")


def source_epoch() -> int:
    value = os.environ.get("SOURCE_DATE_EPOCH", "0")
    try:
        epoch = int(value)
    except ValueError as error:
        raise ValueError("SOURCE_DATE_EPOCH must be an integer") from error
    if epoch < 0:
        raise ValueError("SOURCE_DATE_EPOCH must not be negative")
    return epoch


def collect_binaries(binary_dir: Path) -> list[Path]:
    binaries = [binary_dir / name for name in BINARIES]
    missing = [str(path) for path in binaries if not path.is_file()]
    if missing:
        raise ValueError(f"missing release executable(s): {', '.join(missing)}")
    return binaries


def package(version: str, platform: str, binary_dir: Path, output_dir: Path) -> Path:
    if version.startswith("v") or not version:
        raise ValueError("version must be non-empty and omit the leading v")
    if not platform or "/" in platform or "\\" in platform:
        raise ValueError("platform must be a non-empty archive-safe name")
    binaries = collect_binaries(binary_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    epoch = source_epoch()
    archive_name = f"smorg-{version}-{platform}"
    if platform.startswith("windows"):
        archive = output_dir / f"{archive_name}.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
            for binary in binaries:
                info = zipfile.ZipInfo(binary.name)
                info.date_time = (1980, 1, 1, 0, 0, 0)
                info.external_attr = 0o100755 << 16
                output.writestr(info, binary.read_bytes())
    else:
        archive = output_dir / f"{archive_name}.tar.gz"
        with archive.open("wb") as raw:
            with gzip.GzipFile(fileobj=raw, mode="wb", compresslevel=9, mtime=epoch) as compressed:
                with tarfile.open(fileobj=compressed, mode="w") as output:
                    for binary in binaries:
                        info = tarfile.TarInfo(binary.name)
                        data = binary.read_bytes()
                        info.size = len(data)
                        info.mode = 0o755
                        info.uid = info.gid = 0
                        info.uname = info.gname = ""
                        info.mtime = epoch
                        output.addfile(info, io.BytesIO(data))
    return archive


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--binary-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    print(package(args.version, args.platform, args.binary_dir, args.output_dir))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
