"""Conformance-only native syntax projection; no merge identities or decisions.

Shared by installed-binding tests, the Rust process gate and retained benchmarks.
This is not a production native-layer package.
"""
import bisect
import json
import importlib.metadata
import libcst
from libcst.metadata import MetadataWrapper, ByteSpanPositionProvider, WhitespaceInclusivePositionProvider


class LibCSTHost:
    def __init__(self):
        self.calls = 0

    def descriptor(self):
        import structuredmerge_core as core
        version = importlib.metadata.version("libcst")
        return core.ParserProviderDescriptor(
            id="python.libcst", family="native", runtime="python", package="libcst",
            package_version=version, parser="libcst", parser_version=version,
            languages=["python"], dialects=[], contracts=["structuredmerge.parse-result/v1"],
            capabilities=["native_extensions", "source_spans"], probe_id="libcst.import", priority=0,
            metadata={}, extensions=[], grammar=None, grammar_version=None,
        )

    def probe_batch(self, request):
        import structuredmerge_core as core
        assert isinstance(request, core.ProbeBatchRequest)
        return core.ProbeBatchResult(items=[core.ParserProbeResult(available=True, loadable=True) for _ in request.items])

    def parse_batch(self, request):
        import structuredmerge_core as core
        from structuredmerge_core import _native as native
        assert isinstance(request, core.ParseBatchRequest)
        self.calls += 1
        self.received_batch = request
        outputs = []
        for item in request.items:
            data = bytes(item.source.bytes)
            nodes, diagnostics = [], []
            try:
                module = libcst.parse_module(data)
                assert module.bytes == data
                nodes = project(module, data)
            except libcst.ParserSyntaxError as error:
                diagnostics.append(core.ParseDiagnostic(
                    id="libcst.syntax", severity=core.ParseSeverity.ERROR, category="parse_error",
                    message=str(error), source_role=item.source.descriptor.role, blocking=True,
                    metadata={}, extra={}, code="libcst.syntax", span=None, node_id=None,
                ))
            outputs.append(native.ParseOutput(
                request_id=item.request_id, source=item.source.descriptor, ok=not diagnostics,
                root_id="0" if nodes else None, nodes=nodes, comments=[], diagnostics=diagnostics,
                extensions=[], metadata={}, extra={},
            ))
        return core.ParseBatchResult(items=outputs)


def project_facts(module, data):
    """Parser facts shared by typed-binding tests and the Rust process gate."""
    # ByteSpanPositionProvider measures UTF-8 codegen, excluding a UTF-8 BOM.
    # Fail closed instead of silently changing coordinates for other encodings.
    data.decode("utf-8-sig")
    if module.encoding not in ("utf-8", "utf-8-sig") or b"\r" in data.replace(b"\r\n", b""):
        raise ValueError("test LibCST projection requires UTF-8 and LF/CRLF")
    wrapper = MetadataWrapper(module)
    spans = wrapper.resolve(ByteSpanPositionProvider)
    full_spans = wrapper.resolve(WhitespaceInclusivePositionProvider)
    bom = 3 if data.startswith(b"\xef\xbb\xbf") else 0
    starts = [0] + [index + 1 for index, byte in enumerate(data) if byte == 10]
    nodes = []

    def byte_offset(position):
        # LibCST positions use Unicode columns; source descriptors use bytes.
        # Metadata may end at the virtual line following the last real line.
        if position.line == len(starts) + 1 and position.column == 0:
            return len(data)
        start = starts[position.line - 1]
        if position.line == 1:
            start += bom
        line = data[start:].decode("utf-8").split("\n", 1)[0]
        return start + len(line[:position.column].encode("utf-8"))

    def point(offset):
        row = bisect.bisect_right(starts, offset) - 1
        return dict(row=row, column=offset-starts[row])

    def visit(node, parent=None):
        node_id = str(len(nodes))
        nodes.append(None)
        fields = []
        facts = {}
        if isinstance(node, (libcst.FunctionDef, libcst.ClassDef, libcst.SimpleStatementLine)):
            full = full_spans[node]
            facts["full_span"] = dict(start_byte=byte_offset(full.start), end_byte=byte_offset(full.end))
        if isinstance(node, libcst.Module):
            fields = [("body", child) for child in node.body]
        elif isinstance(node, (libcst.FunctionDef, libcst.ClassDef)):
            fields = [("name", node.name)]
            facts["decorators_count"] = len(node.decorators)
        elif isinstance(node, libcst.SimpleStatementLine):
            fields = [("body", child) for child in node.body]
        elif isinstance(node, libcst.Assign):
            fields = [("targets", child) for child in node.targets]
        elif isinstance(node, libcst.AssignTarget):
            fields = [("target", node.target)]
        elif isinstance(node, libcst.Name):
            facts["value"] = node.value
        children = [dict(node_id=visit(child, node_id), index=index,
            field_name=field) for index, (field, child) in enumerate(fields)]
        if parent is None:
            start, end = 0, len(data)
        else:
            span = spans[node]
            start, end = span.start + bom, span.start + span.length + bom
            if end > len(data):
                raise ValueError("native span exceeds exact source")
        kind = type(node).__name__
        extensions = [dict(
            schema="structuredmerge.extension/python-libcst/v1", namespace="python-libcst",
            capabilities=[], payload=facts)] if facts else []
        nodes[int(node_id)] = dict(
            id=node_id, type=kind, native_type=f"libcst.{kind}", role="structural",
            named=True, missing=False, has_error=False,
            span=dict(range=dict(start_byte=start, end_byte=end),
                start_point=point(start), end_point=point(end)),
            parent_id=parent, children=children, semantic_roles=[], unsupported_features=[],
            extensions=extensions, metadata={},
        )
        return node_id

    visit(wrapper.module)
    return nodes


def project(module, data):
    """Build the real generated DTOs for installed-binding callback tests."""
    import structuredmerge_core as core

    nodes = []
    for fact in project_facts(module, data):
        fact = dict(fact)
        span = fact.pop("span")
        children = [core.ChildEdge(**edge, extra={}) for edge in fact.pop("children")]
        extensions = [core.NativeExtension(
            schema=extension["schema"], namespace=extension["namespace"],
            capabilities=extension["capabilities"], payload=json.dumps(extension["payload"]), extra={}
        ) for extension in fact.pop("extensions")]
        fact.pop("role")
        nodes.append(core.ParseNode(**fact, role=core.NodeRole.STRUCTURAL,
            span=core.SourceSpan(range=core.ByteRange(**span["range"]),
                start_point=core.SourcePoint(**span["start_point"]),
                end_point=core.SourcePoint(**span["end_point"])),
            children=children, extensions=extensions, extra={}))
    return nodes


def process_request(request):
    """Test-only parse subprocess protocol, not a public operation transport."""
    source = request["source"]
    data = bytes(source["bytes"])
    nodes, diagnostics = [], []
    try:
        module = libcst.parse_module(data)
        if module.bytes != data:
            raise ValueError("LibCST changed exact source bytes")
        nodes = project_facts(module, data)
    except libcst.ParserSyntaxError as error:
        diagnostics.append(dict(id="libcst.syntax", severity="error", category="parse_error",
            code="libcst.syntax", message=str(error), source_role=source["descriptor"]["role"],
            span=None, node_id=None, blocking=True, metadata={}))
    return dict(request_id=request["request_id"], source=source["descriptor"], ok=not diagnostics,
        root_id="0" if nodes else None, nodes=nodes, comments=[], diagnostics=diagnostics,
        extensions=[], metadata={})


if __name__ == "__main__":
    import sys
    json.dump([process_request(request) for request in json.load(sys.stdin)], sys.stdout)
