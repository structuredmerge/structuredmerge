# Owned Git configuration steps

`smorg git install` and `smorg-rs git install` now use the shared CLI report
envelope while retaining legacy top-level report keys. The installer no longer
ignores write failures, checks every scope against `.gitattributes`, deletes
unmarked matching lines, or replaces the user's include entries.

## Scope and remaining limits

- Local manages an owned section in the current directory's `.gitattributes`.
  It retains the existing experimental attribute strings, not newly approved
  typed-provider defaults or automatic merge-driver setup.
- Global manages diff configuration at the last path returned by Git's
  `var GIT_CONFIG_GLOBAL`; an explicit override is honored. Tests always use
  disposable overrides, never the operator's global configuration.
- Include-file resolves the repository config with Git, writes a fragment beside
  that config, then appends its quoted absolute include. Other include entries
  are preserved. Linked worktrees use the resolved shared config and fragment.
- Semantic-diff retains `smorg-rs diff-driver`. Builtin-diff does not install an
  external `cat` command; its driver section records that local attributes are
  still needed. Successful scoped configuration is not complete Git-driver setup.

JSON and human output explicitly describe those limits. `setup_complete`,
`driver_configuration_verified` and `default_approved` remain false. Checks
validate the owned component, not every Git precedence rule, executable lookup,
provider availability or a real merge/diff. The provider-aware driver/default
policy and distribution/platform gates remain unfinished.

## Ownership and failure behavior

Only exact versioned installer sections are replaced or retired. The framing
records profile and whether the installer created the file; it is an ownership
convention, not authenticated provenance or protection from a hostile editor.
User configuration is not reparsed/reformatted. Unknown versions, edited bodies,
duplicate/misordered boundaries, profile-mismatched undo and unowned include
fragments fail closed. Unmarked historical installer output is treated as user
data and is not automatically adopted or deleted.

Install/check/undo are idempotent where applicable. Dry-run and check create no
files or directories. Duplicate flags and conflicting check/undo or check/dry-run
options fail before planning; undo/dry-run is supported. Existing comments, blank
lines, CRLF and missing final newlines remain unchanged. Undo restores the
original bytes when no outside edits intervened, removes an installer-created
file only if no unowned content remains, and preserves preexisting empty files.
If appended user content relies on the owned separator after an unterminated
original line, undo retains one newline so it does not concatenate user lines.

Inputs and planned targets are capped at 1 MiB and must be regular UTF-8 files.
Symlink targets and aliased managed include directories reject; Unix hard links
also reject. Git location lookups have concurrently drained bounded pipes and a
ten-second deadline; Unix process groups are retired. Non-Unix descendant
cleanup and complete platform behavior are not verified. Unsupported Git location
queries fail rather than triggering HOME/XDG guesses. Lookup output is not echoed
into diagnostics, which avoids exposing configuration contents.

All targets are planned before applying changes, and each target is reread before
its write to detect intervening edits. Files are staged and atomically replaced
individually. Fragments precede includes on installation; includes precede
fragment removal on undo. Reports distinguish succeeded, failed and not-run steps
and include only source descriptors/digests, not the user's configuration values.
`planned_after` and `change_planned` are plans, not attestations of a failed write.
On partial failure, completed owned steps remain for explicit retry/undo. This is
not multi-file atomicity, crash-safe rollback, an adversarial filesystem lease or
preservation of every inode/ACL/xattr property. Empty managed parent directories
may remain after undo; the installer does not claim ownership of existing dirs.
Input/ownership rejection exits 2, I/O/report failures exit 3, and no failure is
silently presented as success. A report-write failure can occur after a successful
configuration write, so retries must remain idempotent.

## Local evidence

- All 126 CLI tests pass, including three warm grammar tests. Eight new process
  tests exercise both names: exact-byte local lifecycle, empty/missing-file undo,
  isolated global config, preserved include entries/quoted paths, modified sections
  and invalid flags, symlink/hard-link guards, partial failure and retry, linked
  worktrees, and invalid/oversized/nonregular targets. Unit tests also cover stale
  plans, unknown marker versions and stdout failure.
- Log: `tmp/git-install-final-tests.log`; all 74 tooling tests pass in
  `tmp/git-install-tooling.log` with the current binary supplied explicitly.
- Real Git merge/diff reports: `tmp/typed-cli-git-eq86jbtb/report.json` and
  `tmp/typed-cli-git-wmpmk64_/report.json`.
- Both names pass the twelve portable conflict-review cases. Fixture reports:
  `tmp/cli-conflict-review-yam9ke4w/report.json` and
  `tmp/cli-conflict-review-m7kdrp4p/report.json`.
- Discovery stays 19/20 because `languages --json` remains missing. Fixture
  reports: `tmp/cli-conformance-ucgpdm3f/report.json` and
  `tmp/cli-conformance-pmvhwrft/report.json`.
- Current binaries live in `tmp/git-install-bin/`. SHA-256:
  `smorg`: `6a4ed84752c1b020826b77fd42451e551962edb50eafbd27096424215032992c`;
  `smorg-rs`: `5034dd40c4c585431d969abbe63a4e3e5e609bc5610f9a0aef9ea7d459b5debe`.

The 2.5 GiB compiler target and superseded 47 MiB conflict-review binary pair
are removed after verification. Tests remove their disposable repositories,
configuration targets and copied files. Current binaries and small evidence remain.
No package is published, no upstream Alef change is pushed, and no driver is
promoted as a default by this work.
