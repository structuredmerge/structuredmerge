# StructuredMerge core

The typed facade for the shared Rust kernel and its generated language bindings.
It has no dependency on `structuredmerge-host-prototype-core`.

Implementation starts with the source/operation boundary from specification
Slices 722 and 1024–1027:

- immutable, validated source storage is owned by TreeHaver;
- source bytes, digests, encoding and line-ending evidence are preserved;
- semantic roles distinguish analyze, diff2, directional merge2 and merge3;
- merge-provider and parser-backend selections are separate typed values.

`OperationInputs` validates inputs before provider dispatch. It is not yet the
complete operation envelope: policy, extensions, result evidence, parser
selection, and merge execution follow in subsequent implementation slices.
No merge capability or generated binding is advertised by this initial module.

The next vertical slice must route a real native parser through TreeHaver and
execute matching, ownership, conflict decisions and source-preserving rendering
in Rust. Passing an encoded operation through a host-owned merge does not meet
that requirement. Existing kernel mechanics should be reused; new parallel
merge algorithms and independent parser-selection registries are not intended.

Run the boundary tests with:

```sh
cargo test -p structuredmerge-core -p tree-haver --locked
```
