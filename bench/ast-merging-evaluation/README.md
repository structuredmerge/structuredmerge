# StructuredMerge real-world merge evaluation

This exploratory adapter adds the `smorg-rs` Git merge driver to the public
[AST-Merging-Evaluation](https://github.com/benedikt-schesch/AST-Merging-Evaluation)
runner. It evaluates whether repository test suites pass after merges, alongside
the runner's existing Git, Mergiraf, and other merge-tool baselines. It does not
replace the retained Slice 1023 correctness/safety gate documented in
`workspace-scripts/BENCHMARK_RUNTIME.md`.

## Benchmark fit: important limitation

This is **not a primary benchmark for the current StructuredMerge feature set**.
The upstream analyzer selects merge cases containing Java diffs; current SM
profiles cover JSON/JSONC, Python, YAML, Bash, Go, Rust, and TypeScript/JavaScript,
but no Java merge profile. Its results would therefore say little about SM's
structured Java merge effectiveness. Do not provision the 84 GB cache or run the
full analysis as a product comparison until Java support is on the actual
roadmap. Keep this adapter only for a future Java pilot or for testing whatever
SM-supported file classes occur in the Java projects; report that subset and its
denominator separately.

The preferred sequence is:

1. Keep the existing Slice 1023 micro/dev harness as the fast, bounded
   correctness, preservation, and reliability gate. It is already connected to
   the supported adapters and maps case selection to changed paths.
2. For a public external anchor now, begin with the 180-case ConflictBench and
the 135-case SWE-Git-Bench, but only after filtering cases to implemented SM
profiles. ConflictBench is Java-only and thus currently a non-fit; SWE-Git-Bench
has 11 file-type families but was built for LLM resolution and exact developer
match is not a semantic correctness oracle.
3. Treat [Merge-Bench](https://github.com/benedikt-schesch/Merge-Bench) as
   initially **not directly usable**: its 7,938 examples cover 11 languages and
   its 5.5 MB release is cheap to inspect, but its records are LLM prompt/answer
   examples without the base/ours/theirs contract StructuredMerge consumes.
   Developer-match is also not a behavioral oracle.
4. Evaluate the [Zenodo merge-file corpus](https://zenodo.org/records/22165346)
   as the best large direct-input candidate: it provides Base/Left/Right and an
   Expected result, with 639,035 cases across 181 extensions. First filter to
   actual SM-supported profiles, inspect per-source licensing and the quality of
   Expected, then stream a small deterministic sample. The 34.1 GB archive (the
   record reports 476.9 GB of data volume) makes this a later phase, though its
   file-level cases avoid expensive project checkout/test setup.
5. Use AST-Merging-Evaluation only when the tested SM code supports Java, or as
   an explicitly labeled Java-project integration experiment. The repository's
   `make small-test` is its cheap runner smoke, not a fair SM capability score.

The internal plan says to ship one measurable pilot and expand formats only
after pilots stabilize ([12-week execution plan](https://github.com/structuredmerge/plans/blob/main/STRUCTURED_MERGE_12_WEEK_EXECUTION_PLAN.md)).
That makes the smaller supported-profile sample the next evidence step; it does
not justify spending on an unrelated Java-only full run.

The external harness has no native StructuredMerge integration, so
`prepare.py` applies a small, local-only patch to its pinned checkout and adds a
merge-driver script. Do not commit modifications to the external checkout.

The upstream full run says it takes days. Its setup tests candidate repository
HEADs, discovers merge history, samples up to 500 merges per repository, then
tests up to 100 selected merges per repository. Its cache expands to about 84
GB. These are separate cost drivers; reducing the final merge count alone does
not avoid checkout, repository validation, or history discovery.

## AST-Merging-Evaluation adapter stages (future Java scope)

1. **Harness smoke:** upstream `make small-test` with the StructuredMerge tool
   enabled. The upstream README estimates 5–20 minutes and checks reproducible
   output across two small repositories. Do this on the benchmark VM, not as a
   production score.
2. **Integration pilot:** run the small repository CSV once with the normal
   timing option, then inspect merge outcomes, test failures, timeouts, and
   unsupported file formats. Fix integration defects before scaling.
3. **Setup-cost pilot:** select a few whole repositories across observed
   repository-size and test-ecosystem bands; time clone, head validation,
   history discovery, test setup, and per-case merge separately. This is a
   feasibility pilot, not a score.
4. **Full evaluation:** assign whole repositories with measured setup costs to
   about 10 groups using `shard_repos.py --costs-csv`. Run one group first, then
   fan out the remaining groups only after resume and result aggregation work.
   Re-run on the same pinned revisions and VM image; aggregate only after all
   expected repository IDs and case denominators are present.

Do not split individual cases from the same repository across workers: clone,
build, and test setup are per-project costs and should be paid once. Keep each
repository's cases together and compare all tools against the same warmed
checkout/cache. Initially use setup-time estimates, then refine weights with
observed per-repository time. Save each manifest and its weights; changing the
weights creates a new manifest, never an in-place edit.

One hundred shards are not a sensible first choice. They do not reduce total
work, and multiplying small groups repeats orchestration and recovery work.
Start with a few project bundles to measure setup, then 10 weighted repository
groups. Consider 100 only if measured run duration remains too long, and only
after confirming each worker can resume from its persistent cache. For
sub-minute feedback, use the existing Slice 1023 micro profile, not tiny random
fractions of the project-test corpus.

The upstream runner accepts a repository CSV, run name, merge-sampling limit,
and cache directory. One shard can be run with:

```sh
python3 bench/ast-merging-evaluation/shard_repos.py \
  --input /path/to/AST-Merging-Evaluation/input_data/repos_combined.csv \
  --output-dir /srv/structuredmerge-bench/manifests/combined-10 \
  --shards 10
```

Then, from the pinned external checkout, run `run.sh` with that shard CSV as the
first argument, a unique run name, `100` as the merge-sampling limit, and the
shared cache path. Use distinct run names for all shards. Preserve the runner's
results directory and logs per shard. The script stores a manifest with source
and output SHA-256 digests; never edit a shard after it starts. The first set is
balanced by repository count because timing observations are not yet available.
After a small whole-project setup pilot, provide a CSV with
`repository,estimated_seconds` to use deterministic longest-estimated-time-first
assignment:

```sh
python3 bench/ast-merging-evaluation/shard_repos.py \
  --input /path/to/repos_combined.csv \
  --output-dir /srv/structuredmerge-bench/manifests/combined-10-weighted \
  --shards 10 \
  --costs-csv /srv/structuredmerge-bench/manifests/repository-costs.csv
```

The cost should include per-project clone, head-test, history-discovery, and
test setup plus selected merge work. Missing timing rows are imputed to the
median observed repository cost; the cost-file digest and balance method are
recorded in the shard manifest.

## Prepare the pinned evaluator

Use the upstream revision recorded in `UPSTREAM_REVISION`. `prepare.py` refuses
other revisions, inserts StructuredMerge into the evaluator's merge-tool list,
and copies `smorg.sh` into the evaluator's tool directory. Build the kernel
binary from the exact StructuredMerge revision under evaluation and put it on
`PATH` as `smorg-rs` before running the evaluator. The upstream runner requires
Java 8, 11, and 17, GraalVM, Maven 3.9, Python dependencies, Git 2.44+, and its
merging submodule; follow its README and pin all installed versions in the run
manifest.

```sh
git clone --recurse-submodules https://github.com/benedikt-schesch/AST-Merging-Evaluation.git
git -C AST-Merging-Evaluation checkout "$(cat bench/ast-merging-evaluation/UPSTREAM_REVISION)"
python3 bench/ast-merging-evaluation/prepare.py AST-Merging-Evaluation
```

The upstream full-data cache is about 84 GB uncompressed. The smoke run uses
`cache-small` and does not require that full cache. Do not download or unpack the
full cache until the smoke and integration pilot pass and the VM has sufficient
free disk for the cache, repository worktrees, and results. Keep at least 20 GB
free, following the kernel benchmark runtime's reserve policy.

## Evidence and scoring

Retain raw per-shard results; report completed, unsupported, conflict,
incorrect/test-failing, timed-out, and failed-checkout cases separately. Do not
count unsupported or not-run cases as wins. Compare StructuredMerge only with
baseline tools on the same repository set and denominator. Include correctness
and coverage alongside runtime; report both run time and repository/test setup
time where possible. A debug build is suitable for a correctness smoke, not a
performance ranking. The first performance comparison must use a release build
and the same VM, resource limits, and cache state for every compared tool.

Every run record should include kernel, evaluator and fixtures revisions;
shard manifest and digest; build profile and binary digest; VM size and region;
OS, Java, Maven, GraalVM, Python and Git versions; cache identity; timestamps;
per-tool counts; and raw result digests. Keep large data and results on the VM's
attached disk or Blob Storage, never in Git. Do not publish comparative claims
until the full planned set completes and the denominator/coverage audit passes.

## Infrastructure and unattended execution

The VM is provisioned separately through the StructuredMerge DevOps repo's
OpenTofu workflow. The DevOps guide requires an infrastructure PR with a saved
`tofu plan`; apply happens only after merge. Use a dedicated benchmark resource
group, managed disk, SSH key-only access restricted to the approved operator
CIDR, and a budget alert. A Spot VM is a cost option only after testing
checkpoint recovery: Azure can evict it with 30 seconds' notice, so keep the
disk on eviction and make each shard restartable. A standard VM is safer for
the initial smoke. Stop/deallocate the VM after every phase; storage charges
continue while it is deallocated. The $100 budget is an alert, not a spending
cap; check the current Azure Retail Prices API for the selected region/SKU before
applying and stop the VM between phases.

The included `structuredmerge-benchmark.service` runs `run_queue.sh` as a
systemd oneshot with restart-on-failure and persistent logs. For a selected
dataset, copy `benchmark-queue.tsv.example` to the VM's queue file, add one
tab-separated row per phase (`run-name`, repository CSV path, merge-sampling
limit), install the unit, and set `SMORG_BIN` in the root-owned
`/etc/structuredmerge-bench.env`. Completed phase markers prevent reruns; an
interrupted phase retries against the evaluator's persistent cache. Confirm
the evaluator's cache behavior on the smoke before relying on Spot recovery.
Keep system logs and phase output on the attached data disk. Do not start the
long full-corpus phase until the pilot results, selected profiles, and
projected cost are reviewed.

If more throughput is needed, first prepare one pinned VM image with the
evaluator, dependencies, and verified cache layout; then clone that image into
independent workers, assigning each a different repository bundle. This avoids
repeating machine setup while keeping each worker's mutable worktrees and
results isolated. Do not create multiple workers until pilot throughput shows
near-linear speedup after accounting for repository clone traffic, disk I/O,
and VM cost.
