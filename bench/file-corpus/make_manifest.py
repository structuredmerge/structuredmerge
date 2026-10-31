#!/usr/bin/env python3
"""Build a file-case manifest and deterministic repository-level shards."""

import argparse
import heapq
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--extensions", nargs="+", required=True)
    parser.add_argument("--shards", type=int, default=10)
    args = parser.parse_args()
    if args.shards < 1:
        parser.error("--shards must be positive")

    root = args.root.resolve()
    records = []
    for extension in args.extensions:
        ext_root = root / extension
        if not ext_root.exists():
            continue
        for expected in ext_root.glob("*/*/*/*/Expected." + extension):
            case_dir = expected.parent
            relative = case_dir.relative_to(root)
            if len(relative.parts) != 5:
                continue
            base, left, right = [case_dir / f"{name}.{extension}" for name in ("Base", "Left", "Right")]
            if not all(path.is_file() for path in (base, left, right)):
                continue
            file_name = case_dir / "file_name"
            final_path = case_dir / "final_path"
            repository = f"{relative.parts[1]}/{relative.parts[2]}"
            case_id = "/".join(relative.parts)
            input_bytes = sum(path.stat().st_size for path in (base, left, right))
            records.append({
                "id": case_id,
                "repository": repository,
                "extension": extension,
                "path_name": final_path.read_text().strip() if final_path.exists() else file_name.read_text().strip() if file_name.exists() else f"file.{extension}",
                "base": str(base),
                "left": str(left),
                "right": str(right),
                "expected": str(expected),
                "estimated_input_bytes": input_bytes,
            })

    records.sort(key=lambda item: item["id"])
    # File-level cases have no project-clone/test setup cost. Keep all cases
    # from a repository together and use sampled input bytes as a deterministic
    # initial work estimate; replace this estimate with pilot runtimes later.
    by_repository = {}
    for record in records:
        by_repository.setdefault(record["repository"], []).append(record)
    groups = sorted(
        by_repository.values(),
        key=lambda group: (-sum(case["estimated_input_bytes"] for case in group), group[0]["repository"]),
    )
    heap = [(0, 0, shard_id) for shard_id in range(args.shards)]
    heapq.heapify(heap)
    shard_case_counts = [0] * args.shards
    shard_input_bytes = [0] * args.shards
    for group in groups:
        load, _, shard_id = heapq.heappop(heap)
        group_bytes = sum(case["estimated_input_bytes"] for case in group)
        for record in group:
            record["shard"] = shard_id
        shard_case_counts[shard_id] += len(group)
        shard_input_bytes[shard_id] += group_bytes
        heapq.heappush(heap, (load + group_bytes, shard_case_counts[shard_id], shard_id))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as stream:
        for record in records:
            stream.write(json.dumps(record, ensure_ascii=False) + "\n")
    print(f"Wrote {len(records)} case records in {args.shards} repository-balanced shards")


if __name__ == "__main__":
    main()
