#!/usr/bin/env python3
"""Render deterministic provenance and resumable-publication evidence."""

import argparse
import json
import re
from pathlib import Path


REVISION = re.compile(r"\A[0-9a-f]{40}\Z")
TAG = re.compile(r"\Av\d+\.\d+\.\d+\Z")
REGISTRIES = (
    "crates-io", "pypi", "npm", "rubygems", "github-release",
    "homebrew", "scoop",
)


def render(version: str, release_tag: str, source_revision: str, inventory: dict) -> dict:
    if release_tag != f"v{version}" or not TAG.fullmatch(release_tag):
        raise ValueError("release tag must be v plus the exact semantic version")
    if not REVISION.fullmatch(source_revision):
        raise ValueError("source revision must be a full lowercase Git SHA-1")
    if inventory.get("version") != version:
        raise ValueError("asset inventory version does not match release version")
    assets = inventory.get("assets")
    if not isinstance(assets, dict) or set(assets) != {
        "linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-arm64",
        "windows-x86_64", "windows-arm64",
    }:
        raise ValueError("asset inventory must contain exactly six platform assets")
    return {
        "schema": "structuredmerge.release-evidence/v1",
        "version": version,
        "release_tag": release_tag,
        "source_revision": source_revision,
        "assets": assets,
        "publication_authorized": False,
        "registries": {registry: {"state": "pending", "published_digest": None}
                       for registry in REGISTRIES},
        "rollback": {
            "strategy": "retry-only-missing-artifacts-after-digest-verification",
            "partial_release_requires_manual_review": True,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--release-tag", required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--asset-inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads(args.asset_inventory.read_text())
    evidence = render(args.version, args.release_tag, args.source_revision, inventory)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
