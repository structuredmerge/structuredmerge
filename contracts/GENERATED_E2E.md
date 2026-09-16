# Generated typed-core conformance suites

`[crates.e2e]` in `alef.toml` maps the shared fixtures repository's
`polyglot/typed-core` directory to Alef-owned Ruby and Python suites under `e2e/`.
Regenerate with `alef e2e generate`; edit the fixtures or generator, never the
generated suites. Generation requires the fixtures repository beside the kernel.

The existing installed-artifact checks copy the generated tests into isolated
consumer directories and run them against the installed core gem or wheel,
without sibling source paths. The typed-core jobs in `current.yml` invoke these
same checks. Python uses pytest for generated tests in addition to unittest for
the existing boundary tests; Ruby uses the core artifact's standalone RSpec suite.

The profile fixture checks native profile IDs, Rust execution ownership, and
experimental/default-approval flags. The Python native-merge family additionally
executes LibCST callbacks and Rust merge decisions for independent edits, conflicts,
syntax rejection, unsupported owners, and exact BOM/CRLF/Unicode preservation.
The test-only `native_merge_fixture` module registers the boundary suite's existing
provider, builds typed requests, calls the installed binding, and unregisters in
`finally`. It contains no merge decisions or rendering logic. It also reexports
the binding's profile function because Alef uses the default target module for
all generated imports. The installed-artifact runner supplies this module; a
standalone generated Python project currently needs that support too.

The existing hand-authored Psych and LibCST boundary tests remain necessary.
Ruby intentionally excludes the Python-only merge family. Mapping Psych/YAML
and the remaining canonical corpus is still an open Phase 4 gate.

Current generation requires local Alef corrections, including root-array field
access and preservation of explicit null assertion values. Upstream release reproducibility remains open;
this is not approval to publish packages or claim full target conformance.
