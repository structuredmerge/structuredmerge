# Pre-publication Python core artifacts

From `packages/python`, build the committed generated binding into a fresh output
directory, then run the installed-consumer gate under the Python being evaluated:

```sh
python -m maturin build --locked --out ../../tmp/python-matrix-wheels --profile dev
python ../../workspace-scripts/check_core_python_artifact.py ../../tmp/python-matrix-wheels
```

The helper accepts a wheel file or a directory containing exactly one wheel.
It rejects empty or ambiguous directories; no shell glob expansion is needed.
It audits package/license/type-declaration/API-baseline contents before installing
into a fresh virtual environment, then runs runtime tests with native LibCST and
the generated e2e suite. The digest-bearing success report stays in kernel
`tmp/core-python-artifact-*`; failed checks write a failure report instead.
The disposable virtual environment and copied consumer files are removed on
success, failure, and Python-handled interruption. A forced kill or machine crash
can bypass teardown; inspect and clean that invocation's scratch directory when
resuming. Pip's download cache is disabled for this isolated check.

The installed check refuses to start with less than 22 GiB free: a 20 GiB
emergency reserve plus a 2 GiB environment budget. This is a preflight check,
not a filesystem quota or a guard for the separate wheel build. Monitor space
during commands and budget the build independently. No environment is retained
for later benchmarks; use a separately scoped, explicitly cleaned benchmark
environment installed from the same recorded wheel.

The current workflow requests Linux x64 on Python 3.10 and 3.14, and Linux ARM64,
macOS x64/ARM64 and Windows x64/ARM64 on Python 3.14. Runner labels follow the
[GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
Windows checkout disables automatic line-ending conversion so reviewed API byte
digests remain meaningful. Rust native LibCST probes explicitly use the selected
`python` executable. General tooling audits stay on Linux because they also test
POSIX CLI and Ruby workflow behavior; every matrix leg runs installed Python tests.

These are requested platform gates, not confirmed platform support. Local Linux
Python 3.10.19 and 3.14.2 pass 40 runtime and 101 generated tests, including the
fresh-process lifecycle tests. Hosted macOS, Windows and ARM64 remain unverified.
An abi3 wheel tag does not itself prove runtime, grammar-delivery or OS/libc
compatibility. Debug builds provide no release-performance claim. This gate
publishes nothing and does not establish upstream-only generation, registry-mode
installation, default authority or release readiness.
