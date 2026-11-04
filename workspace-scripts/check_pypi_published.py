#!/usr/bin/env python3
"""Verify that a published structuredmerge-core release is really on PyPI.

Run after a real publish. It closes the gap that dry runs leave: the sdist is
only built when the Rust crates are on crates.io, so nothing exercises it before
publication.

Registries lag a fresh upload, so every network step retries with exponential
backoff instead of failing on the first miss. The schedule and the "confirm the
push landed, then give propagation the full window" rule follow kettle-dev's
release availability probe:

* initial delay, then up to ``attempts`` tries;
* wait ``min(interval * 1.6 ** (attempt - 1), max_interval)`` between tries;
* once the PyPI JSON API lists the release (the push landed), later steps keep
  the full window because the CDN serving the files may lag the API;
* if the API never lists it, fail saying the publish itself may have failed.

Steps:
1. every file built for the release is listed on PyPI with the same sha256;
2. the published wheel installs into a fresh venv and imports;
3. the published sdist builds a wheel from source (resolving the Rust crates
   from crates.io) into a scratch directory.

Standard library only, so it runs on a bare runner.
"""
import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

BACKOFF_FACTOR = 1.6


def env_positive_int(name, default, env=os.environ):
    value = env.get(name, "").strip()
    return int(value) if value.isdigit() and int(value) > 0 else default


class Schedule:
    """Retry schedule; defaults mirror kettle-dev's release availability probe."""

    def __init__(self, env=os.environ, sleep=time.sleep):
        self.attempts = env_positive_int("STRUCTUREDMERGE_REGISTRY_PROBE_ATTEMPTS", 15, env)
        self.initial_delay = env_positive_int("STRUCTUREDMERGE_REGISTRY_PROBE_INITIAL_DELAY", 5, env)
        self.interval = env_positive_int("STRUCTUREDMERGE_REGISTRY_PROBE_INTERVAL", 10, env)
        self.max_interval = env_positive_int("STRUCTUREDMERGE_REGISTRY_PROBE_MAX_INTERVAL", 120, env)
        self.sleep = sleep

    def delay(self, attempt):
        return min(round(self.interval * BACKOFF_FACTOR ** (attempt - 1)), self.max_interval)

    def retry(self, label, step):
        """Call step() until it returns without raising Retry; returns its value."""
        self.sleep(self.initial_delay)
        last = None
        for attempt in range(1, self.attempts + 1):
            print(f"{label} (attempt {attempt}/{self.attempts})", flush=True)
            try:
                return step()
            except Retry as error:
                last = error
                print(f"  not ready: {error}", flush=True)
                if attempt < self.attempts:
                    self.sleep(self.delay(attempt))
        raise Failure(f"{label} failed after {self.attempts} attempt(s): {last}")


class Retry(Exception):
    """A transient condition; try again later."""


class Failure(Exception):
    pass


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def expected_files(dist_dir):
    """Map filename -> sha256 of every release artifact built for this version."""
    files = {p.name: sha256_file(p) for p in sorted(pathlib.Path(dist_dir).iterdir())
             if p.name.endswith((".whl", ".tar.gz"))}
    if not any(name.endswith(".tar.gz") for name in files):
        raise Failure(f"no sdist (.tar.gz) in {dist_dir}")
    if not any(name.endswith(".whl") for name in files):
        raise Failure(f"no wheels in {dist_dir}")
    return files


def fetch_release(index_url, package, version, opener=urllib.request.urlopen):
    """Return the PyPI JSON for the release, or None while it is not listed."""
    url = f"{index_url.rstrip('/')}/pypi/{package}/{version}/json"
    try:
        with opener(url, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise Retry(f"{url} answered HTTP {error.code}")
    except (urllib.error.URLError, TimeoutError) as error:
        raise Retry(f"{url} unreachable: {error}")


def check_listed(index_url, package, version, files, schedule, fetch=fetch_release):
    """Wait until PyPI lists every file with the expected sha256.

    Returns True once everything matches. A digest mismatch is never retried:
    the registry holds different bytes than we built.
    """
    seen_listed = {"value": False}

    def step():
        release = fetch(index_url, package, version)
        if release is None:
            raise Retry(f"{package} {version} is not listed on {index_url} yet")
        seen_listed["value"] = True
        published = {entry["filename"]: entry["digests"]["sha256"] for entry in release.get("urls", [])}
        missing = sorted(set(files) - set(published))
        if missing:
            raise Retry("not yet listed: " + ", ".join(missing))
        wrong = sorted(name for name, digest in files.items() if published[name] != digest)
        if wrong:
            raise Failure("sha256 differs from the built artifact for: " + ", ".join(wrong))
        return True

    try:
        return schedule.retry(f"Checking {package} {version} on {index_url}", step)
    except Failure as error:
        if seen_listed["value"] or "sha256 differs" in str(error):
            raise
        raise Failure(f"{error}; the release is not visible at all, so the publish itself may have failed")


def run_pip(args, schedule, label, run=subprocess.run):
    """Run a pip command, retrying while the CDN catches up with the API."""
    def step():
        completed = run(args, capture_output=True, text=True, check=False)
        if completed.returncode != 0:
            tail = (completed.stdout + completed.stderr)[-1500:]
            raise Retry(f"exit {completed.returncode}: {tail}")
        return completed

    return schedule.retry(label, step)


def check_wheel_install(package, version, index_url, schedule, run=subprocess.run):
    with tempfile.TemporaryDirectory(prefix="pypi-wheel-check-") as tmp:
        venv = pathlib.Path(tmp) / "venv"
        run([sys.executable, "-m", "venv", str(venv)], check=True)
        python = str(venv / ("Scripts" if os.name == "nt" else "bin") / "python")
        run_pip([python, "-m", "pip", "install", "--no-deps", "--only-binary", ":all:",
                 "--index-url", f"{index_url.rstrip('/')}/simple", f"{package}=={version}"],
                schedule, f"Installing published {package} {version} wheel")
        module = package.replace("-", "_")
        imported = run([python, "-c",
                        f"import {module}; print('{module}', getattr({module}, '__version__', 'ok'))"],
                       capture_output=True, text=True, check=False)
        if imported.returncode != 0:
            raise Failure("published wheel installed but does not import:\n" + imported.stderr[-1500:])
        print(imported.stdout.strip())


def check_sdist_build(package, version, index_url, schedule, run=subprocess.run):
    with tempfile.TemporaryDirectory(prefix="pypi-sdist-check-") as tmp:
        run_pip([sys.executable, "-m", "pip", "wheel", "--no-deps", "--no-binary", package,
                 "--index-url", f"{index_url.rstrip('/')}/simple", "--wheel-dir", tmp,
                 f"{package}=={version}"],
                schedule, f"Building {package} {version} wheel from the published sdist")
        built = [p.name for p in pathlib.Path(tmp).glob("*.whl")]
        if not built:
            raise Failure("sdist build produced no wheel")
        print("built from sdist:", ", ".join(built))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--package", default="structuredmerge-core")
    parser.add_argument("--version", required=True)
    parser.add_argument("--dist-dir", required=True, help="directory with the wheels and sdist that were published")
    parser.add_argument("--index-url", default="https://pypi.org")
    parser.add_argument("--skip-install-checks", action="store_true",
                        help="only verify listing and digests (used by tests and local runs)")
    args = parser.parse_args(argv)

    schedule = Schedule()
    try:
        files = expected_files(args.dist_dir)
        print(f"Expecting {len(files)} file(s) for {args.package} {args.version}:")
        for name, digest in files.items():
            print(f"  {name} sha256:{digest[:12]}")
        check_listed(args.index_url, args.package, args.version, files, schedule)
        print("All release files are listed on PyPI with matching sha256.")
        if not args.skip_install_checks:
            check_wheel_install(args.package, args.version, args.index_url, schedule)
            check_sdist_build(args.package, args.version, args.index_url, schedule)
    except Failure as error:
        print(f"PyPI post-publish verification failed: {error}", file=sys.stderr)
        return 1
    print("PyPI post-publish verification passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
