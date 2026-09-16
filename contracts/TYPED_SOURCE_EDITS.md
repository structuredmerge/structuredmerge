# Typed explicit source edits

`apply_explicit_source_edits(SourceEditRequest, SourceEditLimits)` is a pure
in-memory primitive exported from `structuredmerge-core` and generated into Ruby
and Python. It reuses `ast_merge::apply_source_edits`; it does not depend on the
legacy host crate, parse source, select AST nodes, or write files.

The request carries a nonempty request ID, one verified UTF-8 `SourceInput` with
role `source`, and explicit half-open byte ranges plus replacement strings.
Ranges refer to the original source, not intermediate edits. The renderer sorts
edits and rejects overlap, ambiguous same-start edits, out-of-range/reversed
ranges, and offsets inside UTF-8 characters. Failure raises a typed core error
and exposes no partial output. No-op, insertion, replacement, and deletion use
the same renderer; unchanged bytes are retained exactly.

Limits bound input bytes, edit count, and projected output bytes before output
allocation. Successful results carry request ID, the validated original source
descriptor, output text, and edit count. The descriptor describes the input,
not the output. Codes are `request.invalid`, `source.invalid`, `resource.limit`,
and `source_edit.rejected`.

This is the typed replacement primitive for the old explicit-edit entry point,
not the complete portable operation envelope or ast-crispr capability. It has
no parser verification, AST-selection authority, cancellation control, render
provenance report, or filesystem apply semantics. Those remain separate work.
The Ruby ast-crispr adapter still needs migration and its downstream tests.

Python source requests can be constructed entirely through the public package:
`SourceInput`, `SourceDescriptor`, and `LineEndings` share native DTO identities.
The Alef configuration explicitly reexports the latter two, and a local generator
fix makes that setting effective for defaultable input types. The installed
test verifies public construction and execution; no `_native` constructor is
required. This remains a development API and depends on the local-only Alef fix.
