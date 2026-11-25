#!/usr/bin/env python3
"""Fail unless every tree-sitter-language-pack, alef and polylint pin is the latest release.

Both projects ship more than once a day, and a stale pin silently changes
behaviour: a caret requirement such as `tree-sitter-language-pack = "1.25.2"`
let source-gem builds (which ship no Cargo.lock) resolve a newer crate than the
one this repo was tested against. So the policy is:

* every Rust requirement on the crate is an exact `=X.Y.Z` pin equal to the latest release;
* Cargo.lock, workflow cache paths and alef version pins all equal the latest release.

Usage:
  check_latest_pins.py            check, exit 1 listing every stale site
  check_latest_pins.py --write    rewrite every site to the latest release
  check_latest_pins.py --offline  only check that all sites agree with each other

Latest versions come from crates.io (TSLP), GitHub releases (alef) and PyPI (polylint, the `poly` formatter).
`--tslp-latest` / `--alef-latest` override the lookups (used by the tests).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
USER_AGENT = "structuredmerge-pin-check"
SKIP_DIRS = {".git", "target", "node_modules", "tmp", "vendor", "docs", "__pycache__"}

TSLP = "tree-sitter-language-pack"
VERSION = r"\d+\.\d+\.\d+"

# (label, file glob, regex with groups (prefix)(version)(suffix)); `=` handling for Cargo.toml is below.
TSLP_SITES = [
    ("Cargo.toml requirement", "**/Cargo.toml",
     re.compile(rf'(^\s*{TSLP}\s*=\s*"=?)({VERSION})(")', re.M)),
    ("Cargo.toml table requirement", "**/Cargo.toml",
     re.compile(rf'({TSLP}\s*=\s*\{{[^}}]*version\s*=\s*"=?)({VERSION})(")')),
    ("Cargo.lock entry", "Cargo.lock",
     re.compile(rf'(name = "{TSLP}"\nversion = ")({VERSION})(")')),
    ("workflow/script cache path", ".github/workflows/*.yml",
     re.compile(rf'({TSLP}/v)({VERSION})(/)')),
    ("script cache path", "workspace-scripts/**/*.*",
     re.compile(rf'({TSLP}/v)({VERSION})(/)')),
]
ALEF_SITES = [
    ("alef.toml", "alef.toml", re.compile(rf'(alef_version = ")({VERSION})(")')),
    ("workspace-scripts/alef-version", "workspace-scripts/alef-version", re.compile(rf'(^)({VERSION})(\s*$)', re.M)),
    ("workflow alef version", ".github/workflows/*.yml",
     re.compile(rf'((?:alef-version|version):\s*")({VERSION})(")')),
]


POLYLINT_SITES = [
    ("workflow polylint pin", ".github/workflows/*.yml",
     re.compile(rf"(polylint==)({VERSION})(['\"\s]|$)", re.M)),
]


def fetch_json(url: str) -> dict:
    """GET JSON. GitHub's API rate-limits anonymous callers hard (shared CI runner IPs
    hit it), so send GITHUB_TOKEN/GH_TOKEN when present and fall back to `gh api`."""
    import os
    import shutil
    import subprocess
    import urllib.error

    headers = {"User-Agent": USER_AGENT, "Accept": "application/json"}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code in (403, 429) and url.startswith("https://api.github.com/") and shutil.which("gh"):
            result = subprocess.run(["gh", "api", url.removeprefix("https://api.github.com/")],
                                    capture_output=True, text=True, check=False)
            if result.returncode == 0:
                return json.loads(result.stdout)
        raise


def latest_tslp() -> str:
    return fetch_json(f"https://crates.io/api/v1/crates/{TSLP}")["crate"]["max_stable_version"]


def latest_polylint() -> str:
    return fetch_json("https://pypi.org/pypi/polylint/json")["info"]["version"]


def latest_alef() -> str:
    return fetch_json("https://api.github.com/repos/xberg-io/alef/releases/latest")["tag_name"].lstrip("v")


def files(pattern: str):
    for path in sorted(ROOT.glob(pattern)):
        if path.is_file() and not (set(path.relative_to(ROOT).parts) & SKIP_DIRS):
            yield path


def scan(sites, kind_filter=None):
    """Yield (label, path, line_no, version, line_text) for every pin site."""
    for label, pattern, regex in sites:
        for path in files(pattern):
            text = path.read_text(errors="replace")
            for match in regex.finditer(text):
                line_no = text.count("\n", 0, match.start(2)) + 1
                line = text.splitlines()[line_no - 1] if text else ""
                yield label, path, line_no, match.group(2), line


def exact_pin_violations(latest_tslp_version):
    """Cargo.toml requirements must be `=X.Y.Z`, not a caret range."""
    for label, path, line_no, version, line in scan(TSLP_SITES[:2]):
        if f'"={version}"' not in line.replace(" ", ""):
            yield f"{path.relative_to(ROOT)}:{line_no}: requirement must be an exact =pin, got: {line.strip()}"


def rewrite(sites, latest, exact_cargo_toml=False):
    for label, pattern, regex in sites:
        if label == "Cargo.lock entry":
            continue  # cargo regenerates it with the right checksum (see update_cargo_lock)
        for path in files(pattern):
            text = path.read_text(errors="replace")

            def replace(match):
                prefix = match.group(1)
                if exact_cargo_toml and label.startswith("Cargo.toml"):
                    prefix = prefix.rstrip("=") + "="
                return prefix + latest + match.group(3)

            new = regex.sub(replace, text)
            if new != text:
                path.write_text(new)


def update_cargo_lock(version: str) -> None:
    """Let cargo write the lock entry; a regex edit would leave the old checksum."""
    import shutil
    import subprocess

    if not shutil.which("cargo"):
        print("cargo not found: run `cargo update -p tree-sitter-language-pack --precise "
              f"{version}` to refresh Cargo.lock", file=sys.stderr)
        return
    subprocess.run(["cargo", "update", "-p", TSLP, "--precise", version], cwd=ROOT, check=True)


def add_alef_checksum(version: str) -> bool:
    """Add the Linux binary checksum for `version` from the digest GitHub publishes for the asset."""
    installer = ROOT / "workspace-scripts" / "install_alef_binary.sh"
    text = installer.read_text()
    if re.search(rf"^\s+{re.escape(version)}\)", text, re.M):
        return False
    release = fetch_json(f"https://api.github.com/repos/xberg-io/alef/releases/tags/v{version}")
    digest = next(
        (asset.get("digest", "") for asset in release["assets"]
         if asset["name"] == "alef-x86_64-unknown-linux-gnu.tar.gz"),
        "",
    )
    if not digest.startswith("sha256:"):
        raise SystemExit(f"GitHub publishes no sha256 digest for alef {version}; add the checksum by hand")
    entry = f"  {version})\n    alef_sha256={digest.removeprefix('sha256:')}\n    ;;\n"
    marker = 'case "$alef_version" in\n'
    installer.write_text(text.replace(marker, marker + entry, 1))
    return True


def installer_missing(alef_version: str) -> list[str]:
    installer = ROOT / "workspace-scripts" / "install_alef_binary.sh"
    if re.search(rf"^\s+{re.escape(alef_version)}\)", installer.read_text(), re.M):
        return []
    return [f"workspace-scripts/install_alef_binary.sh has no reviewed SHA-256 for alef {alef_version}; "
            "download the release tarball, review it, and add its checksum"]


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--only", choices=["tslp", "alef", "polylint"], help="limit --write to one project")
    parser.add_argument("--tslp-latest")
    parser.add_argument("--alef-latest")
    parser.add_argument("--polylint-latest")
    args = parser.parse_args(argv)

    tslp_hits = list(scan(TSLP_SITES))
    alef_hits = list(scan(ALEF_SITES))
    polylint_hits = list(scan(POLYLINT_SITES))
    problems: list[str] = []

    if args.offline:
        for name, hits in (("tree-sitter-language-pack", tslp_hits), ("alef", alef_hits), ("polylint", polylint_hits)):
            versions = {hit[3] for hit in hits}
            if len(versions) > 1:
                problems.append(f"{name} pins disagree: {sorted(versions)}")
    else:
        tslp = args.tslp_latest or latest_tslp()
        alef = args.alef_latest or latest_alef()
        polylint = args.polylint_latest or latest_polylint()
        if args.write:
            changes = []
            if args.only in (None, "tslp") and any(hit[3] != tslp for hit in tslp_hits):
                changes.append(f"tree-sitter-language-pack {', '.join(sorted({h[3] for h in tslp_hits}))} -> {tslp}")
                rewrite(TSLP_SITES, tslp, exact_cargo_toml=True)
                update_cargo_lock(tslp)
            if args.only in (None, "alef") and any(hit[3] != alef for hit in alef_hits):
                changes.append(f"alef {', '.join(sorted({h[3] for h in alef_hits}))} -> {alef}")
                rewrite(ALEF_SITES, alef)
            if args.only in (None, "alef") and add_alef_checksum(alef):
                changes.append(f"alef {alef} binary checksum (GitHub-published sha256 digest)")
            if args.only in (None, "polylint") and any(hit[3] != polylint for hit in polylint_hits):
                changes.append(f"polylint {', '.join(sorted({h[3] for h in polylint_hits}))} -> {polylint}")
                rewrite(POLYLINT_SITES, polylint)
            print("\n".join(changes) if changes else "nothing to update")
            if changes:
                print("\nNow regenerate and verify: alef generate, alef verify --exit-code, then the full test suites.")
            return 0
        for name, hits, latest in (
            ("tree-sitter-language-pack", tslp_hits, tslp),
            ("alef", alef_hits, alef),
            ("polylint", polylint_hits, polylint),
        ):
            for label, path, line_no, version, _line in hits:
                if version != latest:
                    problems.append(f"{path.relative_to(ROOT)}:{line_no}: {name} {version} is not the latest ({latest}) [{label}]")
        problems.extend(exact_pin_violations(tslp))
        problems.extend(installer_missing(alef))

    if problems:
        print("Stale or loose pins found:\n  " + "\n  ".join(problems), file=sys.stderr)
        print("\nFix everything at once with: python workspace-scripts/check_latest_pins.py --write", file=sys.stderr)
        return 1
    print("All tree-sitter-language-pack, alef and polylint pins are current.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
