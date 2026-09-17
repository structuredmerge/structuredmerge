#!/usr/bin/env python3
"""Check the local publication closure, not registry ownership or release approval.

All declared normal/build path dependencies are included, including optional and
target-specific ones. Dev-only dependencies are not runtime publication edges.
Cargo parses manifests; this tool never guesses dependency syntax from text.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "contracts/typed-release-inventory.json"
ROOTS = ("structuredmerge-core", "smorg")


def inventory(metadata, root, roots=ROOTS):
    root = Path(root).resolve()
    packages = {item["name"]: item for item in metadata["packages"] if item["source"] is None}
    paths = {Path(item["manifest_path"]).resolve().parent: name for name, item in packages.items()}
    visiting, completed, ordered = set(), set(), []

    def visit(name):
        if name in completed:
            return
        if name in visiting:
            raise ValueError(f"publication dependency cycle: {name}")
        if name not in packages:
            raise ValueError(f"missing local package: {name}")
        if "prototype" in name:
            raise ValueError(f"prototype must not enter typed publication closure: {name}")
        item = packages[name]
        manifest = Path(item["manifest_path"]).resolve()
        if not manifest.is_relative_to(root):
            raise ValueError(f"package outside kernel repository: {name}")
        if item.get("publish") is not None and "crates-io" not in item["publish"]:
            raise ValueError(f"required package cannot publish to crates.io: {name}")
        visiting.add(name)
        dependencies = set()
        for dependency in item["dependencies"]:
            if dependency.get("kind") == "dev" or not dependency.get("path"):
                continue
            path = Path(dependency["path"]).resolve()
            target = paths.get(path)
            if target is None or target != dependency["name"]:
                raise ValueError(f"unresolved local dependency: {name} -> {dependency['name']}")
            if dependency.get("req", "*") == "*":
                raise ValueError(f"path dependency needs a registry version: {name} -> {target}")
            dependencies.add(target)
        for dependency in sorted(dependencies):
            visit(dependency)
        visiting.remove(name)
        completed.add(name)
        ordered.append({"package": name, "version": item["version"],
                        "manifest": manifest.relative_to(root).as_posix(),
                        "local_dependencies": sorted(dependencies)})

    for name in roots:
        visit(name)
    return {"schema": "structuredmerge.typed-release-inventory/v1",
            "roots": list(roots),
            "scope": "all declared normal/build path dependencies; all targets and optional features; dev-only edges excluded",
            "publication_authorized": False, "registry_state_checked": False,
            "dependency_first_order": ordered}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="regenerate the reviewed local inventory")
    args = parser.parse_args()
    result = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                            cwd=ROOT, check=True, capture_output=True, text=True)
    document = inventory(json.loads(result.stdout), ROOT)
    rendered = json.dumps(document, indent=2) + "\n"
    if args.write:
        INVENTORY.write_text(rendered)
    elif not INVENTORY.is_file() or INVENTORY.read_text() != rendered:
        raise SystemExit("typed release inventory drift; review and regenerate with --write")
    print(f"{len(document['dependency_first_order'])} local crates in typed publication closure; no publication performed")


if __name__ == "__main__":
    main()
