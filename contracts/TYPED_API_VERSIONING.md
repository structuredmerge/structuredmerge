# Typed core compatibility and versioning

Kernel crates and all generated language packages share one release version.
Native-layer packages version independently and must declare the compatible
kernel range. Generated binding availability does not approve a capability or
change the default execution owner.

## API review surfaces

The generated declarations are checked-in review surfaces for the currently
exposed typed API:

- Ruby: `packages/ruby/sig/types.rbs`, generated from `[crates.ruby.stubs]`.
- Python native extension: `packages/python/structuredmerge_core/_native.pyi`,
  generated from `[crates.python.stubs]`. The generated package facade also
  remains part of the Python API; this native stub alone is not its baseline.

Regenerate with `alef stubs` after changing the Rust facade or selection config.
Review declaration diffs with the implementation change. Do not edit generated
declarations to hide a generator defect. Artifact tests validate the installed
Ruby declarations and compare Python stub names/members against the installed
native extension. Those checks do not yet prove complete signature equivalence,
Python facade parity, or semantic compatibility.

Removing operations, fields, accepted inputs or enum values; changing argument
order, required fields, error codes or semantics; and narrowing support require
an explicit compatibility review. New required constructor fields can break
callers even if the corresponding result change appears additive. A successful
regeneration is not compatibility approval. Existing legacy operations retain
their own compatibility records until their consumers migrate; the legacy
`ruby-api-v1.json` is not the typed-core baseline.

## ABI evidence

Development representation correction (2026-09-16): Python `SourceDescriptor`,
`LineEndings`, and dependent `ParseOutput` now expose native DTO classes instead
of facade dataclass twins. This fixes public nested construction and retains the
native constructor/field surface, but changes dataclass introspection, equality,
and class identity; it is not a backwards-compatible dataclass guarantee.
Installed tests verify public source-edit construction and existing parser
callbacks. The local Alef fix has not been upstreamed or released.

Ruby binaries are scoped to their Ruby ABI and platform, not one universal
extension. The artifact gate derives the ABI path and Ruby version bounds from
the build runtime, checks linkage, and loads the isolated gem. Current local
evidence is Linux x86_64 / Ruby 4.0. The plan's other platform/ABI combinations
remain unverified. Cross-compilation alone must not fill those entries.

The Python extension currently uses CPython's `abi3` with a 3.10 floor. The
installed gate records the actual interpreter and wheel digest; local execution
has been verified on 3.14. Wheel tagging alone does not prove 3.10 or other
platform/runtime compatibility. Hosted minimum-runtime and platform checks
remain required.

Neither language publishes a stable raw Rust memory-layout ABI. Language-visible
API review, artifact loader/linkage checks, and semantic conformance are separate
gates. Full per-target API snapshots and supported-platform ABI baselines remain
release work, not implied by these development declarations.

## Release discipline

Use the accepted lockstep kernel/binding version policy; native layers retain
independent semantic versions with explicit dependency ranges. A version bump
does not waive conformance, preservation, licensing, provenance, clean-install,
or default-authority gates. Record intentional compatibility changes in the
changelog and review them before release. No current development artifact or
local generator patch constitutes publication approval.
