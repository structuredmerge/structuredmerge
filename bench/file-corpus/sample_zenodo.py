#!/usr/bin/env python3
"""Stream-sample the Zenodo tar.gz without expanding the full corpus."""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tarfile


KEEP_FILES = {"file_name", "final_path", "source_url"}


def sample_case(case_id: str, modulus: int, bucket: int) -> bool:
    value = int(hashlib.sha256(case_id.encode()).hexdigest(), 16)
    return value % modulus == bucket


def case_location(member_name: str, extensions: set[str]):
    path = PurePosixPath(member_name)
    parts = path.parts
    if not parts or path.is_absolute() or ".." in parts:
        return None
    name = parts[-1]
    for index, extension in enumerate(parts[:-1]):
        if extension not in extensions:
            continue
        # extension/owner/repo/merge_commit/path_hash/file
        if len(parts) - index != 6:
            continue
        if name not in {"Base." + extension, "Left." + extension,
                        "Right." + extension, "Expected." + extension,
                        *KEEP_FILES}:
            continue
        case_id = "/".join(parts[index:index + 5])
        return case_id, extension
    return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--extensions", nargs="+", required=True)
    parser.add_argument("--sample-modulus", type=int, default=100)
    parser.add_argument("--sample-bucket", type=int, default=0)
    args = parser.parse_args()
    if args.sample_modulus < 1 or not 0 <= args.sample_bucket < args.sample_modulus:
        parser.error("sample bucket must be in [0, sample modulus)")
    extensions = set(args.extensions)
    args.output.mkdir(parents=True, exist_ok=True)
    if any(args.output.iterdir()):
        parser.error("output directory must be empty; use a new directory for each sample")

    selected: set[str] = set()
    with tarfile.open(args.archive, mode="r|gz") as archive:
        for member in archive:
            if not member.isfile():
                continue
            located = case_location(member.name, extensions)
            if located is None:
                continue
            case_id, _ = located
            if not sample_case(case_id, args.sample_modulus, args.sample_bucket):
                continue
            source = archive.extractfile(member)
            if source is None:
                continue
            target = args.output.joinpath(*PurePosixPath(case_id).parts,
                                          PurePosixPath(member.name).name)
            target.parent.mkdir(parents=True, exist_ok=True)
            with source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            selected.add(case_id)

    metadata = {
        "schema": "structuredmerge.zenodo-sample/v1",
        "archive": str(args.archive.resolve()),
        "sample_modulus": args.sample_modulus,
        "sample_bucket": args.sample_bucket,
        "extensions": sorted(extensions),
        "case_count": len(selected),
        "case_ids": sorted(selected),
    }
    (args.output / "sample-manifest.json").write_text(
        json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Extracted {len(selected)} cases to {args.output}")


if __name__ == "__main__":
    main()
