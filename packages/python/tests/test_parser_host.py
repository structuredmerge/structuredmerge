"""Installed-wheel native callbacks and Rust-owned declaration merge tests."""
import hashlib
import importlib.metadata
from pathlib import Path
import sys
import unittest

import libcst
import structuredmerge_core as core
from structuredmerge_core import _native as native
from libcst_facts import project


class LibCSTHost:
    def __init__(self):
        self.calls = 0

    def descriptor(self):
        version = importlib.metadata.version("libcst")
        return core.ParserProviderDescriptor(
            id="python.libcst", family="native", runtime="python", package="libcst",
            package_version=version, parser="libcst", parser_version=version,
            languages=["python"], dialects=[], contracts=["structuredmerge.parse-result/v1"],
            capabilities=["native_extensions", "source_spans"], probe_id="libcst.import", priority=0,
            metadata={}, extensions=[], grammar=None, grammar_version=None,
        )

    def probe_batch(self, request):
        assert isinstance(request, core.ProbeBatchRequest)
        return core.ProbeBatchResult(items=[core.ParserProbeResult(available=True, loadable=True) for _ in request.items])

    def parse_batch(self, request):
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


class TypedParserHostTest(unittest.TestCase):
    def merge(self, sources):
        requests = []
        for role, source in zip([core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS], sources):
            data = source.encode("utf-8")
            descriptor = native.SourceDescriptor(
                source_id="merge-output" if role == core.SourceRole.BASE else str(role), role=role, byte_length=len(data), sha256=hashlib.sha256(data).hexdigest(),
                encoding=core.SourceEncoding.UTF8, bom=data.startswith(b"\xef\xbb\xbf"),
                line_endings=native.LineEndings(lf=data.count(b"\n")-data.count(b"\r\n"),
                    crlf=data.count(b"\r\n"), bare_cr=0), final_newline=data.endswith(b"\n"),
            )
            requests.append(core.ParseRequest(
                schema="structuredmerge.parse-request/v1", request_id=str(role),
                source=core.SourceInput(descriptor=descriptor, bytes=data), language="python", dialect=None,
                selection=core.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
                options=core.ParseOptions(native_extensions=True), metadata={}, extra={},
            ))
        return core.merge_python_declarations(requests[::-1], core.ParseLimits(
            max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20))

    def setUp(self):
        self.assertTrue(Path(core.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()),
            "test must load the installed wheel, not the source package")
        self.host = LibCSTHost()
        core.register_parser_host(self.host)

    def tearDown(self):
        core.unregister_parser_host("python.libcst")

    def test_independent_assignments_preserve_exact_source(self):
        sources = [
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 3",
        ]
        result = self.merge(sources)
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.CLEAN)
        self.assertEqual(result.output, "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 3")
        self.assertEqual(self.host.calls, 2)
        self.assertEqual(self.host.received_batch.items[0].source.descriptor.role, core.SourceRole.OUTPUT)
        by_role = dict(zip([str(role) for role in [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS]],
            [source.encode() for source in sources]))
        output = result.output.encode()
        self.assertEqual(result.output_source.sha256, hashlib.sha256(output).hexdigest())
        self.assertEqual(result.output_source.byte_length, len(output))
        descriptors = {source.source_id: source for source in result.sources}
        self.assertNotIn(result.output_source.source_id, descriptors)
        cursor = 0
        self.assertTrue(result.source_segments)
        for segment in result.source_segments:
            self.assertEqual(segment.output_range.start_byte, cursor)
            selected = by_role[str(segment.source_role)][segment.source_range.start_byte:segment.source_range.end_byte]
            rendered = output[segment.output_range.start_byte:segment.output_range.end_byte]
            self.assertEqual(rendered, selected)
            self.assertEqual(segment.sha256, hashlib.sha256(rendered).hexdigest())
            self.assertEqual(descriptors[segment.source_id].role, segment.source_role)
            cursor = segment.output_range.end_byte
        self.assertEqual(cursor, len(output))

    def test_function_and_class_bodies_merge_as_whole_declarations(self):
        base = "def café():\n    return 1\n\nclass Thing:\n    value = 2\n"
        result = self.merge([base, base.replace("return 1", "return 3"), base.replace("value = 2", "value = 4")])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.CLEAN)
        self.assertEqual(result.output, base.replace("return 1", "return 3").replace("value = 2", "value = 4"))

    def test_conflict_retains_three_source_alternatives_without_markers(self):
        result = self.merge(["a = 1\n", "a = 2\n", "a = 3\n"])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.CONFLICT)
        self.assertIsNone(result.output)
        self.assertIsNone(result.output_source)
        self.assertEqual(result.source_segments, [])
        self.assertEqual(result.conflicts[0].path, "/a")
        self.assertEqual([item.revision for item in result.conflicts[0].alternatives],
            [core.SourceRevision.BASE, core.SourceRevision.OURS, core.SourceRevision.THEIRS])

    def test_native_syntax_failure_retains_failing_role(self):
        result = self.merge(["a = 1\n", "a = 2\n", "def broken(:\n"])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertIsNone(result.output)
        self.assertIsNone(result.output_source)
        self.assertEqual(result.source_segments, [])
        self.assertEqual(result.rejected_parse.parsed.source.role, core.SourceRole.THEIRS)
        self.assertEqual(result.rejected_parse.parsed.diagnostics[0].code, "libcst.syntax")

    def test_changed_unowned_comment_layout_fails_closed(self):
        result = self.merge(["# base\na = 1\nb = 2\n", "# edited\na = 3\nb = 2\n", "# base\na = 1\nb = 4\n"])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertIsNone(result.output)
        self.assertTrue(result.diagnostics)

    def test_unsupported_top_level_and_ambiguous_names_fail_closed(self):
        for source in ["import os\n", "a = 1; b = 2\n", "a = b = 1\n", "a, b = (1, 2)\n",
                "a = 1\na = 2\n", "K = 1\nK = 2\n", "@decorator\ndef work():\n    pass\n"]:
            with self.subTest(source=source):
                with self.assertRaises(RuntimeError) as raised:
                    self.merge([source]*3)
                self.assertIn("unsupported_native_profile", str(raised.exception))

    def test_native_batch_and_native_syntax_failure(self):
        host = self.host
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


if __name__ == "__main__":
    unittest.main()
