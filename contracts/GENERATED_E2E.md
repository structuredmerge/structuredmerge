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

Twelve shared capability fixtures now cover all eight profile declarations with
an explicitly missing parser backend, undeclared operations/dialects, and empty
inventories. Helpers only construct typed queries and call the kernel; expected
outcomes live in the shared fixtures. They do not register providers or execute
merge logic. Successful/faulted native probes and snapshot retirement remain in
the installed boundary suites. This observation contract does not replace the
spec's full Slice 1026 workflow-provider negotiation requirements.

Current Alef output compares some nested Python string fields case-insensitively;
Ruby comparisons are exact. Boolean/null eligibility and authority assertions
are exact in both targets. Do not treat these Python identifier assertions as
proof of case-sensitive wire identity; generator classification needs follow-up.

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
Ruby intentionally excludes the Python-only merge family. Its Psych/YAML family
covers independent edits, conflicts, syntax failures, unsupported sequences,
and exact BOM/CRLF/Unicode preservation. The Ruby boundary specs and generated
suite share `packages/ruby/spec/native_merge_fixture.rb`; the artifact runner
copies the helper into the consumer directory and the generated suite explicitly
requires `./native_merge_fixture`, without changing Ruby's load path. The helper
registers the native provider, calls Rust, and unregisters in
`ensure`. No expected output or semantic merge logic lives in either helper.

The installed-artifact runners also stage `test_apps/python` and `test_apps/ruby`
outside the source tree, supply the same test-only native-provider helpers, and
execute their generated tests against the local installed wheel/gem. Python uses
the generated pytest configuration; Ruby uses the isolated artifact runner's
bundle (including its RBS checks), not the test app's registry dependency install.
These checks are explicitly pre-publication: `registry_install` remains
`not_run` in their reports. They neither install the test apps' published core
dependency from a registry nor prove that the apps run without helper staging.

Broader canonical corpus coverage and independently installable generated-project
support packaging remain open Phase 4 work; these suites prove only the stated
profiles. Run the ordinary `workspace-scripts/check_core_python_artifact.py
WHEEL_OR_DIRECTORY` and `workspace-scripts/check_core_ruby_artifact.rb` gates to
exercise both generated layouts. Their temporary consumers and installed
environments are removed on exit; small evidence reports remain under `tmp/`.

Current generation requires local Alef corrections, including root-array field
access and preservation of explicit null assertion values. Upstream release reproducibility remains open;
this is not approval to publish packages or claim full target conformance.
