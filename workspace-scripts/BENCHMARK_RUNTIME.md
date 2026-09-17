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
profiles. It does not import test
fixtures, inspect benchmark oracles, invoke the old CLI or implement a fallback.

Use the Python virtual environment from a successful
`check_core_python_artifact.py` run. Prepend its `venv/bin` to PATH **after**
`mise exec` when invoking the benchmark bundle, and keep the Ruby bundle/runtime
configuration above. Pass:

```sh
--driver "$PWD/workspace-scripts/typed_core_benchmark.py" \
--adapter-descriptor typed-benchmark-adapter.json
```

For `dev`, supply both actual changed paths:
`--changed-path workspace-scripts/typed_core_benchmark.py` and
`--changed-path typed-benchmark-adapter.json`. The canonical corpus maps these
to the five implemented language-pack families. Other families, non-JSON merge2
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
kernel, Ruby, fixtures, bundle-lock and raw result digests. Debug builds establish
no performance ranking. Expected parser failures use the existing diagnostic
wire format `typed-core: parse_error: CODE: MESSAGE`. The initial adapter omitted
the executable prefix, so the retained harness correctly treated its unrecognized
diagnostic as a reliability failure. Keep the category derived from the typed
result, never from the fixture or human message; crashes and uncategorized errors
remain failures. Broader typed/native-provider coverage remains open. This is not
a publication or default-driver approval.
