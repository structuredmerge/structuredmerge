# Retained bounded benchmark harness

Use the existing Ruby `Ast::Merge::Git::LocalBenchmark` implementation and the
fixtures repository's Slice 1023 corpus. `benchmark-runtime.gemfile` supplies only
its runtime dependencies and JSON structural oracle through ENV-driven nomono.
It neither loads nor installs `structuredmerge_host_prototype`. Nomono must
already be installed, as required for local-path bundle bootstrap.

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
