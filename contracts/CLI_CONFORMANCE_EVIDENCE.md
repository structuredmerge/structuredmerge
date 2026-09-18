# Shared CLI v1 migration evidence

The specification's `CLI_DISPATCH_CONTRACT.md` now defines the shared command
surface in addition to external dispatch and file safety. The target includes
typed provider selection, structured reports, availability introspection and
explicit fallback policy. Current binaries do not claim full v1 conformance.

The fixtures repository's `conformance/cli-v1/manifest.json` and
`tools/check_cli_contract.py` execute twenty portable discovery/argument cases.
Initial execution against the retained report-paths build passed 9 and failed
11, including four malformed merge invocations that changed the current-file
sentinel. Evidence: fixtures `tmp/cli-conformance-7gfemw_j/report.json`.

After the argument guards, both executable names pass 18/20. Missing
`--version --json` and `languages --json` remain failures, not skipped cases.
Ambiguous named/positional sources, excess operands, duplicate singleton
options, conflicting logical paths, and missing diff path values now reject
before writes. Incomplete named source roles now include an error diagnostic.
Merge/diff help, option terminators and valid named source forms are covered;
empty optional Git display prefixes remain accepted.

Verified on local Linux:

- `cargo test -p smorg --locked`: 91 tests pass, including actual processes for
  both binary names, shared negative fixtures, valid argument forms, existing
  dispatch, Git, report alias and staged-write tests.
- Fixture runner: 7 self-tests pass (manifest validation, real subprocesses,
  exact output capture, mutation detection, JSON failure, timeout and output
  budgets with cleanup).
- Canonical report: fixtures `tmp/cli-conformance-rg90envi/report.json`.
- Compatibility report: fixtures `tmp/cli-conformance-a97hnor4/report.json`.
- Kernel test log: `tmp/cli-contract-tests-final.log`.
- Retained canonical binary: `tmp/cli-contract-bin/smorg`, SHA-256
  `203db597b2c8ae19212a7080816881cc3fc364c9e63bea02750c96d569c630c1`.
- Retained compatibility binary: `tmp/cli-contract-bin/smorg-rs`, SHA-256
  `5e5e8a8b6c754f60c99be3e84b146d51759e9b63c05ba197f1e9da89a37db51c`.

Both binaries were built with incremental/debug information disabled. The build
target peaked at approximately 2.5 GiB under an 8 GiB target / 30 GiB free-space
watchdog and was removed after verification. Small reports and the new binary
pair remain. The older report-paths binary pair is superseded; its baseline
report retains executable provenance.

Remaining work includes machine discovery, full report schemas/validation,
typed-operation CLI routing, explicit selectors and no-fallback default,
positive portable operation/Git cases, native CLIs, and distribution/platform
gates. Existing legacy merge/fallback implementations were not migrated by
these argument fixes. Shared fixture and kernel changes must be integrated
together before hosted tests can consume the new manifest. No package was
published and no default authority changed.
