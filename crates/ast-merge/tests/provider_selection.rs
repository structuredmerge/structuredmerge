//! Source-free selection evidence, not real parser or merge semantic conformance.
use ast_merge::{provider_registry::*, provider_selection::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tree_haver::{parsed::ParseOutput, service::*};

fn descriptor(id: &str, role: MergeProviderRole, priority: i32) -> MergeProviderDescriptor {
    MergeProviderDescriptor {
        provider_id: id.into(),
        family: "ruby".into(),
        role,
        priority,
        operations: vec!["analyze".into(), "merge3".into()],
        dialects: vec!["ruby".into()],
        profiles: vec!["native.v1".into()],
        capabilities: vec!["ownership".into()],
        preservation_guarantees: vec!["exact_bytes".into()],
        parser_requirements: MergeParserRequirements {
            languages: vec!["ruby".into()],
            contracts: vec![PARSE_RESULT_SCHEMA.into()],
            ..MergeParserRequirements::default()
        },
        allowed_delegation_targets: vec![],
        runtime: "ruby".into(),
        package: "native".into(),
        package_version: "1".into(),
        metadata: Default::default(),
        extensions: vec![],
    }
}
struct Parser {
    descriptor: ParserProviderDescriptor,
    probes: AtomicUsize,
    fault: bool,
    on_probe: Option<Box<dyn Fn() + Send + Sync>>,
}
impl Parser {
    fn new(id: &str) -> Self {
        Self {
            descriptor: ParserProviderDescriptor {
                id: id.into(),
                family: "native".into(),
                runtime: "ruby".into(),
                package: "test".into(),
                package_version: "1".into(),
                parser: "test".into(),
                parser_version: "1".into(),
                grammar: None,
                grammar_version: None,
                languages: vec!["ruby".into()],
                dialects: vec![],
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec!["source_spans".into()],
                probe_id: "test.probe".into(),
                priority: 0,
                metadata: Default::default(),
                extensions: vec![],
            },
            probes: AtomicUsize::new(0),
            fault: false,
            on_probe: None,
        }
    }
}
impl ParserProvider for Parser {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        self.probes.fetch_add(1, Ordering::SeqCst);
        if let Some(callback) = &self.on_probe {
            callback();
        }
        if self.fault {
            return Err(ProviderFault {
                code: "native_probe_fault".into(),
                message: "secret exception text".into(),
            });
        }
        Ok(ParserProbeResult { available: true, loadable: true })
    }
    fn parse_batch(
        &self,
        _: Vec<ParseRequest>,
        _: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        panic!("negotiation must not parse");
    }
}
fn context() -> ExecutionContext {
    ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 4,
        max_input_bytes: 0,
        max_nodes: 0,
        max_diagnostics: 0,
    }
}
fn request() -> MergeSelectionRequest {
    MergeSelectionRequest {
        provider_id: None,
        family: "ruby".into(),
        operation: "merge3".into(),
        dialect: Some("ruby".into()),
        profile: Some("native.v1".into()),
        required_capabilities: vec!["ownership".into()],
        required_preservation: vec!["exact_bytes".into()],
        parser: ParserSelectionRequest {
            language: "ruby".into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: None,
                preference: vec![],
                required_capabilities: vec![],
            },
            options: ParseOptions::default(),
        },
    }
}

#[test]
fn family_selection_ignores_backend_priority_and_is_registration_order_independent() {
    let parsers = ParserRegistry::default();
    parsers.register(Arc::new(Parser::new("prism"))).unwrap();
    let mut reports = vec![];
    for ids in [["z-workflow", "a-workflow", "backend"], ["backend", "a-workflow", "z-workflow"]] {
        let providers = MergeProviderRegistry::default();
        for id in ids {
            let (role, priority) = if id == "backend" {
                (MergeProviderRole::Backend, 100)
            } else {
                (MergeProviderRole::Workflow, 0)
            };
            providers.register(descriptor(id, role, priority), Arc::new(())).unwrap();
        }
        let report = negotiate_merge_provider(
            &request(),
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context(),
        )
        .unwrap();
        assert_eq!(report.selected_provider.as_deref(), Some("a-workflow"));
        assert_eq!(report.candidates.iter().filter(|c| c.selected).count(), 1);
        let backend = report.candidates.iter().find(|c| c.provider_id == "backend").unwrap();
        assert!(backend.rejections.contains(&"not_workflow_provider".into()));
        assert!(backend.parser_report.is_none());
        reports.push(report);
        let mut explicit = request();
        explicit.provider_id = Some("backend".into());
        assert_eq!(
            negotiate_merge_provider(
                &explicit,
                &providers.snapshot().unwrap(),
                &parsers.snapshot().unwrap(),
                &TreeHaverParseService::default(),
                &context()
            )
            .unwrap()
            .selected_provider
            .as_deref(),
            Some("backend")
        );
    }
    assert_eq!(reports[0], reports[1]);
}

#[test]
fn explicit_identity_and_merge_constraints_fail_before_any_parser_probe() {
    let providers = MergeProviderRegistry::default();
    providers
        .register(descriptor("workflow", MergeProviderRole::Workflow, 0), Arc::new(()))
        .unwrap();
    let parsers = ParserRegistry::default();
    let parser = Arc::new(Parser::new("prism"));
    parsers.register(parser.clone()).unwrap();
    for variant in
        ["provider", "family", "operation", "dialect", "profile", "capability", "preservation"]
    {
        let mut input = request();
        match variant {
            "provider" => input.provider_id = Some("prism".into()), // parser ID is not a merge provider ID
            "family" => input.family = "python".into(),
            "operation" => input.operation = "diff2".into(),
            "dialect" => input.dialect = Some("other".into()),
            "profile" => input.profile = Some("other".into()),
            "capability" => input.required_capabilities.push("missing".into()),
            _ => input.required_preservation.push("missing".into()),
        }
        let result = negotiate_merge_provider(
            &input,
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context(),
        )
        .unwrap();
        assert!(result.selected_provider.is_none(), "{variant}");
        assert!(result.candidates[0].parser_report.is_none());
    }
    assert_eq!(parser.probes.load(Ordering::SeqCst), 0);
}

#[test]
fn parser_contracts_are_hard_filters_before_merge_provider_priority() {
    let providers = MergeProviderRegistry::default();
    let mut high = descriptor("high", MergeProviderRole::Workflow, 100);
    high.parser_requirements.contracts.push("unsupported/v2".into());
    providers.register(high, Arc::new(())).unwrap();
    providers.register(descriptor("low", MergeProviderRole::Workflow, 0), Arc::new(())).unwrap();
    let parsers = ParserRegistry::default();
    let parser = Arc::new(Parser::new("prism"));
    parsers.register(parser.clone()).unwrap();
    let result = negotiate_merge_provider(
        &request(),
        &providers.snapshot().unwrap(),
        &parsers.snapshot().unwrap(),
        &TreeHaverParseService::default(),
        &context(),
    )
    .unwrap();
    assert_eq!(result.selected_provider.as_deref(), Some("low"));
    assert_eq!(result.candidates[0].rejections, ["no_eligible_parser"]);
    assert!(
        result.candidates[0].parser_report.as_ref().unwrap().candidates[0]
            .rejections
            .contains(&"missing_contract:unsupported/v2".into())
    );
    assert_eq!(parser.probes.load(Ordering::SeqCst), 1);
    let mut explicit = request();
    explicit.provider_id = Some("high".into());
    assert!(
        negotiate_merge_provider(
            &explicit,
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context()
        )
        .unwrap()
        .selected_provider
        .is_none()
    );
}

#[test]
fn provider_constraints_cannot_weaken_application_constraints_or_explicit_backend() {
    let providers = MergeProviderRegistry::default();
    let mut declaration = descriptor("workflow", MergeProviderRole::Workflow, 0);
    declaration.parser_requirements.allowed_backend_ids = vec!["prism".into()];
    providers.register(declaration, Arc::new(())).unwrap();
    let parsers = ParserRegistry::default();
    let parser = Arc::new(Parser::new("prism"));
    parsers.register(parser.clone()).unwrap();
    let service = TreeHaverParseService::default()
        .with_constraints(ParserConstraints {
            forbidden_backend_families: vec!["native".into()],
            ..ParserConstraints::default()
        })
        .unwrap();
    let result = negotiate_merge_provider(
        &request(),
        &providers.snapshot().unwrap(),
        &parsers.snapshot().unwrap(),
        &service,
        &context(),
    )
    .unwrap();
    assert!(result.selected_provider.is_none());
    assert!(
        result.candidates[0].parser_report.as_ref().unwrap().candidates[0]
            .rejections
            .contains(&"provider_backend_family_forbidden".into())
    );
    let mut explicit = request();
    explicit.parser.selection.backend_id = Some("missing".into());
    assert!(
        negotiate_merge_provider(
            &explicit,
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context()
        )
        .unwrap()
        .selected_provider
        .is_none()
    );
    assert_eq!(parser.probes.load(Ordering::SeqCst), 0);
}

#[test]
fn parser_request_preference_precedes_language_profile_preference() {
    let providers = MergeProviderRegistry::default();
    providers
        .register(descriptor("workflow", MergeProviderRole::Workflow, 0), Arc::new(()))
        .unwrap();
    let parsers = ParserRegistry::default();
    for id in ["a", "z"] {
        parsers.register(Arc::new(Parser::new(id))).unwrap();
    }
    let service =
        TreeHaverParseService::with_profile_preferences([("ruby".into(), vec!["z".into()])].into())
            .unwrap();
    let mut input = request();
    for expected in ["z", "a"] {
        if expected == "a" {
            input.parser.selection.preference = vec!["a".into()];
        }
        let report = negotiate_merge_provider(
            &input,
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &service,
            &context(),
        )
        .unwrap();
        assert_eq!(
            report.candidates[0].parser_report.as_ref().unwrap().selected_backend.as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn snapshots_survive_reentrant_removal_from_both_registries() {
    let providers = Arc::new(MergeProviderRegistry::default());
    providers
        .register(descriptor("workflow", MergeProviderRole::Workflow, 0), Arc::new(()))
        .unwrap();
    let parsers = Arc::new(ParserRegistry::default());
    let mut parser = Parser::new("prism");
    let p = Arc::downgrade(&providers);
    let b = Arc::downgrade(&parsers);
    parser.on_probe = Some(Box::new(move || {
        p.upgrade().unwrap().unregister("workflow", 1).unwrap();
        b.upgrade().unwrap().unregister("prism", 1).unwrap();
    }));
    parsers.register(Arc::new(parser)).unwrap();
    let provider_snapshot = providers.snapshot().unwrap();
    let parser_snapshot = parsers.snapshot().unwrap();
    let report = negotiate_merge_provider(
        &request(),
        &provider_snapshot,
        &parser_snapshot,
        &TreeHaverParseService::default(),
        &context(),
    )
    .unwrap();
    assert_eq!(report.selected_provider.as_deref(), Some("workflow"));
    assert_eq!(report.provider_generation, 1);
    assert_eq!(report.parser_generation, 1);
    assert_eq!(providers.snapshot().unwrap().generation(), 2);
    assert_eq!(parsers.snapshot().unwrap().generation(), 2);
    assert!(provider_snapshot.provider("workflow").is_some());
}

#[test]
fn probe_faults_stay_in_trace_and_cancellation_rejects_late_probe_results() {
    let providers = MergeProviderRegistry::default();
    providers
        .register(descriptor("workflow", MergeProviderRole::Workflow, 0), Arc::new(()))
        .unwrap();
    let parsers = ParserRegistry::default();
    let mut parser = Parser::new("prism");
    parser.fault = true;
    parsers.register(Arc::new(parser)).unwrap();
    let report = negotiate_merge_provider(
        &request(),
        &providers.snapshot().unwrap(),
        &parsers.snapshot().unwrap(),
        &TreeHaverParseService::default(),
        &context(),
    )
    .unwrap();
    assert!(report.selected_provider.is_none());
    assert_eq!(
        report.candidates[0].parser_report.as_ref().unwrap().candidates[0].probe_fault.as_deref(),
        Some("native_probe_fault")
    );
    assert!(!serde_json::to_string(&report).unwrap().contains("secret exception text"));
    let context = context();
    let cancel = context.cancelled.clone();
    let mut replacement = Parser::new("prism");
    replacement.on_probe = Some(Box::new(move || cancel.store(true, Ordering::SeqCst)));
    parsers.replace(Arc::new(replacement), 1).unwrap();
    assert_eq!(
        negotiate_merge_provider(
            &request(),
            &providers.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context
        ),
        Err(ServiceError::Cancelled)
    );
}

#[test]
fn unsupported_parser_profiles_fail_closed_and_invalid_requests_fail_without_candidates() {
    let providers = MergeProviderRegistry::default();
    let mut declaration = descriptor("workflow", MergeProviderRole::Workflow, 0);
    declaration.parser_requirements.profiles = vec!["unproven".into()];
    providers.register(declaration, Arc::new(())).unwrap();
    let parsers = ParserRegistry::default();
    let report = negotiate_merge_provider(
        &request(),
        &providers.snapshot().unwrap(),
        &parsers.snapshot().unwrap(),
        &TreeHaverParseService::default(),
        &context(),
    )
    .unwrap();
    assert!(
        report.candidates[0].rejections.contains(&"unsupported_parser_profile_requirement".into())
    );
    assert!(report.candidates[0].parser_report.is_none());
    let empty = MergeProviderRegistry::<()>::default();
    let mut invalid = request();
    invalid.parser.selection.required_capabilities = vec!["z".into(), "a".into()];
    assert_eq!(
        negotiate_merge_provider(
            &invalid,
            &empty.snapshot().unwrap(),
            &parsers.snapshot().unwrap(),
            &TreeHaverParseService::default(),
            &context()
        ),
        Err(ServiceError::InvalidRequest)
    );
}
