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
The former host-publication workflow is removed. The current core publication
workflow requires the plan's
upstream-generation, ABI/platform, provenance and installed-consumer gates.

## Post-publication RubyGems install gate

After a non-dry-run `structuredmerge-core` RubyGems publication, the release
workflow runs `workspace-scripts/check_core_ruby_registry_install.rb` with the
exact release version. The verifier installs from RubyGems into a fresh
`GEM_HOME`, clears checkout load paths, checks the gem and native extension
origins, validates the installed RBS, and runs the core, generated e2e, and
consumer-app suites. A failed install or test fails the release workflow. Dry
runs skip this job.

The verifier also accepts `--gem PATH` for local candidate testing. On
2026-10-31 it passed against the x86_64 Linux platform gem candidate
`structuredmerge-core 0.2.1` (SHA-256
`8ca7a4f185c74f0c6fcba2af803013dde642b47e142a4e2015092e3267d554ce`):
53 core, 110 generated e2e, and 110 consumer-app examples passed, with RBS and
extension-path checks green. This is local candidate evidence only. The exact
registry-installed source gem gate remains pending publication and the
registry-backed Rust dependency closure; no package was published.


Local candidate rerun (2026-11-01) against the current committed Rust workspace:
the x86_64 Linux / Ruby 4.0.9 platform artifact was built with
--package-only (SHA-256
e0403ab03da895dfc5a7f071d67428e7becf316a0079fe0403d35462932a71ad). The first
isolated install exposed a stale generated Rust analyze fixture that expected
#[test] to be rejected, despite the shared owner-profile fixture now expecting
native syn support. The shared typed-core fixture was corrected and Ruby/Python
generated tests plus the Ruby consumer-app copies were regenerated/synchronized.
The verifier then passed 53 core, 116 generated e2e, and 116 consumer-app
examples, with installed native-extension path and RBS checks green. The report
states registry_install=local-candidate and publication_gate=false. This verifies
the current local platform candidate, not a RubyGems-installed source package;
no package was published.


Local candidate rerun after the shared Rust owner-order resolver (2026-11-01):
the freshly compiled x86_64 Linux / Ruby 4.0.9 artifact has SHA-256
`e7d438c1a3b70fe65a05ff1ed0b146af397e6748892103fde95fb48bd1e0bbfc`.
The isolated candidate verifier passed native-extension origin checks, RBS,
53 core examples, 116 generated e2e examples, and 116 consumer-app examples.
The report remains `registry_install=local-candidate` and
`publication_gate=false`; it does not verify registry installation or authorize
publication.


Local candidate rerun after the Rust reorder/removal parity slices (2026-11-01):
the current x86_64 Linux / Ruby 4.0.9 package-only artifact has SHA-256
`0106d1ed2d3fe515bed7247683e3cfbaf0d5cf461b4c6a871e95977861a96e4e`.
It installed into a fresh GEM_HOME isolated from checkout load paths. Native
extension origin, RBS validation, 53 core examples, 116 generated e2e examples,
and 116 consumer-app examples passed. The verifier report records
`registry_install=local-candidate` and `publication_gate=false`; this is
current local artifact evidence, not registry-install evidence.


Local candidate rerun after Rust membership-aware owner ordering (2026-11-01):
platform gem `structuredmerge-core 0.2.1` was rebuilt from structuredmerge commit
`9b91626` for x86_64 Linux / Ruby 4.0.9. SHA-256:
`0c0a0235db1eeda36c77c9d872be2449080022590d2baaf118967a471be47ff9`. The exact
archive installed into a fresh GEM_HOME with checkout load paths cleared. Native
extension origin checks, RBS validation, 53 installed core examples, 116 generated
E2E examples, and 116 consumer-app examples passed. The verifier reports
`registry_install=local-candidate` and `publication_gate=false`; registry-installed
verification remains open and no package was published.

Local candidate rerun after Rust membership/edit comment ownership (2026-11-01):
structuredmerge commit `0f1cc77942fc56725692b4234b5fe30ca24204ef` produced
`structuredmerge-core 0.2.1` for x86_64 Linux / Ruby 4.0.9. Artifact SHA-256:
`70e2613f7d09dea2d9ba935adfa438f049fe919921d0b8fc91c7ead35e9177f9`. The
exact gem installed into a fresh GEM_HOME with checkout load paths cleared. The
verifier report confirms native extension origin, installed core tests, generated
E2E, consumer-app tests, and RBS validation all passed. The report records
`registry_install=local-candidate` and `publication_gate=false`; this is local
candidate evidence, not registry-install or release evidence.

Local candidate rerun against current Rust core commit `6fc1309` (2026-11-01):
the x86_64 Linux / Ruby 4.0.9 platform gem `structuredmerge-core 0.2.1` has
SHA-256 `c797dc5ce13fb6bf34740d83a1b41ce917891e4f32bf521f635df2aa7a705503`.
The exact archive installed into a fresh `GEM_HOME` with checkout load paths
cleared. Native-extension origin, RBS validation, 53 installed core examples,
116 generated E2E examples, and 116 consumer-app examples passed. The report
records `registry_install=local-candidate` and `publication_gate=false`; this is
not evidence of a RubyGems registry installation, and no package was published.

Local candidate rerun against current Rust core commit `0fdce8d` (2026-11-01):
the x86_64 Linux / Ruby 4.0.9 platform gem `structuredmerge-core 0.2.1` has
SHA-256 `fcf17527cc442c1a4b562ea1c7c5f6786a372054084c1488480661851964f247`.
The exact archive installed into a fresh `GEM_HOME` with checkout load paths
cleared. Native-extension origin, RBS validation, 53 installed core examples,
116 generated E2E examples, and 116 consumer-app examples passed. The report
records `registry_install=local-candidate` and `publication_gate=false`; the
registry-installed release gate remains open, and no package was published.

Local candidate rerun after Bash inline-comment merge3 ownership (2026-11-01):
structuredmerge commit `7c1d3a8` produced the x86_64 Linux / Ruby 4.0.9
platform artifact with SHA-256
`9d8e8947eefb41bd926a213f4236809d50f7af8f4076b9a228c81e6655513172`. The exact
gem installed into a fresh `GEM_HOME` with checkout load paths cleared. Native
extension origin, RBS validation, 53 installed core examples, 116 generated E2E
examples, and 116 consumer-app examples passed. The verifier reports
`registry_install=local-candidate` and `publication_gate=false`; registry
installation and release verification remain pending, and no package was
published.

## Local installed-candidate result (2026-11-01, Rust f78fd84)

Ruby 4.0.9, Linux x86_64: 53 core, 116 generated E2E and 116 consumer-app examples pass.
Artifact structuredmerge-core-0.2.1-x86_64-linux.gem sha256
6a15482cf900d312364170faf03934fe37a7a474338b614b59e769228a18126c. Local candidate
evidence only; the registry-installed gate remains open pending publication prerequisites.
