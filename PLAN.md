# Rust PLAN

## Objective

Build a Rust crate family for the merge stack with tree-sitter as the primary
analysis backend for MVP releases, emphasizing correctness, explicit typing, and
future use as a potential performance-oriented core.

## License

Planned dual license for all new Rust merge-stack crates:

- `AGPL-3.0-only`
- `PolyForm-Small-Business-1.0.0`

Reference:

- `LICENSE_TEMPLATE_PLAN.md`

## Scope Boundary

Initial focus:

1. tree-sitter adapter crate
2. core merge contracts
3. text merge MVP
4. JSON and JSONC merge MVP
5. shared-fixture conformance runner

Deferred:

- project templating/scaffolding
- broad backend parity beyond MVP
- language-native parser experiments beyond tree-sitter
- PEG-backend parity beyond the first tree-sitter and native adapter slices

## Proposed Crate Family

Initial crate candidates:

- `tree-haver`
- `ast-merge`
- `text-merge`
- `json-merge`

Possible later crates:

- `toml-merge`
- `yaml-merge`
- `markdown-merge`
- `merge-ruleset`
- `crate-template`

## Ruby Mapping

Reference Ruby siblings to study first:

- `tree_haver`
- `ast-merge`
- `json-merge`
- `markdown-merge`

MVP parity target:

- parse/runtime abstraction from `tree_haver`
- merge result and diagnostic concepts from `ast-merge`
- JSON/JSONC behavior from `json-merge`

## Tree-Sitter Strategy

Primary backend:

- Rust tree-sitter crates and generated grammar bindings
- currently viable practical backend: published `tree-sitter-language-pack`
  crate for early backend adoption
- first PEG candidate for a second backend path: `pest`

Requirements:

- explicit grammar loading
- stable node wrapper types
- parse diagnostics
- deterministic fixture execution

## MVP Deliverables

### 1. `tree-haver`

- parser loader
- grammar selection API
- parse error reporting

### 2. `ast-merge`

- merge result model
- diagnostic types
- match/refiner traits
- freeze region representation

### 3. `text-merge`

- normalized text segmentation
- block matching
- similarity scoring
- configurable thresholds

### 4. `json-merge`

- strict JSON/JSONC merge behavior
- comment-aware handling where supported by grammar/runtime
- explicit recovery hook boundaries

### 5. Fixture Runner

- shared fixture ingestion
- golden-output comparison
- structured error reports

## Non-Goals For V1

- full Ruby feature parity
- templating/package-scaffold support
- speculative optimization before conformance is stable

## Open Questions

1. Which comment-preservation abstractions are worth porting in v1?
2. Should Rust become the future reference kernel, or remain a peer implementation?

## Decisions

- Use one Cargo workspace monorepo with multiple publishable crates.

## First Implementation Sequence

1. define Rust core traits and diagnostic enums
2. implement `tree-haver`
3. implement fixture runner
4. implement `text-merge`
5. implement `json-merge`
