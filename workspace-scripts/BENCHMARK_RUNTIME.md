# Retained bounded benchmark harness

Use the existing Ruby `Ast::Merge::Git::LocalBenchmark` implementation and the
fixtures repository's Slice 1023 corpus. `benchmark-runtime.gemfile` supplies only
its runtime dependencies and JSON structural oracle through ENV-driven nomono.
It neither loads nor installs `structuredmerge_host_prototype`. Nomono must
already be installed, as required for local-path bundle bootstrap.

Use the `smorg-rs` compatibility executable for the retained harness's unlabelled
positional merge3 calls. Canonical `smorg` reserves unknown command names for
external dispatch; its explicit form is `benchmark-provider-merge3`. This does
not change the kernel providers measured by the existing harness.

Run from the kernel repository with a clean Ruby checkout pinned to the revision
being evaluated. Keep the generated lockfile and raw results in kernel `tmp/`,
not in the user's Ruby development bundle:

```sh
mkdir -p tmp/benchmark-runtime
cp workspace-scripts/benchmark-runtime.gemfile tmp/benchmark-runtime/Gemfile
export BUNDLE_GEMFILE="$PWD/tmp/benchmark-runtime/Gemfile"
export STRUCTUREDMERGE_DEV=/absolute/path/to/clean/structuredmerge-ruby/gems
bundle install
cargo build -p smorg --locked
mkdir -p "$STRUCTUREDMERGE_DEV/ast-merge-git/tmp"
bundle exec ast-merge-git benchmark run \
  --corpus ../fixtures/diagnostics/slice-1023-local-paired-benchmark/corpus.json \
  --profile micro --driver "$PWD/target/debug/smorg-rs" \
  --adapter-descriptor benchmark-adapter.json \
  --tmp-root "$STRUCTUREDMERGE_DEV/ast-merge-git/tmp/core-micro" \
  > tmp/benchmark-runtime/micro.json
```

Repeat with `--profile dev`, each actual changed path supplied as
`--changed-path crates/...`, a distinct temporary root, and `dev.json` output.
Inspect `selection.changed_paths` and `direct_cases`: an unmapped shared crate
can otherwise silently omit affected cases. Keep capability mappings in the
canonical fixture corpus; do not create a substitute selector.

Use the harness's report implementation to evaluate saved raw results:

```sh
bundle exec ruby -r ast/merge/git -r json -e '
  run = JSON.parse(File.read(ARGV.fetch(0)))
  report = Ast::Merge::Git::LocalBenchmarkReport.build(run)
  puts JSON.pretty_generate(report)
  exit(report.fetch("hard_gate_failed") ? 1 : 0)
' tmp/benchmark-runtime/micro.json
```

Record kernel/Ruby/fixtures revisions, bundle lock digest, binary digest, raw
result digests, selected/unsupported counts, and dirty-state details with the
evidence. No performance comparison is justified by this debug build. A passing
safety gate does not prove complete coverage or native binding parity: this
adapter exercises the existing CLI providers, including the generic TSLP Python
provider, not the typed Psych/LibCST facade. Registry/released-package gates and
golden-master authority decisions remain separate.

## Installed typed-core adapter

`workspace-scripts/typed_core_benchmark.py` connects this same harness to an
installed Python `structuredmerge-core` wheel. It contains transport only:
JSON-family merge2 uses `kernel.json.nested.v1`, and merge3 uses the typed Git
profile `kernel.git.json.v1` for review framing. TreeHaver owns language-pack
parser registration; Rust owns all merge decisions. Bash, Go, Rust and
TypeScript/TSX merge3 additionally select their typed `kernel.<family>.owners.v1`
profiles. Python merge2/merge3 select `kernel.python.native_declarations.v1`
through the real registered LibCST callback. The adapter explicitly loads the
shared conformance projection in `packages/python/tests/libcst_facts.py`; this is
not a shipped native-layer package. It does not inspect benchmark input fixtures
or oracles, invoke the old CLI or implement a fallback. The LibCST helper reports
syntax facts only; Rust owns identities, merge decisions and rendering. The
Python corpus case retains its historical generic-provider label; the candidate
result and descriptor report the actual native provider instead.

First verify the wheel with `check_core_python_artifact.py`. That gate now removes
its disposable environment; do not depend on a retained `core-python-artifact-*`
virtual environment. For a benchmark session, create one repository-local
temporary virtual environment, install the exact verified wheel and
`libcst==1.9.0` with pip's `--no-cache-dir`, and remove that environment in the
session runner's guaranteed teardown. Check free space before and during the
session, preserving the 20 GiB reserve plus a separate job budget. Keep result
reports outside the disposable environment. Prepend its `bin` to PATH **after**
`mise exec` when invoking the benchmark bundle, and keep the Ruby bundle/runtime
configuration above. Pass:

```sh
--driver "$PWD/workspace-scripts/typed_core_benchmark.py" \
--adapter-descriptor typed-benchmark-adapter.json
```

For `dev`, supply both actual changed paths:
`--changed-path workspace-scripts/typed_core_benchmark.py` and
`--changed-path typed-benchmark-adapter.json`. The canonical corpus maps these
to the five implemented language-pack families and Python. Include
`--changed-path packages/python/tests/libcst_facts.py` when changing that helper.
Other families, non-JSON/non-Python merge2
and metamorphic diff are explicitly unsupported in this descriptor, not silently
delegated. The common owner profiles return typed conflicts but do not promise
Git review framing; only the JSON merge3 path uses the typed Git profile. File mode
preserves ours on an error and writes only Rust-produced output/review text;
merge2 returns JSON without modifying input files. A persistent JSONL session
uses the existing adapter-request/response v1 protocol. Verify it with the
installed environment's Python:

```sh
python workspace-scripts/check_typed_benchmark_adapter.py
```

Record the installed wheel digest/report and exact interpreter alongside driver,
kernel, Ruby, fixtures, bundle-lock and raw result digests. For Python also record
the conformance helper digest and installed LibCST version (the artifact gate
pins 1.9.0). Debug builds establish
no performance ranking. Expected parser failures use the existing diagnostic
wire format `typed-core: parse_error: CODE: MESSAGE`. The initial adapter omitted
the executable prefix, so the retained harness correctly treated its unrecognized
diagnostic as a reliability failure. Keep the category derived from the typed
result, never from the fixture or human message; crashes and uncategorized errors
remain failures. Broader typed/native-provider coverage remains open. This is not
a publication or default-driver approval.

## Installed Ruby/Psych adapter

`workspace-scripts/typed_ruby_benchmark` launches a separate Ruby interpreter and
`typed_ruby_benchmark.rb` against the installed artifact, using the existing
Psych conformance callback. Merge decisions stay in Rust. This adapter advertises
only YAML merge3 (`kernel.yaml.native_mapping.v1`); no YAML merge2 or other family
is delegated to a legacy implementation. It imports the conformance helper, not
benchmark input/oracle data, and is not a production native-layer package.

Build and verify the current Ruby artifact with `check_core_ruby_artifact.rb`.
The installed gate now removes its disposable gem home. Export the same build
with `--package-only`, verify that its digest matches the installed-test report,
and install that export into one separately scoped benchmark gem home. Remove
that home in the session runner's guaranteed teardown and retain benchmark
reports outside it. Check free space before and during the session, with a job
budget above the 20 GiB reserve. For the benchmark command above, set
`STRUCTUREDMERGE_BENCHMARK_GEM_HOME` to this temporary home and
`STRUCTUREDMERGE_BENCHMARK_RUBY` to the exact absolute `RbConfig.ruby` used to build
it. Use:

```sh
--driver "$PWD/workspace-scripts/typed_ruby_benchmark" \
--adapter-descriptor typed-ruby-benchmark-adapter.json
```

The launcher removes parent Bundler startup hooks (including `BUNDLER_SETUP`),
then sets the isolated gem home/path. No checkout load-path injection is allowed.
With those same environment variables, run
`python workspace-scripts/check_typed_ruby_benchmark.py`; this includes poisoned
parent-Bundler variables and missing-gem failure tests. Setup/runtime failures
return 2, reserving 1 for actual typed conflicts. Inspect raw diagnostics as well
as the official report: an early broken launch returning 1 was classified as the
expected YAML conflict by the older harness and is not valid execution evidence.
The corrected harness requires a complete marker region or the categorized
`EXECUTABLE: merge_conflict: MESSAGE` diagnostic (with an optional separate code)
before accepting exit 1
as a conflict. Unexplained startup failures now remain reliability errors; pin
the harness revision when comparing reports produced before and after this fix.

Include actual adapter, descriptor and conformance-helper changed paths for dev.
The current micro corpus has no YAML merge3 sentinel, so all 16 cases are
unsupported by this narrow adapter. Dev includes one YAML delete/modify conflict;
its passing result is not broad native merge effectiveness evidence. Preserve the
existing Python adapter runs for the other supported families. Record the Ruby,
Psych, gem, launcher, driver and helper revisions/digests with raw reports; no
speed ranking, publication or default-authority decision follows from this gate.
