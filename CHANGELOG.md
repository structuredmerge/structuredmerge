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
- Advertise Python as the first reviewed generic-provider combination through
  the Rust benchmark adapter without claiming generic two-way support.
- Route the Bash provider through the shared normalized named-owner projection
  without broadening its top-level-function-only merge contract.
- Route Go's three-way function ownership through the same shared projection
  while retaining Go-specific import and declaration behavior in its substrate.
- Route Rust-language three-way function ownership through the shared
  projection while preserving its separate TSLP and native analysis contracts.
- Route TypeScript and TSX three-way declaration ownership through the shared
  projection, including the existing export and ambient wrapper policy.
