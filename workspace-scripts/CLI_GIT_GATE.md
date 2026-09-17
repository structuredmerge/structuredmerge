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

Current known failure: the delete/edit conflict returns status 1 but has no review
markers when ours has deleted the owner. The localized renderer requires present
regions for every side. This gate deliberately fails until that rendering contract
is implemented; it does not weaken the fixture or count preservation of ours as
conformance. Other unsupported CLI families, full portable CLI conformance,
platform installation, registry distribution and default approval remain separate.
