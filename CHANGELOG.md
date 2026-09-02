# Changelog

## Unreleased

- Add a shared source-preserving top-level declaration merge kernel.
- Add fail-closed, source-preserving TypeScript and TSX three-way merging through
  TreeHaver's normalized TSLP parser interface.
- Expose the TypeScript three-way provider through the benchmark adapter without
  claiming source-preserving two-way support.
- Add a validated TreeHaver normalized-tree index so format providers share
  fail-closed node lookup, root validation, child traversal, and descendant search.
- Share parser diagnostic conversion and role-attributed three-way parse failure
  construction from `ast-merge` instead of redefining them in format providers.
