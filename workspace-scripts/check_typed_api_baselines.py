#!/usr/bin/env python3
"""Exact API source-review gate, not semantic compatibility or binary ABI proof.

Checking never writes. Recording requires a target and compatibility-review reason.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ("ruby", "python")
SCHEMA = "structuredmerge.typed-api-review-baseline/v1"


def surface_files(root, target):
    package = root / "packages" / target
    if target == "ruby":
        files = [package / "lib/structuredmerge_core.rb"]
        files += list((package / "lib/structuredmerge_core").rglob("*.rb"))
        files += list((package / "sig").rglob("*.rbs"))
        required = {"lib/structuredmerge_core.rb", "lib/structuredmerge_core/native.rb",
                    "lib/structuredmerge_core/version.rb", "sig/types.rbs"}
    elif target == "python":
        files = [p for p in (package / "structuredmerge_core").rglob("*")
                 if p.is_file() and (p.suffix in (".py", ".pyi") or p.name == "py.typed")]
        required = {"structuredmerge_core/__init__.py", "structuredmerge_core/_native.pyi",
                    "structuredmerge_core/py.typed"}
    else:
        raise ValueError(f"unknown target: {target}")
    result = {p.relative_to(package).as_posix(): p.read_bytes() for p in sorted(files)}
    if not required <= result.keys():
        raise ValueError(f"{target}: required API surface files missing: {sorted(required - result.keys())}")
    return result


def record(root, target, reason):
    if not reason.strip():
        raise ValueError("recording requires a nonempty compatibility review reason")
    files = surface_files(root, target)
    baseline = root / "contracts/typed-api" / target
    # Do not delete old snapshots automatically: removals need explicit review.
    for name, content in files.items():
        path = baseline / "snapshot" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
    manifest = {"schema": SCHEMA, "target": target, "review_reason": reason,
                "files": {name: hashlib.sha256(content).hexdigest() for name, content in files.items()}}
    (baseline / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def check(root, target):
    actual = surface_files(root, target)
    baseline = root / "contracts/typed-api" / target
    manifest = json.loads((baseline / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("schema") != SCHEMA or manifest.get("target") != target or not manifest.get("review_reason", "").strip():
        raise ValueError(f"{target}: invalid baseline identity/review record")
    expected = manifest["files"]
    snapshot = baseline / "snapshot"
    stored = {p.relative_to(snapshot).as_posix(): p.read_bytes() for p in snapshot.rglob("*") if p.is_file()}
    if expected.keys() != stored.keys():
        raise ValueError(f"{target}: snapshot/manifest file set differs; review added/removed files")
    for name, content in stored.items():
        if hashlib.sha256(content).hexdigest() != expected[name]:
            raise ValueError(f"{target}: baseline content/hash mismatch: {name}")
    if actual != stored:
        changed = sorted(name for name in actual.keys() | stored.keys() if actual.get(name) != stored.get(name))
        raise ValueError(f"{target}: API review required for {', '.join(changed)}; regenerate with Alef, review compatibility, then explicitly record the baseline")
    return len(actual)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--reason")
    args = parser.parse_args()
    if args.record and (not args.target or not args.reason):
        parser.error("--record requires --target and --reason")
    if args.reason and not args.record:
        parser.error("--reason is only valid with --record")
    try:
        for target in (args.target,) if args.target else TARGETS:
            if args.record:
                record(ROOT, target, args.reason)
            print(f"{target}: {check(ROOT, target)} API surface files match the reviewed baseline")
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
