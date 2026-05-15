# StructuredMerge Rust

StructuredMerge Rust provides Cargo crates for native tools that need portable
structured-merge contracts, fixture-backed behavior, and embeddable merge
components.

The workspace includes the core AST/review contracts, parser substrate support,
format-specific merge crates, binary/ZIP planning helpers, provider adapters,
and a Rust packaging recipe crate.

Project links:

- Website: <https://structuredmerge.org>
- Implementations: <https://structuredmerge.org/implementations.html>
- Specification: <https://github.com/structuredmerge/structuredmerge-spec>
- Shared fixtures: <https://github.com/structuredmerge/structuredmerge-fixtures>

## Install

Add the crates your tool needs:

```toml
[dependencies]
ast-merge = "0.1"
tree-haver = "0.1"
```

Binary and ZIP use StructuredMerge-prefixed package names on crates.io:

```toml
structuredmerge-binary-merge = "0.1"
structuredmerge-zip-merge = "0.1"
```

## Command

The Rust implementation ships the implementation-specific `smorg-rs` command.
Use that name in git configuration unless a package manager or local install has
provided a `smorg` symlink.

Package-manager formulas may expose the selected implementation as `smorg`.
For a local user-created symlink:

```sh
ln -s "$(command -v smorg-rs)" ~/.local/bin/smorg
```

```sh
git config merge.smorg-rs.driver 'smorg-rs merge-driver %O %A %B %P'
git config diff.smorg-rs.command 'smorg-rs diff-driver'
smorg-rs conflicts diff path/to/file-with-conflicts.go
smorg-rs languages --gitattributes
```

`merge-driver` updates Git's `%A` file by default, or writes to `--output` when
used outside git. `diff-driver` accepts both the two-argument local form and the
seven- or nine-argument forms Git passes to external diff commands.
`conflicts diff` reports conflict-marker regions in a file that already contains
Git conflict markers.

## Crates

Core:

- [`tree-haver`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/tree-haver) - parser substrate, byte ranges, backend adapters, and binary tree contracts.
- [`ast-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-merge) - AST merge contracts, diagnostics, planning, review, replay, and nested-merge vocabulary.
- [`ast-template`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-template) - template/session transport contracts.

Format libraries:

- [`plain-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/plain-merge)
- [`json-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/json-merge)
- [`yaml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-merge)
- [`toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/toml-merge)
- [`markdown-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/markdown-merge)
- [`ruby-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ruby-merge)
- [`go-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/go-merge)
- [`rust-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/rust-merge)
- [`typescript-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/typescript-merge)
- [`binary-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/binary-merge)
- [`zip-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/zip-merge)

Provider and recipe crates:

- [`yaml-serde-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-serde-merge)
- [`pest-toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pest-toml-merge)
- [`pulldown-cmark-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pulldown-cmark-merge)
- [`kettle-rusty`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/kettle-rusty)

## Portability

The Rust crates are developed against the shared StructuredMerge fixtures. Those
fixtures define the cross-language behavior expected from the Go, TypeScript,
Rust, and Ruby implementations. Conformance checks live in crate tests and in
the shared spec/fixture tooling rather than in a static launch-status document.

## Development

Common checks:

- `mise run check`
- `cargo test`
