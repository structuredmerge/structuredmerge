# StructuredMerge Kernel

StructuredMerge provides a Rust kernel and generated bindings for tools that need portable
structured-merge contracts, fixture-backed behavior, and embeddable merge
components.

The workspace includes the core AST/review contracts, parser substrate support,
format-specific merge crates, binary/ZIP planning helpers, provider adapters,
and generated host bindings. Rust project tooling, including
[`kettle-rusty`][rust-kettle-rusty], lives in the separate
[Rust native-layer repository](https://github.com/structuredmerge/structuredmerge-rust).

Project links:

- Website: <https://structuredmerge.org>
- Implementations: <https://structuredmerge.org/implementations.html>
- Specification: <https://github.com/structuredmerge/structuredmerge-spec>
- Shared fixtures: <https://github.com/structuredmerge/structuredmerge-fixtures>

The [distribution architecture](docs/distribution.md) defines how the Rust
kernel, parser providers, application bundles, and Alef-generated host adapters
fit together.

## Behavioral Authority

The mature Ruby implementation is the StructuredMerge behavioral golden
master. This Rust implementation is a conformance consumer and currently lags
Ruby in behavior and defect fixes. Shared specifications and fixtures are
portable evidence admitted from reviewed Ruby behavior; historical Rust
behavior does not override Ruby when they disagree.

Names containing `parity` describe only the dimensions asserted by their cited
fixtures. They do not claim complete behavioral equivalence with Ruby unless a
versioned conformance profile explicitly says so.

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

[rust-tree-haver]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/tree-haver
[rust-ast-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-merge
[rust-ast-template]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-template
[rust-ast-crispr]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-crispr
[rust-ast-merge-git]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-merge-git
[rust-plain-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/plain-merge
[rust-json-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/json-merge
[rust-yaml-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/yaml-merge
[rust-toml-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/toml-merge
[rust-markdown-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/markdown-merge
[rust-ruby-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/ruby-merge
[rust-go-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/go-merge
[rust-rust-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/rust-merge
[rust-typescript-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/typescript-merge
[rust-binary-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/binary-merge
[rust-zip-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/zip-merge
[rust-yaml-serde-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/yaml-serde-merge
[rust-pest-toml-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/pest-toml-merge
[rust-pulldown-cmark-merge]: https://github.com/structuredmerge/structuredmerge/tree/main/crates/pulldown-cmark-merge
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

Rust crate `smorg` ships the kernel executable `smorg` and the compatibility alias
`smorg-rs`. Both execute the same kernel commands; no symlink is required. For a
local development build:

```sh
cargo build -p smorg --locked
target/debug/smorg --help
```

Existing opt-in Git configurations may continue to use the compatibility alias:

```sh
git config merge.smorg-rs.driver 'smorg-rs merge-driver %O %A %B %P'
git config diff.smorg-rs.command 'smorg-rs diff-driver'
smorg-rs conflicts diff path/to/file-with-conflicts.go
smorg-rs languages --gitattributes
```

This naming change does not approve a default Git-driver switch or establish
registry/platform release readiness. External subcommand dispatch is not yet
implemented. The source directory remains `crates/smorg-rs` while path-based
benchmark mappings and consumers migrate.

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

- [`tree-haver`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/tree-haver) - parser substrate, byte ranges, backend adapters, and binary tree contracts.
- [`ast-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-merge) - AST merge contracts, diagnostics, planning, review, replay, and nested-merge vocabulary.
- [`ast-template`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/ast-template) - template/session transport contracts.

Format libraries:

- [`plain-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/plain-merge)
- [`json-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/json-merge)
- [`yaml-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/yaml-merge)
- [`toml-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/toml-merge)
- [`markdown-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/markdown-merge)
- [`ruby-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/ruby-merge)
- [`go-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/go-merge)
- [`rust-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/rust-merge)
- [`typescript-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/typescript-merge)
- [`binary-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/binary-merge)
- [`zip-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/zip-merge)

Provider crates:

- [`yaml-serde-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/yaml-serde-merge)
- [`pest-toml-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/pest-toml-merge)
- [`pulldown-cmark-merge`](https://github.com/structuredmerge/structuredmerge/tree/main/crates/pulldown-cmark-merge)

## Portability

The Rust crates are developed against the shared StructuredMerge fixtures.
Those fixtures encode portable behavior derived from the Ruby golden master and
reviewed for cross-runtime use. Conformance checks live in crate tests and in
the shared spec/fixture tooling rather than in a static status document.

## Development

Common checks:

- `mise run check`
- `cargo test`
