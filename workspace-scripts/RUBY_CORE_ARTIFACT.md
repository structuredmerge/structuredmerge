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

The Ruby repository's old `rust-host`/HEAD-dependency workflow still needs conversion
to this typed-core producer/consumer path. Its old repository, prototype artifact
name and prototype-only load check must be replaced together, with matched Ruby
ABI and explicit install checks. Current/coverage's obsolete prototype publication
switches also require migration; do not publish the prototype to unblock them.
