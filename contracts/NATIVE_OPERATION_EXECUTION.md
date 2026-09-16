# Native common-operation execution

`structuredmerge_core::native_operation::execute_native_operation` executes a
validated common request using an immutable TreeHaver parser-registry snapshot
and shared execution controls. Rust owns owner analysis, diff and merge decisions.
This is an explicit experimental profile dispatcher, not a general provider
registry, generated binding export, or default-backend change.

The public Rust entry points `execute_operation` and
`execute_operation_controlled` accept the common request plus `ParseLimits`
(and shared `OperationControl` for the latter). They normalize checked inline
content/bytes and use the existing registered `ParserHost` registry. Source
references fail explicitly without implicit filesystem I/O. The deadline starts
before normalization. Invalid input/control errors return `CoreError`; accepted
requests return the request-correlated operation result, including operational
failures. See [binding integration status](COMMON_OPERATION_BINDING_GAPS.md) for
the current Ruby/Python generator issues; these new functions are not yet exported
by the generated bindings.

Supported profiles are `kernel.yaml.native_mapping.v1` and
`kernel.python.native_declarations.v1`, with their existing bounded syntax and
layout restrictions. Supported operations are bounded owner `analyze`, exact-source owner `diff2` and
source-preserving `merge3` without fallback. Requests must name a profile.
Unsupported operations, selection constraints, policy fields and required
extension capabilities fail before parsing. Parser selection remains TreeHaver's
responsibility; explicit backend requests do not trigger substitution.

Results retain request identity, passive extensions and metadata. Unknown request
fields and original selection/policy fields are nested under `request_forwarding`
so they cannot shadow reserved result fields. Parsed inputs retain their native
facts and selection evidence. Public parser-service errors use stable codes and
do not copy native exception messages.

Diff results retain both revision roles, exact owner changes, ordering and layout
evidence, without merged output. Each present changed region also supplies a typed
`source_spans` entry, with source identity and digest checked against the immutable
request and byte-oriented points computed from it. Absent revisions have no
invented span; UTF-8 columns count bytes and CRLF retains its original bytes.
Merge conflicts use executed Rust decisions and
the canonical conflict projector. Clean merges retain verified source segments
and actual output parse evidence. Even a whole-source selection receives a fresh
native parse and owner comparison; reusing an input parse is not called reparsing.
Output source IDs use the engine's collision-free identity. Cancellation or
deadline expiry discards completed output before returning a failure result.

`exact-source-partition` preservation and structural equivalence refer only to
the implemented owner profile. They are not proofs of general language semantic
equivalence. Marker options are preserved but no conflict markers are emitted.
Complete common merge change/span projection, native diagnostic projection, provider
registry/policy coverage, full analysis policy support, merge2, generated binding adoption and full
portable conformance remain open. The entry point does not authorize filesystem
writes, package publication or default cutover.

The explicit CI gate `cargo test -p structuredmerge-core --test native_operations
--locked -- --ignored --skip python_` runs real Ruby/Psych subprocesses. It covers composition,
whole-source output reparsing, conflict evidence, diff, malformed/unsupported
inputs, unsupported requirements, cancellation, deadlines, native output faults,
source-ID collisions and reserved result-field isolation. It is not an installed
Ruby/Python artifact test.

The Python CI matrix additionally installs LibCST 1.9.0 and runs
`cargo test -p structuredmerge-core --test native_operations --locked python_ -- --ignored`.
Local runs may select a prepared interpreter with `STRUCTUREDMERGE_NATIVE_PYTHON`.
These tests supply real LibCST syntax through TreeHaver and exercise common
analysis validation, NFKC logical identity, diff spans, composed/whole-source
merges, output reparsing, conflicts and failures. BOM, CRLF and exact source
bytes are retained. No Python-side merge, identity or owner decisions are used.

The test-only `libcst_facts.py` shares syntax projection between this subprocess
gate and installed-binding callbacks. Its subprocess JSON carries parser facts,
not complete operations; it is not a new public transport or generated binding
test. The installed-wheel gate still constructs actual generated DTOs and tests
the existing public binding APIs separately.

## Native analysis and provenance

`yaml_merge::typed::mapping_analysis` and `python_merge::declaration_analysis`
retain a shared `NativeOwnerAnalysis`: the existing Rust-owned source document
and an ordered native-node reference list for each logical owner. YAML entries
reference the real key and value nodes; no synthetic pair node is attributed to
Psych. Python declarations reference the actual LibCST statement. Logical
identity remains family-derived and does not depend on parse-local node IDs.

Validation requires unchanged source bytes, valid unique owners, a matching
reference catalog, resolvable unique source-ordered node references contained in
each owner, and exact owner boundaries established by those nodes. Existing
merge/diff family entry points use this validation before returning owner data.
These checks establish reference integrity, not semantic correctness of arbitrary
external ownership decisions; only the Rust family analyzers produce this data.

Common `analyze` uses these executed family analyses and embeds their typed
projection in the analysis-result envelope. The accepted depth is
`exact-source-owners` (also the default). Comment/token requests, disabling
ownership or native extensions, unknown depths and unknown policy fields fail
before parsing. The profile requires native facts for ownership; it does not
silently drop those requests. A selected parser returning unhandled comments
also fails rather than reporting unperformed attachment analysis.

The result retains a complete `CoreParseResult` and its parse request reference,
actual owners, logical match keys, byte spans/digests, layout attachments and
one emission-ownership decision per nonempty gap. `node_id` identifies the first
native boundary node; compatible `node_ids` retains all nodes establishing a
composite owner (Psych key/value pairs). No synthetic parser pair node is created.
Native extensions stay intact inside the embedded parse result.

Comment-region arrays are empty and explicitly marked not requested; source gaps
containing comment bytes are not described as classified comments. There is no
output, diff, edit plan or output-verification claim. Syntax/analysis failures
retain source-role diagnostics and parsed evidence, and cancellation/deadline
checks discard late successful analysis. For these two native profiles, the
common result validator requires a complete embedded parse, checks its identity,
source bytes, tree and selection consistency, reruns the Rust family analyzer,
and compares required owner/layout/attachment fields against that reconstruction.
Removing the embedded parse cannot bypass validation. Compatible extra fields
and well-formed passive extensions are retained. This verifies consistency with
the embedded syntax facts, not the authenticity of an external parser or registry
execution. Other profiles and content-addressed external analysis references
still need their own resolvers/validators; those retain schema-level checking.

`NativeOwnerAnalysis::layout_gaps` now exposes the exact byte-gap plan also used
by the owner renderer. It contains one slot preceding each owner and one suffix,
including empty slots, with source ranges/digests and neighboring owner IDs.
The next retained owner controls emission of its preceding slot; the last owner
controls the suffix. Ownerless sources retain one whole-source slot with no
invented controller. Existing baseline selection and changed-layout rejection
remain in force. These emission controllers do not assert semantic comment
attachment or authorize fallback transfer when an owner is deleted.

`layout_attachments` reuses the existing layout attachment type to reference
nonempty leading/trailing gaps. Adjacent owners share the same gap ID, but only
its declared controller emits it. Empty slots have no attachment reference.

This extends the existing layout module with byte-source evidence; the older
line-based blank-run augmenter remains for its existing callers. Native layout
must not be reconstructed from that augmenter's blank-line counts. Common
analysis rejects nonempty ownerless layout because document ownership is not
implemented. Complete comment/token policy support, generic analysis validation,
full runtime/platform coverage and generated analysis exports remain open.
