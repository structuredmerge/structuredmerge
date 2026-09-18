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

## Linked-kernel version identity

Both binaries now implement `--version` and `--version --json`. The JSON retains
each executable's own name, package identity/version, the actual linked typed
kernel's version and target CLI contract. An empty typed capability query obtains
kernel identity without parsing, probing or grammar acquisition. The version
contract identifier is not a full-conformance claim. Extra arguments fail with
exit 2 even through the compatibility alias; output failures produce exit 3.

`smorg` now depends on `structuredmerge-core` by registry version plus workspace
path. Cargo updated its lock entry and the owning release-inventory script
updated the dependency edge; the required closure still contains 20 crates.
No generated public API changed and no new package publication is authorized.

Local verification: 94 Rust CLI tests and 41 tooling tests pass. Real-process
version tests use an empty PATH, verify exact executable/package/kernel identities
and ensure no grammar cache or other files appear. The portable suite now passes
19/20 for both names; only `languages --json` remains red. Reports:

- fixtures `tmp/cli-conformance-xdp724vj/report.json` (canonical);
- fixtures `tmp/cli-conformance-id4pq6qk/report.json` (compatibility);
- kernel `tmp/cli-discovery-tests-final.log` and `tmp/cli-discovery-script-tests.log`;
- current canonical binary `tmp/cli-discovery-bin/smorg`, SHA-256
  `9fb535ffd7523040cadd9b942be7811736d293277fad66508c50a9aab6d1350a`;
- current alias `tmp/cli-discovery-bin/smorg-rs`, SHA-256
  `b471cd292c264f68c71d0610990eef11660374c94ef738e0d78d6273d97e6f32`.

The earlier `cli-contract-bin` pair is superseded and removed; its reports above
retain provenance. The new 2.5 GiB compiler target and scratch Cargo metadata were
also removed. Keep the small current pair and reports, not per-run build trees.

The existing typed capability manifest alone does not meet Slice 1032's artifact,
asset, health and preflight-staleness requirements. `languages --json` must not be
made green merely by labeling that source-free declaration/observation report as
complete runtime availability. Full typed CLI operation routing and the other
shared contract/platform gates remain open.

## Explicit typed merge-driver lane

Both names now accept an explicit typed selection, for example:

```sh
smorg merge-driver --provider kernel.json --backend kernel.tslp.json \
  --profile kernel.json.nested.v1 --report merge-report.json BASE OURS THEIRS file.txt
```

The logical filename does not select a parser. The kernel's profile catalog and
capability request determine the parser language; the CLI accepts only the
matching `kernel.tslp.LANGUAGE` backend and registers it through the cached-only
factory. Arbitrary host backends are not available in this standalone binary.
Provider, family, dialect, profile and required capabilities remain conjunctive.
Repeatable capability flags become the sorted unique set required by the wire
contract. Registration is removed on exit from the operation scope.

This lane requires provider/backend/profile explicitly. Any typed selector (or
a `kernel.*` profile) enters it; missing or unsupported constraints never fall
back to the legacy route. Without a typed selection, the existing compatibility
route remains unchanged. The source-free observation used to obtain parser
requirements is not availability or default-authority evidence.

All source files must be regular UTF-8 files, limited to 8 MiB in aggregate.
Kernel execution uses a 200,000-node limit, 100 diagnostics, and a cooperative
10-second deadline. These are not process/memory sandbox or hard preemption
guarantees. No fallback is supported in this lane, including explicit legacy
fallback modes. `--require-profile-status available` still requires successful
execution; recommended/default status and legacy `--profile-report` are rejected.

The adapter executes one typed merge3 and retains its result in a
`structuredmerge.cli-report/v1` report. Unresolved conflict evidence, not `ok`
alone, selects exit 1. Check-only plus exit-code also returns 1 for a changed
successful output, with a distinct report outcome. Conflict policy defaults to
leave-ours; write uses only validated provider `conflicted_output` (for example
the `kernel.git.json.v1` profile), never CLI-generated markers. Output aliases of
base/theirs and report aliases are rejected. Complete staging precedes report
commit, then output commit; staging failures clean temporary files. Existing
stable-filesystem and per-file atomicity limits still apply.

Local Linux evidence:

- 96 standard CLI tests pass, plus both explicitly enabled warm-grammar tests
  (98 total), exercising both real binary names.
- Warm tests cover clean/check-only merges despite a `.txt` logical name, true
  conflicts with both write policies, malformed UTF-8/JSON, source-size limits,
  unsupported capabilities, report hardlinks, and output/report staging failures.
- Cold-cache tests return an error result without changing ours or acquiring
  grammars. Tests use isolated local library/cache directories and observe no
  HTTP(S) proxy connection; this is not a network sandbox claim.
- Existing local JSON grammar SHA-256:
  `8e44debe3f89057328a3db45fb5cbb98ddb41f4bcfca82a2a1aa268301a579d4`.
  Run warm tests with `SMORG_TEST_JSON_GRAMMAR` naming that existing library and
  `cargo test -p smorg --locked --test typed_driver -- --ignored`.
- Kernel log `tmp/typed-cli-verified.log`; the earlier
  `tmp/typed-cli-tests-final.log` retains the diagnosed capability-set failure.
- Portable discovery/argument subset stays 19/20, with `languages --json` still
  red: fixtures `tmp/cli-conformance-swjlt784/report.json` (canonical) and
  `tmp/cli-conformance-rwz2khr2/report.json` (alias).
- Retained `tmp/typed-cli-bin/smorg` SHA-256:
  `7d15f8a95796b1349667c16eb34b0a30c8a2316ade8cba0942e0bc123eaf5aca`.
- Retained `tmp/typed-cli-bin/smorg-rs` SHA-256:
  `0986c442c3cc32213007aa01fe0e90a84c659b58ce581a3e78aba13474c3124d`.

The 2.5 GiB compiler target and superseded discovery binaries are removed after
verification; reports, logs and the current binary pair remain. Compilation used
the 8 GiB target / 30 GiB free-space watchdog with debug/incremental output off.

Still open: typed default/project selection, diff migration, complete
pre-execution error reports (currently stderr/exit 2 with no fabricated result),
artifact availability, portable positive operation cases, installed real-Git
validation of this new lane, native CLIs, and platform/distribution gates.
Profiles needing native facts are not made supported by registering TSLP.
This is not full CLI conformance, package publication, or default promotion.

## Typed lane exercised through real Git

The existing real-Git gate now has a separate `--typed` mode, driven by fixtures
`conformance/cli-v1/typed-git.json`. Both retained binaries above pass all six
cases: independent edits, default leave-ours conflict, explicit conflict write,
delete/edit conflict write, malformed ours, and missing local grammar.

Git 2.55.0 actually invokes the copied executable through a repository-local
driver command. Executable and logical paths exercise shell quoting; the logical
path is a Unicode/apostrophe-containing `.txt` filename. The driver report must
retain that path, typed profile/provider identity and merge3 operation, with no
fallback. Conflict cases require canonical unresolved-conflict evidence, not
just a nonzero status or marker bytes. Clean stage-0 output and all unresolved
base/ours/theirs index stages are verified against source bytes. HEAD and all
branch blobs remain unchanged under `git merge --no-commit --no-ff`.

The parser-error and cold-cache cases demonstrate Git exit 1 with reported driver
exit 2 and `outcome: error`, not a manufactured conflict. Both preserve ours.
Real-Git runs are distinct from the preceding direct-process tests; copied local
executables are not proof of registry/release-archive installation.

Current reports:

- `tmp/typed-cli-git-m46wv8y4/report.json` (`smorg`, 6/6);
- `tmp/typed-cli-git-blzj4ojv/report.json` (`smorg-rs`, 6/6);
- fixture SHA-256 `835d920857e0de82bc00198277805c6784cd8b86b5c671209a04171cfab7dfa3`;
- `tmp/typed-cli-git-tooling.log`: all 45 tooling tests pass, including four new
  gate tests for fixture rejection and cleanup on success, case failure and
  report-write failure.

No compilation was needed: binary and grammar hashes are the same as above.
Each case's temporary repository, executable copy and grammar copy was removed
before the next case; only small evidence remains. Free space is 134 GiB.
See `workspace-scripts/CLI_GIT_GATE.md` for reproduction and bounded-resource
policy. This closes initial local POSIX real-Git evidence for the explicit JSON
lane, not hosted integration, packaging/distribution, other languages/platforms,
full report conformance, typed diff or default-selection authority.
