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
- Route Go analysis through TreeHaver's normalized TSLP parser and add a
  fail-closed, source-preserving top-level function three-way merge provider.
- Route Rust analysis through TreeHaver's normalized TSLP parser and add a
  fail-closed, source-preserving top-level function three-way merge provider;
  keep the spanless native `syn` backend explicitly unsupported for merging.
- Add a thin Bash provider that projects top-level function ownership from
  TreeHaver's normalized TSLP AST and reuses the shared source-preserving
  declaration kernel for fail-closed three-way merges.
- Add a shared normalized-tree projection for strict, uniquely named top-level
  owners so language providers can reuse one source-range and identity contract.
- Add an experimental generic TreeHaver/TSLP provider for languages without a
  dedicated substrate, limited to exact-layout three-way merges of uniquely
  named top-level owners and explicit fail-closed behavior everywhere else.
