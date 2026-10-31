#!/usr/bin/env python3
"""Create deterministic, disjoint repository CSV shards and a digest manifest."""

import argparse
import csv
import hashlib
import heapq
import json
from pathlib import Path
import random


def digest(path: Path) -> str:
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(block)
    return sha.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--shards", type=int, required=True)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument(
        "--costs-csv",
        type=Path,
        help="Optional CSV with repository,estimated_seconds from a cold pilot.",
    )
    args = parser.parse_args()
    if args.shards < 1:
        parser.error("--shards must be positive")

    with args.input.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source)
        fields = reader.fieldnames or []
        if "repository" not in fields:
            parser.error("input CSV must have a repository column")
        rows = list(reader)
    repositories = [row["repository"] for row in rows]
    if len(set(repositories)) != len(repositories):
        parser.error("input contains duplicate repositories")

    costs = {}
    if args.costs_csv:
        with args.costs_csv.open(newline="", encoding="utf-8") as source:
            reader = csv.DictReader(source)
            if not {"repository", "estimated_seconds"}.issubset(reader.fieldnames or []):
                parser.error("cost CSV needs repository and estimated_seconds columns")
            for row in reader:
                value = float(row["estimated_seconds"])
                if value <= 0:
                    parser.error("estimated_seconds must be positive")
                costs[row["repository"]] = value
        known = [costs[row["repository"]] for row in rows if row["repository"] in costs]
        default_cost = sorted(known)[len(known) // 2] if known else 1.0
    else:
        default_cost = 1.0

    # Treat a repository, not a merge row, as the indivisible work item: clone,
    # HEAD validation, and test setup are amortized over its selected merges.
    # Longest-estimated-time-first (LPT) reduces tail time once pilot durations
    # exist. A fresh corpus gets deterministic count-balanced groups and is
    # explicitly marked as unweighted in the manifest.
    rng = random.Random(args.seed)
    tie_break = {row["repository"]: rng.random() for row in rows}
    groups = [[] for _ in range(args.shards)]
    loads = [0.0] * args.shards
    if args.costs_csv:
        ordered = sorted(
            rows,
            key=lambda row: (-costs.get(row["repository"], default_cost), tie_break[row["repository"]]),
        )
        heap = [(0.0, 0, shard_id) for shard_id in range(args.shards)]
        heapq.heapify(heap)
        for row in ordered:
            load, count, shard_id = heapq.heappop(heap)
            groups[shard_id].append(row)
            cost = costs.get(row["repository"], default_cost)
            loads[shard_id] += cost
            heapq.heappush(heap, (load + cost, count + 1, shard_id))
    else:
        rng.shuffle(rows)
        for index, row in enumerate(rows):
            shard_id = index % args.shards
            groups[shard_id].append(row)
            loads[shard_id] += 1.0
    args.output_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "schema": "structuredmerge.ast-eval-shards/v1",
        "input": str(args.input.resolve()),
        "input_sha256": digest(args.input),
        "seed": args.seed,
        "shard_count": args.shards,
        "repository_count": len(rows),
        "balance_method": "lpt_estimated_seconds" if args.costs_csv else "equal_repo_count_unweighted",
        "costs_csv": str(args.costs_csv.resolve()) if args.costs_csv else None,
        "costs_csv_sha256": digest(args.costs_csv) if args.costs_csv else None,
        "shards": [],
    }
    for shard_id, group in enumerate(groups):
        output = args.output_dir / f"shard-{shard_id:03d}.csv"
        with output.open("w", newline="", encoding="utf-8") as target:
            writer = csv.DictWriter(target, fieldnames=fields)
            writer.writeheader()
            writer.writerows(group)
        manifest["shards"].append(
            {
                "id": shard_id,
                "path": output.name,
                "repository_count": len(group),
                "estimated_seconds": loads[shard_id] if args.costs_csv else None,
                "sha256": digest(output),
                "repositories": [row["repository"] for row in group],
            }
        )
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"Wrote {args.shards} disjoint shards for {len(rows)} repositories to "
        f"{args.output_dir}"
    )


if __name__ == "__main__":
    main()
