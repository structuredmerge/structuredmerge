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
Historical cold-path tooling is explicitly named `legacy-ruby-cold-paths`, not a
core release gate. A new core publication workflow still requires the plan's
upstream-generation, ABI/platform, provenance and installed-consumer gates.
