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

Repository state, driver reports, merge stdout/stderr and a digest-bearing gate
report remain under kernel `tmp/installed-cli-git-*`. Global/system Git config and
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
python3 workspace-scripts/check_installed_cli_git.py tmp/typed-error-bin/smorg \
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

The existing legacy mode and its CI invocation remain separate. The typed
fixture history must be integrated and hosted grammar provisioning decided
before adding this mode to CI. Neither mode proves full portable CLI conformance,
all platforms, distribution readiness or default authority.
