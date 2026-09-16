# Typed structural operation reports

`report_structural_operations(Vec<CrisprOperationRequest>)` returns a typed
`CrisprBatchOperationReport`: operation count, ordered kinds and complete typed
operation profiles. Both generated bindings expose this function without a JSON
string argument or result. The core calls ast-crispr; classification remains in
that crate, and its older JSON report methods serialize the same typed reports.

Each request supplies operation kind, source/destination requirements,
replacement source, capture flag and missing-destination flag. Existing empty
string defaults are preserved: replace/required/none/explicit_text. Unknown
strings are retained and marked through the corresponding `known_*` flags,
not silently mapped onto a known kind. Those flags describe known vocabulary;
they do not validate combinations or authorize source changes.

This is introspection, not execution. No source is selected, parsed, edited or
written. Reports cannot establish structural selector equivalence, byte-range
correctness, parser availability, or filesystem apply behavior. The explicit
source-edit primitive is separate.

`report_structural_match`, `report_structural_selection`, and
`report_structural_destination` likewise accept typed requests and return the
owning crate's typed reports. Their older JSON methods serialize those same
reports. Selection and destination preserve the existing empty-string defaults;
match strings remain literal. Missing comment regions remain `None`/`nil`,
distinct from an unknown region string. Unknown vocabulary is retained with
false `known_*` flags, not executed or silently substituted.

Boundary, limit, and template/session report contracts remain open, as does
the Ruby ast-crispr consumer cutover. Legacy report regression
tests remain and pass; the facade does not depend on the legacy host crate.
