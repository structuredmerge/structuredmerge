#!/usr/bin/env python3
"""Generate Homebrew and Scoop manifests from verified release archives."""

import argparse
import hashlib
import json
from pathlib import Path


PLATFORMS = {
    "linux-x86_64": ("linux", "x86_64"),
    "linux-aarch64": ("linux", "aarch64"),
    "macos-x86_64": ("macos", "x86_64"),
    "macos-arm64": ("macos", "arm64"),
    "windows-x86_64": ("windows", "x86_64"),
    "windows-arm64": ("windows", "arm64"),
}


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def collect_assets(asset_dir: Path, version: str) -> dict[str, dict[str, str]]:
    assets: dict[str, dict[str, str]] = {}
    for platform in PLATFORMS:
        prefix = f"smorg-{version}-{platform}"
        matches = sorted(asset_dir.glob(f"{prefix}.*"))
        if len(matches) != 1 or matches[0].suffix not in {".gz", ".zip"}:
            raise ValueError(f"expected exactly one .tar.gz or .zip asset for {platform}")
        path = matches[0]
        if path.suffix == ".gz" and not path.name.endswith(".tar.gz"):
            raise ValueError(f"unsupported archive name: {path.name}")
        assets[platform] = {"name": path.name, "sha256": digest(path)}
    return assets


def render_homebrew(version: str, assets: dict[str, dict[str, str]]) -> str:
    lines = [
        "class Smorg < Formula",
        '  desc "StructuredMerge typed merge and diff CLI"',
        '  homepage "https://structuredmerge.org"',
        '  license "AGPL-3.0-only OR PolyForm-Small-Business-1.0.0"',
        f'  version "{version}"',
        "",
    ]
    lines.extend(["  on_macos do", "    if Hardware::CPU.arm?"])
    for platform in ("macos-arm64", "macos-x86_64"):
        asset = assets[platform]
        url = f"https://github.com/structuredmerge/structuredmerge/releases/download/v{version}/{asset['name']}"
        if platform == "macos-x86_64":
            lines.append("    else")
        lines.extend([f'      url "{url}"', f'      sha256 "{asset["sha256"]}"'])
    lines.extend(["    end", "  end", "  on_linux do", "    if Hardware::CPU.arm?"])
    for platform in ("linux-aarch64", "linux-x86_64"):
        asset = assets[platform]
        url = f"https://github.com/structuredmerge/structuredmerge/releases/download/v{version}/{asset['name']}"
        if platform == "linux-x86_64":
            lines.append("    else")
        lines.extend([f'      url "{url}"', f'      sha256 "{asset["sha256"]}"'])
    lines.extend(
        [
            "    end",
            "  end",
            "",
            '  bin "smorg"',
            '  bin "smorg-rs"',
            "end",
            "",
        ]
    )
    return "\n".join(lines)


def render_scoop(version: str, assets: dict[str, dict[str, str]]) -> str:
    architecture = {
        "64bit": assets["windows-x86_64"],
        "arm64": assets["windows-arm64"],
    }
    manifest = {
        "version": version,
        "description": "StructuredMerge typed merge and diff CLI",
        "homepage": "https://structuredmerge.org",
        "license": "AGPL-3.0-only OR PolyForm-Small-Business-1.0.0",
        "architecture": {
            name: {
                "url": f"https://github.com/structuredmerge/structuredmerge/releases/download/v{version}/{asset['name']}",
                "hash": asset["sha256"],
            }
            for name, asset in architecture.items()
        },
        "bin": ["smorg.exe", "smorg-rs.exe"],
        "checkver": {"github": "https://github.com/structuredmerge/structuredmerge"},
        "autoupdate": {},
    }
    return json.dumps(manifest, indent=2) + "\n"


def generate_manifests(version: str, asset_dir: Path, output_dir: Path) -> None:
    assets = collect_assets(asset_dir, version)
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "smorg.rb").write_text(render_homebrew(version, assets))
    (output_dir / "smorg.json").write_text(render_scoop(version, assets))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--asset-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    generate_manifests(args.version, args.asset_dir, args.output_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
