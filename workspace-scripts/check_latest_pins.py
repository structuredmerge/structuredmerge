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
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


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
            if args.only in (None, "tslp"):
                rewrite(TSLP_SITES, tslp, exact_cargo_toml=True)
            if args.only in (None, "alef"):
                rewrite(ALEF_SITES, alef)
            if args.only in (None, "polylint"):
                rewrite(POLYLINT_SITES, polylint)
            print(f"rewrote pins: {TSLP} {tslp}, alef {alef}, polylint {polylint}\n"
                  "Add the reviewed alef SHA-256 to workspace-scripts/install_alef_binary.sh, then run: alef generate && alef verify --exit-code, cargo update -p "
                  f"{TSLP} --precise {tslp}, and the full test suites.")
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
    print("All tree-sitter-language-pack and alef pins are current.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
