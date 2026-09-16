"""Installed-wheel typed callback smoke test; not LibCST merge conformance."""
import hashlib
import importlib.metadata
import unittest

import libcst
import structuredmerge_core as core
from structuredmerge_core import _native as native


class LibCSTHost:
    def __init__(self):
        self.calls = 0

    def descriptor(self):
        version = importlib.metadata.version("libcst")
        return core.ParserProviderDescriptor(
            id="python.libcst", family="native", runtime="python", package="libcst",
            package_version=version, parser="libcst", parser_version=version,
            languages=["python"], dialects=[], contracts=["structuredmerge.parse-result/v1"],
            capabilities=["source_spans"], probe_id="libcst.import", priority=0,
            metadata={}, extensions=[], grammar=None, grammar_version=None,
        )

    def probe_batch(self, request):
        assert isinstance(request, core.ProbeBatchRequest)
        return core.ProbeBatchResult(items=[core.ParserProbeResult(available=True, loadable=True) for _ in request.items])

    def parse_batch(self, request):
        assert isinstance(request, core.ParseBatchRequest)
        self.calls += 1
        outputs = []
        for item in request.items:
            data = bytes(item.source.bytes)
            nodes, diagnostics = [], []
            try:
                module = libcst.parse_module(data)
                assert module.bytes == data
                lines = data.split(b"\n")
                nodes.append(core.ParseNode(
                    id="root", type="module", native_type="libcst.Module", role=core.NodeRole.STRUCTURAL,
                    named=True, missing=False, has_error=False,
                    span=core.SourceSpan(range=core.ByteRange(start_byte=0, end_byte=len(data)),
                        start_point=core.SourcePoint(row=0, column=0),
                        end_point=core.SourcePoint(row=len(lines)-1, column=len(lines[-1]))),
                    children=[], semantic_roles=[], unsupported_features=[], extensions=[],
                    metadata={}, extra={}, parent_id=None,
                ))
            except libcst.ParserSyntaxError as error:
                diagnostics.append(core.ParseDiagnostic(
                    id="libcst.syntax", severity=core.ParseSeverity.ERROR, category="parse_error",
                    message=str(error), source_role=item.source.descriptor.role, blocking=True,
                    metadata={}, extra={}, code="libcst.syntax", span=None, node_id=None,
                ))
            outputs.append(native.ParseOutput(
                request_id=item.request_id, source=item.source.descriptor, ok=not diagnostics,
                root_id="root" if nodes else None, nodes=nodes, comments=[], diagnostics=diagnostics,
                extensions=[], metadata={}, extra={},
            ))
        return core.ParseBatchResult(items=outputs)


class TypedParserHostTest(unittest.TestCase):
    def test_native_batch_and_native_syntax_failure(self):
        host = LibCSTHost()
        core.register_parser_host(host)
        try:
            requests = []
            for index, data in enumerate(["é = 1\r\n".encode(), b"def broken(:\n"]):
                descriptor = native.SourceDescriptor(
                    source_id=f"s{index}", role=core.SourceRole.SOURCE, byte_length=len(data),
                    sha256=hashlib.sha256(data).hexdigest(), encoding=core.SourceEncoding.UTF8,
                    bom=False, line_endings=native.LineEndings(lf=index, crlf=1-index, bare_cr=0),
                    final_newline=True,
                )
                requests.append(core.ParseRequest(
                    schema="structuredmerge.parse-request/v1", request_id=f"r{index}",
                    source=core.SourceInput(descriptor=descriptor, bytes=data), language="python", dialect=None,
                    selection=core.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
                    options=core.ParseOptions(), metadata={}, extra={},
                ))
            results = core.parse_sources(requests, core.ParseLimits(max_batch_items=2, max_input_bytes=1000, max_nodes=100, max_diagnostics=10))
            self.assertEqual(host.calls, 1)
            self.assertTrue(results[0].parsed.ok)
            self.assertEqual(results[0].parsed.source.sha256, requests[0].source.descriptor.sha256)
            self.assertFalse(results[1].parsed.ok)
            self.assertEqual(results[1].parsed.diagnostics[0].code, "libcst.syntax")
            self.assertEqual(results[0].selection.selected_backend, "python.libcst")
        finally:
            core.unregister_parser_host("python.libcst")


if __name__ == "__main__":
    unittest.main()
