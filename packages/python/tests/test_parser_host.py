"""Installed-wheel native callbacks and Rust-owned declaration merge tests."""
import hashlib
import gc
import weakref
import threading
import time
from concurrent.futures import ThreadPoolExecutor
import ast
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
    def merge_requests(self, sources, shared_source_id=None):
        requests = []
        for role, source in zip([core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS], sources):
            data = source.encode("utf-8")
            descriptor = native.SourceDescriptor(
                source_id=shared_source_id or ("merge-output" if role == core.SourceRole.BASE else str(role)), role=role, byte_length=len(data), sha256=hashlib.sha256(data).hexdigest(),
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
        return requests[::-1]

    def test_explicit_source_edits_use_rust_without_parser_dispatch(self):
        data = "\ufeffé: one\r\nlast".encode("utf-8")
        source = core.SourceInput(descriptor=core.SourceDescriptor(
            source_id="edit-source", role=core.SourceRole.SOURCE, byte_length=len(data),
            sha256=hashlib.sha256(data).hexdigest(), encoding=core.SourceEncoding.UTF8,
            bom=True, line_endings=core.LineEndings(lf=0, crlf=1, bare_cr=0), final_newline=False), bytes=data)
        self.assertIs(core.SourceDescriptor, native.SourceDescriptor)
        self.assertIs(core.LineEndings, native.LineEndings)
        limits = core.SourceEditLimits(max_input_bytes=100, max_output_bytes=100, max_edits=2)
        request = core.SourceEditRequest(request_id="edit-1", source=source,
            edits=[core.ExplicitSourceEdit(start_byte=7, end_byte=10, replacement="two")])
        result = core.apply_explicit_source_edits(request, limits)
        self.assertEqual(result.output, "\ufeffé: two\r\nlast")
        self.assertEqual(result.request_id, "edit-1")
        self.assertEqual(result.edit_count, 1)
        self.assertEqual(result.source.sha256, source.descriptor.sha256)
        invalid = core.SourceEditRequest(request_id="invalid", source=source,
            edits=[core.ExplicitSourceEdit(start_byte=4, end_byte=5, replacement="x")])
        with self.assertRaisesRegex(RuntimeError, "source_edit.rejected:"):
            core.apply_explicit_source_edits(invalid, limits)
        self.assertEqual(self.host.calls, 0)

    def merge(self, sources):
        return core.merge_python_declarations(self.merge_requests(sources), core.ParseLimits(
            max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20))

    def setUp(self):
        self.assertTrue(Path(core.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()),
            "test must load the installed wheel, not the source package")
        self.host = LibCSTHost()
        core.register_parser_host(self.host)

    def tearDown(self):
        core.unregister_parser_host("python.libcst")

    def test_registered_callbacks_survive_gc_and_release_after_unregister(self):
        core.unregister_parser_host("python.libcst")
        try:
            for _ in range(12):
                host = LibCSTHost()
                reference = weakref.ref(host)
                core.register_parser_host(host)
                del host
                gc.collect()
                self.assertIsNotNone(reference(), "registry must retain its callback")
                result = self.merge(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
                self.assertEqual(result.output, "a = 3\nb = 4\n")
                core.unregister_parser_host("python.libcst")
                gc.collect()
                self.assertIsNone(reference(), "unregister must release the callback")
                self.assertEqual(self.merge(["a = 1\n"] * 3).input_failure.code, "selection.no_parser")
        finally:
            # Restore the setup provider so the ordinary teardown remains valid.
            try:
                core.unregister_parser_host("python.libcst")
            except RuntimeError:
                pass
            core.register_parser_host(self.host)

    def test_inflight_parse_survives_callback_unregister_and_allows_reregistration(self):
        core.unregister_parser_host("python.libcst")

        class UnregisteringHost(LibCSTHost):
            def parse_batch(self, request):
                core.unregister_parser_host("python.libcst")
                gc.collect()
                return super().parse_batch(request)

        host = UnregisteringHost()
        core.register_parser_host(host)
        requests = self.merge_requests(["a = 1\n"] * 3)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        try:
            results = core.parse_sources(requests, limits)
            self.assertEqual(len(results), 3)
            self.assertTrue(all(result.parsed.ok for result in results))
            self.assertEqual(host.calls, 1)
            with self.assertRaisesRegex(RuntimeError, r"selection\.no_parser:"):
                core.parse_sources(requests, limits)
        finally:
            core.register_parser_host(self.host)
        self.assertTrue(all(result.parsed.ok for result in core.parse_sources(requests, limits)))
        self.assertEqual(self.host.calls, 1)
        self.assertEqual(host.calls, 1, "future calls must not reach the removed provider")

    def test_two_runtime_threads_overlap_native_callbacks_without_crossing_results(self):
        core.unregister_parser_host("python.libcst")
        barrier = threading.Barrier(3, timeout=10)
        release = threading.Event()

        class OverlappingHost(LibCSTHost):
            def parse_batch(self, request):
                barrier.wait()
                if not release.wait(timeout=10):
                    raise RuntimeError("concurrent callback release timed out")
                return super().parse_batch(request)

        host = OverlappingHost()
        core.register_parser_host(host)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        batches = [self.merge_requests([f"worker = {index}\n"] * 3) for index in range(2)]
        try:
            with ThreadPoolExecutor(max_workers=2) as workers:
                pending = [workers.submit(core.parse_sources, requests, limits) for requests in batches]
                barrier.wait()  # Both callbacks hold the old snapshot now.
                core.unregister_parser_host("python.libcst")
                core.register_parser_host(self.host)
                release.set()
                results = [future.result(timeout=15) for future in pending]
            self.assertEqual(host.calls, 2)
            self.assertEqual(self.host.calls, 0)
            for requests, parsed in zip(batches, results):
                self.assertEqual(len(parsed), 3)
                self.assertTrue(all(result.parsed.ok for result in parsed))
                self.assertEqual([result.parsed.source.sha256 for result in parsed],
                    [request.source.descriptor.sha256 for request in requests])
            core.parse_sources(batches[0], limits)
            self.assertEqual(self.host.calls, 1)
            self.assertEqual(host.calls, 2)
        finally:
            release.set()
            barrier.abort()
            core.unregister_parser_host("python.libcst")
            core.register_parser_host(self.host)

    def test_deadlines_reject_before_dispatch_and_discard_late_native_results(self):
        requests = self.merge_requests(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
        def limits(millis):
            return core.ParseLimits(max_batch_items=3, max_input_bytes=10000,
                max_nodes=1000, max_diagnostics=20, timeout_millis=millis)
        for operation in (core.parse_sources, core.merge_python_declarations):
            with self.assertRaisesRegex(RuntimeError, r"execution\.deadline_exceeded:"):
                operation(requests, limits(0))
        self.assertEqual(self.host.calls, 0)

        original = self.host.parse_batch
        def slow_input(request):
            result = original(request)
            time.sleep(0.15)
            return result
        self.host.parse_batch = slow_input
        for operation in (core.parse_sources, core.merge_python_declarations):
            with self.assertRaisesRegex(RuntimeError, r"execution\.deadline_exceeded:"):
                operation(requests, limits(100))
        self.assertEqual(self.host.calls, 2)

        def slow_verification(request):
            result = original(request)
            if request.items[0].source.descriptor.role == core.SourceRole.OUTPUT:
                time.sleep(0.15)
            return result
        self.host.parse_batch = slow_verification
        with self.assertRaisesRegex(RuntimeError, r"execution\.deadline_exceeded:"):
            core.merge_python_declarations(requests, limits(100))
        self.assertEqual(self.host.calls, 4)
        def late_fault(request):
            original(request)
            time.sleep(0.15)
            raise RuntimeError("native failure after deadline")
        self.host.parse_batch = late_fault
        for operation in (core.parse_sources, core.merge_python_declarations):
            with self.assertRaisesRegex(RuntimeError, r"execution\.deadline_exceeded:"):
                operation(requests, limits(100))
        self.assertEqual(self.host.calls, 6)
        self.host.parse_batch = original
        self.assertEqual(core.merge_python_declarations(requests, limits(None)).output, "a = 3\nb = 4\n")

    def test_cancellation_is_shared_and_rejects_late_input_and_verification(self):
        requests = self.merge_requests(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        cancelled = core.create_operation_control()
        self.assertFalse(cancelled.is_cancelled())
        cancelled.cancel()
        cancelled.cancel()
        for operation in (core.parse_sources_controlled, core.merge_python_declarations_controlled):
            with self.assertRaisesRegex(RuntimeError, r"execution\.cancelled:"):
                operation(requests, limits, cancelled)
        self.assertEqual(self.host.calls, 0)
        original = self.host.parse_batch
        for operation, phase in ((core.parse_sources_controlled, "input"),
                (core.merge_python_declarations_controlled, "input"),
                (core.merge_python_declarations_controlled, "output")):
            control = core.create_operation_control()
            arrived, release = threading.Event(), threading.Event()
            def paused(request):
                result = original(request)
                is_output = request.items[0].source.descriptor.role == core.SourceRole.OUTPUT
                if is_output == (phase == "output"):
                    arrived.set()
                    if not release.wait(timeout=10):
                        raise RuntimeError("cancellation barrier timed out")
                return result
            self.host.parse_batch = paused
            try:
                with ThreadPoolExecutor(max_workers=1) as workers:
                    pending = workers.submit(operation, requests, limits, control)
                    self.assertTrue(arrived.wait(timeout=10))
                    control.cancel()
                    self.assertTrue(control.is_cancelled())
                    release.set()
                    with self.assertRaisesRegex(RuntimeError, r"execution\.cancelled:"):
                        pending.result(timeout=15)
            finally:
                release.set()
                self.host.parse_batch = original
        fresh = core.create_operation_control()
        self.assertFalse(fresh.is_cancelled())
        self.assertEqual(core.merge_python_declarations_controlled(requests, limits, fresh).output, "a = 3\nb = 4\n")

    def test_cancelled_callback_failure_does_not_override_operation_control(self):
        requests = self.merge_requests(["a = 1\n"] * 3)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        for operation in (core.parse_sources_controlled, core.merge_python_declarations_controlled):
            control = core.create_operation_control()
            def fail_after_cancel(request):
                control.cancel()
                raise RuntimeError("native failure after cancellation")
            self.host.parse_batch = fail_after_cancel
            with self.assertRaisesRegex(RuntimeError, r"execution\.cancelled:"):
                operation(requests, limits, control)

    def test_native_profiles_declare_scope_without_default_approval(self):
        profiles = core.native_merge_profiles()
        self.assertEqual([profile.family for profile in profiles], ["python", "yaml"])
        self.assertEqual(len({profile.id for profile in profiles}), 2)
        for profile in profiles:
            self.assertEqual(profile.operation, "merge3")
            self.assertEqual(profile.semantic_runtime, "rust")
            self.assertEqual(profile.merge_crate, "ast-merge")
            self.assertEqual(profile.parse_contract, "structuredmerge.parse-result/v1")
            self.assertTrue(profile.experimental)
            self.assertFalse(profile.approved_as_default)
            self.assertTrue(profile.syntax_scope)
            self.assertTrue(profile.limitations)
            self.assertTrue(callable(getattr(core, profile.entry_point)))
        self.assertEqual(self.host.calls, 0)

    def test_installed_native_type_declarations_match_exported_names(self):
        stub = Path(core.__file__).parent / "_native.pyi"
        declarations = ast.parse(stub.read_text(encoding="utf-8"))
        classes = [node for node in declarations.body if isinstance(node, ast.ClassDef)]
        self.assertTrue(classes)
        for declaration in classes:
            exported = getattr(native, declaration.name)
            for member in declaration.body:
                if isinstance(member, ast.FunctionDef):
                    self.assertTrue(hasattr(exported, member.name), f"{declaration.name}.{member.name}")
                elif isinstance(member, ast.AnnAssign):
                    self.assertTrue(hasattr(exported, member.target.id), f"{declaration.name}.{member.target.id}")
        for node in declarations.body:
            if isinstance(node, ast.FunctionDef):
                self.assertTrue(callable(getattr(native, node.name)))

    def test_portable_service_errors_cross_installed_binding(self):
        requests = self.merge_requests(["a = 1\n"] * 3)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        with self.assertRaisesRegex(RuntimeError, r"request\.invalid:"):
            core.parse_sources([], limits)
        limited = core.ParseLimits(max_batch_items=3, max_input_bytes=0, max_nodes=1000, max_diagnostics=20)
        for operation in (core.parse_sources, core.merge_python_declarations):
            with self.assertRaisesRegex(RuntimeError, r"resource\.limit:"):
                operation(requests, limited)
        self.assertEqual(self.host.calls, 0)
        duplicated = self.merge_requests(["a = 1\n", "a = 2\n", "a = 3\n"], shared_source_id="duplicate")
        for operation in (core.parse_sources, core.merge_python_declarations):
            with self.assertRaisesRegex(RuntimeError, r"source\.invalid:"):
                operation(duplicated, limits)
        self.assertEqual(self.host.calls, 0)

        def explode(request):
            self.host.calls += 1
            raise ValueError("native test failure")

        self.host.parse_batch = explode
        with self.assertRaisesRegex(RuntimeError, r"parser\.provider_fault:.*native test failure"):
            core.parse_sources(requests, limits)
        self.assertEqual(self.host.calls, 1)  # no retry or parser substitution
        failed = core.merge_python_declarations(requests, limits)
        self.assertEqual(failed.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertEqual(failed.input_failure.code, "parser.provider_fault")
        self.assertEqual(failed.input_failure.backend_id, "python.libcst")
        self.assertEqual(failed.profile_id, "kernel.python.native_declarations.v1")
        self.assertIn("native test failure", failed.input_failure.native_message)
        self.assertIsNone(failed.output)
        self.assertIsNone(failed.output_parse)
        self.assertEqual(failed.input_parses, [])
        self.assertEqual([source.source_id for source in failed.sources],
            [request.source.descriptor.source_id for request in requests[::-1]])
        self.assertEqual([source.sha256 for source in failed.sources],
            [request.source.descriptor.sha256 for request in requests[::-1]])
        self.assertEqual(self.host.calls, 2)

        self.host.parse_batch = lambda request: core.ParseBatchResult(items=[])
        with self.assertRaisesRegex(RuntimeError, r"parser\.invalid_batch:"):
            core.parse_sources(requests, limits)
        self.assertEqual(core.merge_python_declarations(requests, limits).input_failure.code, "parser.invalid_batch")

        core.unregister_parser_host("python.libcst")
        try:
            with self.assertRaisesRegex(RuntimeError, r"selection\.no_parser:"):
                core.parse_sources(requests, limits)
            failed = core.merge_python_declarations(requests, limits)
            self.assertEqual(failed.input_failure.code, "selection.no_parser")
            self.assertEqual(failed.input_failure.selection.requested.backend_id, "python.libcst")
            self.assertIsNone(failed.input_failure.selection.selected_backend)
            self.assertTrue(failed.input_failure.selection.digest)
            self.assertEqual(len(failed.sources), 3)
        finally:
            core.register_parser_host(self.host)

    def test_independent_assignments_preserve_exact_source(self):
        sources = [
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 3",
        ]
        result = self.merge(sources)
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.CLEAN)
        self.assertEqual(result.profile_id, "kernel.python.native_declarations.v1")
        self.assertEqual(result.output, "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 3")
        self.assertEqual(self.host.calls, 2)
        self.assertEqual([parsed.parsed.source.role for parsed in result.input_parses],
            [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS])
        self.assertTrue(all(parsed.parsed.ok for parsed in result.input_parses))
        self.assertEqual({parsed.selection.selected_backend for parsed in result.input_parses}, {"python.libcst"})
        self.assertEqual(len({parsed.selection.digest for parsed in result.input_parses}), 1)
        self.assertEqual(self.host.received_batch.items[0].source.descriptor.role, core.SourceRole.OUTPUT)
        by_role = dict(zip([str(role) for role in [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS]],
            [source.encode() for source in sources]))
        output = result.output.encode()
        self.assertEqual(result.output_source.sha256, hashlib.sha256(output).hexdigest())
        self.assertEqual(result.output_source.byte_length, len(output))
        self.assertTrue(result.output_parse.parsed.ok)
        self.assertEqual(result.output_parse.parsed.source.role, core.SourceRole.OUTPUT)
        self.assertEqual(result.output_parse.parsed.source.sha256, result.output_source.sha256)
        self.assertEqual(result.output_parse.selection.selected_backend, "python.libcst")
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

    def test_verification_rejection_retains_diagnostics_without_exposing_output(self):
        original = self.host.parse_batch
        def reject_output(request):
            if request.items[0].source.descriptor.role != core.SourceRole.OUTPUT:
                return original(request)
            item = request.items[0]
            # Controlled provider rejection tests the verification boundary,
            # not LibCST's acceptance of this otherwise valid rendered source.
            diagnostic = core.ParseDiagnostic(id="verification.reject", severity=core.ParseSeverity.ERROR,
                category="parse_error", message="controlled output rejection", source_role=core.SourceRole.OUTPUT,
                blocking=True, metadata={}, extra={}, code="test.output_rejected", span=None, node_id=None)
            return core.ParseBatchResult(items=[native.ParseOutput(request_id=item.request_id,
                source=item.source.descriptor, ok=False, root_id=None, nodes=[], comments=[],
                diagnostics=[diagnostic], extensions=[], metadata={}, extra={})])
        self.host.parse_batch = reject_output
        result = self.merge(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertIsNone(result.output)
        self.assertIsNone(result.output_source)
        self.assertEqual(result.source_segments, [])
        self.assertEqual(len(result.input_parses), 3)
        self.assertFalse(result.output_parse.parsed.ok)
        self.assertEqual(result.output_parse.parsed.diagnostics[0].code, "test.output_rejected")

    def test_verification_service_failures_retain_structured_origin(self):
        original = self.host.parse_batch
        for mode, code in [("raise", "parser.provider_fault"), ("empty", "parser.invalid_batch")]:
            calls = []
            def fail_output(request):
                if request.items[0].source.descriptor.role != core.SourceRole.OUTPUT:
                    return original(request)
                calls.append(mode)
                if mode == "raise":
                    raise ValueError("verification exploded")
                return core.ParseBatchResult(items=[])
            self.host.parse_batch = fail_output
            result = self.merge(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
            self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
            self.assertIsNone(result.output)
            self.assertIsNone(result.output_source)
            self.assertIsNone(result.output_parse)
            self.assertEqual(result.source_segments, [])
            self.assertEqual(len(result.input_parses), 3)
            self.assertEqual(result.verification_failure.code, code)
            self.assertEqual(result.verification_failure.backend_id, "python.libcst")
            self.assertEqual(calls, [mode])
            if mode == "raise":
                self.assertTrue(result.verification_failure.native_code)
                self.assertIn("verification exploded", result.verification_failure.native_message)
            else:
                self.assertIsNone(result.verification_failure.native_code)

    def test_whole_source_selection_does_not_fabricate_verification_parse(self):
        result = self.merge(["a = 1\n"] * 3)
        self.assertEqual(result.output, "a = 1\n")
        self.assertIsNone(result.output_parse)
        self.assertEqual(self.host.calls, 1)
        self.assertEqual(len(result.input_parses), 3)

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
        self.assertEqual(len(result.sources), 3)
        self.assertEqual([source.role for source in result.sources],
            [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertIsNone(result.output)
        self.assertIsNone(result.output_source)
        self.assertEqual(result.source_segments, [])
        self.assertEqual(result.rejected_parse.parsed.source.role, core.SourceRole.THEIRS)
        self.assertEqual(result.rejected_parse.parsed.diagnostics[0].code, "libcst.syntax")
        self.assertEqual(len(result.diagnostics), 1)
        self.assertEqual(result.diagnostics[0].severity, core.DiagnosticSeverity.ERROR)
        self.assertEqual(result.diagnostics[0].category, core.DiagnosticCategory.PARSE_ERROR)

    def test_native_failure_precedes_analysis_and_is_independent_of_request_order(self):
        sources = ["import os\n", "def broken(:\n", "class broken(:\n"]
        requests = self.merge_requests(sources)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        for ordered in (requests, requests[::-1]):
            result = core.merge_python_declarations(ordered, limits)
            self.assertEqual(result.rejected_parse.parsed.source.role, core.SourceRole.OURS)
            self.assertIsNone(result.output)
            self.assertIsNone(result.output_source)
            self.assertEqual(result.source_segments, [])
            self.assertEqual([source.sha256 for source in result.sources],
                [hashlib.sha256(source.encode()).hexdigest() for source in sources])
            self.assertEqual([parsed.parsed.source.sha256 for parsed in result.input_parses],
                [source.sha256 for source in result.sources])
            rejected = [parsed for parsed in result.input_parses if not parsed.parsed.ok]
            self.assertEqual([parsed.parsed.source.role for parsed in rejected],
                [core.SourceRole.OURS, core.SourceRole.THEIRS])
            self.assertEqual([parsed.parsed.diagnostics[0].code for parsed in rejected],
                ["libcst.syntax", "libcst.syntax"])
            self.assertEqual({parsed.backend.id for parsed in rejected}, {"python.libcst"})

    def test_changed_unowned_comment_layout_fails_closed(self):
        result = self.merge(["# base\na = 1\nb = 2\n", "# edited\na = 3\nb = 2\n", "# base\na = 1\nb = 4\n"])
        self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
        self.assertIsNone(result.output)
        self.assertTrue(result.diagnostics)

    def test_unsupported_top_level_and_ambiguous_names_fail_closed(self):
        for source in ["import os\n", "a = 1; b = 2\n", "a = b = 1\n", "a, b = (1, 2)\n",
                "a = 1\na = 2\n", "K = 1\nK = 2\n", "@decorator\ndef work():\n    pass\n"]:
            with self.subTest(source=source):
                result = self.merge([source]*3)
                self.assertEqual(result.outcome, core.ThreeWayMergeOutcome.ERROR)
                self.assertIsNone(result.output)
                self.assertIsNone(result.output_parse)
                self.assertEqual(result.source_segments, [])
                self.assertEqual(len(result.input_parses), 3)
                self.assertTrue(all(parsed.parsed.ok for parsed in result.input_parses))
                self.assertEqual([rejection.source_role for rejection in result.analysis_rejections],
                    [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS])
                self.assertEqual({rejection.code for rejection in result.analysis_rejections},
                    {"analysis.unsupported_profile"})
                self.assertTrue(all(rejection.message for rejection in result.analysis_rejections))
                self.assertEqual([rejection.source_id for rejection in result.analysis_rejections],
                    [source.source_id for source in result.sources])

        result = self.merge(["a = 1\n", "import os\n", "a = 2\n"])
        self.assertEqual([rejection.source_role for rejection in result.analysis_rejections], [core.SourceRole.OURS])

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
