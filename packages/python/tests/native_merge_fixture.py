"""Test-only Alef call adapter: native parsing in Python, all merge logic in Rust.

Reuse the installed-boundary suite's provider and request builder. No expected
output, ownership decision, or rendering logic belongs in this adapter.
"""
import structuredmerge_core as core
from structuredmerge_core import native_merge_profiles
from test_parser_host import LibCSTHost, TypedParserHostTest


def run_python_common(operation, sources):
    host = LibCSTHost()
    core.register_parser_host(host)
    try:
        request = TypedParserHostTest().common_request(operation, sources)
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        result = core.execute_operation(request, limits)
        assert host.calls > 0, "common fixture must execute the native parser callback"
        return result
    finally:
        core.unregister_parser_host("python.libcst")


def run_python_common_analyze(source):
    return run_python_common("analyze", [source])


def run_python_common_diff(before, after):
    return run_python_common("diff2", [before, after])


def run_python_common_merge(base, ours, theirs):
    return run_python_common("merge3", [base, ours, theirs])


def run_python_common_merge2(incoming, current):
    return run_python_common("merge2", [incoming, current])


def run_python_native_merge(base: str, ours: str, theirs: str):
    host = LibCSTHost()
    core.register_parser_host(host)
    try:
        result = TypedParserHostTest().merge([base, ours, theirs])
        assert host.calls > 0, "fixture must execute the native parser callback"
        return result
    finally:
        core.unregister_parser_host("python.libcst")
