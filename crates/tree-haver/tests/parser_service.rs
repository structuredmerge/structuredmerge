//! Dispatcher tests, not proof of any real parser or merge capability.
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
use tree_haver::parsed::{ParseNode, ParseOutput};
use tree_haver::service::*;
use tree_haver::source::{SourceDocument, SourceEncoding, SourceRole, source_input};
use tree_haver::{ByteRange, NodeRole, SourceSpan};

#[derive(Clone, Copy)]
enum Behavior {
    Success,
    Fault,
    Panic,
    Duplicate,
    WrongSource,
    Cancel,
}

struct TestParser {
    descriptor: ParserProviderDescriptor,
    available: bool,
    behavior: Behavior,
    probes: AtomicUsize,
    calls: AtomicUsize,
    registry: Option<Arc<ParserRegistry>>,
}

impl TestParser {
    fn new(id: &str, priority: i32) -> Self {
        Self {
            descriptor: ParserProviderDescriptor {
                id: id.into(),
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
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec!["source_spans".into()],
                probe_id: "test.probe".into(),
                priority,
                metadata: BTreeMap::new(),
                extensions: vec![],
            },
            available: true,
            behavior: Behavior::Success,
            probes: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            registry: None,
        }
    }
}

impl ParserProvider for TestParser {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        self.probes.fetch_add(1, Ordering::SeqCst);
        Ok(ParserProbeResult { available: self.available, loadable: self.available })
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(registry) = &self.registry {
            // Acquiring a write lock here proves dispatch is outside registry locks.
            let generation = registry.snapshot().unwrap().generation();
            registry.unregister(&self.descriptor.id, generation).unwrap();
        }
        match self.behavior {
            Behavior::Fault => {
                return Err(ProviderFault {
                    code: "test.failure".into(),
                    message: "native failure".into(),
                });
            }
            Behavior::Panic => panic!("test provider panic"),
            Behavior::Cancel => context.cancelled.store(true, Ordering::Release),
            _ => (),
        }
        let mut outputs: Vec<_> = requests
            .into_iter()
            .map(|request| {
                let source = SourceDocument::validate(request.source, 100).unwrap();
                let end = source.bytes().len();
                ParseOutput {
                    request_id: request.request_id,
                    source: source.descriptor().clone(),
                    ok: true,
                    root_id: Some("root".into()),
                    nodes: vec![ParseNode {
                        id: "root".into(),
                        kind: "document".into(),
                        native_type: "test_document".into(),
                        role: NodeRole::Structural,
                        named: true,
                        missing: false,
                        has_error: false,
                        span: SourceSpan {
                            range: ByteRange { start_byte: 0, end_byte: end },
                            start_point: source.point(0).unwrap(),
                            end_point: source.point(end).unwrap(),
                        },
                        parent_id: None,
                        children: vec![],
                        semantic_roles: vec![],
                        unsupported_features: vec![],
                        extensions: vec![],
                        metadata: BTreeMap::new(),
                        extra: BTreeMap::new(),
                    }],
                    comments: vec![],
                    diagnostics: vec![],
                    extensions: vec![],
                    metadata: BTreeMap::new(),
                    extra: BTreeMap::new(),
                }
            })
            .collect();
        outputs.reverse(); // Service correlates identity, not result position.
        match self.behavior {
            Behavior::Duplicate => outputs[1] = outputs[0].clone(),
            Behavior::WrongSource => outputs[0].source.role = SourceRole::Base,
            _ => (),
        }
        Ok(outputs)
    }
}

fn request(id: &str) -> ParseRequest {
    ParseRequest {
        schema: PARSE_REQUEST_SCHEMA.into(),
        request_id: id.into(),
        source: source_input(
            id.into(),
            SourceRole::Source,
            SourceEncoding::Utf8,
            "é\r\nx".as_bytes().to_vec(),
        )
        .unwrap(),
        language: "test".into(),
        dialect: None,
        selection: ParserSelection {
            backend_id: None,
            preference: vec![],
            required_capabilities: vec!["source_spans".into()],
        },
        options: ParseOptions::default(),
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    }
}

fn context() -> ExecutionContext {
    ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 10,
        max_input_bytes: 100,
        max_nodes: 20,
        max_diagnostics: 20,
    }
}

#[test]
fn stable_selection_and_digest_do_not_depend_on_registration_order() {
    let service = TreeHaverParseService::default();
    let mut digests = vec![];
    for ids in [["beta", "alpha"], ["alpha", "beta"]] {
        let registry = ParserRegistry::default();
        for id in ids {
            registry.register(Arc::new(TestParser::new(id, 0))).unwrap();
        }
        let snapshot = registry.snapshot().unwrap();
        digests.push(snapshot.digest().to_owned());
        let selected = service.parser_for(&request("r"), &snapshot, &context()).unwrap();
        assert_eq!(selected.report.selected_backend.as_deref(), Some("alpha"));
        assert_eq!(selected.report.candidates.len(), 2);
    }
    assert_eq!(digests[0], digests[1]);
}

#[test]
fn explicit_capabilities_preferences_and_availability_are_independent() {
    let registry = ParserRegistry::default();
    let mut unavailable = TestParser::new("unavailable", 100);
    unavailable.available = false;
    registry.register(Arc::new(unavailable)).unwrap();
    registry.register(Arc::new(TestParser::new("priority", 50))).unwrap();
    registry.register(Arc::new(TestParser::new("profile", 0))).unwrap();
    let snapshot = registry.snapshot().unwrap();
    let service = TreeHaverParseService::with_profile_preferences(BTreeMap::from([(
        "test".into(),
        vec!["profile".into()],
    )]))
    .unwrap();
    let mut input = request("r");
    assert_eq!(
        service
            .parser_for(&input, &snapshot, &context())
            .unwrap()
            .report
            .selected_backend
            .as_deref(),
        Some("profile")
    );
    input.selection.preference = vec!["unavailable".into(), "priority".into()];
    assert_eq!(
        service
            .parser_for(&input, &snapshot, &context())
            .unwrap()
            .report
            .selected_backend
            .as_deref(),
        Some("priority")
    );
    for id in ["unavailable", "missing"] {
        input.selection.backend_id = Some(id.into());
        assert!(matches!(
            service.parser_for(&input, &snapshot, &context()),
            Err(ServiceError::Selection(_))
        ));
    }
    input.selection.backend_id = Some("priority".into());
    input.options.comments = true;
    let Err(ServiceError::Selection(report)) = service.parser_for(&input, &snapshot, &context())
    else {
        panic!("must fail closed")
    };
    assert!(
        report
            .candidates
            .iter()
            .find(|candidate| candidate.backend_id == "priority")
            .unwrap()
            .rejections
            .contains(&"missing_capability:comments".into())
    );
}

#[test]
fn one_coarse_callback_and_independent_result_identities() {
    let parser = Arc::new(TestParser::new("native", 0));
    let registry = ParserRegistry::default();
    registry.register(parser.clone()).unwrap();
    let service: &dyn ParseService = &TreeHaverParseService::default();
    let results = service
        .parse_batch(
            vec![request("first"), request("second")],
            &registry.snapshot().unwrap(),
            &context(),
        )
        .unwrap();
    assert_eq!(parser.calls.load(Ordering::SeqCst), 1);
    assert_eq!(results[0].document.output().request_id, "first");
    assert_eq!(results[1].document.output().request_id, "second");
    assert_eq!(results[0].source.bytes(), "é\r\nx".as_bytes());
}

#[test]
fn invalid_inputs_and_limits_fail_before_any_provider_callback() {
    let parser = Arc::new(TestParser::new("native", 0));
    let registry = ParserRegistry::default();
    registry.register(parser.clone()).unwrap();
    let snapshot = registry.snapshot().unwrap();
    let service = TreeHaverParseService::default();
    let mut corrupt = request("bad");
    corrupt.source.bytes[0] = 0;
    assert!(service.parse_batch(vec![request("good"), corrupt], &snapshot, &context()).is_err());
    assert!(
        service.parse_batch(vec![request("same"), request("same")], &snapshot, &context()).is_err()
    );
    for bytes in [request("first").source.bytes, b"different\n".to_vec()] {
        let first = request("first");
        let mut second = request("second");
        second.source = source_input(
            first.source.descriptor.source_id.clone(),
            SourceRole::Source,
            SourceEncoding::Utf8,
            bytes,
        )
        .unwrap();
        assert!(matches!(
            service.parse_batch(vec![first, second], &snapshot, &context()),
            Err(ServiceError::Source(error))
                if error.code == tree_haver::source::SourceErrorCode::DuplicateId
                    && error.source_id == "first"
        ));
    }
    let mut limited = context();
    limited.max_batch_items = 1;
    assert!(matches!(
        service.parse_batch(vec![request("a"), request("b")], &snapshot, &limited),
        Err(ServiceError::LimitExceeded)
    ));
    limited = context();
    limited.max_input_bytes = 1;
    assert!(matches!(
        service.parse_batch(vec![request("a")], &snapshot, &limited),
        Err(ServiceError::Source(_))
    ));
    assert_eq!(parser.probes.load(Ordering::SeqCst), 0);
    assert_eq!(parser.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn no_retry_after_fault_panic_or_invalid_results() {
    for behavior in [Behavior::Fault, Behavior::Panic, Behavior::Duplicate, Behavior::WrongSource] {
        let registry = ParserRegistry::default();
        let mut broken = TestParser::new("selected", 10);
        broken.behavior = behavior;
        registry.register(Arc::new(broken)).unwrap();
        let spare = Arc::new(TestParser::new("spare", 0));
        registry.register(spare.clone()).unwrap();
        let error = TreeHaverParseService::default()
            .parse_batch(
                vec![request("a"), request("b")],
                &registry.snapshot().unwrap(),
                &context(),
            )
            .unwrap_err();
        match behavior {
            Behavior::Fault => assert!(
                matches!(error, ServiceError::Provider { fault, .. } if fault.code == "test.failure")
            ),
            Behavior::Panic => assert!(matches!(error, ServiceError::ProviderPanic { .. })),
            Behavior::Duplicate => assert!(matches!(error, ServiceError::InvalidBatch { .. })),
            Behavior::WrongSource => assert!(matches!(error, ServiceError::InvalidResult { .. })),
            _ => unreachable!(),
        }
        assert_eq!(spare.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn snapshots_hold_strong_leases_and_callbacks_run_without_registry_locks() {
    let registry = Arc::new(ParserRegistry::default());
    let mut parser = TestParser::new("native", 0);
    parser.registry = Some(registry.clone());
    let parser = Arc::new(parser);
    let weak = Arc::downgrade(&parser);
    registry.register(parser.clone()).unwrap();
    let snapshot = registry.snapshot().unwrap();
    drop(parser);
    TreeHaverParseService::default()
        .parse_batch(vec![request("r")], &snapshot, &context())
        .unwrap();
    let later = registry.snapshot().unwrap();
    assert_eq!(later.generation(), 2);
    assert_ne!(snapshot.digest(), later.digest());
    assert!(matches!(
        TreeHaverParseService::default().parser_for(&request("r"), &later, &context()),
        Err(ServiceError::Selection(_))
    ));
    assert!(weak.upgrade().is_some());
    drop(snapshot);
    assert!(weak.upgrade().is_none());
}

#[test]
fn registration_rejects_duplicates_invalid_descriptors_and_stale_removal() {
    let registry = ParserRegistry::default();
    registry.register(Arc::new(TestParser::new("native", 0))).unwrap();
    assert_eq!(
        registry.register(Arc::new(TestParser::new("native", 0))),
        Err(RegistrationError::DuplicateId)
    );
    assert_eq!(registry.unregister("native", 0), Err(RegistrationError::StaleGeneration));
    let mut invalid = TestParser::new("bad", 0);
    invalid.descriptor.languages.clear();
    assert_eq!(registry.register(Arc::new(invalid)), Err(RegistrationError::InvalidDescriptor));
    assert_eq!(registry.snapshot().unwrap().generation(), 1);
}

#[test]
fn cancellation_and_deadline_reject_output_without_abandoning_callback() {
    let registry = ParserRegistry::default();
    let mut parser = TestParser::new("native", 0);
    parser.behavior = Behavior::Cancel;
    let parser = Arc::new(parser);
    registry.register(parser.clone()).unwrap();
    let snapshot = registry.snapshot().unwrap();
    let service = TreeHaverParseService::default();
    let cancelled = context();
    cancelled.cancelled.store(true, Ordering::Release);
    assert!(matches!(
        service.parse_batch(vec![request("a")], &snapshot, &cancelled),
        Err(ServiceError::Cancelled)
    ));
    let mut expired = context();
    expired.deadline = Some(Instant::now());
    assert!(matches!(
        service.parse_batch(vec![request("a")], &snapshot, &expired),
        Err(ServiceError::DeadlineExceeded)
    ));
    assert_eq!(parser.calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        service.parse_batch(vec![request("a")], &snapshot, &context()),
        Err(ServiceError::Cancelled)
    ));
    assert_eq!(parser.calls.load(Ordering::SeqCst), 1);
}
