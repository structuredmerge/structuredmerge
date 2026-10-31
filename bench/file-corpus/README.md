# Supported-profile file corpus

This is the candidate external benchmark for current StructuredMerge profiles.
It uses the 639,035-case [Zenodo corpus](https://zenodo.org/records/22165346),
whose cases provide `Base`, `Left`, `Right`, and a historical `Expected` file.
The full archive is 34.1 GB compressed; the record reports 476.9 GB of data
volume. The sampler reads the archive once and extracts only a deterministic
sample of supported extensions, so it does not require unpacking the entire
corpus.

The first target is the current typed adapter's file-level set: JSON/JSONC/JSON5,
Python, Go, Rust, TypeScript/TSX, and Bash. Confirm the exact path-to-dialect mapping against
the pinned adapter descriptor before each run. Do not count other extensions as
unsupported failures; exclude them from the selected denominator and retain the
all-corpus extension inventory separately.

## First 1% phase

Keep the downloaded archive on the VM data disk, outside the Docker image and
Git. Then stream-extract a deterministic 1% of supported cases:

```sh
python3 bench/file-corpus/sample_zenodo.py \
  --archive /srv/structuredmerge-bench/data/merge_cases.tar.gz \
  --output /srv/structuredmerge-bench/cases/profile-sample \
  --sample-modulus 100 --sample-bucket 0 \
  --extensions json jsonc json5 py go rs ts tsx sh
```

Generate the manifest and ten stable job shards. All sampled cases for a
repository stay together; repositories are assigned with deterministic
longest-first balancing using total input bytes as the initial runtime proxy.
After the pilot, replace this estimate with measured per-repository runtimes.

```sh
python3 bench/file-corpus/make_manifest.py \
  --root /srv/structuredmerge-bench/cases/profile-sample \
  --output /srv/structuredmerge-bench/manifests/profile-sample.jsonl \
  --extensions json jsonc json5 py go rs ts tsx sh
```

The case input itself needs no repository clone or project test setup. A 1%
case sample is roughly 6,400 cases before filtering to supported extensions.
Use 10 jobs first. Parallelism reduces wall time, not total merge operations.

## Run and score

Build the worker image from a clean StructuredMerge checkout. The image contains
the exact `smorg-rs` binary and runner; mount manifests, inputs, and outputs at
runtime. Pin the image by digest and record the kernel SHA, adapter descriptor
digest, archive SHA-256/MD5, manifest digest, and run parameters.

```sh
docker build -f bench/file-corpus/Dockerfile -t smorg-bench:$(git rev-parse HEAD) .
docker run --rm \
  -v /srv/structuredmerge-bench/cases/profile-sample:/cases:ro \
  -v /srv/structuredmerge-bench/manifests:/manifests:ro \
  -v /srv/structuredmerge-bench/results:/results \
  smorg-bench:PINNED \
  --manifest /manifests/profile-sample.jsonl \
  --results /results/profile-sample.jsonl \
  --artifacts-dir /results/profile-sample-artifacts \
  --shard-index 0 --shards 10 --workers 1
```

The runner executes both StructuredMerge and Git's `merge-file` baseline. It
records tool status, runtime, conflict-marker status, exact and whitespace-
normalized agreement with `Expected`, and output digests. It also saves each
StructuredMerge output, its report, and the Git baseline output and diagnostics
under a case-keyed artifact directory; JSONL rows contain their paths and
digests. The source inputs and historical `Expected` remain in the sampled case
directory. This preserves disagreements for manual review. Expected results can
be wrong or incomplete; exact agreement is historical developer agreement, not
semantic correctness. Keep unsupported, errors, conflicts, and timeouts apart
from clean results. Manually review disagreements and run behavioral tests for
the sampled source projects before making correctness claims.

## Container and parallel plan

Keep the image free of benchmark data. The 1% sample is about 1% of the extracted
data, but downloading the compressed source archive still requires 34.1 GB.
Do not bake that archive into a 34 GB image or pull it once per worker. Place it
on the persistent VM disk, extract the selected sample once, and pass each
worker its manifest shard and result path as parameters.

Start on one D4as_v5 VM with two workers, then compare throughput and disk wait.
Increase concurrency only when CPU measurements justify it; use a larger VM for
project builds/tests. The Dockerfile builds one reusable worker image (Rust tool
plus runner); each worker receives `--shard-index`, `--shards`, `--workers`,
manifest, and result path. The archive and extracted cases stay on mounted
storage, so the image does not grow with the 34.1 GB corpus. We cannot report a
measured image size until a Docker build completes. Only publish an ACR image or
clone to more VMs after the pilot confirms useful speedup. For repository-test
benchmarks, keep repositories indivisible and balance by measured setup plus
merge time. ACR and additional VMs add recurring cost; the OpenTofu budget is
an alert, not a hard spend cap.
