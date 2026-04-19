# Structured Merge Rust

Cargo workspace for the Rust implementation of the Structured Merge library
family.

Initial workspace crates:

- `tree-haver`
- `ast-merge`
- `text-merge`
- `json-merge`

## Development

Standard repo tasks are exposed through `mise` and `cargo`:

- `mise run format`
- `mise run format-check`
- `mise run lint`
- `mise run typecheck`
- `mise run test`
- `mise run check`

The Rust workspace uses:

- `cargo fmt` for formatting
- `cargo clippy` for linting
- `cargo check` for type checking
- `cargo test` for unit and integration tests
