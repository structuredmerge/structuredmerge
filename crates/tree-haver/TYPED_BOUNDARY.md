# TreeHaver

The typed boundary under implementation lives in `source`, `parsed`, and
`service`. The older root-module adapters and descriptive registry remain for
existing consumers; they are not a second selection path for the typed service.

- `SourceDocument` retains verified, immutable bytes and indexed byte points.
- `ParsedDocument` validates parser facts and preserves unknown compatible
  fields and native extensions. It does not assign merge ownership.
- `ParserRegistry` caches validated descriptors and supplies immutable snapshots
  with strong provider references, monotonically increasing generations, and
  deterministic descriptor digests.
- `TreeHaverParseService` selects by explicit ID, language, dialect, capabilities,
  availability, request/profile preferences, priority, and stable ID. It batches
  calls by selected provider, correlates results by request identity, and checks
  source/tree evidence. Provider failure does not trigger fallback.

This is an **in-process foundation**, not full Slice 1024–1031 conformance or a
generated host ABI. Providers must support concurrent calls; a Rust trait bound
does not make a Ruby or Python object thread-safe. Runtime-affine dispatch,
host finalization, atomic replacement/shutdown, byte-bounded bridge framing,
complete portable result/error envelopes, provider requirement constraints,
and real native-parser merge integrations remain to implement. Current span
validation supports nested UTF-8 text trees, not binary or explicitly
non-nesting native profiles. In-process batch dispatch currently fails the
batch on service/provider faults, while valid parser-returned syntax failures
remain parsed documents with their diagnostics intact.

The dispatcher tests use deliberately synthetic providers. They prove selection
and boundary mechanics, not Psych/LibCST behavior or Rust-owned merge semantics.
