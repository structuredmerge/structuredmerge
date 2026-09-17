# Pre-publication Ruby core artifacts

Build the committed generated extension from `packages/ruby`:

```sh
bundle install
bundle exec rake compile
bundle exec ruby ../../workspace-scripts/check_core_ruby_artifact.rb
```

The default command builds an allowlisted platform gem, installs it into an
isolated consumer and runs runtime/generated tests, linkage and RBS validation.
It does not publish or establish source-gem or other-platform compatibility.

Both helpers below clean disposable staging on completion, failure, `abort`,
and Ruby-handled interruption. The installed gate removes its package copy,
consumer, and gem home; source preparation removes its checkout snapshot, tar
archive, and staging package. Reports/logs remain in the printed scratch directory,
and explicit package exports remain in the requested output directory. Failure
evidence is recorded separately in `failure.json`. The default installed report's
artifact path identifies the temporary package that was tested, not a retained
package; use `--package-only` when an exported package is needed.

Check free disk space before and during these commands, reserve at least 20 GiB,
and allocate a separate job budget. Cleanup is not a disk quota or a guard for
the preceding native compilation. Forced kills or machine crashes can bypass
teardown; inspect and clean that invocation's scratch paths before resuming.
Do not retain a new installed environment for every benchmark run.

## Source archive preparation (not installation approval)

The separate source helper prepares **committed HEAD**, not dirty source files,
in a fresh kernel `tmp/` snapshot. It invokes Alef's registry preparation and
builds an explicitly allowlisted source gem with no prototype files or binaries.
The worktree's generated manifests remain untouched:

```sh
ruby workspace-scripts/prepare_core_ruby_source.rb \
  --alef /absolute/path/to/alef \
  --output tmp/new-core-source-export
```

Preparation requires Git, tar, Ruby and Python >=3.11 (TOML audit). The output
directory must not already exist. The report records the source commit, executable
digest and version output for Alef, artifact and per-file digests, and explicit
`not_run` states for registry resolution, installation and runtime tests. It
always leaves upstream-generation verification and publication approval false.
This helper is not a publishing command and does not claim source-build parity.

The local 2026-09-17 prepared archive reached the native Cargo build during a real
isolated Ruby installation, then failed because crates.io could not resolve
`structuredmerge-core`. Publish no package merely to unblock this check: the
20-crate typed publication inventory and all upstream-generation/release gates
must be satisfied first. See `contracts/TYPED_RELEASE_INVENTORY.md`. Once those
prerequisites are met, rerun source installation and the full installed tests;
successful preparation alone is not enough.

## Installed binary artifact gates

The `typed-core-ruby-artifact` CI matrix runs that full installed gate on Linux
x86_64 (Ruby 3.2 and 4.0), Linux ARM64, macOS ARM64/x86_64 and Windows UCRT
(Ruby 4.0). Windows builds select the GNU target to match RubyInstaller.
These are requested CI checks, not proof of support until each hosted leg passes.
The separately named legacy checkout regression matrix remains unchanged; it
does not substitute for installed typed-core tests or authorize prototype releases.

Local minimum-runtime check (2026-09-17): Ruby 3.2.11 built after supplying
`BINDGEN_EXTRA_CLANG_ARGS='-include stdbool.h'` for this machine's Ruby header /
libclang mismatch. This is a local build workaround, not a CI requirement.
The initial installed runtime suite passed 35/36 examples: the fresh-process
registered-host shutdown case timed out after completing its parse and GC work.
Regenerating with the local interrupt-aware dispatcher fix resolves that failure:
Ruby 3.2.11 now passes all 36 runtime and 98 generated examples, plus linkage and
RBS checks. This Linux result does not establish the hosted matrix or eliminate
the local header workaround. The generator fix remains local, so upstream-only
generation and release approval are still open.
The rebuilt Ruby 4.0.6 artifact also passes the same 36 runtime / 98 generated
examples, linkage and RBS checks after this change.

CI artifact producers may build/export without installing the test harness:

```sh
bundle exec ruby ../../workspace-scripts/check_core_ruby_artifact.rb \
  --package-only ../../tmp/core-artifacts
```

This uses the **same** generated gemspec, file allowlist, reviewed API checks,
current Ruby ABI/version restriction and linkage validation. It exports one
`structuredmerge-core` platform gem and `core-ruby-artifact.json`, containing its
SHA-256, platform, Ruby ABI, version requirement and explicit verification states.
Existing artifact/report paths are not overwritten. Use a fresh output directory
for each build. All staging stays under kernel `tmp/`.

Package-only reports mark installed runtime tests, generated e2e tests and RBS
validation as not run/not validated. A consumer must install the exported gem
under the matching Ruby minor/ABI and execute its own tests; successful packaging
does not satisfy that gate. No prototype gem, second registry, executable product,
runtime merge callback or publication is involved.

Before installing a downloaded export, consumers must check it under the Ruby
that will run their tests:

```sh
ruby workspace-scripts/verify_core_ruby_export.rb \
  tmp/core-artifacts/structuredmerge-core-0.2.0-x86_64-linux.gem \
  tmp/core-artifacts/core-ruby-artifact.json
```

This independently checks the digest, package metadata, exact file allowlist,
platform, Ruby minor/ABI and producer report states. Missing or mismatched fields
fail before installation. A passing result is only an integrity/compatibility
check: it neither loads the extension nor authenticates the producer. The report
and gem must come from a trusted build artifact; consumers must still install,
require the typed core and run their tests. The check never executes gem payloads.

This producer uses already generated, reviewed bindings. It does not regenerate
via a forked Alef or establish upstream-only generation (D4). The native extension
must be freshly compiled by the preceding build step. No stale-extension provenance
claim is made from the presence of a shared library alone.

The Ruby repository's current/coverage/dependency-HEAD workflow has a local
typed-core producer/consumer conversion, with matched Ruby ABI and explicit export
verification. Its hosted validation remains open until the coordinated commits
are available remotely; a local workflow edit is not a hosted test result.

Kernel CI's `ruby-package` job now exports only the allowlisted typed core. The
`mise run ruby-package` task uses the same package-only path; its fixed export
directory must be absent or use the direct command above with a fresh directory.
The former `release-ruby-host.yml` workflow is removed: prototype tags and manual
runs must never publish that package. The workflow remains recoverable in Git
history, and legacy API/lifecycle regression sources are retained separately.
The legacy checkout job explicitly builds that historical extension with
`bundle exec ruby ../../workspace-scripts/build_legacy_ruby_regression.rb` from
`packages/ruby` before `bundle exec rake spec` and `check_ruby_api.rb`. The helper
uses the retained rb_sys configuration in a fresh kernel `tmp/` directory and
copies only a successfully built library into the ignored checkout `lib/` path.
It creates no gem. Neither the installed typed-core gate nor its package producer
calls this helper or depends on this legacy extension. This keeps historical
regressions executable without restoring the abandoned publication product.
Historical cold-path tooling is explicitly named `legacy-ruby-cold-paths`, not a
core release gate. A new core publication workflow still requires the plan's
upstream-generation, ABI/platform, provenance and installed-consumer gates.
