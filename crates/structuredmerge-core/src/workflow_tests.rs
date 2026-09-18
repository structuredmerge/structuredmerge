use super::*;
use ast_merge::provider_registry::{MergeParserRequirements, MergeProviderRole};
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tree_haver::service::{ParserProvider, ParserRegistry, ProviderFault};

#[test]
fn compiled_registry_has_executors_without_parser_registration_or_host_mutation() {
    let parsers = crate::parser_registry_inventory().unwrap();
    let snapshot = registry().snapshot().unwrap();
    for mut descriptor in crate::artifact_inventory::compiled_provider_inventory().workflows {
        descriptor.dialects.sort();
        let id = &descriptor.provider_id;
        assert_eq!(snapshot.descriptor(id), Some(&descriptor));
        assert!(matches!(snapshot.provider(id).unwrap().as_ref(), WorkflowExecutor::Kernel));
        assert_eq!(require_host_id(id).unwrap_err().code, "workflow.reserved_provider");
        assert_eq!(
            unregister_workflow_host(id.clone(), snapshot.generation()).unwrap_err().code,
            "workflow.reserved_provider"
        );
    }
    assert_eq!(snapshot.inventory(), registry().snapshot().unwrap().inventory());
    assert_eq!(parsers, crate::parser_registry_inventory().unwrap());
}

fn kernel_request(operation: &str) -> WorkflowBatchRequest {
    let roles: &[&str] = match operation {
        "analyze" => &["source"],
        "diff2" => &["before", "after"],
        "merge2" => &["incoming", "current"],
        _ => &["base", "ours", "theirs"],
    };
    let mut sources = serde_json::Map::new();
    for role in roles {
        let text = "{\"a\":1}";
        let source = source_input(
            (*role).into(),
            serde_json::from_value(json!(role)).unwrap(),
            SourceEncoding::Utf8,
            text.as_bytes().to_vec(),
        )
        .unwrap();
        sources.insert((*role).into(), json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8","byte_length":source.descriptor.byte_length,"sha256":source.descriptor.sha256}));
    }
    let policy = match operation {
        "merge2" => {
            json!({"directional_merge":"template-into-current","render_policy":"source-preserving"})
        }
        "merge3" => json!({"render_policy":"source-preserving"}),
        _ => json!({}),
    };
    let operation: OperationRequest = serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":operation,"operation":operation,"policy":policy,
        "provider_selection":{"provider_id":"kernel.json","family":"json","profile_id":"kernel.json.nested.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"json.cached","preference":[],"required_capabilities":[]}, "sources":sources,"extensions":[],"metadata":{}})).unwrap();
    let parse_options = crate::profiles::operation_parse_options(
        "kernel.json.nested.v1",
        operation.operation.kind(),
    );
    WorkflowBatchRequest {
        items: vec![WorkflowOperation {
            operation,
            parser_language: "json".into(),
            parser_dialect: None,
            parse_options,
        }],
    }
}

#[test]
fn compiled_batch_rejects_mismatched_queries_and_cold_registry() {
    let parsers = ParserRegistry::default();
    let providers = registry().snapshot().unwrap();
    for variant in ["cold", "language", "dialect", "options", "profile", "budget"] {
        let mut request = kernel_request("analyze");
        let mut limits = limits();
        match variant {
            "language" => request.items[0].parser_language = "python".into(),
            "dialect" => request.items[0].parser_dialect = Some("json5".into()),
            "options" => request.items[0].parse_options.tokens = true,
            "profile" => request.items[0].operation.provider_selection.profile_id = None,
            "budget" => limits.parse.max_input_bytes = 0,
            _ => (),
        }
        let control = OperationControl::new();
        let context = limits.parse.clone().controlled_context(&control).unwrap();
        let failure = execute(
            "kernel.json",
            request,
            &limits,
            &control,
            &context,
            &providers,
            &parsers.snapshot().unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            failure.code,
            match variant {
                "cold" => "workflow.no_eligible_provider",
                "budget" => "resource.limit",
                _ => "workflow.invalid_request",
            },
            "{variant}"
        );
    }
}

#[test]
#[ignore = "requires explicitly supplied cached JSON grammar; never downloads"]
fn compiled_batch_executes_all_json_operations_with_kernel_ownership() {
    let parsers = ParserRegistry::default();
    parsers
        .register(Arc::new(
            tree_haver::language_pack_provider::LanguagePackProvider::new_cached_only(
                "json.cached".into(),
                "json".into(),
            )
            .unwrap(),
        ))
        .unwrap();
    let providers = registry().snapshot().unwrap();
    let mut limits = limits();
    limits.max_operations = 4;
    limits.max_response_bytes = 1024 * 1024;
    limits.parse.max_nodes = 1000;
    let mut request = WorkflowBatchRequest { items: vec![] };
    for operation in ["analyze", "diff2", "merge2", "merge3"] {
        request.items.extend(kernel_request(operation).items);
    }
    let control = OperationControl::new();
    let context = limits.parse.clone().controlled_context(&control).unwrap();
    let result = execute(
        "kernel.json",
        request.clone(),
        &limits,
        &control,
        &context,
        &providers,
        &parsers.snapshot().unwrap(),
    )
    .unwrap();
    assert_eq!(result.execution_owner, WorkflowExecutionOwner::Kernel);
    assert!(!result.approved_as_default);
    assert_eq!(result.results.len(), 4);
    assert!(result.results.iter().all(|r| r.ok), "{:?}", result.results);
    assert!(result.selections.iter().all(|s| s.provider_generation == providers.generation()));
    limits.max_response_bytes = 1;
    assert_eq!(
        execute(
            "kernel.json",
            request,
            &limits,
            &control,
            &context,
            &providers,
            &parsers.snapshot().unwrap()
        )
        .unwrap_err()
        .code,
        "resource.limit"
    );
}

fn descriptor() -> MergeProviderDescriptor {
    MergeProviderDescriptor {
        provider_id: "test.host".into(),
        family: "test".into(),
        role: MergeProviderRole::Workflow,
        operations: vec!["analyze".into()],
        dialects: vec![],
        profiles: vec!["test.analysis.v1".into()],
        capabilities: vec![],
        preservation_guarantees: vec![],
        priority: 0,
        parser_requirements: MergeParserRequirements {
            languages: vec!["test".into()],
            ..MergeParserRequirements::default()
        },
        allowed_delegation_targets: vec![],
        runtime: "test-host".into(),
        package: "test".into(),
        package_version: "1".into(),
        metadata: Metadata::new(),
        extensions: vec![],
    }
}
struct Parser {
    descriptor: ParserProviderDescriptor,
    calls: AtomicUsize,
    probes: AtomicUsize,
    fault: AtomicBool,
}
impl ParserProvider for Parser {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        self.probes.fetch_add(1, Ordering::SeqCst);
        Ok(ParserProbeResult { available: true, loadable: true })
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        _: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fault.load(Ordering::SeqCst) {
            return Err(ProviderFault {
                code: "native_fault".into(),
                message: "secret parser exception".into(),
            });
        }
        Ok(requests
            .into_iter()
            .map(|request| {
                let source = SourceDocument::validate(request.source, 10000).unwrap();
                ParseOutput {
                    request_id: request.request_id,
                    source: source.descriptor().clone(),
                    ok: true,
                    root_id: Some("root".into()),
                    nodes: vec![ParseNode {
                        id: "root".into(),
                        kind: "document".into(),
                        native_type: "test_root".into(),
                        role: NodeRole::Structural,
                        named: true,
                        missing: false,
                        has_error: false,
                        span: SourceSpan {
                            range: ByteRange { start_byte: 0, end_byte: source.bytes().len() },
                            start_point: source.point(0).unwrap(),
                            end_point: source.point(source.bytes().len()).unwrap(),
                        },
                        parent_id: None,
                        children: vec![],
                        semantic_roles: vec![],
                        unsupported_features: vec![],
                        extensions: vec![],
                        metadata: Metadata::new(),
                        extra: Metadata::new(),
                    }],
                    comments: vec![],
                    diagnostics: vec![],
                    extensions: vec![],
                    metadata: Metadata::new(),
                    extra: Metadata::new(),
                }
            })
            .collect())
    }
}
#[derive(Clone, Copy)]
enum Behavior {
    Good,
    WrongId,
    WrongProvider,
    WrongParser,
    WrongMode,
    Cardinality,
    Delegation,
    Schema,
    Fault,
    Panic,
    CancelFault,
    Oversized,
    Reorder,
    ShadowedField,
}
struct Host {
    calls: AtomicUsize,
    behavior: Behavior,
    after: Option<Box<dyn Fn() + Send + Sync>>,
}
impl WorkflowHost for Host {
    fn descriptor(&self) -> Result<MergeProviderDescriptor, CoreError> {
        Ok(descriptor())
    }
    fn execute_batch(
        &self,
        request: PreparedWorkflowBatch,
        control: OperationControl,
    ) -> Result<WorkflowBatchResult, CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(callback) = &self.after {
            callback();
        }
        match self.behavior {
            Behavior::Panic => panic!("test workflow panic"),
            Behavior::Fault => return Err(error("execution.cancelled", "secret host source text")),
            Behavior::CancelFault => {
                control.cancel();
                return Err(error("native", "secret host source text"));
            }
            _ => (),
        }
        let mut items = vec![];
        for item in request.items {
            assert_eq!(item.parses.len(), 1);
            assert_eq!(item.parses[0].backend.id, "test.parser");
            assert_eq!(
                item.parses[0].parsed.source.source_id,
                item.operation.sources[&SourceRole::Source].source_id
            );
            let validated = item.operation.clone().validate(10000, |_, _| panic!()).unwrap();
            let mut result = crate::native_operation::empty_result(&validated);
            result.ok = true;
            result.provider.provider_id = Some("test.host".into());
            result.provider.family = Some("test".into());
            result.profile.profile_id = Some("test.analysis.v1".into());
            result.profile.parser = Some(ResultParserSelection {
                requested_backend: item.operation.parser_selection.backend.clone(),
                selected_backend: Some("test.parser".into()),
                selection_mode: Some(
                    if item.operation.parser_selection.backend.is_some() {
                        "explicit"
                    } else {
                        "policy"
                    }
                    .into(),
                ),
                extra: Metadata::new(),
            });
            result.verification.classification_reached = Some(true);
            result.verification.consumed_source_roles = Some(vec![SourceRole::Source]);
            result.analysis = Some(ResultAnalysis {
                schema: "structuredmerge.analysis-result/v1".into(),
                extra: [("parsed_node_count".into(), json!(item.parses[0].parsed.nodes.len()))]
                    .into(),
            });
            match self.behavior {
                Behavior::WrongId => result.request_id = "forged".into(),
                Behavior::WrongProvider => result.provider.provider_id = Some("kernel.fake".into()),
                Behavior::WrongParser => {
                    result.profile.parser.as_mut().unwrap().selected_backend = Some("forged".into())
                }
                Behavior::WrongMode => {
                    result.profile.parser.as_mut().unwrap().selection_mode = Some("forged".into())
                }
                Behavior::Delegation => {
                    result.provider.delegation = Some(vec![result.provider.clone()])
                }
                Behavior::Schema => result.schema = "bad".into(),
                Behavior::Oversized => {
                    result.metadata.insert("large".into(), "x".repeat(10000).into());
                }
                Behavior::ShadowedField => {
                    result.extra.insert("ok".into(), false.into());
                }
                _ => (),
            }
            items.push(result);
        }
        if matches!(self.behavior, Behavior::Cardinality) {
            items.pop();
        }
        if matches!(self.behavior, Behavior::Reorder) {
            items.reverse();
        }
        Ok(WorkflowBatchResult { items })
    }
}
fn request() -> WorkflowBatchRequest {
    let text = "é\r\n";
    let source = source_input(
        "source".into(),
        SourceRole::Source,
        SourceEncoding::Utf8,
        text.as_bytes().to_vec(),
    )
    .unwrap();
    let operation = serde_json::from_value(json!({ "schema": OPERATION_SCHEMA, "request_id": "one", "operation": "analyze", "policy": {},
        "provider_selection": {"provider_id": "test.host", "family": "test", "profile_id": "test.analysis.v1", "required_capabilities": []},
        "parser_selection": {"backend": "test.parser", "preference": [], "required_capabilities": []},
        "sources": {"source": {"source_id": "source", "role": "source", "bytes": source.bytes, "encoding": "utf-8", "byte_length": source.descriptor.byte_length, "sha256": source.descriptor.sha256}},
        "extensions": [], "metadata": {}, "future": {"opaque": true}
    })).unwrap();
    WorkflowBatchRequest {
        items: vec![WorkflowOperation {
            operation,
            parser_language: "test".into(),
            parser_dialect: None,
            parse_options: ParseOptions::default(),
        }],
    }
}
fn limits() -> WorkflowLimits {
    WorkflowLimits {
        max_operations: 3,
        max_request_bytes: 10000,
        max_response_bytes: 10000,
        parse: ParseLimits {
            max_batch_items: 3,
            max_input_bytes: 10000,
            max_nodes: 20,
            max_diagnostics: 20,
            timeout_millis: None,
        },
    }
}
struct Fixture {
    providers: Arc<MergeProviderRegistry<WorkflowExecutor>>,
    parsers: ParserRegistry,
    parser: Arc<Parser>,
    host: Arc<Host>,
}
fn fixture(behavior: Behavior) -> Fixture {
    let providers: Arc<MergeProviderRegistry<WorkflowExecutor>> =
        Arc::new(MergeProviderRegistry::default());
    let host = Arc::new(Host { calls: AtomicUsize::new(0), behavior, after: None });
    providers.register(descriptor(), Arc::new(WorkflowExecutor::Host(host.clone()))).unwrap();
    let parsers = ParserRegistry::default();
    let parser = Arc::new(Parser {
        descriptor: ParserProviderDescriptor {
            id: "test.parser".into(),
            family: "test".into(),
            runtime: "rust".into(),
            package: "test".into(),
            package_version: "1".into(),
            parser: "test".into(),
            parser_version: "1".into(),
            grammar: None,
            grammar_version: None,
            languages: vec!["test".into()],
            dialects: vec![],
            contracts: vec![tree_haver::service::PARSE_RESULT_SCHEMA.into()],
            capabilities: vec![],
            probe_id: "test".into(),
            priority: 0,
            metadata: Metadata::new(),
            extensions: vec![],
        },
        calls: AtomicUsize::new(0),
        probes: AtomicUsize::new(0),
        fault: AtomicBool::new(false),
    });
    parsers.register(parser.clone()).unwrap();
    Fixture { providers, parsers, parser, host }
}
fn run(
    fixture: &Fixture,
    request: WorkflowBatchRequest,
    limits: WorkflowLimits,
    control: &OperationControl,
) -> Result<WorkflowExecution, CoreError> {
    let context = limits.parse.clone().controlled_context(control)?;
    execute(
        "test.host",
        request,
        &limits,
        control,
        &context,
        &fixture.providers.snapshot().unwrap(),
        &fixture.parsers.snapshot().unwrap(),
    )
}

#[test]
fn workflow_receives_source_bound_parses_in_one_batch_and_reports_host_ownership() {
    let fixture = fixture(Behavior::Good);
    let mut batch = request();
    let mut second = batch.items[0].clone();
    second.operation.request_id = "two".into();
    batch.items.push(second);
    let result = run(&fixture, batch, limits(), &OperationControl::new()).unwrap();
    assert_eq!(result.execution_owner, WorkflowExecutionOwner::Host);
    assert!(!result.approved_as_default);
    assert_eq!(result.provider.runtime, "test-host");
    assert_eq!(
        result.results.iter().map(|item| item.request_id.as_str()).collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert_eq!(result.results[0].analysis.as_ref().unwrap().extra["parsed_node_count"], 1);
    assert_eq!(result.results[0].extra["request_forwarding"]["extra"]["future"]["opaque"], true);
    assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.parser.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn workflow_rejects_invalid_batches_and_limits_before_provider_callbacks() {
    for variant in [
        "duplicate",
        "digest",
        "reference",
        "identity",
        "profile",
        "bytes",
        "operations",
        "frame",
        "cancel",
        "deadline",
    ] {
        let fixture = fixture(Behavior::Good);
        let mut request = request();
        let mut limits = limits();
        let control = OperationControl::new();
        match variant {
            "duplicate" => request.items.push(request.items[0].clone()),
            "digest" => {
                request.items[0].operation.sources.get_mut(&SourceRole::Source).unwrap().sha256 =
                    "0".repeat(64)
            }
            "reference" => {
                let source =
                    request.items[0].operation.sources.get_mut(&SourceRole::Source).unwrap();
                source.bytes = None;
                source.reference = Some("file:///never-open".into());
            }
            "identity" => {
                request.items[0].operation.provider_selection.provider_id = Some("other".into())
            }
            "profile" => {
                request.items[0].operation.parser_selection.profile_id = Some("unproven".into())
            }
            "bytes" => limits.parse.max_input_bytes = 0,
            "operations" => limits.max_operations = 0,
            "frame" => limits.max_request_bytes = 0,
            "cancel" => control.cancel(),
            _ => limits.parse.timeout_millis = Some(0),
        }
        assert!(run(&fixture, request, limits, &control).is_err(), "{variant}");
        assert_eq!(fixture.parser.probes.load(Ordering::SeqCst), 0, "{variant}");
        assert_eq!(fixture.parser.calls.load(Ordering::SeqCst), 0, "{variant}");
        assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 0, "{variant}");
    }
}

#[test]
fn workflow_rejects_forged_results_faults_and_late_cancellation_without_retry() {
    for behavior in [
        Behavior::WrongId,
        Behavior::WrongProvider,
        Behavior::WrongParser,
        Behavior::WrongMode,
        Behavior::Cardinality,
        Behavior::Delegation,
        Behavior::Schema,
        Behavior::Fault,
        Behavior::Panic,
        Behavior::CancelFault,
        Behavior::Oversized,
        Behavior::Reorder,
        Behavior::ShadowedField,
    ] {
        let fixture = fixture(behavior);
        let mut request = request();
        let mut second = request.items[0].clone();
        second.operation.request_id = "two".into();
        request.items.push(second);
        let failure = run(&fixture, request, limits(), &OperationControl::new()).unwrap_err();
        let expected = match behavior {
            Behavior::Fault => "workflow.provider_fault",
            Behavior::Panic => "workflow.provider_panic",
            Behavior::CancelFault => "execution.cancelled",
            Behavior::Oversized => "resource.limit",
            _ => "workflow.invalid_result",
        };
        assert_eq!(failure.code, expected);
        assert!(!failure.message.contains("secret"));
        assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn workflow_callback_can_retire_itself_without_invalidating_inflight_selection() {
    let mut fixture = fixture(Behavior::Good);
    let registry = Arc::downgrade(&fixture.providers);
    let host = Arc::new(Host {
        calls: AtomicUsize::new(0),
        behavior: Behavior::Good,
        after: Some(Box::new(move || {
            registry.upgrade().unwrap().unregister("test.host", 2).unwrap();
        })),
    });
    fixture
        .providers
        .replace(descriptor(), Arc::new(WorkflowExecutor::Host(host.clone())), 1)
        .unwrap();
    fixture.host = host;
    let result = run(&fixture, request(), limits(), &OperationControl::new()).unwrap();
    assert_eq!(result.selections[0].provider_generation, 2);
    assert_eq!(fixture.providers.snapshot().unwrap().generation(), 3);
    assert!(fixture.providers.snapshot().unwrap().provider("test.host").is_none());
}

#[test]
fn whole_batch_queries_and_total_source_budget_are_checked_before_probing() {
    for variant in ["malformed_query", "total_bytes", "unknown_selector"] {
        let fixture = fixture(Behavior::Good);
        let mut request = request();
        let mut second = request.items[0].clone();
        second.operation.request_id = "two".into();
        let mut limits = limits();
        match variant {
            "malformed_query" => second.parser_language = "invalid language".into(),
            "unknown_selector" => {
                second.operation.parser_selection.extra.insert("unimplemented".into(), true.into());
            }
            _ => {
                limits.parse.max_input_bytes =
                    request.items[0].operation.sources[&SourceRole::Source].byte_length
            }
        }
        request.items.push(second);
        assert!(run(&fixture, request, limits, &OperationControl::new()).is_err());
        assert_eq!(fixture.parser.probes.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn prepared_parse_payload_has_a_byte_budget_before_host_callback() {
    let fixture = fixture(Behavior::Good);
    let request = request();
    let mut limits = limits();
    limits.max_request_bytes = serde_json::to_vec(&request).unwrap().len();
    let failure = run(&fixture, request, limits, &OperationControl::new()).unwrap_err();
    assert_eq!(failure.code, "resource.limit");
    assert_eq!(fixture.parser.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn parser_failure_keeps_portable_code_without_leaking_native_exception_text() {
    let fixture = fixture(Behavior::Good);
    fixture.parser.fault.store(true, Ordering::SeqCst);
    let failure = run(&fixture, request(), limits(), &OperationControl::new()).unwrap_err();
    assert_eq!(failure.code, "parser.provider_fault");
    assert!(!failure.message.contains("secret"));
    assert_eq!(fixture.host.calls.load(Ordering::SeqCst), 0);
}
