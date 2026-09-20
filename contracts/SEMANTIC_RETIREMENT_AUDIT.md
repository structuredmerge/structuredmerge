# Semantic Retirement Audit

This is a review-only Phase 10 work list generated from
`legacy-operation-migration.json`. It authorizes no deletion, default
change, publication or parser promotion.

Inventory schema: `structuredmerge.legacy-operation-migration/v1`; groups: **12**.

| Group | Disposition | Consumers | Required action |
| --- | --- | ---: | --- |
| `json-family` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `bash-family` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `go-family` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `rust-family` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `typescript-family` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `git-driver` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `structural-editing` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `template-session` | `local_consumer_migrated` | 1 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `parser-services` | `local_consumer_migrated` | 2 | Candidate for explicit authority review; retain legacy regression evidence and do not delete yet. |
| `capabilities` | `retain_until_consumer_migration` | 1 | Retain until every listed consumer is migrated and installed gates pass. |
| `workflow-hosts` | `retain_regression_only` | 2 | Retain as regression evidence; no product/default promotion is authorized. |
| `runtime-identity-probes` | `retain_regression_only` | 1 | Retain as regression evidence; no product/default promotion is authorized. |

## Group review records

### `json-family`

- Methods: `parse_json_analysis, merge_json_two_way, merge_json_three_way`
- Consumers: `ruby/gems/json-merge/lib/json/merge/rust_host_provider.rb`
- Replacement: Typed common JSON analyze/diff2/merge2/merge3 operations with TreeHaver-owned parser registration and Rust-owned decisions.
- Disposition: `local_consumer_migrated`
- Gap / gate: Local Ruby JSON provider no longer inherits the prototype adapter or performs Ruby diff comparisons/text lookup. Complete core evidence is projected into portable Ruby values. All 93 JSON examples pass against the installed core artifact, including native semantic comparisons. JSON Pointer identities, root/document subjects and strict constraint rejection are intentional compatibility changes. Broader golden-master authority, downstream, hosted/released-package and lint/coverage gates remain open; retain legacy regression evidence without publishing the prototype.

### `bash-family`

- Methods: `parse_bash_analysis, merge_bash_two_way, merge_bash_three_way`
- Consumers: `ruby/gems/bash-merge/lib/bash/merge/rust_host_provider.rb`
- Replacement: Typed common Bash analyze/diff2/merge2/merge3 operations with TreeHaver parser registration, Rust-owned owner matching and source verification.
- Disposition: `local_consumer_migrated`
- Gap / gate: The opt-in Ruby adapter no longer depends on the prototype or computes diffs in Ruby. Installed-artifact tests pass all 13 provider cases and the 463-example Bash suite (two existing removal-mode pending cases), JSON regressions and real Git Bash selection. Current-preferred merge2 intentionally replaces fake-base template selection. Kernel owner identities/spans and complete typed records replace legacy analysis shapes. Neutral Git framing is accepted, custom marker requests rejected. Full-language/golden-master authority, downstream, hosted/released-package and lint/coverage gates remain open; native defaults and legacy regression evidence remain unchanged.

### `go-family`

- Methods: `parse_go_analysis, merge_go_two_way, merge_go_three_way`
- Consumers: `ruby/gems/go-merge/lib/go/merge/rust_host_provider.rb`
- Replacement: Typed common Go analyze/diff2/merge2/merge3 with TreeHaver parser registration and Rust-owned decisions, source evidence and canonical ownership-guard conflicts.
- Disposition: `local_consumer_migrated`
- Gap / gate: The opt-in Go adapter no longer inherits the prototype or computes diff decisions in Ruby. All 57 Go examples (including 16 focused provider cases), Bash/JSON regressions and real Git Go paths pass against the installed core gem. Current-preferred merge2 intentionally differs from native Ruby incoming preference; both outcomes remain asserted. Kernel identities/spans replace legacy host-rendered signatures. A formerly inert reorder test now actually reorders declarations. Full-language authority, broader golden-master/downstream, hosted/released-package and full lint/coverage gates remain open; native defaults are unchanged.

### `rust-family`

- Methods: `parse_rust_analysis, merge_rust_two_way, merge_rust_three_way`
- Consumers: `ruby/gems/rust-merge/lib/rust/merge/rust_host_provider.rb`
- Replacement: Typed common Rust analyze/diff2/merge2/merge3 using TreeHaver and Rust-owned declaration matching, source verification and canonical ownership-guard conflicts.
- Disposition: `local_consumer_migrated`
- Gap / gate: The opt-in Ruby adapter no longer inherits or loads the prototype. All 47 Rust examples, including 18 focused provider cases, pass against the installed core gem, along with Go/JSON/Bash regressions. Native three-way comparisons pass, including actual declaration reordering. Current-preferred merge2 intentionally differs from native incoming preference; both outcomes are asserted. Native IDs/spans and complete typed records replace legacy signatures. Full-language/golden-master, downstream, hosted/released-package and full lint/coverage gates remain open; native defaults remain unchanged.

### `typescript-family`

- Methods: `parse_typescript_analysis, merge_typescript_two_way, merge_typescript_three_way`
- Consumers: `ruby/gems/typescript-merge/lib/typescript/merge/rust_host_provider.rb`
- Replacement: Typed TypeScript/TSX analyze/diff2/merge2/merge3 using explicit grammar selection, native ownership and Rust-owned source verification.
- Disposition: `local_consumer_migrated`
- Gap / gate: All 55 TypeScript examples, including 18 focused provider cases, pass against the installed core with no skips or prototype dependency. Native TS/TSX merge3 comparisons pass; current-preferred merge2 intentionally differs from native incoming preference and both outcomes are asserted. Native IDs/spans, canonical conflicts and actual verification replace host-derived signatures and claims. JSON/Bash/Go/Rust regressions pass. Full-language/compiler, downstream, platform/ABI, hosted/released-package and lint/coverage gates remain open. Native defaults and generic legacy regression evidence are retained.

### `git-driver`

- Methods: `merge_ast_merge_git_json`
- Consumers: `ruby/gems/ast-merge-git/lib/ast/merge/git/rust_host_provider.rb`
- Replacement: Typed Git merge3 report over ast-merge-git and shared core operation results.
- Disposition: `local_consumer_migrated`
- Gap / gate: Local Ruby Git provider now selects kernel.git.json.v1 through the typed core, preserving full portable core records, canonical conflicts and actual render evidence. It advertises merge3 only; registry methods remain complete but capabilities may be nonempty subsets. Git write/leave-ours/error and command selection tests run against the installed core artifact. The existing marker review artifact retains ours outside conflicts and is not a partially resolved merge. The five family Ruby adapters now use the typed core and all 98 real-Git examples run without pending cases; broader authority, hosted/released-package, lint/coverage and publication gates remain open.

### `structural-editing`

- Methods: `report_ast_crispr_json, apply_ast_crispr_source_edits_json`
- Consumers: `ruby/gems/ast-crispr/lib/ast/crispr/rust_host_provider.rb`
- Replacement: Typed ast-crispr profile reports and explicit byte-range edit requests/results exported from structuredmerge-core.
- Disposition: `local_consumer_migrated`
- Gap / gate: Local Ruby main now uses typed reports and explicit edits, with request normalization and legacy Hash result projection. Revalidated 51 examples against the current installed core artifact. Hosted/released-package, remaining downstream, lint/coverage and exact malformed-input message parity remain open. Neither reports nor explicit edits prove structural selection parity; retain legacy regression evidence without publishing the prototype.

### `template-session`

- Methods: `report_ast_template_json`
- Consumers: `ruby/gems/ast-template/lib/ast/template/rust_host_provider.rb`
- Replacement: Typed options/profile/plan reports from ast-template through structuredmerge-core.
- Disposition: `local_consumer_migrated`
- Gap / gate: Local Ruby main now uses typed options/profile/read-only directory plans with transport normalization and legacy report projection. Revalidated 78 examples against the current installed core artifact. Hosted/released-package, remaining downstream and lint/coverage gates remain open. Report generation is not template execution or filesystem apply; Ruby apply behavior is retained.

### `parser-services`

- Methods: `register_parser_host, replace_parser_host, unregister_parser_host, clear_parser_hosts, registered_parser_hosts, register_tslp_parser_host, parse_normalized_with_tslp, parse_with_parser, probe_with_parser`
- Consumers: `ruby/gems/tree_haver/lib/tree_haver/backends/rust_tslp.rb`, `structuredmerge/packages/ruby/spec/structuredmerge_host_prototype_spec.rb`
- Replacement: Typed ParserHost registration, TreeHaver selection and parse_sources; explicit Rust TSLP provider registration and typed normalized results.
- Disposition: `local_consumer_migrated`
- Gap / gate: Ruby TreeHaver now registers and parses through typed structuredmerge-core, including a versioned native extra-node flag; its full 90-example suite passes against the installed core gem. Provider identity and diagnostic wording intentionally follow typed results. Hosted/released-package, downstream and full lint/coverage gates remain open. Legacy facade regression specs are retained; typed inventory, source-free selection and generation-checked replacement are deliberate new contracts, not drop-in legacy signatures. Process-wide bulk-clear remains unproven as a product need. Typed Rust facade regressions now prove in-flight snapshot retention, same-ID re-registration isolation, retired-host release and cancellation. See contracts/TYPED_PARSER_LIFECYCLE.md for explicit method dispositions. Typed declaration inventory now reaches installed Ruby/Python artifacts. Source-free selection reports now expose shared dispatch probes in Rust/Ruby/Python installed artifacts; no-eligible results retain candidate evidence without parsing. Atomic host replacement now passes installed Ruby/Python in-callback, stale-generation and unknown-ID tests. Full merge capability/authority reporting and broader binding runtime guarantees remain open; matching method names are not drop-in compatibility.

### `capabilities`

- Methods: `capability_manifest`
- Consumers: `structuredmerge/packages/ruby/spec/structuredmerge_host_prototype_spec.rb`
- Replacement: Typed capability manifest and provider identity with observable availability, support and default authority.
- Disposition: `retain_until_consumer_migration`
- Gap / gate: operation_profile_catalog declares all eight common-operation profiles through Rust and generated Ruby/Python bindings. Typed capability_manifest composes these declarations with one registry snapshot and explicitly requested parser eligibility observations; operation/dialect support, probe evidence and default authority remain separate. Installed Ruby 4.0.6/Psych and Python 3.14.2/LibCST artifacts pass manifest tests, including empty non-probing queries, faults, limits, cancellation and reentrant host retirement. See contracts/TYPED_CAPABILITY_MANIFEST.md. Wider platform/consumer integration remains open; parser eligibility is not source-specific merge support or default approval.

### `workflow-hosts`

- Methods: `register_workflow_host, replace_workflow_host, unregister_workflow_host, clear_workflow_hosts, registered_workflow_hosts, execute_typed_workflow`
- Consumers: `structuredmerge/packages/ruby/spec/structuredmerge_host_prototype_spec.rb`, `structuredmerge/packages/ruby/lib/structuredmerge_host_prototype/workflow_provider.rb`
- Replacement: Typed WorkflowHost coarse callbacks with explicit host-owned workflow boundaries; Rust retains merge semantics.
- Disposition: `retain_regression_only`
- Gap / gate: Rust structuredmerge-core now exports a typed WorkflowHost boundary using the ast-merge registry, explicit-provider batch dispatch, TreeHaver-prepared parse facts, bounded requests/results, cancellation and common-result validation. Execution is explicitly host-owned and not default-approved. See contracts/TYPED_WORKFLOW_HOST.md. Ruby/Python Alef exposure, real native consumers, installed runtime gates, host availability, versioned parser profiles, family-default dispatch, delegation and full portable failure/batch envelopes remain open. Retain boundary regressions, not a separately released host product. Consumer discovery beyond the listed local evidence remains open.

### `runtime-identity-probes`

- Methods: `execute_identity, execute_async_identity, execute_detached_identity, execute_in_process_identity, execute_typed_identity, start_identity_worker, prepare_identity_worker, dispatch_identity_worker, cancel_identity_worker, identity_worker_cancelled, poll_identity_worker, start_host_runtime, shutdown_host_runtime`
- Consumers: `structuredmerge/packages/ruby/spec/structuredmerge_host_prototype_spec.rb`
- Replacement: Real typed-operation lifecycle tests, OperationControl and generated runtime-affine callback infrastructure; no identity-worker product API by default.
- Disposition: `retain_regression_only`
- Gap / gate: GC, concurrency, cancellation and deadline cases are migrated in scoped installed Ruby/Python tests. Fresh installed-runtime processes now verify normal exit with idle registered providers, retired providers, and cancelled/drained callbacks (three repetitions per mode). This is not active-callback interpreter finalization, foreign-thread entry, embedding/subinterpreter support or broad stress evidence; retain legacy regressions and the remaining runtime gates.
