# Installed kernel CLI real-Git gate

Build/install the kernel with Cargo into a dedicated directory, then run:

```sh
cargo install --path crates/smorg-rs --debug --locked --root "$PWD/tmp/cli-install"
python3 workspace-scripts/check_installed_cli_git.py tmp/cli-install/bin/smorg \
  --fixtures ../fixtures/diagnostics/slice-951-git-driver-json-integration/git-driver-json-integration.json
```

The gate runs every canonical case, without the delete/edit skip retained in the
older in-process CLI test. Each case creates actual base/ours/theirs commits,
registers only the supplied executable as the repository-local merge driver and
invokes `git merge`. A machine report proves the driver executed. The gate checks
exit codes, output, diagnostics, clean commit/index state and unresolved index
stages, including preservation of ours on parse failure. Git's `%P` placeholder
is already shell-quoted; adding another quote layer changes the logical path.

Driver reports, merge stdout/stderr and a digest-bearing gate report remain under
kernel `tmp/installed-cli-git-*`. Disposable repositories are removed per case,
even on assertion or evidence-write failure. Evidence lives in numbered directories
outside the removed repositories; CI upload paths match that layout.
Global/system Git config and
inherited Git environment overrides are excluded; no user Git configuration or
repository is changed. All validation remains active under Python optimization.

Delete/edit conflicts must include review markers, not merely preserve ours.
An absent ours owner is represented by an empty side in an appended review block;
the original ours bytes remain outside that block. The report distinguishes this
placement from an owned-region conflict. Other unsupported CLI families, full portable CLI conformance,
platform installation, registry distribution and default approval remain separate.

The `installed-kernel-cli` job in `.github/workflows/current.yml` installs this
checkout with Cargo and runs the gate against both executable names on Linux.
It checks out an immutable fixture revision and uploads digest-bearing reports
plus driver reports and merge stdout/stderr on success or failure. It does not
publish binaries, invoke Alef, or infer macOS/Windows conformance. A local run is
not evidence that the hosted workflow has executed; that remains a separate gate.

## Explicit typed lane (local POSIX gate)

Use an already built/installed binary and an explicitly supplied existing JSON
grammar. The gate does not build, download, or publish either artifact:

```sh
python3 workspace-scripts/check_installed_cli_git.py tmp/workflow-query-bin/smorg \
  --typed --fixtures ../fixtures/conformance/cli-v1/typed-git.json \
  --grammar-library tmp/typed-tslp-cache/tree-sitter-language-pack/v1.17.0/libs/libtree_sitter_json.so
```

Repeat with `smorg-rs`. Typed mode selects `kernel.git.json.v1` explicitly and
registers `kernel.tslp.json` through the CLI's cached-only route. It runs the
fixture set through actual `git merge --no-commit --no-ff`, checking independent
edits, leave-ours conflicts, explicit conflict writes, delete/edit conflicts,
parse failure and a cold grammar cache. Every case requires a driver report;
driver errors must remain errors, even though Git returns 1 and leaves unmerged
index stages for both driver exit 1 and driver exit 2.

For the clean case it also registers the typed external-diff command and invokes
`git diff --ext-diff base ours`. The diff report must retain the logical path,
kernel provider identity and nonempty typed diff2 change IDs. The worktree,
stage-0 index and HEAD must remain unchanged. `git_diff_verified` records this
separately in the gate report; conflict/error cases do not claim diff coverage.
It also compares an actual empty Git tree against `ours` in both directions,
letting Git supply the absent-side protocol token. Added/deleted reports require
the zero-byte source digest and corresponding kernel owner classifications.
`git_absent_side_diff_verified` records these checks separately. This currently
proves the explicit JSON profile, not absent-side behavior for every provider.

The gate checks the stage-0 index for a clean merge and all three unresolved
stages otherwise, plus unchanged HEAD and branch source blobs. There is no merge
commit: fixture history is imported into a disposable repository using Git's
fast-import protocol, without changing or disabling user commit-signing settings.
Git invokes a copied executable in a path containing spaces and an apostrophe,
on a logical `.txt` filename containing an apostrophe and Unicode. Only repository
local Git attributes/configuration are written. This is local executable-copy
installation evidence, not a registry install, release archive or hosted CI gate.

All disposable repositories, copied executables and copied grammar libraries
are removed per case, including failed cases and report-write failures. Small
reports, actual sources, index evidence and merge stdout/stderr remain in
`tmp/typed-cli-git-*`. The gate checks a 20 GiB reserve plus a 1 GiB job budget,
caps artifacts at 128 MiB each and source fixtures at 16 KiB each, and enforces
20-second process-group timeouts. Inputs/executables are trusted local test
artifacts, not a sandbox. Isolated grammar/cache directories and a monitored
proxy detect acquisition regressions without claiming network isolation.

Both modes now use a shared POSIX command observer. Standard output/error each
have a 1 MiB acceptance budget, with file-backed capture instead of unbounded
in-memory pipes. An inherited 8 MiB per-file hard limit bounds writes between
checks; core dumps are disabled. The observer checks free space during execution,
not just between cases, kills the process group on timeout, output excess or a
20 GiB reserve breach, and retires descendants when the leader exits. Legacy
commands retain a 30-second deadline; typed commands use 20 seconds. Input is
limited to 1 MiB. Capture scratch is outside the Git worktree so `git add .`
cannot stage it, and is removed on success, launch failure and observation failure.
Per-file limits and polling do not constitute an aggregate quota or a sandbox for
hostile executables; only trusted local artifacts belong in these gates.

Local resource-safety verification passes all 71 tooling tests. New tests cover
both output streams flooding, deadlines, live reserve breach, inherited limits,
descendant retirement, launch failure, capture exclusion from Git staging, and
legacy cleanup on command/report failure. Evidence:
`tmp/cli-git-resource-tooling.log`. Both executable names pass typed merge/diff
and legacy Git gates; logs are `tmp/cli-git-resource-{smorg,smorg-rs}.log`,
`tmp/cli-git-resource-legacy.log`, and `tmp/cli-git-resource-legacy-smorg-rs.log`.
No compiler run is needed for this tooling change. Per-run repositories and
captures are removed; retained evidence is small. Hosted execution remains unproven.

The installed Linux CI job now runs both modes separately. It pins published
fixtures `be4bf25d59bceac6a4adcc27dea081a2bab4c798`. The preceding legacy tests
prepare JSON through TSLP's public loader; the subsequent typed gates receive
the exact locked-1.17.0 Linux cache library path and fail if it is absent. They
never acquire a grammar. A separate signed-asset test checks that prepared
file's bytes, without claiming identity of a retained loaded object. Both typed
executable names cover all six merge cases plus modified/added/deleted diffs,
and their report JSON is retained by CI.

Local revalidation passed for both retained binaries:
`tmp/typed-cli-git-wabj4ek9/report.json` and
`tmp/typed-cli-git-3jvkaslo/report.json`; transient Git workspaces and copies were
removed by the runner. All 94 tooling tests and workflow lint pass. Fixtures
publication included 23 verified signed commits, 14 fixture-tool unit tests,
seven validated provider snapshots, and syntax checks of tracked JSON files.
This does not assert that every new native fixture was executed during that
publication review. No kernel workflow was dispatched or claimed green remotely.
Neither mode proves full portable CLI conformance, all platforms, distribution
readiness or default authority.
