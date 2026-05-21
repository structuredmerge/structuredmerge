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

The family is intentionally layered:

- [`tree-haver`][rust-tree-haver] provides parser portability, backend discovery, byte ranges, and runtime capability reporting.
- [`ast-merge`][rust-ast-merge] provides the cross-format merge substrate: shared contracts, diagnostics, review state, and execution reports.
- Family crates such as [`markdown-merge`][rust-markdown-merge], [`yaml-merge`][rust-yaml-merge], and [`toml-merge`][rust-toml-merge] own parser-neutral behavior for one format family.
- Provider crates such as [`pulldown-cmark-merge`][rust-pulldown-cmark-merge], [`yaml-serde-merge`][rust-yaml-serde-merge], and [`pest-toml-merge`][rust-pest-toml-merge] bind those families to concrete Rust parser libraries.

| Crate | Layer | What it provides |
| --- | --- | --- |
| [`tree-haver`][rust-tree-haver] | Parser substrate | Parser backend registry, byte ranges, node wrappers, source locations, and binary tree contracts. |
| [`ast-merge`][rust-ast-merge] | Merge substrate | AST merge contracts, diagnostics, planning, review, replay, and nested merge vocabulary. |
| [`ast-template`][rust-ast-template] | Template substrate | Template/session transport contracts. |
| [`ast-crispr`][rust-ast-crispr] | Structured edits | AST edit recipes for generated blocks and template-owned regions. |
| [`ast-merge-git`][rust-ast-merge-git] | Git integration | Merge-driver, diff-driver, conflict inspection, and language registry plumbing for `smorg-rs`. |
| [`plain-merge`][rust-plain-merge] | Text | Plain-text fallback contracts. |
| [`json-merge`][rust-json-merge] | JSON and JSONC | Object/array-aware JSON merge behavior using [tree-sitter-language-pack][tree-sitter-language-pack] where selected. |
| [`yaml-merge`][rust-yaml-merge] | YAML | YAML-family merge contracts. |
| [`toml-merge`][rust-toml-merge] | TOML | TOML-family merge contracts. |
| [`markdown-merge`][rust-markdown-merge] | Markdown | Markdown-family merge contracts. |
| [`ruby-merge`][rust-ruby-merge] | Ruby source | Ruby source merge contracts. |
| [`go-merge`][rust-go-merge] | Go source | Go source merge contracts. |
| [`rust-merge`][rust-rust-merge] | Rust source | Rust source merge contracts. |
| [`typescript-merge`][rust-typescript-merge] | TypeScript source | TypeScript source merge contracts. |
| [`binary-merge`][rust-binary-merge] | Binary | Binary tree planning contracts. |
| [`zip-merge`][rust-zip-merge] | Archives | ZIP archive planning helpers. |
| [`yaml-serde-merge`][rust-yaml-serde-merge] | YAML provider | Uses [`serde_yaml`][serde-yaml] as the YAML parser/emitter provider path. |
| [`pest-toml-merge`][rust-pest-toml-merge] | TOML provider | Uses [Pest][pest] with [`pest_grammars`][pest-grammars] as the TOML parser provider path. |
| [`pulldown-cmark-merge`][rust-pulldown-cmark-merge] | Markdown provider | Uses [pulldown-cmark][pulldown-cmark] as the Markdown parser provider path. |
| [`kettle-rusty`][rust-kettle-rusty] | Recipe tooling | Cargo workspace maintenance and package recipe helpers. |

[rust-tree-haver]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/tree-haver
[rust-ast-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-merge
[rust-ast-template]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-template
[rust-ast-crispr]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-crispr
[rust-ast-merge-git]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-merge-git
[rust-plain-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/plain-merge
[rust-json-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/json-merge
[rust-yaml-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-merge
[rust-toml-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/toml-merge
[rust-markdown-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/markdown-merge
[rust-ruby-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ruby-merge
[rust-go-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/go-merge
[rust-rust-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/rust-merge
[rust-typescript-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/typescript-merge
[rust-binary-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/binary-merge
[rust-zip-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/zip-merge
[rust-yaml-serde-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-serde-merge
[rust-pest-toml-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pest-toml-merge
[rust-pulldown-cmark-merge]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pulldown-cmark-merge
[rust-kettle-rusty]: https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/kettle-rusty
[tree-sitter-language-pack]: https://github.com/kreuzberg-dev/tree-sitter-language-pack
[serde-yaml]: https://docs.rs/serde_yaml
[pest]: https://pest.rs/
[pest-grammars]: https://docs.rs/pest_grammars
[pulldown-cmark]: https://github.com/pulldown-cmark/pulldown-cmark

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
