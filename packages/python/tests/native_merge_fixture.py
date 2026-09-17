"""Test-only Alef call adapter: explicit parser setup, all merge logic in Rust.

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


def run_json_common(operation, dialect, sources, git_options=None, family="json"):
    provider_id = f"python.fixture.{family}"
    core.register_language_pack_parser(provider_id, family if family in ("bash", "go", "rust") else ("json" if dialect == "json" else "json5"))
    try:
        original = TypedParserHostTest().common_request(operation, sources)
        policy = original.operation if git_options is None else core.OperationPolicy.from_merge3(
            core.ThreeWayMergePolicy(render_policy="source-preserving", conflict_marker_size=git_options[0],
                labels={"ours": git_options[1]}, extra={}))
        request = core.OperationRequest(schema=original.schema, request_id=original.request_id,
            operation=policy, sources=original.sources, extensions=[], metadata={}, extra={},
            provider_selection=core.MergeProviderSelection(provider_id=f"kernel.{family}" if family in ("bash", "go", "rust") else ("kernel.json" if git_options is None else "kernel.git.json"), family=family, dialect=dialect,
                profile_id=f"kernel.{family}.owners.v1" if family in ("bash", "go", "rust") else ("kernel.json.nested.v1" if git_options is None else "kernel.git.json.v1"), required_capabilities=[operation], extra={}),
            parser_selection=core.OperationParserSelection(backend=provider_id, preference=[], required_capabilities=[], extra={}))
        limits = core.ParseLimits(max_batch_items=3, max_input_bytes=10000, max_nodes=1000, max_diagnostics=20)
        return core.execute_operation(request, limits)
    finally:
        core.unregister_parser_provider(provider_id)


def run_json_common_analyze(dialect, source):
    return run_json_common("analyze", dialect, [source])


def run_json_common_diff(dialect, before, after):
    return run_json_common("diff2", dialect, [before, after])


def run_json_common_merge2(dialect, incoming, current):
    return run_json_common("merge2", dialect, [incoming, current])


def run_json_common_merge3(dialect, base, ours, theirs):
    return run_json_common("merge3", dialect, [base, ours, theirs])


def run_git_common_merge3(dialect, base, ours, theirs, marker_size, ours_label):
    return run_json_common("merge3", dialect, [base, ours, theirs], (marker_size, ours_label))


def run_rust_common_analyze(source):
    return run_json_common("analyze", "rust", [source], family="rust")


def run_rust_common_diff(before, after):
    return run_json_common("diff2", "rust", [before, after], family="rust")


def run_rust_common_merge2(incoming, current):
    return run_json_common("merge2", "rust", [incoming, current], family="rust")


def run_rust_common_merge3(base, ours, theirs):
    return run_json_common("merge3", "rust", [base, ours, theirs], family="rust")


def run_go_common_analyze(source):
    return run_json_common("analyze", "go", [source], family="go")


def run_go_common_diff(before, after):
    return run_json_common("diff2", "go", [before, after], family="go")


def run_go_common_merge2(incoming, current):
    return run_json_common("merge2", "go", [incoming, current], family="go")


def run_go_common_merge3(base, ours, theirs):
    return run_json_common("merge3", "go", [base, ours, theirs], family="go")


def run_bash_common_analyze(source):
    return run_json_common("analyze", "bash", [source], family="bash")


def run_bash_common_diff(before, after):
    return run_json_common("diff2", "bash", [before, after], family="bash")


def run_bash_common_merge2(incoming, current):
    return run_json_common("merge2", "bash", [incoming, current], family="bash")


def run_bash_common_merge3(base, ours, theirs):
    return run_json_common("merge3", "bash", [base, ours, theirs], family="bash")


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
