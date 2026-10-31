#!/usr/bin/env python3
"""Apply the local SM adapter to the pinned AST-Merging-Evaluation checkout."""

import argparse
from pathlib import Path
import shutil
import subprocess


HERE = Path(__file__).resolve().parent
EXPECTED_REVISION = (HERE / "UPSTREAM_REVISION").read_text().strip()
ENUM_END = '        "mergiraf",\n'


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("evaluator", type=Path)
    args = parser.parse_args()
    root = args.evaluator.resolve()
    revision = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != EXPECTED_REVISION:
        raise SystemExit(
            f"Expected evaluator {EXPECTED_REVISION}, found {revision}; refusing to patch"
        )

    repo_py = root / "src/python/repo.py"
    source = repo_py.read_text()
    if '        "smorg",\n' in source:
        raise SystemExit("smorg adapter is already present")
    if source.count(ENUM_END) != 1:
        raise SystemExit("Could not find the expected merge-tool enum insertion point")
    repo_py.write_text(source.replace(ENUM_END, ENUM_END + '        "smorg",\n', 1))

    tool = root / "src/scripts/merge_tools/smorg.sh"
    if tool.exists():
        raise SystemExit(f"Refusing to overwrite {tool}")
    shutil.copy2(HERE / "smorg.sh", tool)
    tool.chmod(0o755)
    print(f"Prepared evaluator {revision} with the StructuredMerge adapter")


if __name__ == "__main__":
    main()
