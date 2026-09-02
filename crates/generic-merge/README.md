# generic-merge

`generic-merge` is the explicitly limited StructuredMerge provider for
languages that have a TreeHaver-normalized parser but no dedicated language
substrate.

It is not a claim that every Tree-sitter grammar has correct merge semantics.
The provider currently supports only three-way merges where every non-comment
top-level node has exactly one direct `name` field, all names are unique, all
three revisions retain the same ordered owner identities, and source outside
those owners is byte-identical. Output is reparsed through the same TreeHaver
language-pack provider. Anything outside that contract fails closed.

Dedicated language substrates supersede this generic policy because they can
model imports, declarations, comments, gaps, nesting, and language-specific
identity more accurately.
