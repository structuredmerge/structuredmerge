#!/usr/bin/env python3
"""Installed typed-core adapter for the retained Slice 1023 harness, not a harness."""
import base64
import hashlib
import json
import os
from pathlib import Path
import sys
import time

import structuredmerge_core as core

# Require the installed environment, never source-tree import/path injection.
if not Path(core.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()):
    raise RuntimeError("benchmark requires structuredmerge-core installed in its Python environment")

ROLES = {"merge2": ("incoming", "current"), "merge3": ("base", "ours", "theirs")}
PARSERS = set()


def execute(operation, family, dialect, texts, request_id):
    if family != "json" or dialect not in ("json", "jsonc", "json5") or operation not in ROLES:
        raise ValueError("unsupported typed benchmark combination")
    if len(texts) != len(ROLES[operation]):
        raise ValueError("incorrect source count")
    language = "json" if dialect == "json" else "json5"
    backend = "benchmark.typed." + language
    if backend not in PARSERS:
        core.register_language_pack_parser(backend, language)
        PARSERS.add(backend)
    sources = {}
    for name, text in zip(ROLES[operation], texts):
        role = getattr(core.SourceRole, name.upper())
        data = text.encode("utf-8")
        sources[role] = core.OperationSource(source_id=name, role=role, encoding="utf-8",
            content=text, byte_length=len(data), sha256=hashlib.sha256(data).hexdigest(), extra={})
    if operation == "merge2":
        policy = core.OperationPolicy.from_merge2(core.DirectionalMergePolicy(
            directional_merge="template-into-current", render_policy="source-preserving", extra={}))
        provider, profile = "kernel.json", "kernel.json.nested.v1"
    else:
        policy = core.OperationPolicy.from_merge3(core.ThreeWayMergePolicy(
            render_policy="source-preserving", fallback_policy="none", extra={}))
        provider, profile = "kernel.git.json", "kernel.git.json.v1"
    request = core.OperationRequest(schema="structuredmerge.operation-request/v1",
        request_id=request_id, operation=policy, sources=sources,
        provider_selection=core.MergeProviderSelection(provider_id=provider, family="json",
            dialect=dialect, profile_id=profile, required_capabilities=[operation], extra={}),
        parser_selection=core.OperationParserSelection(backend=backend, preference=[],
            required_capabilities=[], extra={}), extensions=[], metadata={}, extra={})
    result = core.execute_operation(request, core.ParseLimits(max_batch_items=3,
        max_input_bytes=16 * 1024 * 1024, max_nodes=1000000, max_diagnostics=1000))
    status = 0 if result.ok else (1 if result.conflicts else 2)
    output = result.output if result.ok else result.conflicted_output
    diagnostics = [{"category": str(item.canonical.category), "code": item.canonical.code, "message": item.canonical.message}
                   for item in result.diagnostics]
    summary = {"ok": result.ok, "provider_id": result.provider.provider_id,
               "profile_id": result.profile.profile_id, "conflict_count": len(result.conflicts),
               "diagnostics": diagnostics, "output": output}
    return status, output, summary


def serve():
    for line in sys.stdin:
        if not line.strip():
            continue
        started = time.monotonic_ns()
        request_id = None
        operation = None
        try:
            request = json.loads(line)
            request_id = request["request_id"]
            if request["schema_version"] != "structuredmerge.benchmark.adapter-request/v1":
                raise ValueError("unsupported adapter schema")
            operation = request["operation"]
            texts = [base64.b64decode(request["sources"][role], validate=True).decode("utf-8")
                     for role in ROLES[operation]]
            selector = request["selector"]
            status, output, result = execute(operation, selector["family"], selector["dialect"], texts, request_id)
            stderr = ""
        except (ValueError, KeyError, TypeError, RuntimeError) as error:
            status, output, result, stderr = 2, None, {}, str(error)
        response = {"schema_version": "structuredmerge.benchmark.adapter-response/v1",
            "request_id": request_id, "process_id": os.getpid(), "status": status,
            "operation": operation,
            "duration_ns": time.monotonic_ns() - started,
            "output_base64": base64.b64encode((output or "").encode("utf-8")).decode("ascii"),
            "result": result, "stderr": stderr}
        print(json.dumps(response), flush=True)


def main(args):
    if args == ["benchmark-provider-session"]:
        serve()
        return 0
    operation = "merge2" if args[:1] == ["benchmark-provider-merge2"] else "merge3"
    paths = args[1:] if operation == "merge2" else args
    expected = 3 if operation == "merge2" else 5
    if len(paths) != expected:
        raise ValueError("expected incoming current path or base ours theirs path marker-size")
    if operation == "merge3" and paths[4] != "7":
        raise ValueError("unsupported conflict marker size")
    texts = [Path(path).read_bytes().decode("utf-8") for path in paths[:len(ROLES[operation])]]
    status, output, result = execute(operation, os.environ.get("AST_MERGE_FAMILY"),
        os.environ.get("AST_MERGE_DIALECT"), texts, "benchmark.cold." + operation)
    if operation == "merge2":
        print(json.dumps(result))
    elif output is not None:
        Path(paths[1]).write_bytes(output.encode("utf-8"))
    for item in result["diagnostics"]:
        print(f"typed-core: {item['category']}: {item['code']}: {item['message']}", file=sys.stderr)
    return status


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except (ValueError, KeyError, TypeError, RuntimeError, OSError) as error:
        print(f"typed-core benchmark: {error}", file=sys.stderr)
        sys.exit(2)
