# Typed publication closure

`typed-release-inventory.json` is generated from Cargo's manifest metadata for
the currently implemented Rust publication roots `structuredmerge-core` and
`smorg`. It is not registry ownership evidence, release authorization, or proof
that a source gem/wheel can build. The planned `structuredmerge` facade is not
invented as a completed package; its addition requires updating the roots.

Run `python workspace-scripts/check_typed_release_inventory.py` to check drift,
or use `--write` after reviewing a manifest change. All declared normal/build
path dependencies are included, even optional and target-specific ones. Dev-only
edges are excluded. The check rejects missing local crates, paths outside the
kernel, missing registry version requirements, cycles, prototype dependencies and
dependencies that disallow crates.io publication. Package ownership, license
contents, registry availability and feature-complete builds are separate gates.

The release command consumes this dependency-first order before its retained
legacy leaves. Its new read-only mode performs no packaging or publication:

```sh
ruby workspace-scripts/release_rust_crates.rb --list
ruby workspace-scripts/release_rust_crates.rb --list --only structuredmerge-core
```

Other modes retain their existing semantics: the command publishes by default
unless `--no-push` is supplied. Do not invoke them as an inventory check. `--only`
selects exactly one crate; it does not automatically publish prerequisites.
Neither a successful inventory check nor `--list` lifts any plan release gate.

## Source Ruby package gap

The current generated Ruby gemspec is not a verified source distribution. Its
extension manifest points to `../../../../../crates/structuredmerge-core`, outside
an unpacked gem. Its broad `lib`/`ext` glob can also include retained prototype
files. The platform-gem artifact helper already uses a strict allowlist; that
does not establish a working source gem.

Source preparation must use the Alef packaging pipeline, preserve the typed API
and license files, exclude prototype files, and include or resolve this full
dependency closure. Alef's inspected `vendor_core_only` implementation copies the
facade; that is not itself proof that every local dependency is vendored. Under
the accepted public-crate strategy, remaining dependencies must be available at
verified registry versions before a registry-backed source installation passes.
Keep source packaging, isolated native compilation, installed runtime tests,
reproducible upstream generation, and publication as distinct gates. Do not
hand-edit generated manifests or publish placeholder/prototype packages to
work around these gaps. This inventory is a prerequisite, not source-gem approval.
