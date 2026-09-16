# Typed native-parser integration

`typed::merge_mapping_sources` is an internal Rust family operation over the
TreeHaver `ParseService`, not the completed public operation envelope.
It assigns roles explicitly, requires one selected backend across revisions,
derives top-level YAML entry ownership in Rust, and delegates matching,
conflict classification, and exact-source rendering to the shared
`ast_merge::merge_source_preserving_owners` implementation. Render verification
uses the same TreeHaver registry snapshot and explicit selected backend.

The initial profile consumes `structuredmerge.extension/ruby-psych/v1` syntax
facts: native node kinds, scalar `value`/`plain`, mapping `style`, and native
`anchor`/`tag` fields. Mapping child edges identify alternating `key` and `value`
relationships. These are syntax facts, not parser-supplied merge owners or
decisions. Only a single block mapping with unique, unambiguous string keys is
currently accepted. Complex keys, explicit tags, anchors/aliases, merge keys,
and ambiguous plain scalar keys fail closed. Nested values are whole owners;
recursive YAML merge parity is unfinished. Changed unowned layout also fails
closed instead of guessing comment attachment.

Run the real native-parser gate with Ruby/Psych installed:

```sh
cargo test -p yaml-merge --test typed_psych_merge --locked -- --ignored
```

The test-only `tests/support/psych_facts.rb` process harness implements parsing
only, registered behind TreeHaver's provider trait. It never receives a merge
operation or executes merge logic. It handles Psych's character columns,
implicit EOF row, and BOM offsets while retaining exact UTF-8 bytes; bare-CR
coordinates are explicitly unsupported in this harness. The integration tests
exercise independent edits, conflicts, malformed input, explicit selection,
role ordering, nested values, and preservation of comments, CRLF, BOM,
non-ASCII bytes, and missing final newlines. Current CI invokes this gate
explicitly; normal workspace tests mark it ignored because Ruby is optional
for Rust-only development.

This harness is not a production process protocol, generated binding, or
installed-package gate. The generated Ruby `ParserHost`, full diagnostics and
preservation envelopes, wider YAML golden-master coverage, comment ownership,
and Python LibCST integration remain required by the active plan. No prototype
package is used or published by this path.
