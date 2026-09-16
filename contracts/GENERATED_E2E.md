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

The initial fixture checks native profile IDs, Rust execution ownership, and
experimental/default-approval flags. It does not exercise parsing or merging.
The existing hand-authored Psych and LibCST tests remain necessary. Mapping the
canonical native merge corpus and provider setup into generated tests remains
an open Phase 4 gate.

Current generation requires local Alef corrections, including root-array field
access in generated assertions. Upstream release reproducibility remains open;
this is not approval to publish packages or claim full target conformance.
