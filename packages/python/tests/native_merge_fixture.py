"""Test-only Alef call adapter: native parsing in Python, all merge logic in Rust.

Reuse the installed-boundary suite's provider and request builder. No expected
output, ownership decision, or rendering logic belongs in this adapter.
"""
import structuredmerge_core as core
from structuredmerge_core import native_merge_profiles
from test_parser_host import LibCSTHost, TypedParserHostTest


def run_python_native_merge(base: str, ours: str, theirs: str):
    host = LibCSTHost()
    core.register_parser_host(host)
    try:
        result = TypedParserHostTest().merge([base, ours, theirs])
        assert host.calls > 0, "fixture must execute the native parser callback"
        return result
    finally:
        core.unregister_parser_host("python.libcst")
