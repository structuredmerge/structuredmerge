# Independently packaged Psych parser host

The Ruby native layer now provides `Psych::Merge::CoreParserHost` in the existing
`psych-merge` gem, through the optional `psych/merge/core_parser_host` require.
The ordinary entry point is unchanged. Importing the adapter does not register
a provider; the caller explicitly registers `ruby.psych` through the generated
core and owns its lifetime. Package identity is `psych-merge`; native parser
identity is Psych with its actual version. No kernel test projector is imported.

The adapter supplies native AST nodes, exact UTF-8 byte positions and requested
Psych extensions. It contains no ownership, matching, diff, conflict or rendering
algorithm. The explicit `kernel.yaml.native_mapping.v1` profile remains Rust
owned; currently it supports analyze, diff2 and merge3, not merge2. Unsupported
profile structures remain Rust rejections. Comments/token streams, other
languages/dialects, bare CR and invalid UTF-8 are not silently accepted. Psych
syntax messages are redacted rather than copying potentially private source.

## Explicit native version compatibility

This optional integration accepts Psych `~> 5.5.0`, the tested series, separately
from the existing Ruby-native workflows. Applications must declare it alongside
the generated core in their bundle. A different loaded version is unavailable
in probes and returns `psych.unsupported_version` in direct parse callbacks.
There is no runtime gem-version switch, parser substitution or source rewrite.

The first artifact bundle omitted an explicit Psych dependency and selected MRI
4.0.6's bundled Psych 5.3.1 (libyaml 0.2.5). It rejected the valid source
`"\uFEFFé: one\r\nlast: old"` with `Psych::SyntaxError`. A fresh invocation that
loaded only Psych 5.3.1 reproduced this rejection without the core present;
Psych 5.5.0 accepted the same bytes. This is evidence for the scoped version
restriction, not a claim about every intervening/future Psych release. The
provider never strips the BOM to get a successful parse.

## Installed evidence (2026-09-18)

Ruby commit `0a57f6883` contains the adapter and tests. On Linux x86-64, MRI 4.0.6
and Psych 5.5.0, nine installed-artifact examples pass
through `bundle exec kettle-test` / turbo_tests2. They cover:

- package/parser identity, supported/unsupported probes and source-free import;
- independent edits composed in Rust and output reparse through Psych;
- competing edits remaining conflicts with no clean output/fallback;
- exact BOM, Unicode, CRLF and absent-final-newline preservation;
- typed source identity and native-extension opt-in;
- redacted malformed source and explicit unsupported options/encoding;
- Rust analysis/diff and rejected merge2;
- guarded kernel execution, stale same-descriptor re-registration and
  cancellation before native probes/parses;
- rejection of an unverified runtime before calling Psych.

Both core and provider are built gems installed into the job's disposable gem
home, with realpath checks in the spec. Tests are copied to a fresh consumer;
no sibling implementation or kernel projector is loaded. The job uses shared
**installed** dependencies for the rest of the Ruby stack: this is not a fresh
registry/cache or fully hermetic dependency-closure test. Bundler resolves its
temporary lock locally and records it. The checked-in spec lives at Ruby
`gems/psych-merge/spec/integration/core_parser_host_spec.rb`; setting
`STRUCTUREDMERGE_TYPED_PSYCH_TEST=true` makes missing prerequisites fatal.

- Core gem SHA-256:
  `b427daabf5cf677ef42937c2926a8dd12f453fd75022594b6e8a51338f17a8c4`.
  Export report: `tmp/psych-native-core-export/core-ruby-artifact.json`.
- Psych gem SHA-256:
  `352c3ea571031d65bf9332a18207f4252d36585f32621447a8d687ffe5927b3a`.
- Ruby report and temporary bundle lock:
  `ruby/tmp/worktrees/typed-core-main/tmp/psych-core-artifact-20260918-2761135-xrlw43/`.
- Kernel log: `tmp/psych-native-installed-final.log`.
- Local reproduction runner: Ruby `tmp/verify-native-psych.rb CORE_GEM`.
  It packages the native-layer gem, installs both artifacts, runs the installed
  spec, and removes its consumer and gem home in teardown. Run with core dumps
  disabled and the plan's free-space checks; it does not compile Rust.

Full downstream suites, hosted CI, other Ruby/platform combinations, package
publication and default authority remain open. No Alef change, Rust rebuild,
push or publication occurred.
Temporary installations were removed after each run; intermediate gem copies
were removed after verification, retaining the final package and small reports.

## Full installed binding gate

The existing artifact checker now accepts an explicit independent provider:

```sh
ruby workspace-scripts/check_core_ruby_artifact.rb \
  --provider-gem /absolute/path/to/psych-merge-7.1.9.gem
```

It verifies the archive identity/entry point, installs the local core/provider
and their declared dependencies into an isolated gem home, and declares Psych
5.5.x in the temporary bundle. It does not copy the conformance projector in this
mode. The helper supplies counters/batch observations and aliases the provider
ID to `ruby.typed.psych` for existing fixtures, but delegates native parsing and
probing to the installed adapter. Package/parser provenance is retained. This
fixture alias is not a new production registration API; the native-layer tests
above independently cover the production ID `ruby.psych`.

MRI 4.0.6/Psych 5.5.0 passes all 53 boundary, 110 generated e2e and 110 generated
app tests in this mode, plus linkage, RBS and API-baseline checks. The core and
provider hashes are the same as above. Report and resolved dependency lock:
`tmp/core-ruby-artifact-20260918-2768447-nuqp9u/`; log:
`tmp/psych-installed-full.log`. Unlike the initial nine-example consumer gate,
this run does not use shared installed Ruby dependencies or sibling source paths.
Core/provider remain local artifacts, so it is not their registry-install gate.

The default checker retains the conformance-only mode as a separate regression
oracle. It also passes 53/110/110 tests with an intentionally inherited installed
mode flag, proving that the runner clears it unless `--provider-gem` is supplied.
Report: `tmp/core-ruby-artifact-20260918-2770481-d4ae59/report.json`; log:
`tmp/psych-conformance-regression.log`. Reports explicitly distinguish modes and
record both fixture/production identities for independent-provider runs.

All 107 tooling tests pass, including invalid argument combinations, wrong
provider archives and missing-provider rejection without projector fallback.
Log: `tmp/psych-provider-tooling.log`. Both runs removed their package, consumer
and gem-home directories; only reports and the independent-mode bundle lock
remain. These are local Linux runtime results, not hosted CI or the complete
Ruby ABI/platform matrix.

## Ruby CI wiring (local, not yet hosted)

The Ruby `current.yml` core-artifact producer now packages `psych-merge` from
the Ruby candidate revision and requires the full `--provider-gem` gate before
exporting the core to downstream jobs. It reuses the existing core compilation,
limits Cargo to one job with incremental compilation disabled, preserves gate
logs/reports, and removes its explicit compiler target in an `always()` step.
No package is published and provider defaults are unchanged.

All 11 Ruby workflow/dependency structural tests and actionlint pass locally.
Replaying the exact packaging step produced the same provider SHA-256 recorded
above; the disposable packaging directory was removed. Replay report:
`ruby/tmp/worktrees/typed-core-main/tmp/psych-ci-package-report.json`.
The workflow requires the kernel's new checker to be available on its checked-out
main branch. These local histories have not been pushed for this slice, and
there is no hosted CI result yet.
