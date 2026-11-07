"""Installed-wheel native callbacks and Rust-owned declaration merge tests."""

import hashlib
import json
import gc
import weakref
import threading
import time
from concurrent.futures import ThreadPoolExecutor
import ast
import inspect
from pathlib import Path
import sys
import unittest
import tempfile
import subprocess
import textwrap

import structuredmerge_core as core
from structuredmerge_core import _native as native
from libcst_facts import LibCSTHost
import pytest


class TypedParserHostTest(unittest.TestCase):
    def test_registry_guard_executes_matching_state_and_rejects_retirement(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        generation = core.register_workflow_host(host)
        retired = False
        try:
            workflows = core.workflow_registry_inventory()
            parsers = core.parser_registry_inventory()
            expected = native.WorkflowRegistryExpectation(
                provider_generation=workflows.generation,
                provider_digest=workflows.descriptor_digest,
                parser_generation=parsers.generation,
                parser_digest=parsers.descriptor_digest,
            )
            result = core.execute_workflow_batch_at_registry(provider_id, request, expected, limits)
            assert result.selections[0].provider_generation == generation
            assert not result.approved_as_default
            assert len(host.calls) == 1
            parse_calls = self.host.calls
            core.unregister_workflow_host(provider_id, generation)
            retired = True
            with pytest.raises(RuntimeError, match="workflow.registry_stale"):
                core.execute_workflow_batch_at_registry(provider_id, request, expected, limits)
            control = core.create_operation_control()
            control.cancel()
            with pytest.raises(RuntimeError, match="execution.cancelled"):
                core.execute_workflow_batch_at_registry_controlled(provider_id, request, expected, limits, control)
            assert len(host.calls) == 1
            assert self.host.calls == parse_calls
        finally:
            if not retired:
                core.unregister_workflow_host(provider_id, generation)

    def test_source_free_workflow_reports_retain_snapshots_across_probe_retirement(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        generation = core.register_workflow_host(host)

        def descriptor_must_be_cached():
            raise AssertionError("source-free selection must use the cached workflow descriptor")

        host.descriptor = descriptor_must_be_cached
        query = native.MergeSelectionRequest(
            provider_id=provider_id,
            family="python",
            operation="analyze",
            profile="python.libcst.analysis.v1",
            required_capabilities=["analyze"],
            required_preservation=[],
            parser=native.ParserSelectionRequest(
                language="python",
                options=request.items[0].parse_options,
                selection=native.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
            ),
        )
        retired = []
        original_probe = self.host.probe_batch

        def retire_during_probe(batch):
            if not retired:
                retired.append(core.unregister_workflow_host(provider_id, generation))
            return original_probe(batch)

        self.host.probe_batch = retire_during_probe
        try:
            reports = core.workflow_selection_reports([query, query], limits)
            assert len(reports) == 2
            assert all(r.selected_provider == provider_id and r.provider_generation == generation for r in reports)
            assert reports[0].provider_digest == reports[1].provider_digest
            assert reports[0].parser_digest == reports[1].parser_digest
            assert self.host.calls == 0
            assert host.calls == []
            later = core.workflow_selection_reports([query], limits)[0]
            assert later.selected_provider is None
            assert "unknown_explicit_provider" in later.rejections
            control = core.create_operation_control()
            control.cancel()
            with pytest.raises(RuntimeError, match="execution.cancelled"):
                core.workflow_selection_reports_controlled([query], limits, control)
        finally:
            self.host.probe_batch = original_probe
            if not retired:
                core.unregister_workflow_host(provider_id, generation)

    def test_compiled_batch_preserves_policy_selection_over_native_parser(self):
        original = self.common_request("analyze", ["a = 1\n"])
        operation = native.OperationRequest(
            schema=original.schema,
            request_id=original.request_id,
            operation=original.operation,
            sources=original.sources,
            provider_selection=original.provider_selection,
            parser_selection=native.OperationParserSelection(
                preference=["python.libcst"], required_capabilities=[], extra={}
            ),
            extensions=[],
            metadata={},
            extra={},
        )
        item = native.WorkflowOperation(
            operation=operation,
            parser_language="python",
            parse_options=native.ParseOptions(comments=False, tokens=False, diagnostics=False, native_extensions=True),
        )
        _, _, _, limits = self.workflow_fixture()
        result = core.execute_workflow_batch("kernel.python", native.WorkflowBatchRequest(items=[item]), limits)
        assert result.execution_owner == native.WorkflowExecutionOwner.KERNEL
        assert not result.approved_as_default
        assert result.results[0].ok
        parser = result.results[0].profile.parser
        assert parser.requested_backend is None
        assert parser.selected_backend == "python.libcst"
        assert parser.selection_mode == "policy"
        assert self.host.calls == 1

    def test_compiled_workflows_are_listed_and_cannot_be_retired_by_hosts(self):
        inventory = core.workflow_registry_inventory()
        profiles = core.operation_profile_catalog().profiles
        ids = {p.provider_id for p in inventory.providers}
        assert {p.provider_id for p in profiles} <= ids
        assert native.WorkflowExecutionOwner.KERNEL != native.WorkflowExecutionOwner.HOST

        class ReservedHost:
            def descriptor(self):
                return next(p for p in inventory.providers if p.provider_id == "kernel.json")

            def execute_batch(self, request, control):
                raise AssertionError("reserved host must never execute")

        with pytest.raises(RuntimeError, match="workflow.reserved_provider"):
            core.register_workflow_host(ReservedHost())
        with pytest.raises(RuntimeError, match="workflow.reserved_provider"):
            core.replace_workflow_host(ReservedHost(), inventory.generation)
        for profile in profiles:
            with pytest.raises(RuntimeError, match="workflow.reserved_provider"):
                core.unregister_workflow_host(profile.provider_id, inventory.generation)
        assert core.workflow_registry_inventory().generation == inventory.generation

    def workflow_fixture(self):
        provider_id = "python.libcst.workflow"
        profile = "python.libcst.analysis.v1"

        class Host:
            def __init__(self, tag="original"):
                self.tag = tag
                self.calls = []
                self.cancel = False
                self.before_return = None

            def descriptor(self):
                return core.MergeProviderDescriptor(
                    provider_id=provider_id,
                    family="python",
                    role=native.MergeProviderRole.WORKFLOW,
                    operations=["analyze"],
                    dialects=[],
                    profiles=[profile],
                    capabilities=["analyze"],
                    preservation_guarantees=[],
                    priority=0,
                    parser_requirements=core.MergeParserRequirements(
                        languages=["python"], allowed_backend_ids=["python.libcst"]
                    ),
                    allowed_delegation_targets=[],
                    runtime="python",
                    package="libcst",
                    package_version="1.9.0",
                    metadata={"host_tag": json.dumps(self.tag)},
                    extensions=[],
                )

            def execute_batch(self, prepared, control):
                assert isinstance(prepared, native.PreparedWorkflowBatch)
                assert isinstance(control, native.OperationControl)
                assert not control.is_cancelled()
                self.calls.append(prepared)
                if self.before_return:
                    self.before_return()
                if self.cancel:
                    control.cancel()
                    raise RuntimeError("private host exception")
                results = []
                for item in prepared.items:
                    parsed = item.parses[0].parsed
                    source = item.operation.sources[native.SourceRole.SOURCE]
                    assert parsed.source.sha256 == source.sha256
                    assert parsed.source.source_id == source.source_id
                    assert len(parsed.nodes) > 0
                    results.append(
                        native.OperationResult(
                            schema="https://structuredmerge.org/schemas/provider-result/v1.json",
                            request_id=item.operation.request_id,
                            operation=native.OperationKind.ANALYZE,
                            ok=True,
                            provider=native.ResultProvider(provider_id=provider_id, family="python", extra={}),
                            profile=native.ResultProfile(
                                profile_id=profile,
                                parser=native.ResultParserSelection(
                                    requested_backend="python.libcst",
                                    selected_backend="python.libcst",
                                    selection_mode="explicit",
                                    extra={},
                                ),
                                extra={},
                            ),
                            diagnostics=[],
                            changes=[],
                            conflicts=[],
                            fallbacks=[],
                            render_report={},
                            verification=native.ResultVerification(
                                consumed_source_roles=[native.SourceRole.SOURCE], classification_reached=True, extra={}
                            ),
                            analysis=native.ResultAnalysis(
                                schema="structuredmerge.analysis-result/v1",
                                extra={"native_node_count": json.dumps(len(parsed.nodes))},
                            ),
                            extensions=[],
                            metadata={"host_tag": json.dumps(self.tag)},
                            extra={},
                        )
                    )
                return native.WorkflowBatchResult(items=results)

        items = []
        for index, text in enumerate(["a = 'λ'\n", "\ufeffb = 2\n"]):
            original = self.common_request("analyze", [text])
            operation = native.OperationRequest(
                schema=original.schema,
                request_id=f"workflow-{index}",
                operation=original.operation,
                sources=original.sources,
                parser_selection=original.parser_selection,
                provider_selection=native.MergeProviderSelection(
                    provider_id=provider_id,
                    family="python",
                    profile_id=profile,
                    required_capabilities=["analyze"],
                    extra={},
                ),
                extensions=[],
                metadata={},
                extra={},
            )
            items.append(
                native.WorkflowOperation(
                    operation=operation,
                    parser_language="python",
                    parse_options=native.ParseOptions(native_extensions=True),
                )
            )
        request = native.WorkflowBatchRequest(items=items)
        limits = native.WorkflowLimits(
            max_operations=4,
            max_request_bytes=1000000,
            max_response_bytes=1000000,
            parse=native.ParseLimits(max_batch_items=4, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20),
        )
        return Host, provider_id, request, limits

    def test_workflow_batch_uses_native_facts_and_shared_cancellation_handle(self):
        assert core.MergeProviderDescriptor is native.MergeProviderDescriptor
        assert core.MergeParserRequirements is native.MergeParserRequirements
        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        generation = core.register_workflow_host(host)
        try:
            execution = core.execute_workflow_batch(provider_id, request, limits)
            assert len(host.calls) == 1
            assert len(host.calls[0].items) == 2
            assert [result.request_id for result in execution.results] == ["workflow-0", "workflow-1"]
            assert execution.execution_owner == native.WorkflowExecutionOwner.HOST
            assert not execution.approved_as_default
            assert all(json.loads(result.analysis.extra["native_node_count"]) > 0 for result in execution.results)
            host.cancel = True
            control = core.create_operation_control()
            with pytest.raises(RuntimeError, match="execution.cancelled"):
                core.execute_workflow_batch_controlled(provider_id, request, limits, control)
            assert control.is_cancelled(), "callback must share the caller's cancellation state"
            assert len(host.calls) == 2, "workflow callbacks must not be retried"
        finally:
            core.unregister_workflow_host(provider_id, generation)

    def test_workflow_registry_retains_and_releases_python_callbacks(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        for index in range(8):
            host = host_factory(str(index))
            reference = weakref.ref(host)
            generation = core.register_workflow_host(host)
            del host
            try:
                gc.collect()
                assert reference() is not None
                execution = core.execute_workflow_batch(provider_id, request, limits)
                assert json.loads(execution.results[0].metadata["host_tag"]) == str(index)
            finally:
                core.unregister_workflow_host(provider_id, generation)
            gc.collect()
            assert reference() is None, "registry retirement must release the Python callback"

    def test_workflow_reentrant_replacement_keeps_inflight_snapshot_and_rejects_stale_writes(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        old, replacement = host_factory("old"), host_factory("replacement")
        generation = core.register_workflow_host(old)
        inventory = core.workflow_registry_inventory()
        current = [generation]

        def replace():
            current[0] = core.replace_workflow_host(replacement, generation)
            with pytest.raises(RuntimeError, match="StaleGeneration"):
                core.unregister_workflow_host(provider_id, generation)
            with pytest.raises(RuntimeError, match="StaleGeneration"):
                core.replace_workflow_host(host_factory("stale"), generation)

        old.before_return = replace
        try:
            execution = core.execute_workflow_batch(provider_id, request, limits)
            assert json.loads(execution.provider.metadata["host_tag"]) == "old"
            assert all(json.loads(result.metadata["host_tag"]) == "old" for result in execution.results)
            assert all(report.provider_generation == generation for report in execution.selections)
            assert len(old.calls) == 1
            assert replacement.calls == []
            assert (
                json.loads(next(p for p in inventory.providers if p.provider_id == provider_id).metadata["host_tag"])
                == "old"
            )
            subsequent = core.execute_workflow_batch(provider_id, request, limits)
            assert json.loads(subsequent.provider.metadata["host_tag"]) == "replacement"
            assert all(report.provider_generation == current[0] for report in subsequent.selections)
            assert len(replacement.calls) == 1
        finally:
            core.unregister_workflow_host(provider_id, current[0])

    def test_workflow_callback_can_unregister_itself_without_retry(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        generation = core.register_workflow_host(host)
        retired = []

        def unregister():
            retired.append(core.unregister_workflow_host(provider_id, generation))

        host.before_return = unregister
        try:
            execution = core.execute_workflow_batch(provider_id, request, limits)
            assert len(execution.results) == 2
            assert len(host.calls) == 1
            assert provider_id not in [p.provider_id for p in core.workflow_registry_inventory().providers]
            with pytest.raises(RuntimeError, match="workflow"):
                core.execute_workflow_batch(provider_id, request, limits)
            assert len(host.calls) == 1
        finally:
            if not retired:
                core.unregister_workflow_host(provider_id, generation)

    def test_workflow_callbacks_preserve_worker_thread_and_context(self):
        import contextvars

        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        worker_context = contextvars.ContextVar("workflow_worker")
        observations = []
        start = threading.Barrier(4, timeout=10)
        callback_barrier = threading.Barrier(4, timeout=10)

        def observe():
            observations.append((worker_context.get(), threading.get_ident()))
            callback_barrier.wait()

        host.before_return = observe
        generation = core.register_workflow_host(host)
        try:

            def execute(index):
                token = worker_context.set(index)
                try:
                    start.wait()
                    result = core.execute_workflow_batch(provider_id, request, limits)
                    return index, threading.get_ident(), len(result.results)
                finally:
                    worker_context.reset(token)

            with ThreadPoolExecutor(max_workers=4) as pool:
                results = list(pool.map(execute, range(16)))
            assert sorted(observations) == sorted((index, thread) for index, thread, _ in results)
            assert all(count == 2 for _, _, count in results)
            assert len({thread for _, thread, _ in results}) == 4
            assert len(host.calls) == 16
        finally:
            core.unregister_workflow_host(provider_id, generation)

    def test_workflow_discards_late_result_after_cross_thread_cancel_and_retirement(self):
        host_factory, provider_id, request, limits = self.workflow_fixture()
        host = host_factory()
        arrived, release = threading.Event(), threading.Event()

        def pause():
            arrived.set()
            if not release.wait(timeout=10):
                raise RuntimeError("workflow cancellation barrier timed out")

        host.before_return = pause
        generation = core.register_workflow_host(host)
        retired = False
        control = core.create_operation_control()
        try:
            with ThreadPoolExecutor(max_workers=1) as workers:
                pending = workers.submit(core.execute_workflow_batch_controlled, provider_id, request, limits, control)
                try:
                    assert arrived.wait(timeout=10)
                    control.cancel()
                    core.unregister_workflow_host(provider_id, generation)
                    retired = True
                finally:
                    release.set()
                with pytest.raises(RuntimeError, match="execution.cancelled"):
                    pending.result(timeout=15)
            assert len(host.calls) == 1
            assert provider_id not in [p.provider_id for p in core.workflow_registry_inventory().providers]
        finally:
            release.set()
            if not retired:
                core.unregister_workflow_host(provider_id, generation)

    def test_workflow_runtime_exits_with_registered_retired_and_cancelled_drained_hosts(self):
        script = textwrap.dedent("""
            import gc, sys, threading
            import structuredmerge_core as core
            from test_parser_host import LibCSTHost, TypedParserHostTest
            mode = sys.argv[1]
            fixture = TypedParserHostTest()
            core.register_parser_host(LibCSTHost())
            host_factory, provider_id, request, limits = fixture.workflow_fixture()
            host = host_factory()
            generation = core.register_workflow_host(host)
            if mode == "drained":
                entered, release = threading.Event(), threading.Event()
                def pause():
                    entered.set()
                    if not release.wait(5):
                        raise RuntimeError("workflow not released")
                host.before_return = pause
                control = core.create_operation_control()
                outcomes = []
                def run():
                    try:
                        core.execute_workflow_batch_controlled(provider_id, request, limits, control)
                        outcomes.append("unexpected success")
                    except RuntimeError as error:
                        outcomes.append(str(error))
                worker = threading.Thread(target=run)
                worker.start()
                try:
                    assert entered.wait(5), "workflow never entered"
                    core.unregister_workflow_host(provider_id, generation)
                    control.cancel()
                finally:
                    release.set()
                    worker.join(5)
                assert not worker.is_alive(), "workflow failed to drain"
                assert len(outcomes) == 1 and "execution.cancelled" in outcomes[0], outcomes
            else:
                result = core.execute_workflow_batch(provider_id, request, limits)
                assert len(result.results) == 2
                if mode == "retired":
                    core.unregister_workflow_host(provider_id, generation)
            assert len(host.calls) == 1
            del host
            gc.collect()
            print("ready-to-exit:" + mode, flush=True)
        """)
        for mode in ("registered", "retired", "drained"):
            for repetition in range(3):
                with self.subTest(mode=mode, repetition=repetition):
                    completed = subprocess.run(
                        [sys.executable, "-c", script, mode],
                        cwd=Path(__file__).resolve().parent,
                        capture_output=True,
                        text=True,
                        timeout=20,
                        check=False,
                    )
                    assert completed.returncode == 0, completed.stdout + completed.stderr
                    assert completed.stdout.strip() == "ready-to-exit:" + mode

    def test_capability_manifest_separates_support_probes_and_authority(self):
        limits = core.ParseLimits(max_batch_items=4, max_input_bytes=0, max_nodes=0, max_diagnostics=0)

        def query(profile="kernel.python.native_declarations.v1", backend="python.libcst"):
            return core.CapabilityQuery(
                profile_id=profile,
                operation=core.OperationKind.MERGE3,
                dialect=None,
                parser_selection=core.ParserSelection(backend_id=backend, preference=[], required_capabilities=[]),
            )

        probes = []
        original = self.host.probe_batch

        def probe(request):
            probes.append(request)
            return original(request)

        self.host.probe_batch = probe
        inventory = core.capability_manifest([], limits)
        assert isinstance(inventory, core.CapabilityManifest)
        assert inventory.schema == "structuredmerge.typed-capability-manifest/v1"
        assert len(inventory.profiles.profiles) == 8
        assert probes == []
        with pytest.raises(RuntimeError, match="capability.unknown_profile"):
            core.capability_manifest([query(), query("unknown")], limits)
        assert probes == []
        unsupported = core.CapabilityQuery(
            profile_id="kernel.git.json.v1",
            operation=core.OperationKind.ANALYZE,
            dialect=None,
            parser_selection=query().parser_selection,
        )
        observations = core.capability_manifest([unsupported, query(), query(backend="missing")], limits).observations
        assert not observations[0].operation_declared
        assert observations[0].parser_report is None
        assert observations[0].parser_eligible is None
        assert observations[1].parser_eligible
        assert observations[1].parser_report.selected_backend == "python.libcst"
        assert observations[1].parser_request.options.native_extensions
        assert not observations[2].parser_eligible
        assert all(not item.approved_as_default for item in observations)
        assert self.host.calls == 0
        control = core.create_operation_control()
        control.cancel()
        with pytest.raises(RuntimeError, match="execution.cancelled"):
            core.capability_manifest_controlled([], limits, control)
        with pytest.raises(RuntimeError, match="resource.limit"):
            core.capability_manifest([query()] * 5, limits)

        def faulty_probe(request):
            raise RuntimeError("injected capability probe failure")

        self.host.probe_batch = faulty_probe
        fault = core.capability_manifest([query()], limits).observations[0]
        assert not fault.parser_eligible
        assert fault.parser_report.candidates[0].probe_fault
        assert not fault.approved_as_default

    def test_capability_manifest_retains_snapshot_across_probe_retirement(self):
        limits = core.ParseLimits(max_batch_items=2, max_input_bytes=0, max_nodes=0, max_diagnostics=0)
        query = core.CapabilityQuery(
            profile_id="kernel.python.native_declarations.v1",
            operation=core.OperationKind.MERGE3,
            dialect=None,
            parser_selection=core.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
        )
        original = self.host.probe_batch
        probes = []

        def retire(request):
            if not probes:
                core.unregister_parser_host("python.libcst")
            probes.append(request)
            return original(request)

        self.host.probe_batch = retire
        try:
            manifest = core.capability_manifest([query, query], limits)
            assert len(probes) == 2
            for item in manifest.observations:
                assert item.parser_eligible
                assert item.parser_report.generation == manifest.parsers.generation
                assert item.parser_report.digest == manifest.parsers.descriptor_digest
            assert not core.parser_registry_inventory().providers
            assert self.host.calls == 0
        finally:
            self.host.probe_batch = original
            if probes:
                core.register_parser_host(self.host)

    def test_installed_runtime_exits_after_registered_retired_and_drained_callbacks(self):
        script = textwrap.dedent("""
            import gc, sys, threading
            import structuredmerge_core as core
            from test_parser_host import LibCSTHost, TypedParserHostTest
            mode = sys.argv[1]
            fixture = TypedParserHostTest()
            host = LibCSTHost()
            core.register_parser_host(host)
            requests = fixture.merge_requests(["a = 1\\n"] * 3)
            limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000,
                max_nodes=1000, max_diagnostics=20)
            if mode == "drained":
                entered, release = threading.Event(), threading.Event()
                parse = host.parse_batch
                def blocked(request):
                    entered.set()
                    if not release.wait(5):
                        raise RuntimeError("callback not released")
                    return parse(request)
                host.parse_batch = blocked
                control = core.create_operation_control()
                outcomes = []
                def run():
                    try:
                        core.parse_sources_controlled(requests, limits, control)
                        outcomes.append("unexpected success")
                    except RuntimeError as error:
                        outcomes.append(str(error))
                worker = threading.Thread(target=run)
                worker.start()
                try:
                    assert entered.wait(5), "callback never entered"
                    core.unregister_parser_host("python.libcst")
                    control.cancel()
                finally:
                    release.set()
                    worker.join(5)
                assert not worker.is_alive(), "operation failed to drain"
                assert len(outcomes) == 1 and "execution.cancelled" in outcomes[0], outcomes
                assert host.calls == 1
            else:
                parsed = core.parse_sources(requests, limits)
                assert len(parsed) == 3 and all(item.parsed.ok for item in parsed)
                assert host.calls == 1
                if mode == "retired":
                    core.unregister_parser_host("python.libcst")
            del host
            gc.collect()
            print("ready-to-exit:" + mode, flush=True)
        """)
        for mode in ("registered", "retired", "drained"):
            for repetition in range(3):
                with self.subTest(mode=mode, repetition=repetition):
                    completed = subprocess.run(
                        [sys.executable, "-c", script, mode],
                        cwd=Path(__file__).resolve().parent,
                        capture_output=True,
                        text=True,
                        timeout=20,
                        check=False,
                    )
                    assert completed.returncode == 0, completed.stdout + completed.stderr
                    assert completed.stdout.strip() == "ready-to-exit:" + mode

    def test_common_operation_catalog_is_not_availability_or_authority(self):
        probe_calls = []

        def unexpected_probe(request):
            probe_calls.append(request)
            raise AssertionError("catalog must not probe")

        self.host.probe_batch = unexpected_probe
        generation = core.parser_registry_inventory().generation
        catalog = core.operation_profile_catalog()
        assert isinstance(catalog, core.OperationProfileCatalog)
        assert catalog.schema == "structuredmerge.operation-profile-catalog/v1"
        assert len(catalog.profiles) == 8
        ids = [profile.id for profile in catalog.profiles]
        assert ids == sorted(ids)
        for profile in catalog.profiles:
            assert profile.parser_available is None
            assert not profile.approved_as_default
            assert profile.experimental
            assert profile.semantic_runtime == "rust"
            assert profile.limitations
            operations = [str(operation) for operation in profile.operations]
            expected = (
                ["merge3"]
                if profile.provider_id == "kernel.git.json"
                else ["analyze", "diff2", "merge3"]
                if profile.provider_id == "kernel.yaml"
                else ["analyze", "diff2", "merge2", "merge3"]
            )
            assert operations == expected
        assert core.parser_registry_inventory().generation == generation
        assert probe_calls == []
        assert self.host.calls == 0
        assert len(core.native_merge_profiles()) == 2

    def test_atomic_replacement_during_callback_preserves_old_operation(self):
        replacement = LibCSTHost()
        generation = core.parser_registry_inventory().generation
        with pytest.raises(TypeError):
            core.replace_parser_host(replacement, None)
        with pytest.raises(RuntimeError, match="StaleGeneration"):
            core.replace_parser_host(replacement, generation - 1)
        assert core.parser_registry_inventory().generation == generation
        original_parse = self.host.parse_batch

        def replace_during_parse(request):
            assert core.replace_parser_host(replacement, generation) == generation + 1
            return original_parse(request)

        self.host.parse_batch = replace_during_parse
        requests = self.merge_requests(["a = 1\n"] * 3)[:1]
        limits = core.ParseLimits(max_batch_items=1, max_input_bytes=1000, max_nodes=100, max_diagnostics=20)
        assert core.parse_sources(requests, limits)[0].parsed.ok
        assert self.host.calls == 1
        assert replacement.calls == 0
        assert core.parse_sources(requests, limits)[0].parsed.ok
        assert self.host.calls == 1
        assert replacement.calls == 1
        with pytest.raises(RuntimeError, match="StaleGeneration"):
            core.replace_parser_host(self.host, generation)
        core.unregister_parser_host("python.libcst")
        try:
            with pytest.raises(RuntimeError, match="UnknownId"):
                core.replace_parser_host(replacement, core.parser_registry_inventory().generation)
        finally:
            core.register_parser_host(self.host)

    def test_selection_report_probes_without_parsing_source(self):
        def query(backend, comments=False):
            return core.ParserSelectionRequest(
                language="python",
                dialect=None,
                selection=core.ParserSelection(backend_id=backend, preference=[], required_capabilities=[]),
                options=core.ParseOptions(comments=comments, tokens=False, diagnostics=False, native_extensions=False),
            )

        limits = core.ParseLimits(
            max_batch_items=1, max_input_bytes=0, max_nodes=0, max_diagnostics=0, timeout_millis=None
        )
        report = core.parser_selection_report(query("python.libcst"), limits)
        assert report.selected_backend == "python.libcst"
        candidate = next(item for item in report.candidates if item.backend_id == "python.libcst")
        assert candidate.available
        assert candidate.loadable
        unsupported = core.parser_selection_report(query("python.libcst", comments=True), limits)
        assert unsupported.selected_backend is None
        rejected = next(item for item in unsupported.candidates if item.backend_id == "python.libcst")
        assert "missing_capability:comments" in rejected.rejections
        assert rejected.available is None
        missing = core.parser_selection_report(query("missing.inventory.parser"), limits)
        assert missing.selected_backend is None
        assert not any(item.selected for item in missing.candidates)
        assert next(item for item in missing.candidates if item.backend_id == "python.libcst").available is None
        control = core.create_operation_control()
        control.cancel()
        with pytest.raises(RuntimeError, match="execution.cancelled"):
            core.parser_selection_report_controlled(query("python.libcst"), limits, control)
        assert self.host.calls == 0

    def test_inventory_is_owned_and_does_not_probe(self):
        class UnprobedHost(LibCSTHost):
            def probe_batch(self, request):
                raise AssertionError("inventory must not probe")

        host = UnprobedHost()
        core.unregister_parser_host("python.libcst")
        core.register_parser_host(host)
        try:
            before = core.parser_registry_inventory()
            assert isinstance(before, core.ParserRegistryInventory)
            assert before.schema == "structuredmerge.parser-registry-inventory/v1"
            ids = [provider.id for provider in before.providers]
            assert ids == sorted(ids)
            assert "python.libcst" in ids
            next(provider for provider in before.providers if provider.id == "python.libcst").languages.clear()
            unchanged = core.parser_registry_inventory()
            assert (
                "python"
                in next(provider for provider in unchanged.providers if provider.id == "python.libcst").languages
            )
            assert unchanged.generation == before.generation
            assert unchanged.descriptor_digest == before.descriptor_digest
            core.unregister_parser_host("python.libcst")
            removed = core.parser_registry_inventory()
            assert "python.libcst" not in [provider.id for provider in removed.providers]
            assert removed.generation > before.generation
            assert "python.libcst" in [provider.id for provider in before.providers]
            assert host.calls == 0
        finally:
            if any(provider.id == "python.libcst" for provider in core.parser_registry_inventory().providers):
                core.unregister_parser_host("python.libcst")
            core.register_parser_host(self.host)

    def test_common_json_merges_execute_in_rust_with_edit_evidence_and_conflicts(self):
        provider_id = "python.common.json"
        core.register_language_pack_parser(provider_id, "json")
        try:

            def request(operation, texts, dialect="json"):
                original = self.common_request(operation, texts)
                return core.OperationRequest(
                    schema=original.schema,
                    request_id=original.request_id,
                    operation=original.operation,
                    sources=original.sources,
                    extensions=[],
                    metadata={},
                    extra={},
                    provider_selection=core.MergeProviderSelection(
                        provider_id="kernel.json",
                        family="json",
                        dialect=dialect,
                        profile_id="kernel.json.nested.v1",
                        required_capabilities=[operation],
                        extra={},
                    ),
                    parser_selection=core.OperationParserSelection(
                        backend=provider_id, preference=[], required_capabilities=[], extra={}
                    ),
                )

            limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
            directional = core.execute_operation(request("merge2", ['{"x":{"add":2}}', '{"x":{"keep":1}}']), limits)
            assert directional.ok
            assert json.loads(directional.output) == {"x": {"keep": 1, "add": 2}}
            proof = json.loads(directional.render_report["evidence"])
            assert proof["baseline"]["role"] == "current"
            assert proof["edits"]
            assert directional.verification.directional_roles_preserved
            merged = core.execute_operation(
                request("merge3", ['{"a":1,"b":2}', '{"a":3,"b":2}', '{"a":1,"b":4}']), limits
            )
            assert merged.ok
            assert json.loads(merged.output) == {"a": 3, "b": 4}
            conflict = core.execute_operation(request("merge3", ['{"a":1}', '{"a":2}', '{"a":3}']), limits)
            assert not conflict.ok
            assert conflict.output is None
            assert len(conflict.conflicts[0].canonical.alternatives) == 3
            diff = core.execute_operation(request("diff2", ['{"x":1}', '{"x":2}']), limits)
            assert diff.ok
            assert diff.output is None
            assert [c.path for c in diff.changes] == [None, "", "/x"]
            assert json.loads(diff.diff.extra["document_bytes_compared"])
            trivia = core.execute_operation(request("diff2", ["{}", "{}\r\n"]), limits)
            assert trivia.ok
            assert len(trivia.changes) == 1
            assert trivia.changes[0].subject_ref == "json.document"
            core.unregister_parser_provider(provider_id)
            core.register_language_pack_parser(provider_id, "json5")
            analysis = core.execute_operation(request("analyze", ["{} /* unclaimed */"], "json5"), limits)
            assert analysis.ok
            assert analysis.output is None
            owners = json.loads(analysis.analysis.extra["owners"])
            comments = json.loads(analysis.analysis.extra["comment_regions"])
            assert owners[0]["id"] == "json:"
            assert comments[0]["owner_id"] == "json:"
            assert not comments[0]["attachment_resolved"]
            assert json.loads(analysis.analysis.extra["diagnostics"])[0]["code"] == "json.comment_attachment_unresolved"
            assert self.host.calls == 0
        finally:
            core.unregister_parser_provider(provider_id)

    def test_cached_language_pack_registration_fails_closed_without_acquisition(self):
        provider_id = "python.typed.cached.missing"
        descriptor = core.register_cached_language_pack_parser(provider_id, "not-a-real-grammar")
        try:
            assert json.loads(descriptor.metadata["grammar_policy"]) == "cached-only"
            with pytest.raises(RuntimeError, match="DuplicateId"):
                core.register_language_pack_parser(provider_id, "json")
            query = core.ParserSelectionRequest(
                language="not-a-real-grammar",
                selection=core.ParserSelection(backend_id=provider_id, preference=[], required_capabilities=[]),
                options=core.ParseOptions(),
            )
            report = core.parser_selection_report(
                query, core.ParseLimits(max_batch_items=1, max_input_bytes=0, max_nodes=0, max_diagnostics=0)
            )
            assert report.selected_backend is None
        finally:
            core.unregister_parser_provider(provider_id)

    def test_rust_language_pack_uses_typed_parse_and_shared_registry(self):
        provider_id = "python.typed.tslp.json"

        def request(text, backend=provider_id):
            source = self.merge_requests([text], roles=[core.SourceRole.SOURCE])[0].source
            return core.ParseRequest(
                schema="structuredmerge.parse-request/v1",
                request_id="json",
                source=source,
                language="json",
                dialect=None,
                selection=core.ParserSelection(backend_id=backend, preference=[], required_capabilities=[]),
                options=core.ParseOptions(comments=True, diagnostics=True, native_extensions=True),
                metadata={},
                extra={},
            )

        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        # Explicit acquisition in setup makes the cached-only test independent
        # of suite order and old caches. Registration alone is lazy.
        preparation_id = "python.typed.tslp.json.prepare"
        core.register_language_pack_parser(preparation_id, "json")
        try:
            assert core.parse_sources([request("{}", preparation_id)], limits)[0].parsed.ok
        finally:
            core.unregister_parser_provider(preparation_id)
        descriptor = core.register_cached_language_pack_parser(provider_id, "json")
        try:
            assert descriptor.runtime == "rust"
            assert descriptor.languages == ["json"]
            with pytest.raises(Exception, match="DuplicateId"):
                core.register_language_pack_parser(provider_id, "python")
            parsed = core.parse_sources([request('// note\r\n{"é": [true]}')], limits)[0]
            assert parsed.backend.id == provider_id
            assert parsed.selection.selected_backend == provider_id
            assert parsed.parsed.ok
            assert len(parsed.parsed.comments) == 1
            comment_id = parsed.parsed.comments[0].node_id
            comment = next(node for node in parsed.parsed.nodes if node.id == comment_id)
            assert comment.extensions[0].schema == "tree-haver.tree-sitter.node/v1"
            assert comment.extensions[0].payload == '{"extra":true}'
            assert any(edge.field_name == "key" for node in parsed.parsed.nodes for edge in node.children)
            broken = core.parse_sources([request('{"x":')], limits)[0].parsed
            assert not broken.ok
            assert broken.nodes
            assert any(node.has_error for node in broken.nodes)
            assert broken.diagnostics[0].blocking
            assert self.host.calls == 0
        finally:
            core.unregister_parser_provider(provider_id)
        with pytest.raises(Exception, match="selection.no_parser"):
            core.parse_sources([request("{}")], limits)

    def merge_requests(self, sources, shared_source_id=None, roles=None):
        requests = []
        for role, source in zip(
            roles or [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS], sources, strict=False
        ):
            data = source.encode("utf-8")
            descriptor = native.SourceDescriptor(
                source_id=shared_source_id or ("merge-output" if role == core.SourceRole.BASE else str(role)),
                role=role,
                byte_length=len(data),
                sha256=hashlib.sha256(data).hexdigest(),
                encoding=core.SourceEncoding.UTF8,
                bom=data.startswith(b"\xef\xbb\xbf"),
                line_endings=native.LineEndings(
                    lf=data.count(b"\n") - data.count(b"\r\n"), crlf=data.count(b"\r\n"), bare_cr=0
                ),
                final_newline=data.endswith(b"\n"),
            )
            requests.append(
                core.ParseRequest(
                    schema="structuredmerge.parse-request/v1",
                    request_id=str(role),
                    source=core.SourceInput(descriptor=descriptor, bytes=data),
                    language="python",
                    dialect=None,
                    selection=core.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
                    options=core.ParseOptions(native_extensions=True),
                    metadata={},
                    extra={},
                )
            )
        return requests[::-1]

    def diff_request(self, sources, roles=None):
        assert core.ParseRequest is native.ParseRequest
        assert core.ParseOptions is native.ParseOptions
        return core.NativeDiffRequest(
            request_id="diff-python",
            profile_id="kernel.python.native_declarations.v1",
            parses=self.merge_requests(sources, roles=roles or [core.SourceRole.BEFORE, core.SourceRole.AFTER]),
        )

    def test_native_diff_uses_rust_classification_and_exact_roles(self):
        result = core.diff_native_owners(
            self.diff_request(["# header\r\na = 1\r\nb = 2", "# header\r\na = 3\r\nc = 4"]),
            core.ParseLimits(max_batch_items=2, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20),
        )
        assert result.ok
        assert result.request_id == "diff-python"
        assert [c.kind for c in result.diff.changes] == [
            core.OwnerChangeKind.EDITED,
            core.OwnerChangeKind.DELETED,
            core.OwnerChangeKind.ADDED,
        ]
        assert [c.id for c in result.diff.changes] == ["change-0", "change-1", "change-2"]
        assert result.diff.changes[0].before.range.start_byte == len(b"# header\r\n")
        assert result.diff.changes[1].after is None
        assert result.diff.changes[2].before is None
        assert [p.parsed.source.role for p in result.input_parses] == [core.SourceRole.BEFORE, core.SourceRole.AFTER]
        assert not hasattr(result, "output")
        assert self.host.calls == 1

    def test_native_diff_failures_retain_origin_and_never_return_partial_changes(self):
        limits = core.ParseLimits(max_batch_items=2, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        result = core.diff_native_owners(self.diff_request(["a = (\n", "a = 2\n"]), limits)
        assert not result.ok
        assert result.diff is None
        assert result.input_parses[0].parsed.source.role == core.SourceRole.BEFORE
        assert not result.input_parses[0].parsed.ok
        result = core.diff_native_owners(self.diff_request(["a = 1\n", "import os\n"]), limits)
        assert not result.ok
        assert result.diff is None
        assert [r.source_role for r in result.analysis_rejections] == [core.SourceRole.AFTER]

    def test_native_diff_rejects_role_substitution_and_cancellation(self):
        limits = core.ParseLimits(max_batch_items=2, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        with pytest.raises(RuntimeError, match="invalid_diff_inputs"):
            core.diff_native_owners(
                self.diff_request(["a = 1", "a = 2"], roles=[core.SourceRole.INCOMING, core.SourceRole.CURRENT]), limits
            )
        control = core.create_operation_control()
        control.cancel()
        with pytest.raises(RuntimeError, match="execution.cancelled"):
            core.diff_native_owners_controlled(self.diff_request(["a = 1", "a = 2"]), limits, control)
        assert self.host.calls == 0

    def test_template_reports_use_public_typed_inputs(self):
        def options(mode):
            return core.TemplateSessionOptions(
                mode=mode,
                template_root="",
                destination_root="",
                context=core.TemplateDestinationContext(project_name=None),
                default_strategy=core.TemplateStrategy.MERGE,
                overrides=[],
                replacements={},
                allowed_families=None,
                config=None,
            )

        assert core.TemplateDestinationContext is native.TemplateDestinationContext
        report = core.report_template_options(options(core.DirectorySessionMode.PLAN))
        assert not report.ready
        assert [d.reason for d in report.diagnostics] == ["missing_destination_root", "missing_template_root"]
        report = core.report_template_profile(
            core.TemplateProfileRequest(
                profile_name="missing", profiles={}, options=options(core.DirectorySessionMode.PLAN)
            )
        )
        assert "missing_profile" in [d.reason for d in report.diagnostics]
        profile = core.DirectorySessionProfile(
            mode=core.DirectorySessionMode.APPLY,
            context=core.TemplateDestinationContext(project_name="widget"),
            default_strategy=core.TemplateStrategy.RAW_COPY,
            overrides=[],
            replacements={"NAME": "widget"},
            allowed_families=["markdown"],
            config=None,
        )
        configured = core.TemplateSessionOptions(
            mode=core.DirectorySessionMode.PLAN,
            template_root="/not-read/templates",
            destination_root="/not-read/destination",
            context=core.TemplateDestinationContext(project_name=None),
            default_strategy=core.TemplateStrategy.MERGE,
            overrides=[],
            replacements={},
            allowed_families=None,
            config=None,
        )
        report = core.report_template_profile(
            core.TemplateProfileRequest(profile_name="known", profiles={"known": profile}, options=configured)
        )
        assert report.ready
        assert report.resolved_options.context.project_name == "widget"
        assert report.resolved_options.replacements == {"NAME": "widget"}
        assert report.mode == core.DirectorySessionMode.APPLY
        with pytest.raises(RuntimeError, match="template.request.invalid"):
            core.plan_template_directory(options(core.DirectorySessionMode.APPLY))

    def test_template_directory_plan_does_not_write(self):
        Path("tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="typed-template-", dir="tmp") as root:
            template, destination = Path(root) / "template", Path(root) / "destination"
            template.mkdir()
            destination.mkdir()
            text = "# é\r\n".encode("utf-8")
            (template / "README.md").write_bytes(text)
            report = core.plan_template_directory(
                core.TemplateSessionOptions(
                    mode=core.DirectorySessionMode.PLAN,
                    template_root=str(template),
                    destination_root=str(destination),
                    context=core.TemplateDestinationContext(project_name="widget"),
                    default_strategy=core.TemplateStrategy.RAW_COPY,
                    overrides=[],
                    replacements={},
                    allowed_families=None,
                    config=None,
                )
            )
            assert report.runner_report.plan_report.summary.create == 1
            assert report.runner_report.preview.result_files["README.md"].encode("utf-8") == text
            assert list(destination.iterdir()) == []
            assert (template / "README.md").read_bytes() == text

    def test_explicit_source_edits_use_rust_without_parser_dispatch(self):
        data = "\ufeffé: one\r\nlast".encode("utf-8")
        source = core.SourceInput(
            descriptor=core.SourceDescriptor(
                source_id="edit-source",
                role=core.SourceRole.SOURCE,
                byte_length=len(data),
                sha256=hashlib.sha256(data).hexdigest(),
                encoding=core.SourceEncoding.UTF8,
                bom=True,
                line_endings=core.LineEndings(lf=0, crlf=1, bare_cr=0),
                final_newline=False,
            ),
            bytes=data,
        )
        assert core.SourceDescriptor is native.SourceDescriptor
        assert core.LineEndings is native.LineEndings
        limits = core.SourceEditLimits(max_input_bytes=100, max_output_bytes=100, max_edits=2)
        request = core.SourceEditRequest(
            request_id="edit-1",
            source=source,
            edits=[core.ExplicitSourceEdit(start_byte=7, end_byte=10, replacement="two")],
        )
        result = core.apply_explicit_source_edits(request, limits)
        assert result.output == "\ufeffé: two\r\nlast"
        assert result.request_id == "edit-1"
        assert result.edit_count == 1
        assert result.source.sha256 == source.descriptor.sha256
        invalid = core.SourceEditRequest(
            request_id="invalid",
            source=source,
            edits=[core.ExplicitSourceEdit(start_byte=4, end_byte=5, replacement="x")],
        )
        with pytest.raises(RuntimeError, match="source_edit.rejected:"):
            core.apply_explicit_source_edits(invalid, limits)
        assert self.host.calls == 0

    def test_structural_operation_reports_are_typed_and_preserve_unknowns(self):
        boundary = core.report_structural_boundary()
        assert boundary.package == "ast-crispr"
        assert boundary.metadata.source == "legacy_crispr_reference"
        assert [item.language for item in boundary.implementations] == ["go", "ruby", "rust", "typescript"]
        assert boundary.relationship.ast_merge
        coordinate = core.CrisprBoundaryImplementation(language="rust", package_name="ast-crispr", crate_="ast_crispr")
        assert coordinate.crate_name == "ast_crispr"
        limit = core.report_structural_limit(core.CrisprLimitRequest(constraints=None, counts=[0, 1, 2]))
        assert limit.description == "== 1"
        assert limit.allowed == [False, True, False]
        limit = core.report_structural_limit(
            core.CrisprLimitRequest(
                constraints=[
                    core.CrisprLimitConstraint(operator=core.CrisprLimitOperator.AT_LEAST, value=1),
                    core.CrisprLimitConstraint(operator=core.CrisprLimitOperator.AT_MOST, value=2),
                ],
                counts=[0, 1, 2, 3],
            )
        )
        assert limit.description == ">= 1 and <= 2"
        assert limit.allowed == [False, True, True, False]
        match = core.report_structural_match(
            core.CrisprMatchRequest(
                start_boundary="future", end_boundary="owner_end_plus_trailing_gap", payload_kind="comment_owned_body"
            )
        )
        assert not match.known_start_boundary
        assert match.trailing_gap_extended and match.comment_anchored
        selection = core.report_structural_selection(
            core.CrisprSelectionRequest(
                owner_scope="",
                owner_selector="",
                selector_kind="",
                selection_intent="",
                comment_region=None,
                include_trailing_gap=True,
            )
        )
        assert selection.comment_region is None
        assert selection.owner_selector == "line_bound_statements"
        destination = core.report_structural_destination(
            core.CrisprDestinationRequest(
                resolution_kind="", resolution_source="future", anchor_boundary="", used_if_missing=True
            )
        )
        assert destination.append_fallback and destination.used_if_missing
        assert not destination.known_resolution_source

        def request(kind):
            return core.CrisprOperationRequest(
                operation_kind=kind,
                source_requirement="required",
                destination_requirement="none",
                replacement_source="explicit_text",
                captures_source_text=True,
                supports_if_missing=False,
            )

        report = core.report_structural_operations([request("replace"), request("future")])
        assert report.operation_count == 2
        assert report.operation_kinds == ["replace", "future"]
        assert report.operation_profiles[0].requires_source
        assert report.operation_profiles[0].known_operation_kind
        assert not report.operation_profiles[1].known_operation_kind
        assert report.operation_profiles[1].operation_family == "unknown"
        assert self.host.calls == 0

    def merge(self, sources):
        return core.merge_python_declarations(
            self.merge_requests(sources),
            core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20),
        )

    def setUp(self):
        assert Path(core.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()), (
            "test must load the installed wheel, not the source package"
        )
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
                assert reference() is not None, "registry must retain its callback"
                result = self.merge(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
                assert result.output == "a = 3\nb = 4\n"
                core.unregister_parser_host("python.libcst")
                gc.collect()
                assert reference() is None, "unregister must release the callback"
                assert self.merge(["a = 1\n"] * 3).input_failure.code == "selection.no_parser"
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
            assert len(results) == 3
            assert all(result.parsed.ok for result in results)
            assert host.calls == 1
            with pytest.raises(RuntimeError, match=r"selection\.no_parser:"):
                core.parse_sources(requests, limits)
        finally:
            core.register_parser_host(self.host)
        assert all(result.parsed.ok for result in core.parse_sources(requests, limits))
        assert self.host.calls == 1
        assert host.calls == 1, "future calls must not reach the removed provider"

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
            assert host.calls == 2
            assert self.host.calls == 0
            for requests, parsed in zip(batches, results, strict=False):
                assert len(parsed) == 3
                assert all(result.parsed.ok for result in parsed)
                assert [result.parsed.source.sha256 for result in parsed] == [
                    request.source.descriptor.sha256 for request in requests
                ]
            core.parse_sources(batches[0], limits)
            assert self.host.calls == 1
            assert host.calls == 2
        finally:
            release.set()
            barrier.abort()
            core.unregister_parser_host("python.libcst")
            core.register_parser_host(self.host)

    def test_deadlines_reject_before_dispatch_and_discard_late_native_results(self):
        requests = self.merge_requests(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])

        def limits(millis):
            return core.ParseLimits(
                max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20, timeout_millis=millis
            )

        for operation in (core.parse_sources, core.merge_python_declarations):
            with pytest.raises(RuntimeError, match=r"execution\.deadline_exceeded:"):
                operation(requests, limits(0))
        assert self.host.calls == 0

        original = self.host.parse_batch

        def slow_input(request):
            result = original(request)
            time.sleep(0.15)
            return result

        self.host.parse_batch = slow_input
        for operation in (core.parse_sources, core.merge_python_declarations):
            with pytest.raises(RuntimeError, match=r"execution\.deadline_exceeded:"):
                operation(requests, limits(100))
        assert self.host.calls == 2

        def slow_verification(request):
            result = original(request)
            if request.items[0].source.descriptor.role == core.SourceRole.OUTPUT:
                time.sleep(0.15)
            return result

        self.host.parse_batch = slow_verification
        with pytest.raises(RuntimeError, match=r"execution\.deadline_exceeded:"):
            core.merge_python_declarations(requests, limits(100))
        assert self.host.calls == 4

        def late_fault(request):
            original(request)
            time.sleep(0.15)
            raise RuntimeError("native failure after deadline")

        self.host.parse_batch = late_fault
        for operation in (core.parse_sources, core.merge_python_declarations):
            with pytest.raises(RuntimeError, match=r"execution\.deadline_exceeded:"):
                operation(requests, limits(100))
        assert self.host.calls == 6
        self.host.parse_batch = original
        assert core.merge_python_declarations(requests, limits(None)).output == "a = 3\nb = 4\n"

    def test_cancellation_is_shared_and_rejects_late_input_and_verification(self):
        requests = self.merge_requests(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        cancelled = core.create_operation_control()
        assert not cancelled.is_cancelled()
        cancelled.cancel()
        cancelled.cancel()
        for operation in (core.parse_sources_controlled, core.merge_python_declarations_controlled):
            with pytest.raises(RuntimeError, match=r"execution\.cancelled:"):
                operation(requests, limits, cancelled)
        assert self.host.calls == 0
        original = self.host.parse_batch
        for operation, phase in (
            (core.parse_sources_controlled, "input"),
            (core.merge_python_declarations_controlled, "input"),
            (core.merge_python_declarations_controlled, "output"),
        ):
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
                    assert arrived.wait(timeout=10)
                    control.cancel()
                    assert control.is_cancelled()
                    release.set()
                    with pytest.raises(RuntimeError, match=r"execution\.cancelled:"):
                        pending.result(timeout=15)
            finally:
                release.set()
                self.host.parse_batch = original
        fresh = core.create_operation_control()
        assert not fresh.is_cancelled()
        assert core.merge_python_declarations_controlled(requests, limits, fresh).output == "a = 3\nb = 4\n"

    def test_cancelled_callback_failure_does_not_override_operation_control(self):
        requests = self.merge_requests(["a = 1\n"] * 3)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        for operation in (core.parse_sources_controlled, core.merge_python_declarations_controlled):
            control = core.create_operation_control()

            def fail_after_cancel(request):
                control.cancel()
                raise RuntimeError("native failure after cancellation")

            self.host.parse_batch = fail_after_cancel
            with pytest.raises(RuntimeError, match=r"execution\.cancelled:"):
                operation(requests, limits, control)

    def test_native_profiles_declare_scope_without_default_approval(self):
        profiles = core.native_merge_profiles()
        assert [profile.family for profile in profiles] == ["python", "yaml"]
        assert len({profile.id for profile in profiles}) == 2
        for profile in profiles:
            assert profile.operation == "merge3"
            assert profile.semantic_runtime == "rust"
            assert profile.merge_crate == "ast-merge"
            assert profile.parse_contract == "structuredmerge.parse-result/v1"
            assert profile.experimental
            assert not profile.approved_as_default
            assert profile.syntax_scope
            assert profile.limitations
            assert callable(getattr(core, profile.entry_point))
        assert self.host.calls == 0

    def test_installed_native_type_declarations_match_exported_names(self):
        stub = Path(core.__file__).parent / "_native.pyi"
        declarations = ast.parse(stub.read_text(encoding="utf-8"))
        classes = [node for node in declarations.body if isinstance(node, ast.ClassDef)]
        assert classes
        for declaration in classes:
            # TypedDict wire shapes exist only in the stub; actual native classes
            # still require every declared runtime member to be present.
            if any(isinstance(base, ast.Name) and base.id == "TypedDict" for base in declaration.bases):
                continue
            exported = getattr(native, declaration.name)
            for member in declaration.body:
                if isinstance(member, ast.FunctionDef):
                    assert hasattr(exported, member.name), f"{declaration.name}.{member.name}"
                elif isinstance(member, ast.AnnAssign):
                    assert hasattr(exported, member.target.id), f"{declaration.name}.{member.target.id}"
        for node in declarations.body:
            if isinstance(node, ast.FunctionDef):
                assert callable(getattr(native, node.name))

    @staticmethod
    def declared_parameters(declaration):
        args = declaration.args
        positional = args.posonlyargs + args.args
        defaults = [inspect.Parameter.empty] * (len(positional) - len(args.defaults))
        defaults += [ast.literal_eval(value) for value in args.defaults]
        expected = []
        for index, (argument, default) in enumerate(zip(positional, defaults, strict=False)):
            kind = (
                inspect.Parameter.POSITIONAL_ONLY
                if index < len(args.posonlyargs)
                else inspect.Parameter.POSITIONAL_OR_KEYWORD
            )
            expected.append((argument.arg, kind, default))
        if args.vararg:
            expected.append((args.vararg.arg, inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.empty))
        for argument, default in zip(args.kwonlyargs, args.kw_defaults, strict=False):
            expected.append(
                (
                    argument.arg,
                    inspect.Parameter.KEYWORD_ONLY,
                    inspect.Parameter.empty if default is None else ast.literal_eval(default),
                )
            )
        if args.kwarg:
            expected.append((args.kwarg.arg, inspect.Parameter.VAR_KEYWORD, inspect.Parameter.empty))
        return expected

    def test_installed_function_signatures_match_native_declarations(self):
        declarations = ast.parse((Path(core.__file__).parent / "_native.pyi").read_text(encoding="utf-8"))
        functions = [node for node in declarations.body if isinstance(node, ast.FunctionDef)]
        assert functions
        for declaration in functions:
            with self.subTest(function=declaration.name):
                expected = self.declared_parameters(declaration)
                for module in (native, core):
                    signature = inspect.signature(getattr(module, declaration.name))
                    actual = [
                        (parameter.name, parameter.kind, parameter.default)
                        for parameter in signature.parameters.values()
                    ]
                    assert actual == expected, f"{module.__name__}.{declaration.name}"

    def test_installed_constructor_and_method_signatures_match_native_declarations(self):
        declarations = ast.parse((Path(core.__file__).parent / "_native.pyi").read_text(encoding="utf-8"))
        checked = 0
        for declaration in declarations.body:
            if not isinstance(declaration, ast.ClassDef):
                continue
            if any(isinstance(base, ast.Name) and base.id == "TypedDict" for base in declaration.bases):
                continue
            for member in declaration.body:
                if not isinstance(member, ast.FunctionDef):
                    continue
                expected = self.declared_parameters(member)
                has_receiver = bool(expected and expected[0][0] in ("self", "cls"))
                if has_receiver:
                    expected = expected[1:]
                for module in (native, core):
                    with self.subTest(module=module.__name__, cls=declaration.name, method=member.name):
                        exported = getattr(module, declaration.name)
                        function = exported if member.name == "__init__" else getattr(exported, member.name)
                        parameters = list(inspect.signature(function).parameters.values())
                        # Compare caller-supplied arguments, not the implicit
                        # receiver in an unbound native method descriptor.
                        if (
                            has_receiver
                            and member.name != "__init__"
                            and parameters
                            and parameters[0].name in ("self", "cls")
                        ):
                            parameters = parameters[1:]
                        actual = [(parameter.name, parameter.kind, parameter.default) for parameter in parameters]
                        assert actual == expected
                checked += 1
        assert checked > 0

    def test_struct_enum_annotations_do_not_advertise_implicit_string_coercion(self):
        declarations = ast.parse((Path(core.__file__).parent / "_native.pyi").read_text(encoding="utf-8"))
        classes = [node for node in declarations.body if isinstance(node, ast.ClassDef)]
        enum_names = {
            node.name
            for node in classes
            if any(
                isinstance(member, ast.AnnAssign)
                and isinstance(member.target, ast.Name)
                and member.target.id.isupper()
                and isinstance(member.annotation, ast.Name)
                and member.annotation.id == node.name
                for member in node.body
            )
        }
        assert "OperationKind" in enum_names

        def union_names(annotation):
            if isinstance(annotation, ast.BinOp) and isinstance(annotation.op, ast.BitOr):
                return union_names(annotation.left) | union_names(annotation.right)
            return {annotation.id} if isinstance(annotation, ast.Name) else set()

        checked = 0
        for declaration in classes:
            for member in declaration.body:
                if not isinstance(member, ast.FunctionDef) or member.name != "__init__":
                    continue
                for argument in member.args.posonlyargs + member.args.args + member.args.kwonlyargs:
                    names = union_names(argument.annotation)
                    if names & enum_names:
                        with self.subTest(cls=declaration.name, parameter=argument.arg):
                            assert "str" not in names
                        checked += 1
        assert checked > 0

    def test_struct_enum_inputs_require_explicit_enum_construction(self):
        for module in (native, core):
            selection = module.ParserSelection(backend_id=None, preference=[], required_capabilities=[])

            def query(operation):
                return module.CapabilityQuery(
                    profile_id="kernel.python.native_declarations.v1",
                    operation=operation,
                    dialect=None,
                    parser_selection=selection,
                )

            assert str(query(module.OperationKind("merge3")).operation) == "merge3"
            assert str(query(module.OperationKind(0)).operation) == "analyze"
            for raw in ("merge3", 0, None):
                with self.subTest(module=module.__name__, required=raw), pytest.raises(TypeError):
                    query(raw)

            def diagnostic(**optional):
                return module.ResultDiagnostic(
                    id="test",
                    severity="info",
                    category="test",
                    code="test",
                    message="test",
                    blocking=False,
                    metadata={},
                    extra={},
                    **optional,
                )

            assert diagnostic().source_role is None
            assert diagnostic(source_role=None).source_role is None
            assert str(diagnostic(source_role=module.SourceRole("ours")).source_role) == "ours"
            for raw in ("ours", 0):
                with self.subTest(module=module.__name__, optional=raw), pytest.raises(TypeError):
                    diagnostic(source_role=raw)

    def test_rust_default_constructors_allow_omission_but_reject_none(self):
        for module in (native, core):
            for name, defaults in (
                ("LineEndings", {"lf": 0, "crlf": 0, "bare_cr": 0}),
                (
                    "ParseOptions",
                    {"comments": False, "tokens": False, "diagnostics": False, "native_extensions": False},
                ),
            ):
                constructor = getattr(module, name)
                value = constructor()
                for field, expected in defaults.items():
                    with self.subTest(module=module.__name__, cls=name, field=field):
                        assert getattr(value, field) == expected
                        assert type(getattr(value, field)) is type(expected)
                        with pytest.raises(TypeError):
                            constructor(**{field: None})
                        explicit = True if isinstance(expected, bool) else 3
                        assert getattr(constructor(**{field: explicit}), field) == explicit

    def test_native_source_roles_are_hashable_and_agree_with_integer_equality(self):
        roles = ["SOURCE", "BEFORE", "AFTER", "INCOMING", "CURRENT", "BASE", "OURS", "THEIRS", "OUTPUT"]
        for name in roles:
            role = getattr(native.SourceRole, name)
            copy = native.SourceRole(str(role))
            assert role == copy
            assert hash(role) == hash(copy)
            discriminant = int(role)
            assert role == discriminant
            assert hash(role) == hash(discriminant)
            assert {role: name}[copy] == name
            assert {role: name}[discriminant] == name

    def common_request(self, operation, texts, policy_override=None):
        roles = {
            "analyze": ["SOURCE"],
            "diff2": ["BEFORE", "AFTER"],
            "merge2": ["INCOMING", "CURRENT"],
            "merge3": ["BASE", "OURS", "THEIRS"],
        }[operation]
        sources = {}
        for role_name, text in zip(roles, texts, strict=False):
            role = getattr(native.SourceRole, role_name)
            data = text.encode("utf-8")
            sources[role] = native.OperationSource(
                source_id=role_name.lower(),
                role=role,
                byte_length=len(data),
                sha256=hashlib.sha256(data).hexdigest(),
                encoding="utf-8",
                content=text,
                extra={},
            )
        if operation == "analyze":
            policy = native.OperationPolicy.from_analyze(native.AnalyzePolicy(extra={}))
            assert isinstance(policy.analyze, native.AnalyzePolicy)
        elif operation == "diff2":
            policy = native.OperationPolicy.from_diff2(native.DiffPolicy(extra={}))
            assert isinstance(policy.diff2, native.DiffPolicy)
        elif operation == "merge2":
            policy = native.OperationPolicy.from_merge2(
                native.DirectionalMergePolicy(
                    directional_merge="template-into-current", render_policy="source-preserving", extra={}
                )
            )
        else:
            policy = native.OperationPolicy.from_merge3(
                native.ThreeWayMergePolicy(render_policy="source-preserving", fallback_policy="none", extra={})
            )
            assert isinstance(policy.merge3, native.ThreeWayMergePolicy)
        return native.OperationRequest(
            schema="structuredmerge.operation-request/v1",
            request_id="typed-common-" + operation,
            operation=policy_override if policy_override is not None else policy,
            sources=sources,
            provider_selection=native.MergeProviderSelection(
                provider_id="kernel.python",
                family="python",
                profile_id="kernel.python.native_declarations.v1",
                required_capabilities=[operation],
                extra={},
            ),
            parser_selection=native.OperationParserSelection(
                backend="python.libcst", preference=[], required_capabilities=[], extra={}
            ),
            extensions=[],
            metadata={},
            extra={},
        )

    def test_common_operations_use_typed_factories_maps_and_registered_libcst(self):
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        cases = [
            ("analyze", ["a = 1\n"]),
            ("diff2", ["a = 1\n", "a = 2\n"]),
            ("merge3", ["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"]),
        ]
        for operation, texts in cases:
            request = self.common_request(operation, texts)
            assert len(request.sources) == len(texts)
            assert isinstance(request.operation, native.OperationPolicy)
            request = self.common_request(operation, texts, policy_override=request.operation)
            result = core.execute_operation(request, limits)
            assert result.ok, str(result.diagnostics)
            assert result.request_id == request.request_id
            if operation == "analyze":
                assert isinstance(result.analysis, native.ResultAnalysis)
            elif operation == "diff2":
                assert result.changes
            else:
                assert result.output == "a = 3\nb = 4\n"
                assert result.verification.output_reparsed
        assert self.host.calls == 4

    def test_common_parser_result_limits_have_resource_limit_diagnostics(self):
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1, max_diagnostics=20)
        for operation, count in [("analyze", 1), ("diff2", 2), ("merge2", 2), ("merge3", 3)]:
            with self.subTest(operation=operation):
                request = self.common_request(operation, ["a = 1\nb = 2\n"] * count)
                result = core.execute_operation(request, limits)
                assert not result.ok
                assert result.output is None
                assert result.analysis is None
                assert not result.verification.classification_reached
                assert len(result.diagnostics) == 1
                diagnostic = result.diagnostics[0].canonical
                assert diagnostic.code == "resource.limit"
                assert str(diagnostic.category) == "resource_limit"
                assert diagnostic.blocking
                assert diagnostic.origin.backend_id == "python.libcst"
        assert self.host.calls == 4

    def test_common_merge2_preserves_current_trivia_and_direction(self):
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        request = self.common_request("merge2", ["a = 1\nb = 2 # new\n", "a = 9 # keep\n# footer"])
        result = core.execute_operation(request, limits)
        assert result.ok, str(result.diagnostics)
        assert result.output == "a = 9 # keep\nb = 2 # new\n# footer"
        assert result.verification.directional_roles_preserved
        assert result.verification.output_reparsed
        assert result.verification.base_participated is None
        assert result.verification.consumed_source_roles == [core.SourceRole.INCOMING, core.SourceRole.CURRENT]
        assert len(result.changes) == 1
        assert result.changes[0].path == "/b"
        reversed_result = core.execute_operation(self.common_request("merge2", ["a = 9\n", "a = 1\nb = 2\n"]), limits)
        assert reversed_result.output == "a = 1\nb = 2\n"
        assert self.host.calls == 4
        rejected = core.execute_operation(
            self.common_request("merge2", ["a = 1\nb = 2\nc = 3\n", "c = 30\na = 10\n"]), limits
        )
        assert not rejected.ok
        assert rejected.output is None
        assert rejected.diagnostics[0].canonical.code == "merge2.plan_unsupported"

    def test_common_control_and_factory_type_errors_fail_before_callbacks(self):
        with pytest.raises(TypeError):
            native.OperationPolicy.from_analyze(native.DiffPolicy(extra={}))
        request = self.common_request("analyze", ["a = 1\n"])
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        control = core.create_operation_control()
        control.cancel()
        with pytest.raises(RuntimeError, match=r"execution\.cancelled:"):
            core.execute_operation_controlled(request, limits, control)
        assert self.host.calls == 0

    def test_all_policy_variants_preserve_optional_values_and_typed_payloads(self):
        extra = {"future": "[null,false,7]"}
        policies = {
            "analyze": native.AnalyzePolicy(comments=False, ownership=True, extra=extra),
            "diff2": native.DiffPolicy(equivalence=[], source_preservation_evidence=False, extra=extra),
            "merge2": native.DirectionalMergePolicy(
                directional_merge="template-into-current", render_policy="source-preserving", extra=extra
            ),
            "merge3": native.ThreeWayMergePolicy(
                render_policy="source-preserving", labels={"ours": "local"}, conflict_marker_size=9, extra=extra
            ),
        }
        for name, payload in policies.items():
            policy = getattr(native.OperationPolicy, "from_" + name)(payload)
            count = {"analyze": 1, "diff2": 2, "merge2": 2, "merge3": 3}[name]
            restored = self.common_request(name, ["a = 1\n"] * count, policy_override=policy).operation
            assert isinstance(restored, native.OperationPolicy)
            for other in policies:
                if other != name:
                    assert getattr(restored, other) is None
            value = getattr(restored, name)
            assert isinstance(value, type(payload))
            assert value.extra == extra
            if name == "analyze":
                assert value.comments is False
                assert value.ownership is True
                assert value.tokens is None
            elif name == "diff2":
                assert value.equivalence == []
                assert value.source_preservation_evidence is False
            elif name == "merge2":
                assert value.directional_merge == "template-into-current"
            else:
                assert value.labels == {"ours": "local"}
                assert value.conflict_marker_size == 9
        assert self.host.calls == 0

    def test_common_canonical_records_keep_variants_and_typed_payloads(self):
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        request = self.common_request("merge3", ["a = 1\n", "a = 2\n", "a = 3\n"])
        result = core.execute_operation(request, limits)
        assert not result.ok
        assert result.output is None
        record = result.conflicts[0]
        assert isinstance(record, native.ConflictRecord)
        assert record.migration is None
        conflict = record.canonical
        assert isinstance(conflict, native.PortableConflict)
        assert conflict.code == "merge.edit_edit"
        assert [str(role) for role in conflict.roles] == ["base", "ours", "theirs"]
        restored = native.ConflictRecord.from_canonical(conflict)
        assert restored.canonical.id == conflict.id
        assert restored.canonical.decision_ids == conflict.decision_ids
        assert [a.source_id for a in restored.canonical.alternatives] == [a.source_id for a in conflict.alternatives]
        diagnostic = result.diagnostics[0]
        assert isinstance(diagnostic, native.DiagnosticRecord)
        assert diagnostic.migration is None
        assert diagnostic.canonical.code == "merge.edit_edit"
        restored_diagnostic = native.DiagnosticRecord.from_canonical(diagnostic.canonical)
        assert restored_diagnostic.canonical.id == diagnostic.canonical.id
        assert restored_diagnostic.canonical.blocking
        legacy = native.ResultDiagnostic(
            id="legacy",
            severity="error",
            category="unsupported",
            code="unsupported",
            message="legacy",
            blocking=True,
            metadata={},
            extra={},
        )
        migration = native.DiagnosticRecord.from_migration(legacy)
        assert migration.canonical is None
        assert migration.migration.id == "legacy"
        with pytest.raises(TypeError):
            native.DiagnosticRecord.from_canonical(legacy)
        legacy_conflict = native.ResultConflict(
            id="legacy-conflict",
            category="structural",
            roles=[],
            source_regions=[],
            localized=False,
            resolution="unresolved",
            metadata={},
            extra={},
        )
        migration_conflict = native.ConflictRecord.from_migration(legacy_conflict)
        assert migration_conflict.canonical is None
        assert migration_conflict.migration.id == "legacy-conflict"
        with pytest.raises(TypeError):
            native.ConflictRecord.from_canonical(legacy_conflict)
        rejected = core.execute_operation(self.common_request("analyze", ["a = (\n"]), limits)
        assert not rejected.ok
        parse_diagnostic = rejected.diagnostics[0].canonical
        assert parse_diagnostic.code == "parse.rejected"
        assert str(parse_diagnostic.source_refs[0].role) == "source"

    def test_portable_service_errors_cross_installed_binding(self):
        requests = self.merge_requests(["a = 1\n"] * 3)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        with pytest.raises(RuntimeError, match=r"request\.invalid:"):
            core.parse_sources([], limits)
        limited = core.ParseLimits(max_batch_items=3, max_input_bytes=0, max_nodes=1000, max_diagnostics=20)
        for operation in (core.parse_sources, core.merge_python_declarations):
            with pytest.raises(RuntimeError, match=r"resource\.limit:"):
                operation(requests, limited)
        assert self.host.calls == 0
        duplicated = self.merge_requests(["a = 1\n", "a = 2\n", "a = 3\n"], shared_source_id="duplicate")
        for operation in (core.parse_sources, core.merge_python_declarations):
            with pytest.raises(RuntimeError, match=r"source\.invalid:"):
                operation(duplicated, limits)
        assert self.host.calls == 0

        def explode(request):
            self.host.calls += 1
            raise ValueError("native test failure")

        self.host.parse_batch = explode
        with pytest.raises(RuntimeError, match=r"parser\.provider_fault:.*native test failure"):
            core.parse_sources(requests, limits)
        assert self.host.calls == 1  # no retry or parser substitution
        failed = core.merge_python_declarations(requests, limits)
        assert failed.outcome == core.ThreeWayMergeOutcome.ERROR
        assert failed.input_failure.code == "parser.provider_fault"
        assert failed.input_failure.backend_id == "python.libcst"
        assert failed.profile_id == "kernel.python.native_declarations.v1"
        assert "native test failure" in failed.input_failure.native_message
        assert failed.output is None
        assert failed.output_parse is None
        assert failed.input_parses == []
        assert [source.source_id for source in failed.sources] == [
            request.source.descriptor.source_id for request in requests[::-1]
        ]
        assert [source.sha256 for source in failed.sources] == [
            request.source.descriptor.sha256 for request in requests[::-1]
        ]
        assert self.host.calls == 2

        self.host.parse_batch = lambda request: core.ParseBatchResult(items=[])
        with pytest.raises(RuntimeError, match=r"parser\.invalid_batch:"):
            core.parse_sources(requests, limits)
        assert core.merge_python_declarations(requests, limits).input_failure.code == "parser.invalid_batch"

        core.unregister_parser_host("python.libcst")
        try:
            with pytest.raises(RuntimeError, match=r"selection\.no_parser:"):
                core.parse_sources(requests, limits)
            failed = core.merge_python_declarations(requests, limits)
            assert failed.input_failure.code == "selection.no_parser"
            assert failed.input_failure.selection.requested.backend_id == "python.libcst"
            assert failed.input_failure.selection.selected_backend is None
            assert failed.input_failure.selection.digest
            assert len(failed.sources) == 3
        finally:
            core.register_parser_host(self.host)

    def test_independent_assignments_preserve_exact_source(self):
        sources = [
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 2",
            "\ufeff# header\r\né = 'one'  # stable\r\nbeta = 3",
        ]
        result = self.merge(sources)
        assert result.outcome == core.ThreeWayMergeOutcome.CLEAN
        assert result.profile_id == "kernel.python.native_declarations.v1"
        assert result.output == "\ufeff# header\r\né = 'ours'  # stable\r\nbeta = 3"
        assert self.host.calls == 2
        assert [parsed.parsed.source.role for parsed in result.input_parses] == [
            core.SourceRole.BASE,
            core.SourceRole.OURS,
            core.SourceRole.THEIRS,
        ]
        assert all(parsed.parsed.ok for parsed in result.input_parses)
        assert {parsed.selection.selected_backend for parsed in result.input_parses} == {"python.libcst"}
        assert len({parsed.selection.digest for parsed in result.input_parses}) == 1
        assert self.host.received_batch.items[0].source.descriptor.role == core.SourceRole.OUTPUT
        by_role = dict(
            zip(
                [str(role) for role in [core.SourceRole.BASE, core.SourceRole.OURS, core.SourceRole.THEIRS]],
                [source.encode() for source in sources],
                strict=False,
            )
        )
        output = result.output.encode()
        assert result.output_source.sha256 == hashlib.sha256(output).hexdigest()
        assert result.output_source.byte_length == len(output)
        assert result.output_parse.parsed.ok
        assert result.output_parse.parsed.source.role == core.SourceRole.OUTPUT
        assert result.output_parse.parsed.source.sha256 == result.output_source.sha256
        assert result.output_parse.selection.selected_backend == "python.libcst"
        descriptors = {source.source_id: source for source in result.sources}
        assert result.output_source.source_id not in descriptors
        cursor = 0
        assert result.source_segments
        for segment in result.source_segments:
            assert segment.output_range.start_byte == cursor
            selected = by_role[str(segment.source_role)][
                segment.source_range.start_byte : segment.source_range.end_byte
            ]
            rendered = output[segment.output_range.start_byte : segment.output_range.end_byte]
            assert rendered == selected
            assert segment.sha256 == hashlib.sha256(rendered).hexdigest()
            assert descriptors[segment.source_id].role == segment.source_role
            cursor = segment.output_range.end_byte
        assert cursor == len(output)

    def test_verification_rejection_retains_diagnostics_without_exposing_output(self):
        original = self.host.parse_batch

        def reject_output(request):
            if request.items[0].source.descriptor.role != core.SourceRole.OUTPUT:
                return original(request)
            item = request.items[0]
            # Controlled provider rejection tests the verification boundary,
            # not LibCST's acceptance of this otherwise valid rendered source.
            diagnostic = core.ParseDiagnostic(
                id="verification.reject",
                severity=core.ParseSeverity.ERROR,
                category="parse_error",
                message="controlled output rejection",
                source_role=core.SourceRole.OUTPUT,
                blocking=True,
                metadata={},
                extra={},
                code="test.output_rejected",
                span=None,
                node_id=None,
            )
            return core.ParseBatchResult(
                items=[
                    native.ParseOutput(
                        request_id=item.request_id,
                        source=item.source.descriptor,
                        ok=False,
                        root_id=None,
                        nodes=[],
                        comments=[],
                        diagnostics=[diagnostic],
                        extensions=[],
                        metadata={},
                        extra={},
                    )
                ]
            )

        self.host.parse_batch = reject_output
        result = self.merge(["a = 1\nb = 2\n", "a = 3\nb = 2\n", "a = 1\nb = 4\n"])
        assert result.outcome == core.ThreeWayMergeOutcome.ERROR
        assert result.output is None
        assert result.output_source is None
        assert result.source_segments == []
        assert len(result.input_parses) == 3
        assert not result.output_parse.parsed.ok
        assert result.output_parse.parsed.diagnostics[0].code == "test.output_rejected"

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
            assert result.outcome == core.ThreeWayMergeOutcome.ERROR
            assert result.output is None
            assert result.output_source is None
            assert result.output_parse is None
            assert result.source_segments == []
            assert len(result.input_parses) == 3
            assert result.verification_failure.code == code
            assert result.verification_failure.backend_id == "python.libcst"
            assert calls == [mode]
            if mode == "raise":
                assert result.verification_failure.native_code
                assert "verification exploded" in result.verification_failure.native_message
            else:
                assert result.verification_failure.native_code is None

    def test_whole_source_selection_does_not_fabricate_verification_parse(self):
        result = self.merge(["a = 1\n"] * 3)
        assert result.output == "a = 1\n"
        assert result.output_parse is None
        assert self.host.calls == 1
        assert len(result.input_parses) == 3

    def test_function_and_class_bodies_merge_as_whole_declarations(self):
        base = "def café():\n    return 1\n\nclass Thing:\n    value = 2\n"
        result = self.merge([base, base.replace("return 1", "return 3"), base.replace("value = 2", "value = 4")])
        assert result.outcome == core.ThreeWayMergeOutcome.CLEAN
        assert result.output == base.replace("return 1", "return 3").replace("value = 2", "value = 4")

    def test_conflict_retains_three_source_alternatives_without_markers(self):
        result = self.merge(["a = 1\n", "a = 2\n", "a = 3\n"])
        assert result.outcome == core.ThreeWayMergeOutcome.CONFLICT
        assert result.output is None
        assert result.output_source is None
        assert result.source_segments == []
        assert result.conflicts[0].path == "/a"
        assert [item.revision for item in result.conflicts[0].alternatives] == [
            core.SourceRevision.BASE,
            core.SourceRevision.OURS,
            core.SourceRevision.THEIRS,
        ]

    def test_native_syntax_failure_retains_failing_role(self):
        result = self.merge(["a = 1\n", "a = 2\n", "def broken(:\n"])
        assert len(result.sources) == 3
        assert [source.role for source in result.sources] == [
            core.SourceRole.BASE,
            core.SourceRole.OURS,
            core.SourceRole.THEIRS,
        ]
        assert result.outcome == core.ThreeWayMergeOutcome.ERROR
        assert result.output is None
        assert result.output_source is None
        assert result.source_segments == []
        assert result.rejected_parse.parsed.source.role == core.SourceRole.THEIRS
        assert result.rejected_parse.parsed.diagnostics[0].code == "libcst.syntax"
        assert len(result.diagnostics) == 1
        assert result.diagnostics[0].severity == core.DiagnosticSeverity.ERROR
        assert result.diagnostics[0].category == core.DiagnosticCategory.PARSE_ERROR

    def test_native_failure_precedes_analysis_and_is_independent_of_request_order(self):
        sources = ["import os\n", "def broken(:\n", "class broken(:\n"]
        requests = self.merge_requests(sources)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        for ordered in (requests, requests[::-1]):
            result = core.merge_python_declarations(ordered, limits)
            assert result.rejected_parse.parsed.source.role == core.SourceRole.OURS
            assert result.output is None
            assert result.output_source is None
            assert result.source_segments == []
            assert [source.sha256 for source in result.sources] == [
                hashlib.sha256(source.encode()).hexdigest() for source in sources
            ]
            assert [parsed.parsed.source.sha256 for parsed in result.input_parses] == [
                source.sha256 for source in result.sources
            ]
            rejected = [parsed for parsed in result.input_parses if not parsed.parsed.ok]
            assert [parsed.parsed.source.role for parsed in rejected] == [core.SourceRole.OURS, core.SourceRole.THEIRS]
            assert [parsed.parsed.diagnostics[0].code for parsed in rejected] == ["libcst.syntax", "libcst.syntax"]
            assert {parsed.backend.id for parsed in rejected} == {"python.libcst"}

    def test_changed_unowned_comment_layout_fails_closed(self):
        result = self.merge(["# base\na = 1\nb = 2\n", "# edited\na = 3\nb = 2\n", "# base\na = 1\nb = 4\n"])
        assert result.outcome == core.ThreeWayMergeOutcome.ERROR
        assert result.output is None
        assert result.diagnostics

    def test_unsupported_top_level_and_ambiguous_names_fail_closed(self):
        for source in [
            "import os\n",
            "a = 1; b = 2\n",
            "a = b = 1\n",
            "a, b = (1, 2)\n",
            "a = 1\na = 2\n",
            "K = 1\nK = 2\n",
            "@decorator\ndef work():\n    pass\n",
        ]:
            with self.subTest(source=source):
                result = self.merge([source] * 3)
                assert result.outcome == core.ThreeWayMergeOutcome.ERROR
                assert result.output is None
                assert result.output_parse is None
                assert result.source_segments == []
                assert len(result.input_parses) == 3
                assert all(parsed.parsed.ok for parsed in result.input_parses)
                assert [rejection.source_role for rejection in result.analysis_rejections] == [
                    core.SourceRole.BASE,
                    core.SourceRole.OURS,
                    core.SourceRole.THEIRS,
                ]
                assert {rejection.code for rejection in result.analysis_rejections} == {"analysis.unsupported_profile"}
                assert all(rejection.message for rejection in result.analysis_rejections)
                assert [rejection.source_id for rejection in result.analysis_rejections] == [
                    source.source_id for source in result.sources
                ]

        result = self.merge(["a = 1\n", "import os\n", "a = 2\n"])
        assert [rejection.source_role for rejection in result.analysis_rejections] == [core.SourceRole.OURS]

    def test_native_batch_and_native_syntax_failure(self):
        host = self.host
        requests = []
        for index, data in enumerate(["é = 1\r\n".encode(), b"def broken(:\n"]):
            descriptor = native.SourceDescriptor(
                source_id=f"s{index}",
                role=core.SourceRole.SOURCE,
                byte_length=len(data),
                sha256=hashlib.sha256(data).hexdigest(),
                encoding=core.SourceEncoding.UTF8,
                bom=False,
                line_endings=native.LineEndings(lf=index, crlf=1 - index, bare_cr=0),
                final_newline=True,
            )
            requests.append(
                core.ParseRequest(
                    schema="structuredmerge.parse-request/v1",
                    request_id=f"r{index}",
                    source=core.SourceInput(descriptor=descriptor, bytes=data),
                    language="python",
                    dialect=None,
                    selection=core.ParserSelection(backend_id="python.libcst", preference=[], required_capabilities=[]),
                    options=core.ParseOptions(),
                    metadata={},
                    extra={},
                )
            )
        results = core.parse_sources(
            requests, core.ParseLimits(max_batch_items=2, max_input_bytes=1000, max_nodes=100, max_diagnostics=10)
        )
        assert host.calls == 1
        assert results[0].parsed.ok
        assert results[0].parsed.source.sha256 == requests[0].source.descriptor.sha256
        assert not results[1].parsed.ok
        assert results[1].parsed.diagnostics[0].code == "libcst.syntax"
        assert results[0].selection.selected_backend == "python.libcst"


if __name__ == "__main__":
    unittest.main()
