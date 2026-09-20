# Legacy operations to typed core

The [machine-readable inventory](legacy-operation-migration.json) assigns each
method in the existing [Ruby compatibility record](ruby-api-v1.json) exactly one
migration disposition. It is a removal/migration work list, not a requirement to
publish, stabilize, or reproduce the retired package. The typed core must not
depend on that crate as a shortcut.

## Evidence and limits

Current lifecycle work is recorded in [Typed parser lifecycle](TYPED_PARSER_LIFECYCLE.md).
The typed Rust facade now has deterministic in-flight removal/re-registration,
host-release and cancellation regressions. Atomic replacement, registry/capability
observability and binding-runtime shutdown/affinity guarantees remain open.
The dated consumer snapshots below are historical; consult the machine-readable
inventory for the subsequently migrated JSON/Bash/Go/Rust/TypeScript/Git consumers.

Revalidated 2026-09-16: Ruby local main was fast-forwarded from `4c31387bf` to
`0a1f4ee32`, integrating the typed ast-crispr and ast-template adapters. Their
artifact bundles pass 51 and 78 examples respectively against core gem SHA-256
`96291c720058b22cc80d2992943f65878faf36808639ebfc06f52ec24b71209e`.
The inventory marks these two groups `local_consumer_migrated`, retaining the
consumer revision and remaining gates. This is not a claim of publication,
hosted acceptance, complete downstream parity, or permission to delete legacy
regressions. Other consumer groups remain unmigrated.

Inspected the legacy Rust facade, Ruby adapters, generated legacy loader, and
existing compatibility record on 2026-09-16. Kernel starting revision:
`922c513`; Ruby worktree: `9ace81a5e`. The inspected shared merge, structural-edit,
template, Git, and TreeHaver adapters have no diff against Ruby main `4c31387bf`.
Consumer paths in the inventory are workspace-relative. Listed consumers are
concrete local evidence, not an exhaustive third-party usage search.

The inventory check uses the explicit compatibility record, not regex parsing
of source. It proves each recorded exported method has one disposition; it does
not prove the old record matches every runtime export, prove parity, or authorize
deletion. Runtime export validation and typed API baselines remain separate.

```sh
python -m unittest discover -s workspace-scripts/tests -p test_legacy_operation_migration.py -v
```

Legacy constants also require deliberate treatment: `HostBatchRequest` and
`HostSourceSegment` carry the older batch/source transport and must yield to the
typed operation/source contracts, not merely be renamed. `HostPrototypeError`
maps to typed core error/diagnostic behavior, not a preserved public exception
name. `VERSION` becomes the lockstep core package version. `WorkflowProvider`
and its descriptor, identity, synchronous/asynchronous/cancellable/typed/detached
methods supply regression evidence for the future typed `WorkflowHost`; they
are not proof that the new host contract exists.

## Migration hazards found in the actual consumers

- The common Ruby `Ast::Merge::RustHostProvider#diff2` constructs owner maps and
  classifies changes in Ruby. Swapping its require/module name would keep merge
  semantics outside Rust. Typed Rust analysis and diff operations must precede
  replacing that adapter.
- Its merge2 adapter sends current before incoming. The typed replacement must
  use explicit semantic roles; positional reuse risks reversing policy ownership.
- Its verification reports source preservation and base participation from the
  operation path, rather than transporting all kernel evidence. The replacement
  must report actual verification, including failure and unverified states.
- The Git adapter advertises the common four operations but returns unsupported
  for three of them. Capability negotiation must reflect actual support.
- The ast-crispr bridge applies explicit byte ranges but does not select AST
  nodes. The ast-template bridge reports/plans but does not apply templates or
  mutate the filesystem. Preserve those separate Ruby capabilities until their
  own replacements pass the retained downstream gates.
- The typed YAML/Python profiles cannot substitute for JSON, Bash, Go, Rust,
  TypeScript, or normalized TSLP behavior simply because bindings load.

## Next implementation order

Start removing concrete consumer blockers: expose typed explicit source-edit
requests/results over the existing Rust source renderer, followed by typed
profile/session reports. Keep structural selection and filesystem apply native
until separately proven. In parallel with that sequence, complete the common
operation envelope and Rust-owned analyze/diff2/merge2/merge3 family adapters.
The `ast-merge::provider_registry` foundation now owns merge-behavior
registrations and immutable executor snapshots, as required by Slice 1026;
TreeHaver still owns the separate parser registry. Typed `WorkflowHost` dispatch
now connects these boundaries, without a third registry or JSON-string core
facade. Explicit-provider negotiation and prepared-batch execution do not
establish default approval or prove that host-owned semantics moved to Rust.

Migrate a consumer only after its exact behavior and installed artifact are
verified. Retain legacy regression code while it is useful, but do not make its
publication a prerequisite. Alef fixes remain local at the maintainer's request;
upstream-release reproducibility is still an unmet gate.
