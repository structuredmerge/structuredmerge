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

## Package Family

StructuredMerge Rust is a layered crate family. The lower layers provide parser,
range, AST, merge, and template contracts; format crates apply those contracts to
specific languages and data formats; provider crates bind a format family to a
parser or serializer; workflow crates package Rust project maintenance and
Git-driver behavior.

Each crate README keeps this section short and links here. This root guide is
the implementation inventory for Rust users who need to choose crates,
understand backend coverage, or wire a focused backend into a test suite.

| Crate | Layer | What it provides |
| --- | --- | --- |
| [`tree-haver`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/tree-haver) | Parser substrate | Parser backend registry, byte ranges, node wrappers, source locations, and binary tree contracts. |
| [`ast-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-merge) | Merge substrate | AST merge contracts, diagnostics, planning, review, replay, and nested merge vocabulary. |
| [`ast-template`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-template) | Template substrate | Template/session transport contracts. |
| [`plain-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/plain-merge) | Text | Plain-text fallback contracts. |
| [`json-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/json-merge) | JSON and JSONC | Object/array-aware JSON merge behavior. |
| [`yaml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-merge) | YAML | YAML-family merge contracts. |
| [`toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/toml-merge) | TOML | TOML-family merge contracts. |
| [`markdown-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/markdown-merge) | Markdown | Markdown-family merge contracts. |
| [`ruby-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ruby-merge) | Ruby source | Ruby source merge contracts. |
| [`go-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/go-merge) | Go source | Go source merge contracts. |
| [`rust-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/rust-merge) | Rust source | Rust source merge contracts. |
| [`typescript-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/typescript-merge) | TypeScript source | TypeScript source merge contracts. |
| [`binary-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/binary-merge) | Binary | Binary tree planning contracts. |
| [`zip-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/zip-merge) | Archives | ZIP archive planning helpers. |
| [`yaml-serde-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-serde-merge) | YAML provider | Serde-backed YAML parser/emitter provider path. |
| [`pest-toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pest-toml-merge) | TOML provider | Pest-backed TOML parser provider path. |
| [`pulldown-cmark-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pulldown-cmark-merge) | Markdown provider | Pulldown-cmark-backed Markdown parser provider path. |
| [`kettle-rusty`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/kettle-rusty) | Recipe tooling | Cargo workspace maintenance and package recipe helpers. |

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

Semantic merge-driver coverage is fixture-backed for JSON. Other language and
format paths are git-compatible command surfaces without semantic driver
coverage.

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
the shared spec/fixture tooling rather than in a static status document.

## Development

Common checks:

- `mise run check`
- `cargo test`
