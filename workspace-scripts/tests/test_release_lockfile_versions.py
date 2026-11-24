"""Release-time lockfiles that vendor the path gem must carry the release version.

CI installs the typed core gemfile in frozen mode, so a stale
`structuredmerge-core (X)` entry fails every Ruby platform gem verification job.
"""
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]


def crate_version():
    manifest = (ROOT / "crates/structuredmerge-core/Cargo.toml").read_text()
    return re.search(r'^version = "(\d+\.\d+\.\d+)"', manifest, re.M).group(1)


def test_typed_core_gemfile_lock_matches_release_version():
    lock = (ROOT / ".github/typed_core.gemfile.lock").read_text()
    assert f"structuredmerge-core ({crate_version()})" in lock, (
        "re-lock with: BUNDLE_GEMFILE=.github/typed_core.gemfile bundle lock --update"
    )
