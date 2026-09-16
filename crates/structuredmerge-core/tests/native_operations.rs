//! Real Psych/LibCST facts through the common Rust operation dispatcher.
//! Explicit ignored runtime gates, not generated-binding artifact tests.
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
use structuredmerge_core::{
    native_operation::execute_native_operation,
    operation::{OperationRequest, ValidatedOperationRequest},
    portable_diagnostic::DiagnosticRecord,
    *,
};
use tree_haver::service::*;

#[derive(Clone, Copy)]
enum Behavior {
    Normal,
    Cancel,
    FailOutput,
}
#[derive(Clone, Copy)]
enum Runtime {
    Ruby,
    Python,
}
struct NativeParser {
    descriptor: ParserProviderDescriptor,
    calls: AtomicUsize,
    behavior: Behavior,
    runtime: Runtime,
}
impl ParserProvider for NativeParser {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        Ok(ParserProbeResult { available: true, loadable: true })
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let fault = |message: String| ProviderFault { code: self.descriptor.id.clone(), message };
        if matches!(self.behavior, Behavior::FailOutput)
            && requests[0].source.descriptor.role == SourceRole::Output
        {
            return Err(fault(
                "private parser exception that must not become a public message".into(),
            ));
        }
        let (variable, executable, script) = match self.runtime {
            Runtime::Ruby => (
                "STRUCTUREDMERGE_NATIVE_RUBY",
                "ruby",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../yaml-merge/tests/support/psych_facts.rb"),
            ),
            Runtime::Python => (
                "STRUCTUREDMERGE_NATIVE_PYTHON",
                "python3",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/python/tests/libcst_facts.py"),
            ),
        };
        let mut child = Command::new(std::env::var(variable).unwrap_or_else(|_| executable.into()))
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| fault(error.to_string()))?;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&requests).unwrap())
            .map_err(|error| fault(error.to_string()))?;
        let output = child.wait_with_output().map_err(|error| fault(error.to_string()))?;
        if !output.status.success() {
            return Err(fault(String::from_utf8_lossy(&output.stderr).into()));
        }
        if matches!(self.behavior, Behavior::Cancel) {
            context.cancelled.store(true, Ordering::Release);
        }
        serde_json::from_slice(&output.stdout).map_err(|error| fault(error.to_string()))
    }
}
fn setup(behavior: Behavior) -> (Arc<NativeParser>, ParserRegistrySnapshot, ExecutionContext) {
    setup_runtime(Runtime::Ruby, behavior)
}
fn setup_runtime(
    runtime: Runtime,
    behavior: Behavior,
) -> (Arc<NativeParser>, ParserRegistrySnapshot, ExecutionContext) {
    let (id, host, parser, language) = match runtime {
        Runtime::Ruby => ("test.psych", "ruby", "psych", "yaml"),
        Runtime::Python => ("test.libcst", "python", "libcst", "python"),
    };
    let provider = Arc::new(NativeParser {
        descriptor: serde_json::from_value(json!({
            "id": id, "family": "native", "runtime": host, "package": parser, "package_version": "test-runtime",
            "parser": parser, "parser_version": "test-runtime", "grammar": null, "grammar_version": null,
            "languages": [language], "dialects": [], "contracts": [PARSE_RESULT_SCHEMA],
            "capabilities": ["native_extensions", "source_spans"], "probe_id": id, "priority": 0, "metadata": {}, "extensions": []
        })).unwrap(), calls: AtomicUsize::new(0), behavior, runtime,
    });
    let registry = ParserRegistry::default();
    registry.register(provider.clone()).unwrap();
    (
        provider,
        registry.snapshot().unwrap(),
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 3,
            max_input_bytes: 4096,
            max_nodes: 1000,
            max_diagnostics: 20,
        },
    )
}
fn wire(operation: &str, sources: &[&str]) -> Value {
    let roles = match operation {
        "merge3" => vec!["base", "ours", "theirs"],
        "merge2" => vec!["incoming", "current"],
        "diff2" => vec!["before", "after"],
        "analyze" => vec!["source"],
        _ => unreachable!(),
    };
    let mut inputs = serde_json::Map::new();
    for (role, source) in roles.into_iter().zip(sources) {
        let input = source_input(
            role.into(),
            serde_json::from_value(json!(role)).unwrap(),
            SourceEncoding::Utf8,
            source.as_bytes().to_vec(),
        )
        .unwrap();
        inputs.insert(role.into(), json!({"source_id": role, "role": role, "content": source, "byte_length": input.descriptor.byte_length, "sha256": input.descriptor.sha256, "encoding": "utf-8"}));
    }
    json!({"schema": OPERATION_SCHEMA, "request_id": "native-op-1", "operation": operation,
        "provider_selection": {"provider_id": "kernel.yaml", "family": "yaml", "profile_id": "kernel.yaml.native_mapping.v1", "required_capabilities": [operation]},
        "parser_selection": {"backend": "test.psych", "preference": [], "required_capabilities": []}, "sources": inputs,
        "policy": match operation { "merge3" => json!({"render_policy": "source-preserving", "fallback_policy": "none"}), "merge2" => json!({"directional_merge": "template-into-current", "render_policy": "source-preserving"}), _ => json!({}) },
        "extensions": [], "metadata": {"caller": "integration"}, "future": {"opaque": [null, false, 7]}})
}
fn request(value: Value) -> ValidatedOperationRequest {
    serde_json::from_value::<OperationRequest>(value)
        .unwrap()
        .validate(4096, |_, _| panic!())
        .unwrap()
}
fn python_request(operation: &str, sources: &[&str]) -> ValidatedOperationRequest {
    let mut value = wire(operation, sources);
    value["provider_selection"]["provider_id"] = json!("kernel.python");
    value["provider_selection"]["family"] = json!("python");
    value["provider_selection"]["profile_id"] = json!("kernel.python.native_declarations.v1");
    value["parser_selection"]["backend"] = json!("test.libcst");
    request(value)
}
fn code(result: &operation_result::OperationResult) -> &str {
    let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
    &diagnostic.code
}

fn directional_requests(runtime: Runtime, incoming: &str, current: &str) -> Vec<ParseRequest> {
    let (language, backend) = match runtime {
        Runtime::Ruby => ("yaml", "test.psych"),
        Runtime::Python => ("python", "test.libcst"),
    };
    [(SourceRole::Incoming, incoming), (SourceRole::Current, current)]
        .into_iter()
        .map(|(role, text)| ParseRequest {
            schema: PARSE_REQUEST_SCHEMA.into(),
            request_id: format!("directional-{role:?}"),
            source: source_input(
                if role == SourceRole::Current {
                    "merge2-output".into()
                } else {
                    "incoming".into()
                },
                role,
                SourceEncoding::Utf8,
                text.as_bytes().to_vec(),
            )
            .unwrap(),
            language: language.into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: Some(backend.into()),
                preference: vec![],
                required_capabilities: vec![],
            },
            options: ParseOptions {
                comments: false,
                tokens: false,
                diagnostics: false,
                native_extensions: true,
            },
            metadata: Default::default(),
            extra: Default::default(),
        })
        .collect()
}

// Explicit unit/integration seam for one tail insertion; not production family
// placement policy. Native providers cannot supply this Rust planner callback.
fn directional_tail_plan(
    _: &ParsedResult,
    incoming: &ast_merge::SourcePreservingOwnerDocument,
    _: &ParsedResult,
    current: &ast_merge::SourcePreservingOwnerDocument,
) -> Result<Vec<ast_merge::directional_render::DirectionalInsertion>, String> {
    let added: Vec<_> =
        incoming.owners.iter().filter(|o| !current.owners.iter().any(|c| c.id == o.id)).collect();
    if added.len() != 1 || incoming.owners.last() != added.first().copied() {
        return Err("test requires one incoming-only tail owner".into());
    }
    Ok(vec![ast_merge::directional_render::DirectionalInsertion {
        owner_id: added[0].id.clone(),
        before_current_owner_id: None,
        current_offset: current.source.len(),
        source_range: ByteRange {
            start_byte: added[0].start_byte,
            end_byte: incoming.source.len(),
        },
    }])
}

fn check_directional_runtime(runtime: Runtime) {
    use ast_merge::typed_merge::NativeMergeError;
    use ast_merge::typed_merge2::merge_directional_native_sources;
    type Analyzer = fn(&ParsedResult) -> Result<ast_merge::SourcePreservingOwnerDocument, String>;
    let (language, incoming, current, expected, analyzer): (_, _, _, _, Analyzer) = match runtime {
        Runtime::Ruby => (
            "yaml",
            "alpha: incoming\nbeta: added\n",
            "alpha: current\n",
            "alpha: current\nbeta: added\n",
            yaml_merge::typed::mapping_owners,
        ),
        Runtime::Python => (
            "python",
            "alpha = 1\nbeta = 2\n",
            "alpha = 9\n",
            "alpha = 9\nbeta = 2\n",
            python_merge::declaration_owners,
        ),
    };
    for behavior in [Behavior::Normal, Behavior::FailOutput, Behavior::Cancel] {
        let (parser, snapshot, context) = setup_runtime(runtime, behavior);
        let mut requests = directional_requests(runtime, incoming, current);
        requests.reverse(); // Source roles, not incoming request order, control execution.
        let result = merge_directional_native_sources(
            language,
            requests,
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            analyzer,
            directional_tail_plan,
        );
        if matches!(behavior, Behavior::Cancel) {
            assert!(matches!(result, Err(NativeMergeError::Parse(ServiceError::Cancelled))));
            assert_eq!(parser.calls.load(Ordering::SeqCst), 1);
            continue;
        }
        let result = result.unwrap();
        assert_eq!(result.input_parses.len(), 2);
        assert_eq!(parser.calls.load(Ordering::SeqCst), 2);
        if matches!(behavior, Behavior::FailOutput) {
            assert!(result.rendered.is_err());
            assert!(result.verification_error.is_some());
            assert!(result.output_parse.is_none());
            continue;
        }
        let rendered = result.rendered.unwrap();
        assert_eq!(rendered.output, expected);
        assert!(result.verification_error.is_none());
        let verified = result.output_parse.unwrap();
        assert_eq!(verified.source.descriptor().role, SourceRole::Output);
        assert_eq!(verified.source.descriptor().source_id, "merge2-output_");
        assert_eq!(verified.source.bytes(), expected.as_bytes());
        assert!(
            rendered
                .segments
                .iter()
                .all(|s| matches!(s.source_role, SourceRole::Incoming | SourceRole::Current))
        );
    }
    let (parser, snapshot, context) = setup_runtime(runtime, Behavior::Normal);
    let mut requests = directional_requests(runtime, incoming, current);
    requests.pop();
    assert!(matches!(
        merge_directional_native_sources(
            language,
            requests,
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            analyzer,
            directional_tail_plan
        ),
        Err(NativeMergeError::InvalidInputs)
    ));
    assert_eq!(parser.calls.load(Ordering::SeqCst), 0);
    let (parser, snapshot, context) = setup_runtime(runtime, Behavior::Normal);
    let malformed = match runtime {
        Runtime::Ruby => "alpha: [\n",
        Runtime::Python => "alpha = (\n",
    };
    let failed = merge_directional_native_sources(
        language,
        directional_requests(runtime, malformed, current),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        analyzer,
        directional_tail_plan,
    );
    let Err(NativeMergeError::NativeParseRejected { parses, sources }) = failed else {
        panic!("syntax failure lost")
    };
    assert_eq!(sources.len(), 2);
    assert!(
        parses
            .iter()
            .any(|p| p.source.descriptor().role == SourceRole::Incoming && !p.document.output().ok)
    );
    assert_eq!(parser.calls.load(Ordering::SeqCst), 1);

    let (parser, snapshot, context) = setup_runtime(runtime, Behavior::Normal);
    let failed_plan = merge_directional_native_sources(
        language,
        directional_requests(runtime, current, current),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        analyzer,
        directional_tail_plan,
    )
    .unwrap();
    assert!(failed_plan.rendered.is_err());
    assert_eq!(failed_plan.input_parses.len(), 2);
    assert!(failed_plan.output_parse.is_none());
    assert_eq!(parser.calls.load(Ordering::SeqCst), 1);
}

#[test]
#[ignore = "requires native Ruby/Psych"]
fn directional_native_psych_orchestration() {
    check_directional_runtime(Runtime::Ruby);
}

#[test]
#[ignore = "requires native Python/LibCST"]
fn python_directional_native_orchestration() {
    check_directional_runtime(Runtime::Python);
}

#[test]
#[ignore = "requires native Python/LibCST"]
fn python_directional_family_placement_preserves_native_trivia() {
    let cases = [
        (
            "# incoming header\nalpha = 1 # incoming tail\n\n# new\nbeta = 2 # added\nomega = 3\n# incoming footer\n",
            "\u{feff}# current header\r\nalpha = 9 # keep\r\n\r\n# omega keep\r\nomega = 30\r\n# current footer",
            "\u{feff}# current header\r\nalpha = 9 # keep\r\n\n# new\nbeta = 2 # added\n\r\n# omega keep\r\nomega = 30\r\n# current footer",
        ),
        (
            "alpha = 1\nbeta = 2 # beta\n",
            "alpha = 9 # alpha\n# footer",
            "alpha = 9 # alpha\nbeta = 2 # beta\n# footer",
        ),
        (
            "first = 1\nsecond = 2\nanchor = 3\n",
            "# header\nanchor = 9",
            "# header\nfirst = 1\nsecond = 2\nanchor = 9",
        ),
        ("alpha = 1\n", "alpha = 9 # no final newline", "alpha = 9 # no final newline"),
        ("é = 1 # Unicode\r\n", "# header\r\n", "# header\r\né = 1 # Unicode\r\n"),
        (
            "alpha = 1\n\n# function\ndef f():\n    return 2\n",
            "alpha = 9\n# footer\n",
            "alpha = 9\n\n# function\ndef f():\n    return 2\n# footer\n",
        ),
    ];
    for (incoming, current, expected) in cases {
        let (parser, snapshot, context) = setup_runtime(Runtime::Python, Behavior::Normal);
        let result = ast_merge::typed_merge2::merge_directional_native_sources(
            "python",
            directional_requests(Runtime::Python, incoming, current),
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            python_merge::declaration_owners,
            python_merge::directional::plan_insertions,
        )
        .unwrap();
        assert_eq!(result.rendered.unwrap().output, expected);
        assert!(result.output_parse.is_some());
        assert_eq!(parser.calls.load(Ordering::SeqCst), 2);
    }
    let (_, snapshot, context) = setup_runtime(Runtime::Python, Behavior::Normal);
    let result = ast_merge::typed_merge2::merge_directional_native_sources(
        "python",
        directional_requests(Runtime::Python, "a = 1\nb = 2\nc = 3\n", "c = 30\na = 10\n"),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        python_merge::declaration_owners,
        python_merge::directional::plan_insertions,
    )
    .unwrap();
    assert!(result.rendered.unwrap_err().contains("reordered"));
    assert!(result.output_parse.is_none());
}

struct FacadeHost {
    parser: Arc<NativeParser>,
    context: ExecutionContext,
}
impl ParserHost for FacadeHost {
    fn descriptor(&self) -> Result<ParserProviderDescriptor, CoreError> {
        Ok(self.parser.descriptor.clone())
    }
    fn probe_batch(&self, request: ProbeBatchRequest) -> Result<ProbeBatchResult, CoreError> {
        Ok(ProbeBatchResult {
            items: request
                .items
                .iter()
                .map(|_| ParserProbeResult { available: true, loadable: true })
                .collect(),
        })
    }
    fn parse_batch(&self, request: ParseBatchRequest) -> Result<ParseBatchResult, CoreError> {
        self.parser
            .parse_batch(request.items, &self.context)
            .map(|items| ParseBatchResult { items })
            .map_err(|error| CoreError { code: error.code, message: error.message })
    }
}
fn operation_limits() -> ParseLimits {
    ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 4096,
        max_nodes: 1000,
        max_diagnostics: 20,
        timeout_millis: None,
    }
}

#[test]
fn public_common_facade_checks_inline_sources_and_controls_before_dispatch() {
    let input: OperationRequest = serde_json::from_value(wire("analyze", &["a: one"])).unwrap();
    let control = create_operation_control();
    control.cancel();
    assert_eq!(
        execute_operation_controlled(input.clone(), operation_limits(), &control).unwrap_err().code,
        "execution.cancelled"
    );
    let mut limits = operation_limits();
    limits.timeout_millis = Some(0);
    assert_eq!(
        execute_operation(input.clone(), limits).unwrap_err().code,
        "execution.deadline_exceeded"
    );
    let mut limits = operation_limits();
    limits.max_input_bytes = 1;
    assert_eq!(execute_operation(input.clone(), limits).unwrap_err().code, "resource.limit");
    let mut invalid = input.clone();
    invalid.sources.get_mut(&SourceRole::Source).unwrap().sha256 = "0".repeat(64);
    assert_eq!(
        execute_operation(invalid, operation_limits()).unwrap_err().code,
        "operation.invalid_request"
    );
    let mut referenced = input;
    let source = referenced.sources.get_mut(&SourceRole::Source).unwrap();
    source.content = None;
    source.reference = Some("file:///must-not-be-opened".into());
    assert_eq!(
        execute_operation(referenced, operation_limits()).unwrap_err().code,
        "source.unresolved_reference"
    );
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn public_common_facade_executes_through_registered_typed_parser_host() {
    let (parser, _, context) = setup(Behavior::Normal);
    register_parser_host(Arc::new(FacadeHost { parser: parser.clone(), context })).unwrap();
    for input in [
        wire("analyze", &["a: one"]),
        wire("diff2", &["a: one", "a: two"]),
        wire("merge3", &["a: one\nb: two", "a: ours\nb: two", "a: one\nb: theirs"]),
    ] {
        let result =
            execute_operation(serde_json::from_value(input).unwrap(), operation_limits()).unwrap();
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.request_id, "native-op-1");
    }
    unregister_parser_host("test.psych".into()).unwrap();
    assert_eq!(parser.calls.load(Ordering::SeqCst), 4);
}

#[test]
#[ignore = "native Python/LibCST common-operation integration gate"]
fn python_analysis_retains_nfkc_identity_native_statement_and_exact_layout() {
    let (provider, registry, context) = setup_runtime(Runtime::Python, Behavior::Normal);
    let input = python_request("analyze", &["\u{feff}# café\r\nK = 'été'\r\nb = 2"]);
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok, "{:?}", result.diagnostics);
    let analysis = result.analysis.as_ref().unwrap();
    assert_eq!(analysis.extra["owners"][0]["logical_identity"], json!(["python", "/K"]));
    assert_eq!(analysis.extra["owners"][0]["node_ids"].as_array().unwrap().len(), 1);
    assert_eq!(
        analysis.extra["layout_gaps"][0]["span"]["range"]["end_byte"],
        "\u{feff}# café\r\n".len()
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(result.output.is_none());
    result.validate_against(&input).unwrap();
    let mut corrupted = result;
    corrupted.analysis.as_mut().unwrap().extra.get_mut("owners").unwrap()[0]["match_keys"] =
        json!(["forged"]);
    assert!(corrupted.validate_against(&input).is_err());
}

#[test]
#[ignore = "native Python/LibCST common-operation integration gate"]
fn python_diff_classifies_edit_delete_add_with_real_statement_spans() {
    let (provider, registry, context) = setup_runtime(Runtime::Python, Behavior::Normal);
    let input = python_request("diff2", &["# café\r\na = 1\r\nb = 2", "# café\r\na = 3\r\nc = 4"]);
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok, "{:?}", result.diagnostics);
    assert_eq!(
        result.changes.iter().map(|c| c.classification.as_str()).collect::<Vec<_>>(),
        ["edited", "deleted", "added"]
    );
    assert_eq!(
        result.changes[0].source_spans[&SourceRole::After].range.start_byte,
        "# café\r\n".len()
    );
    assert!(!result.changes[1].source_spans.contains_key(&SourceRole::After));
    assert!(!result.changes[2].source_spans.contains_key(&SourceRole::Before));
    assert!(result.output.is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[test]
#[ignore = "native Python/LibCST common-operation integration gate"]
fn python_merge_composition_and_whole_source_selection_both_reparse_output() {
    for (sources, expected) in [
        (
            [
                "\u{feff}# café\r\na = 1\r\nb = 2",
                "\u{feff}# café\r\na = 3\r\nb = 2",
                "\u{feff}# café\r\na = 1\r\nb = 4",
            ],
            "\u{feff}# café\r\na = 3\r\nb = 4",
        ),
        (["a = 1", "a = 2", "a = 1"], "a = 2"),
    ] {
        let (provider, registry, context) = setup_runtime(Runtime::Python, Behavior::Normal);
        let result =
            execute_native_operation(&python_request("merge3", &sources), &registry, &context)
                .unwrap();
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), Some(expected));
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(result.extra["output_parse"]["parsed"]["source"]["role"], "output");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }
}

#[test]
#[ignore = "native Python/LibCST common-operation integration gate"]
fn python_conflicts_and_failures_keep_real_decisions_and_source_roles() {
    for (sources, expected) in [
        (["a = 1", "a = 2", "a = 3"], "merge.edit_edit"),
        (["anchor = 0\na = 1", "anchor = 0", "anchor = 0\na = 2"], "merge.delete_edit"),
        (["anchor = 0", "anchor = 0\na = 1", "anchor = 0\na = 2"], "merge.add_add"),
    ] {
        let (_, registry, context) = setup_runtime(Runtime::Python, Behavior::Normal);
        let result =
            execute_native_operation(&python_request("merge3", &sources), &registry, &context)
                .unwrap();
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert_eq!(code(&result), expected);
    }
    for (source, expected) in [
        ("a = (", "parse.rejected"),
        ("import os", "analysis.unsupported"),
        ("K = 1\nK = 2", "analysis.unsupported"),
        ("# ownerless", "analysis.unsupported"),
    ] {
        let (_, registry, context) = setup_runtime(Runtime::Python, Behavior::Normal);
        let result =
            execute_native_operation(&python_request("analyze", &[source]), &registry, &context)
                .unwrap();
        assert!(!result.ok);
        assert_eq!(code(&result), expected);
        let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
        assert_eq!(diagnostic.source_refs[0].role, SourceRole::Source);
    }
    let (_, registry, context) = setup_runtime(Runtime::Python, Behavior::Cancel);
    let result =
        execute_native_operation(&python_request("analyze", &["a = 1"]), &registry, &context)
            .unwrap();
    assert_eq!(code(&result), "execution.cancelled");
    assert!(result.analysis.is_none());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn common_merge_executes_rust_composition_with_real_reparse_and_exact_evidence() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "merge3",
        &[
            "# header\r\na: one\r\nb: two",
            "# header\r\na: ours\r\nb: two",
            "# header\r\na: one\r\nb: theirs",
        ],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("# header\r\na: ours\r\nb: theirs"));
    assert_eq!(result.verification.base_participated, Some(true));
    assert_eq!(result.verification.output_reparsed, Some(true));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert!(result.extra["output_parse"]["parsed"]["ok"].as_bool().unwrap());
    assert!(!result.verification.retained_source_regions.as_ref().unwrap().is_empty());
    assert_eq!(
        result.extra["request_forwarding"]["extra"]["future"],
        json!({"opaque": [null, false, 7]})
    );
    assert_eq!(result.metadata["caller"], "integration");
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn whole_source_selection_is_actually_reparsed_not_reported_as_a_reparse_by_assumption() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut input = wire("merge3", &["a: one\n", "a: two\n", "a: one\n"]);
    input["sources"]["base"]["source_id"] = json!("merge-output");
    input["ok"] = json!(false);
    input["output"] = json!("untrusted request extension");
    let request = request(input);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("a: two\n"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.extra["output_parse"]["parsed"]["source"]["role"], "output");
    assert_eq!(result.verification.output_reparsed, Some(true));
    assert_ne!(result.extra["output_parse"]["parsed"]["source"]["source_id"], "merge-output");
    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized["ok"], true);
    assert_eq!(serialized["output"], "a: two\n");
    assert_eq!(serialized["request_forwarding"]["extra"]["output"], "untrusted request extension");
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn real_conflicts_use_canonical_records_and_executed_decision_catalogs() {
    let (_, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "merge3",
        &["anchor: kept\na: one\n", "anchor: kept\n", "anchor: kept\na: theirs\n"],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(!result.ok);
    assert_eq!(code(&result), "merge.delete_edit");
    assert!(result.output.is_none());
    assert!(result.conflicted_output.is_none());
    let portable_conflict::ConflictRecord::Canonical(conflict) = &result.conflicts[0] else {
        panic!()
    };
    assert_eq!(conflict.classification.base_participated, Some(true));
    assert_eq!(conflict.alternatives[1].state, portable_conflict::AlternativeState::Absent);
    assert!(conflict.alternatives[1].regions.is_empty());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn diff_has_both_roles_and_preserves_exact_changes_and_layout_without_output() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let request = request(wire("diff2", &["# header\na: one\n", "# changed\na: two\nb: three\n"]));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.changes.len(), 2);
    assert!(result.output.is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        result.verification.consumed_source_roles,
        Some(vec![SourceRole::Before, SourceRole::After])
    );
    assert!(
        !result.diff.unwrap().extra["owner_diff"]["before_layout"].as_array().unwrap().is_empty()
    );
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn diff_spans_use_exact_utf8_bytes_and_do_not_invent_absent_revision_ranges() {
    let (_, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "diff2",
        &["# café\r\na: ancien\r\ngone: oui\r\n", "# café\r\na: été\r\nnew: oui\r\n"],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.changes.len(), 3);
    let edited = &result.changes[0];
    assert_eq!(edited.classification, "edited");
    assert_eq!(edited.source_spans.len(), 2);
    let after = &edited.source_spans[&SourceRole::After];
    assert_eq!(after.range.start_byte, "# café\r\n".len());
    assert_eq!((after.start_point.row, after.start_point.column), (1, 0));
    assert_eq!((after.end_point.row, after.end_point.column), (1, "a: été".len()));
    for change in &result.changes {
        for (role, span) in &change.source_spans {
            let key = if *role == SourceRole::Before { "before" } else { "after" };
            assert_eq!(
                serde_json::to_value(&span.range).unwrap(),
                change.role_states[key]["range"]
            );
        }
    }
    assert_eq!(result.changes[1].classification, "deleted");
    assert!(result.changes[1].source_spans.contains_key(&SourceRole::Before));
    assert!(!result.changes[1].source_spans.contains_key(&SourceRole::After));
    assert_eq!(result.changes[2].classification, "added");
    assert!(!result.changes[2].source_spans.contains_key(&SourceRole::Before));
    assert!(result.changes[2].source_spans.contains_key(&SourceRole::After));
    result.validate_against(&request).unwrap();
    let mut corrupt = result;
    corrupt.changes[0].source_spans.get_mut(&SourceRole::After).unwrap().end_point.column -= 1;
    assert!(corrupt.validate_against(&request).is_err());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn unsupported_operations_policies_and_selection_never_invoke_the_parser() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut cases = vec![wire("merge2", &["a: one", "a: two"])];
    for (key, value) in [
        ("comments", json!(true)),
        ("tokens", json!(true)),
        ("ownership", json!(false)),
        ("native_extensions", json!(false)),
        ("analysis_depth", json!("full")),
    ] {
        let mut analysis = wire("analyze", &["a: one"]);
        analysis["policy"][key] = value;
        cases.push(analysis);
    }
    let base = wire("merge3", &["a: one", "a: two", "a: three"]);
    for pointer in [
        "/policy/fallback_policy",
        "/policy/render_policy",
        "/provider_selection/provider_id",
        "/provider_selection/profile_id",
    ] {
        let mut value = base.clone();
        *value.pointer_mut(pointer).unwrap() = json!("unknown");
        cases.push(value);
    }
    let mut constraints = base.clone();
    constraints["parser_selection"]["language_version"] = json!("9.9");
    cases.push(constraints);
    let mut policy = base;
    policy["policy"]["unknown_required_behavior"] = json!(true);
    cases.push(policy);
    for value in cases {
        let request = request(value);
        let result = execute_native_operation(&request, &registry, &context).unwrap();
        assert!(!result.ok);
        assert!(code(&result).contains("unsupported"));
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn native_parse_and_analysis_failures_keep_exact_revision_and_original_parse_evidence() {
    let (_, registry, context) = setup(Behavior::Normal);
    for (text, expected) in [("a: [", "parse.rejected"), ("a: one\na: two", "analysis.unsupported")]
    {
        let request = request(wire("merge3", &["a: one", text, "a: three"]));
        let result = execute_native_operation(&request, &registry, &context).unwrap();
        assert!(!result.ok);
        assert_eq!(code(&result), expected);
        let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
        assert_eq!(diagnostic.source_refs[0].role, SourceRole::Ours);
        assert_eq!(result.extra["input_parses"].as_array().unwrap().len(), 3);
        assert_eq!(result.verification.classification_reached, Some(false));
    }
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn analysis_retains_native_nodes_and_exact_shared_layout_without_rendering() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut value = wire("analyze", &["# café\r\na: one\r\nb: two"]);
    value["policy"] = json!({"analysis_depth": "exact-source-owners", "comments": false,
        "tokens": false, "ownership": true, "native_extensions": true});
    let input = request(value);
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(result.output.is_none());
    assert!(result.diff.is_none());
    assert!(result.changes.is_empty());
    assert!(result.conflicts.is_empty());
    assert_eq!(result.verification.consumed_source_roles, Some(vec![SourceRole::Source]));
    assert_eq!(result.verification.output_reparsed, None);
    let analysis = serde_json::to_value(result.analysis.as_ref().unwrap()).unwrap();
    assert_eq!(analysis["schema"], "structuredmerge.analysis-result/v1");
    assert_eq!(analysis["parse_result_ref"], analysis["parse_result"]["parsed"]["request_id"]);
    assert_eq!(analysis["parse_result"]["parsed"]["source"]["role"], "source");
    let nodes = analysis["parse_result"]["parsed"]["nodes"].as_array().unwrap();
    let owners = analysis["owners"].as_array().unwrap();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0]["logical_identity"], json!(["yaml", "/a"]));
    for owner in owners {
        let ids = owner["node_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(owner["node_id"], ids[0]);
        for id in ids {
            assert!(nodes.iter().any(|node| node["id"] == *id));
        }
    }
    let gaps = analysis["layout_gaps"].as_array().unwrap();
    assert_eq!(gaps.len(), 2); // zero-width suffix is not a reported gap
    assert_eq!(gaps[0]["kind"], "preamble");
    assert_eq!(gaps[0]["span"]["range"]["end_byte"], "# café\r\n".len());
    assert_eq!(gaps[1]["kind"], "interstitial");
    assert!(gaps.iter().all(|gap| gap["fallback_controller_side"].is_null()));
    let source = input.sources().get("source").unwrap();
    for gap in gaps {
        let range: ByteRange = serde_json::from_value(gap["span"]["range"].clone()).unwrap();
        assert_eq!(gap["source_sha256"], source.range_digest(range).unwrap());
    }
    assert_eq!(
        analysis["attachments"][0]["trailing_gap_id"],
        analysis["attachments"][1]["leading_gap_id"]
    );
    assert_eq!(analysis["ownership"][1]["selected_owner_ref"], "/b");
    assert!(analysis["comment_regions"].as_array().unwrap().is_empty());
    assert_eq!(analysis["metadata"]["comment_analysis"], "not-requested");
    result.validate_against(&input).unwrap();
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn common_analysis_validation_rejects_forged_owner_layout_and_parser_evidence() {
    let (_, registry, context) = setup(Behavior::Normal);
    let input = request(wire("analyze", &["# header\na: one\nb: two\n"]));
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok);
    let original = serde_json::to_value(result.analysis.as_ref().unwrap()).unwrap();
    for (pointer, value) in [
        ("/parse_result_ref", json!("different-parse")),
        ("/request_id", json!("different-request")),
        ("/parse_result/schema", json!("unknown/v9")),
        ("/parse_result/parsed/source/sha256", json!("0".repeat(64))),
        ("/parse_result/parsed/source/role", json!("before")),
        ("/parse_result/selection/selected_backend", json!("invented.backend")),
        ("/parse_result/selection/requested/backend_id", json!("invented.backend")),
        ("/owners/0/node_id", json!("missing-node")),
        ("/owners/0/node_ids", json!([])),
        ("/owners/0/logical_identity", json!(["yaml", "/forged"])),
        ("/owners/0/match_keys", json!(["forged"])),
        ("/owners/0/source_sha256", json!("0".repeat(64))),
        ("/owners/0/span/start_point/column", json!(1)),
        ("/layout_gaps/0/source_sha256", json!("0".repeat(64))),
        ("/layout_gaps/1/controller_side", json!("before")),
        ("/layout_gaps/1/fallback_controller_side", json!("before")),
        ("/attachments/0/trailing_gap_id", json!("unknown-gap")),
        ("/ownership/1/selected_owner_ref", json!("/a")),
        ("/ownership/1/confidence", json!("guessed")),
        ("/comment_regions", json!([{"invented": true}])),
        (
            "/extensions",
            json!([{"schema": "example/v1", "namespace": "", "capabilities": [], "payload": {}}]),
        ),
    ] {
        let mut corrupted = original.clone();
        *corrupted.pointer_mut(pointer).unwrap_or_else(|| panic!("missing pointer: {pointer}")) =
            value;
        let mut changed = result.clone();
        changed.analysis = Some(serde_json::from_value(corrupted).unwrap());
        assert!(changed.validate_against(&input).is_err(), "accepted mutation at {pointer}");
    }
    let mut changed = result.clone();
    changed.analysis.as_mut().unwrap().extra.remove("parse_result");
    assert!(changed.validate_against(&input).is_err());
    let mut unsupported = input.request().clone();
    unsupported.parser_selection.language_version = Some("unimplemented-version".into());
    let unsupported = unsupported.validate(4096, |_, _| panic!()).unwrap();
    assert!(result.validate_against(&unsupported).is_err());
    let mut future = original;
    future["future"] = json!({"opaque": [null, false, 7]});
    future["owners"][0]["future"] = json!("retained");
    future["extensions"] = json!([{"schema": "example.analysis/v1", "namespace": "example.analysis",
        "capabilities": [], "payload": {"opaque": "preserved"}}]);
    let mut changed = result;
    changed.analysis = Some(serde_json::from_value(future.clone()).unwrap());
    changed.validate_against(&input).unwrap();
    assert_eq!(serde_json::to_value(changed.analysis.unwrap()).unwrap(), future);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn analysis_failures_preserve_source_role_and_discard_late_results() {
    for (text, expected) in [("a: [", "parse.rejected"), ("a: one\na: two", "analysis.unsupported")]
    {
        let (_, registry, context) = setup(Behavior::Normal);
        let input = request(wire("analyze", &[text]));
        let result = execute_native_operation(&input, &registry, &context).unwrap();
        assert!(!result.ok);
        assert!(result.analysis.is_none());
        assert_eq!(code(&result), expected);
        let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
        assert_eq!(diagnostic.source_refs[0].role, SourceRole::Source);
        assert_eq!(result.extra["input_parses"].as_array().unwrap().len(), 1);
    }
    let (_, registry, context) = setup(Behavior::Cancel);
    let result =
        execute_native_operation(&request(wire("analyze", &["a: one"])), &registry, &context)
            .unwrap();
    assert_eq!(code(&result), "execution.cancelled");
    assert!(!result.ok);
    assert!(result.analysis.is_none());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn missing_explicit_backend_and_input_budget_fail_without_parser_substitution() {
    let (provider, registry, mut context) = setup(Behavior::Normal);
    let mut input = wire("diff2", &["a: one", "a: two"]);
    input["parser_selection"]["backend"] = json!("missing.backend");
    let result = execute_native_operation(&request(input), &registry, &context).unwrap();
    assert!(!result.ok);
    assert!(code(&result).starts_with("selection."));
    context.max_input_bytes = 1;
    let result = execute_native_operation(
        &request(wire("diff2", &["a: one", "a: two"])),
        &registry,
        &context,
    )
    .unwrap();
    assert!(!result.ok);
    assert!(code(&result).contains("limit"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn controls_and_output_failures_do_not_fabricate_success_or_expose_native_exception_text() {
    let request = request(wire("merge3", &["a: one", "a: two", "a: one"]));
    let (_, registry, context) = setup(Behavior::Cancel);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert_eq!(code(&result), "execution.cancelled");
    assert!(!result.ok);
    assert!(result.output.is_none());
    let (provider, registry, mut context) = setup(Behavior::Normal);
    context.deadline = Some(Instant::now());
    assert_eq!(
        code(&execute_native_operation(&request, &registry, &context).unwrap()),
        "execution.deadline_exceeded"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let (_, registry, context) = setup(Behavior::FailOutput);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(!result.ok);
    assert!(result.output.is_none());
    assert_eq!(code(&result), "parser.provider_fault");
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(!encoded.contains("private parser exception"));
}
