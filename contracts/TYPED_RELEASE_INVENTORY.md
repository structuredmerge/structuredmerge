# Typed publication closure

`typed-release-inventory.json` is generated from Cargo's manifest metadata for
the currently implemented Rust publication roots `structuredmerge-core` and
`smorg`. It is not registry ownership evidence, release authorization, or proof
that a source gem/wheel can build. The planned `structuredmerge` facade is not
invented as a completed package; its addition requires updating the roots.

Run `python workspace-scripts/check_typed_release_inventory.py` to check drift,
or use `--write` after reviewing a manifest change. All declared normal/build
path dependencies are included, even optional and target-specific ones. Dev-only
edges are excluded. The check rejects missing local crates, paths outside the
kernel, missing registry version requirements, cycles, prototype dependencies and
dependencies that disallow crates.io publication. Package ownership, license
contents, registry availability and feature-complete builds are separate gates.

The release command consumes this dependency-first order before its retained
legacy leaves. Its new read-only mode performs no packaging or publication:

```sh
ruby workspace-scripts/release_rust_crates.rb --list
ruby workspace-scripts/release_rust_crates.rb --list --only structuredmerge-core
```

Other modes retain their existing semantics: the command publishes by default
unless `--no-push` is supplied. Do not invoke them as an inventory check. `--only`
selects exactly one crate; it does not automatically publish prerequisites.
Neither a successful inventory check nor `--list` lifts any plan release gate.

## Source Ruby package gap

The current generated Ruby gemspec is not a verified source distribution. Its
extension manifest points to `../../../../../crates/structuredmerge-core`, outside
an unpacked gem. Its broad `lib`/`ext` glob can also include retained prototype
files. The platform-gem artifact helper already uses a strict allowlist; that
does not establish a working source gem.

Source preparation must use the Alef packaging pipeline, preserve the typed API
and license files, exclude prototype files, and include or resolve this full
dependency closure. Alef's inspected `vendor_core_only` implementation copies the
facade; that is not itself proof that every local dependency is vendored. Under
the accepted public-crate strategy, remaining dependencies must be available at
verified registry versions before a registry-backed source installation passes.
Keep source packaging, isolated native compilation, installed runtime tests,
reproducible upstream generation, and publication as distinct gates. Do not
hand-edit generated manifests or publish placeholder packages to
work around these gaps. This inventory is a prerequisite, not source-gem approval.

The separate `prepare_core_ruby_source.rb` helper now exercises Alef's default
registry preparation in an isolated committed snapshot and creates an allowlisted
source archive. The binding manifest's external path is removed by Alef, not a
hand edit. A real local installation reached Cargo but failed to resolve the
unpublished facade in the crates.io index. This resolves the preparation/file
selection gap for that development archive, not registry resolution, compilation,
installed conformance, reproducibility or release readiness.

## Python source distribution gate

`workspace-scripts/check_core_python_source.py` checks a trusted development
sdist using Python 3.11+ on POSIX. It rejects oversized exports, links, traversal,
duplicate archive entries, missing package inputs and incorrect license bytes.
Cargo metadata must place the typed facade, Python binding and all local package
dependencies inside the extraction, without unrelated packages. It then builds
an isolated debug wheel with `--locked --offline`, one compiler job, and runs the
existing wheel-content/API-baseline and installed native-merge/e2e/app gate.
Cargo registry dependencies must already be cached; this is not a fresh-cache,
registry-install, release-profile or cross-platform claim.

Maturin 1.15.0's initial export carries the entire workspace lockfile even though
its exported workspace contains only the Python dependency closure. A locked
build rejects that lock. Explicit `--prepare` lets **Cargo** prune it offline in
a disposable extraction, checks that every retained package version/source/
checksum and dependency edge already existed, then asks **Maturin** to create
the final source archive. The wheel is built from a second, fresh extraction of
that retained archive with `--locked`; preparation never edits the checkout's
lockfile or generated manifests. Without `--prepare`, an incompatible input
lock fails rather than being silently repaired.

With maturin installed in the selected interpreter, run from the kernel:

```sh
# The output directory belongs to this invocation; choose a fresh one.
(cd packages/python && python -m maturin sdist --out ../../tmp/core-source-input)
python workspace-scripts/check_core_python_source.py \
  tmp/core-source-input/structuredmerge_core-0.2.0.tar.gz --prepare
```

The verified export invocation used `python -m maturin sdist --out
../../tmp/python-sdist-probe` from `packages/python`. The check requires 38 GiB
free before starting (30 GiB live floor plus an 8 GiB disposable-tree budget),
disables child core dumps, bounds capture and per-file writes, and limits the
compiler to 900 seconds. These are monitored process limits, not a filesystem
sandbox. Source trees, compiler output and the duplicate built wheel are removed
in guaranteed teardown; the installed-wheel checker removes its own environment.
Small reports, logs and the prepared sdist remain. Only trusted sources may be
passed because Cargo runs their build scripts.

### Local evidence, 2026-09-18

The kernel source at `75870d6` (guard implementation `fe639e4`) exported 17 local
crates. Cargo pruned 51 unrelated lock entries, retaining 192 packages with no new
or changed pins. Linux x86-64, CPython 3.14.2, Rust/Cargo 1.95.0 and Maturin 1.15.0
built the fresh final archive in 1m51s. Its installed wheel passed 60 boundary,
113 generated e2e and 114 test-app tests with LibCST 1.9.0. This is the existing
conformance-provider mode, not independent provider-package or registry evidence.

- Original archive SHA-256:
  `444341ba93ba341b9b8ec2b3121d835237e539029b82e7e25f5dc1e0bce44948`.
- Prepared archive SHA-256:
  `14e7535fb2596af0dc6e1132d073f488095fe6329c522420eedb01a15342ba95`.
- Source-built wheel SHA-256:
  `2c06225d9c41caae6b478288b5bce86bc32d3aae2452373b20a236e10ec0e39a`.
- Source report: `tmp/core-python-source-psdg912q/report.json`; final archive is
  in its `prepared/` directory. Installed report:
  `tmp/core-python-artifact-99_iudox/report.json`.
- Ten source-gate regression tests and 104 tooling tests pass; log:
  `tmp/python-source-tooling-final.log`. Cleanup tests now own their capture
  parent instead of mistaking another live job's capture directory for a leak.

The initial unprepared locked-build failure is retained in
`tmp/core-python-source-r3461xjf/report.json`. Both runs removed their disposable
trees; the superseded initial archive was removed after recording its digest.
This closes one local source-to-installed-wheel path; hosted source gates,
all supported platforms, release-profile builds, upstream-only generation,
reproducibility and publication remain separate requirements.
