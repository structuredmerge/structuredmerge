# StructuredMerge Rust

Rust implementation of the StructuredMerge contract.

This repository is one of four peer launch implementations: [Go](https://github.com/structuredmerge/structuredmerge-go), [TypeScript](https://github.com/structuredmerge/structuredmerge-typescript), [Rust](https://github.com/structuredmerge/structuredmerge-rust), and [Ruby](https://github.com/structuredmerge/structuredmerge-ruby). The language repos are not separate products. They consume the same public spec and shared fixture corpus so tools can choose the runtime surface that fits their environment.

Project links:

- Website: <https://structuredmerge.org>
- Implementations overview: <https://structuredmerge.org/implementations.html>
- Conformance model: <https://structuredmerge.org/conformance.html>
- Specification: <https://github.com/structuredmerge/structuredmerge-spec>
- Shared fixtures: <https://github.com/structuredmerge/structuredmerge-fixtures>

## Workspace

This is a Cargo workspace for StructuredMerge packages.

Package directories:

- [`ast-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-merge)
- [`ast-template`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ast-template)
- [`binary-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/binary-merge)
- [`go-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/go-merge)
- [`json-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/json-merge)
- [`kettle-rusty`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/kettle-rusty)
- [`markdown-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/markdown-merge)
- [`pest-toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pest-toml-merge)
- [`pulldown-cmark-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/pulldown-cmark-merge)
- [`ruby-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/ruby-merge)
- [`rust-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/rust-merge)
- [`plain-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/plain-merge)
- [`toml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/toml-merge)
- [`tree-haver`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/tree-haver)
- [`typescript-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/typescript-merge)
- [`yaml-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-merge)
- [`yaml-serde-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/yaml-serde-merge)
- [`zip-merge`](https://github.com/structuredmerge/structuredmerge-rust/tree/main/crates/zip-merge)

## Conformance

Integration tests should consume the shared fixture corpus from the sibling `../structuredmerge-fixtures` checkout. A ruleset, fixture, diagnostic shape, or review outcome should mean the same thing whether exercised through Go, TypeScript, Rust, or Ruby.

Use the spec repository's conformance matrix for the current launch-readiness snapshot:

- <https://github.com/structuredmerge/structuredmerge-spec/blob/main/conformance-matrix.md>
- <https://github.com/structuredmerge/structuredmerge-spec/blob/main/IMPLEMENTATION_STATUS.md>

## Development

Standard repo tasks are exposed through `mise` and native Rust tooling.

Common checks:

- `mise run check`
- `cargo test`

## Status

Early implementation work. Public compatibility claims should be tied to shared fixtures and documented conformance status rather than runtime-specific assumptions.
