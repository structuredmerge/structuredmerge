# Owner-render byte evidence

`merge_source_preserving_owners_with_evidence` exposes the source fragments
emitted by the existing owner renderer. The compatibility entry point still
returns the shared three-way result, but it uses the same verified renderer.
There is no second merge algorithm or post-hoc search for matching text.

Each retained owner and layout gap records a deterministic fragment ID, source
revision and byte range, output byte range, SHA-256, and optional owner ID.
Whole-source selections have one fragment (empty output has none). The renderer
appends the selected source slices directly rather than replacing substrings
and discarding their origins. The independent verifier rejects gaps, overlaps,
incomplete coverage, duplicate/empty IDs, missing revisions, invalid ranges,
unequal bytes, and incorrect digests. Verification failure removes output;
all non-clean results have no output-fragment records.

The native merge facade binds revisions to the actual verified input IDs and
roles, includes input/output source descriptors, and exposes generated typed
`RetainedSourceSegment` records in both Ruby and Python. Output IDs avoid input
ID collisions. Installed-artifact tests independently verify the partition and
digests using each runtime's own byte slicing and SHA-256 implementation.

This is the exact output-partition component of Slice 1027, **not** the complete
`structuredmerge.preservation-evidence/v1` envelope. It does not yet prove
policy-specific comment/unknown-syntax claims, dispositions for unselected or
deleted protected regions, authorized synthesized/conflict-marker fragments,
or all per-property required/passed states. A clean internal merge is not a
claim that every public preservation policy is supported. The current profiles
emit retained-source fragments only and cannot hide synthesized output as
source-preserved bytes.
