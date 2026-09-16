# Native Python declaration profile

Rust derives whole top-level declaration owners from TreeHaver-validated LibCST
nodes, native field edges, name values, and byte spans. It supports simple
single-name assignments and undecorated function/class definitions. Nested
bodies remain whole exact-source owners; this is not recursive Python merging.
Duplicate names (including NFKC-equivalent spellings), imports, decorated
declarations, chained/destructuring assignments, and other unsupported
top-level forms fail closed. This is not an authority claim for all Python
program semantics or a Ruby golden-master parity claim.

TreeHaver selects and calls the native parser; `ast_merge::typed_merge` shares
input/verification orchestration with YAML. The existing shared owner engine
does matching, conflict classification, layout checks, and rendering. Python
supplies no merge decisions. Exact source, not LibCST code generation, supplies
output bytes. The test adapter's code generation equality check only verifies
that native parsing retained the input.

The test-only provider in `packages/python/tests/libcst_facts.py` projects native
nodes and uses [LibCST byte-span metadata](https://libcst.readthedocs.io/en/latest/metadata.html).
It corrects UTF-8 BOM offsets and rejects non-UTF-8 encodings/bare CR rather
than inventing coordinates. Rust uses NFKC for owner identity, consistent with
[Python's identifier rules](https://docs.python.org/3.14/reference/lexical_analysis.html#names),
while preserving the original spelling. Runtime/Unicode-version compatibility
still needs the broader platform matrix.

Run the installed-wheel tests (with LibCST installed in that environment):

```sh
python -m unittest discover -s packages/python/tests -v
```

The tests explicitly reject loading the package from the source checkout.
They cover independent assignments, whole function/class bodies, structured
conflicts, malformed revisions, exact bytes, changed-layout rejection, and
unsupported profiles. Expected Python outputs are explicit test assertions,
not independently reviewed fixture authority yet.

Open gates include the full portable operation/preservation envelopes, public
Python DTO ergonomics, lifecycle stress, generated e2e/CI coverage, reviewed
capability evidence, native-layer packaging, registry name/dependency-closure
checks, and upstream-only reproducible generation. Alef currently represents
Python `serde_json::Value` extension fields as JSON strings and silently maps
invalid JSON to null; that conversion must be fixed or rejected at the boundary
before claiming forward-compatible extension transport. No package publication
or default-provider authority is granted by these tests.
