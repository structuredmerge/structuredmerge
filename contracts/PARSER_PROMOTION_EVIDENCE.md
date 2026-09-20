# Parser promotion evidence matrix

This is a Phase 7 evidence register, not an authority or default-promotion
decision. A parser crate supplies syntax; it does not by itself reproduce
StructuredMerge merge semantics. Every candidate must pass the retained Ruby
golden-master and bounded `micro`/affected-`dev` benchmark gates before kernel
promotion.

| Candidate | Existing authority/baseline | Current kernel integration | Decision state | Next evidence required |
| --- | --- | --- | --- | --- |
| `ruby-prism` | Ruby Prism provider and retained Slice 1021–1023 reports | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Port candidate provider, run Ruby golden-master and micro/dev with source-preservation and reliability gates |
| `oxc_parser` | TypeScript compiler provider and TS/TSX installed-artifact fixtures | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Compare TS/TSX ownership, diagnostics and source bytes against the installed provider and golden corpus |
| `swc_ecma_parser` | TypeScript compiler provider and TS/TSX installed-artifact fixtures | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Run the same TS/TSX comparison as `oxc_parser`; retain whichever evidence actually passes |
| `saphyr` | Ruby Psych/YAML provider and typed YAML fixtures | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Compare YAML comments, anchors, diagnostics, ownership and source preservation against Psych |
| `taplo` | Ruby TOML provider and TOML fixture corpus | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Compare TOML formatting, comments, diagnostics and merge outcomes against the current authority |
| `comrak` | Ruby Markly/Markdown provider and Markdown fixture corpus | Not present in `Cargo.lock` or workspace manifests | Deferred; no promotion | Compare Markdown node ownership, source preservation and conflict rendering against the current authority |

## Evidence already available

- The retained benchmark harness and its safety rules are documented in
  `workspace-scripts/BENCHMARK_RUNTIME.md`; existing raw reports under
  `tmp/benchmark-runtime/` are baseline evidence for current providers, not
  candidate-promotion evidence.
- The installed Python typed-core gate and benchmark protocol pass against the
  checked-in generated artifact, but they exercise current typed providers and
  do not claim any candidate parser promotion.
- Absence from the workspace dependency graph is verified by the current
  `Cargo.lock` and manifests. That is a reproducible integration-state fact,
  not a quality or rejection judgment about any upstream parser crate.

## Promotion guard

Until each candidate has a recorded comparison report, raw benchmark result,
provenance and explicit authority decision, it must remain out of the kernel
defaults and provider registry. The optional foreign-runtime sidecar remains
deferred and does not block binding or release work.
