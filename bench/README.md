# StructuredMerge benchmark project

## Recommendation

Use the retained Slice 1023 `micro`/`dev` profiles for the fast feedback loop.
They already match current adapters and plans, and their run-time selection is
bounded and reviewable.

Do not use the full AST-Merging-Evaluation run as the first external score.
That evaluator selects Java-file conflicts and runs project test suites; current
StructuredMerge merge profiles do not include Java. Its full data cache expands
to 84 GB and its maintainers say the full run takes days. The integration code
under [`ast-merging-evaluation/`](ast-merging-evaluation/README.md) is explicitly
future-scope for Java support, not current product evidence.

The best public direct-input candidate found so far is [the Zenodo merge-file
corpus](https://zenodo.org/records/22165346): 639,035 historical cases with
Base/Left/Right and an Expected file across 181 extensions. It is a useful
candidate for testing current file-level merge profiles after filtering to
supported formats. Expected is not guaranteed to be a correct semantic oracle,
and license/source metadata must be checked before publishing outputs. It has a
34.1 GB archive and therefore is not the cheapest initial setup.

The cheap 5.5 MB [Merge-Bench release](https://github.com/benedikt-schesch/Merge-Bench/releases/tag/main)
contains 7,938 cases across 11 languages, but the records are LLM prompt/answer
examples without the three-way inputs StructuredMerge consumes. It is not a
direct deterministic merge benchmark. [SWE-Git-Bench](https://doi.org/10.7910/DVN/FSFEZX)
and [ConflictBench](https://github.com/UBOWENVT/ConflictBench) also center LLM
conflict-resolution or Java-only cases, so neither should drive an SM capability
score today.

## Rubric

Every run should report these separately; one aggregate "pass rate" hides
important failure modes:

1. **Applicability:** supported profiles / all selected inputs; unsupported
   cases remain visible and are never counted as passes.
2. **Safety:** false clean merges, preservation violations, malformed outputs,
   crashes, and parse/adapter failures.
3. **Resolution:** exact developer-output match and normalized match, clearly
   labeled as historical agreement rather than semantic correctness.
4. **Coverage:** clean merges, conflict detections, manual-review outcomes, and
   not-run cases, all with explicit denominators.
5. **Behavioral correctness:** run project tests when a corpus supplies
   reproducible project context; verify both branch behaviors on sampled cases.
6. **Performance:** release binary, fixed VM and cache state, cold setup time,
   warm per-case time, p50/p95, throughput, and dollars per completed case.

Compare to Git and other baselines on the same supported case IDs. Keep raw
results, input manifests, tool revisions, binary digests, hardware, and cache
identity. A developer resolution is one reference, not guaranteed truth.

## Phased execution

1. Run the existing retained micro/dev gate locally or on the VM.
2. Run the benchmark adapter's two-project smoke after choosing a dataset.
3. For a project-test corpus, measure setup cost on a few representative whole
   repositories first. Keep each repository and all its selected cases together.
4. Build a cost file (`repository,estimated_seconds`) from measured setup plus
   selected case runtime. Use `ast-merging-evaluation/shard_repos.py --costs-csv`
   to assign whole repositories with longest-estimated-time-first balancing.
5. Start with several project bundles, then about 10 weighted groups. Run one
   group end-to-end and check resume/aggregation before fanning out. Do not
   switch to 100 groups unless measured per-group time remains too long.
6. For parallel execution, clone one pinned, prepared VM image into independent
   workers and give each a disjoint repository bundle. Keep each project's
   mutable worktrees/cache private to its worker; upload completed shard results
   to the durable output location. Measure scaling before adding workers.

Equal case counts are a poor shard target when setup time is paid per project.
Group by repository first, estimate each project's setup and case time, then
balance those project bundles. Parallelism can reduce elapsed time, but it does
not reduce total compute and may increase storage/network costs. The dedicated
OpenTofu root is in the [StructuredMerge DevOps repo](https://github.com/structuredmerge/devops/tree/main/infra/bench);
it defaults to one standard VM, with Spot and scale-out deferred until recovery
and throughput have been measured.
