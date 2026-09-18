# Git absent-side typed diff

The explicit typed diff lane now normalizes the exact `/dev/null` token in
Git's seven/nine-argument external-diff protocol to an in-memory zero-byte
source. It never opens the device or creates a source temporary file. Two-file
invocations and merge sources still require regular files; aliases and missing
paths are not absent-side tokens. Hash/mode arguments remain metadata.
The protocol is documented by [Git](https://git-scm.com/docs/git#Documentation/git.txt-GIT_EXTERNAL_DIFF).

The kernel JSON owner comparator now accepts a successfully parsed zero-byte
native document as an empty owner set for diff only. Native language, root,
error flags, span, and children are checked. Nonempty documents use the existing
AST owner analysis; whitespace-only input still rejects. Analyze and merge
continue to require JSON values. The CLI does not fabricate classifications,
replace absence with `{}`/`null`, or use text comparison as fallback.

This is source comparison, not file-existence/mode-change reporting: a regular
zero-byte file and a Git absent side have the same bytes. Both empty sources
compare cleanly. Broader Git file metadata semantics and other providers are
not proved by this JSON-profile evidence.

## Verification

- 128 CLI tests pass, including the warm grammar tests for both executable
  names and added/deleted protocol roles, exact empty-source hashes and kernel
  owner classifications: `tmp/git-null-final-tests.log`.
- The JSON regression in that log verifies added/deleted owner sets, both-empty
  equality, rejected incompatible parser language/whitespace, and unchanged
  analyze rejection. It is a focused test, not the full JSON-family suite.
- All 74 tooling tests pass: `tmp/git-null-tooling.log`.
- Both real-Git gates pass all six merge cases and modified/added/deleted
  external diffs. They compare an actual empty tree in both directions, check
  empty-source hashes/classifications, and preserve worktree/index/HEAD.
  Reports: `tmp/typed-cli-git-21zjwktc/report.json` and
  `tmp/typed-cli-git-95hk04d0/report.json`.
- Alef verification, Ruby/Python API baseline checks, and the 20-crate typed
  release inventory pass. No generated API changes or binding rebuilds are
  claimed; retained older extension/wheel binaries do not gain this fix.
- Current binaries: `tmp/git-null-bin/smorg`, SHA-256
  `df1be2c23a3e4ec27ffbd388bb9e0f116bb64f577fcac39c8d6630ac5c4ebba1`;
  `tmp/git-null-bin/smorg-rs`, SHA-256
  `97cb483bd2ef0e43acfb81c2c9f30931128c792efa87394ee652e37fef6f7df8`.

Builds use one compiler job, no incremental/debug data, disabled core dumps,
and live 30 GiB free-space/8 GiB target guards. The 2.5 GiB build target and
superseded Git-install binaries are removed after verification. Current binaries
and small reports remain; Git test repositories and artifact copies are cleaned
per case. No publication, default promotion, or upstream Alef push is performed.
Full availability, provider registry integration, and distribution gates remain.
