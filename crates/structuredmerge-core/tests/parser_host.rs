use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
use structuredmerge_core::*;

// Registry mutation uses generation checks; isolate independent test scenarios.
static REGISTRY_TEST: Mutex<()> = Mutex::new(());

struct Host {
    calls: AtomicUsize,
    descriptions: AtomicUsize,
}

struct LifecycleHost {
    inner: Host,
    revision: &'static str,
    entered: Option<mpsc::Sender<()>>,
    resume: Option<Mutex<mpsc::Receiver<()>>>,
}

impl ParserHost for LifecycleHost {
    fn descriptor(&self) -> Result<ParserProviderDescriptor, CoreError> {
        let mut descriptor = self.inner.descriptor()?;
        descriptor.id = "core-test.lifecycle".into();
        descriptor.parser_version = self.revision.into();
        Ok(descriptor)
    }

    fn probe_batch(&self, request: ProbeBatchRequest) -> Result<ProbeBatchResult, CoreError> {
        self.inner.probe_batch(request)
    }

    fn parse_batch(&self, request: ParseBatchRequest) -> Result<ParseBatchResult, CoreError> {
        if let Some(entered) = &self.entered {
            entered.send(()).map_err(|error| CoreError::new(error.to_string()))?;
            self.resume
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .map_err(|error| CoreError::new(error.to_string()))?;
        }
        let mut result = self.inner.parse_batch(request)?;
        for output in &mut result.items {
            output.diagnostics[0].message = self.revision.into();
        }
        Ok(result)
    }
}

#[test]
fn removal_and_reregistration_do_not_retarget_or_release_an_inflight_host() {
    let _scenario = REGISTRY_TEST.lock().unwrap();
    exercise_inflight_retirement(false);
    exercise_inflight_retirement(true);
}

fn exercise_inflight_retirement(cancel_old: bool) {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let old = Arc::new(LifecycleHost {
        inner: Host { calls: AtomicUsize::new(0), descriptions: AtomicUsize::new(0) },
        revision: "old",
        entered: Some(entered_tx),
        resume: Some(Mutex::new(resume_rx)),
    });
    let retained = Arc::downgrade(&old);
    register_parser_host(old.clone()).unwrap();
    drop(old);
    let request = ParseRequest {
        schema: service::PARSE_REQUEST_SCHEMA.into(),
        request_id: "lifecycle-request".into(),
        source: source_input(
            "lifecycle-source".into(),
            SourceRole::Current,
            SourceEncoding::Utf8,
            b"x".to_vec(),
        )
        .unwrap(),
        language: "test".into(),
        dialect: None,
        selection: ParserSelection {
            backend_id: Some("core-test.lifecycle".into()),
            preference: vec![],
            required_capabilities: vec![],
        },
        options: ParseOptions::default(),
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let limits = ParseLimits {
        max_batch_items: 1,
        max_input_bytes: 100,
        max_nodes: 100,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let worker_request = request.clone();
    let worker_limits = limits.clone();
    let control = OperationControl::new();
    let worker_control = control.clone();
    let worker = std::thread::spawn(move || {
        parse_sources_controlled(vec![worker_request], worker_limits, &worker_control)
    });
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();

    // Mutation while the callback is blocked must not wait for it. The old
    // operation owns a snapshot; future selection observes removal immediately.
    unregister_parser_provider("core-test.lifecycle".into()).unwrap();
    assert!(retained.upgrade().is_some());
    assert_eq!(
        parse_sources(vec![request.clone()], limits.clone()).unwrap_err().code,
        "selection.no_parser"
    );
    let new = Arc::new(LifecycleHost {
        inner: Host { calls: AtomicUsize::new(0), descriptions: AtomicUsize::new(0) },
        revision: "new",
        entered: None,
        resume: None,
    });
    register_parser_host(new.clone()).unwrap();
    if cancel_old {
        control.cancel();
    }
    resume_tx.send(()).unwrap();
    let old_result = worker.join().unwrap();
    if cancel_old {
        assert_eq!(old_result.unwrap_err().code, "execution.cancelled");
    } else {
        let old_result = old_result.unwrap();
        assert_eq!(old_result[0].backend.parser_version, "old");
        assert_eq!(old_result[0].parsed.diagnostics[0].message, "old");
    }
    assert!(retained.upgrade().is_none(), "completed operation leaked the retired host");
    assert_eq!(new.inner.calls.load(Ordering::SeqCst), 0);

    let new_result = parse_sources(vec![request], limits).unwrap();
    assert_eq!(new_result[0].backend.parser_version, "new");
    assert_eq!(new_result[0].parsed.diagnostics[0].message, "new");
    assert_eq!(new.inner.descriptions.load(Ordering::SeqCst), 1);
    assert_eq!(new.inner.calls.load(Ordering::SeqCst), 1);
    unregister_parser_provider("core-test.lifecycle".into()).unwrap();
}
impl ParserHost for Host {
    fn descriptor(&self) -> Result<ParserProviderDescriptor, CoreError> {
        self.descriptions.fetch_add(1, Ordering::SeqCst);
        Ok(ParserProviderDescriptor {
            id: "core-test".into(),
            family: "native".into(),
            runtime: "test".into(),
            package: "test".into(),
            package_version: "1".into(),
            parser: "test".into(),
            parser_version: "1".into(),
            grammar: None,
            grammar_version: None,
            languages: vec!["test".into(), "yaml".into()],
            dialects: vec![],
            contracts: vec![service::PARSE_RESULT_SCHEMA.into()],
            capabilities: vec![],
            probe_id: "test.probe".into(),
            priority: 0,
            metadata: BTreeMap::new(),
            extensions: vec![],
        })
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ParseBatchResult {
            items: request
                .items
                .into_iter()
                .map(|request| ParseOutput {
                    request_id: request.request_id,
                    source: request.source.descriptor.clone(),
                    ok: false,
                    root_id: None,
                    nodes: vec![],
                    comments: vec![],
                    extensions: vec![],
                    metadata: BTreeMap::new(),
                    extra: BTreeMap::new(),
                    diagnostics: vec![ParseDiagnostic {
                        id: "native.failure".into(),
                        severity: ParseSeverity::Error,
                        category: "parse_error".into(),
                        code: Some("test.syntax".into()),
                        message: "native syntax error".into(),
                        source_role: request.source.descriptor.role,
                        span: None,
                        node_id: None,
                        blocking: true,
                        metadata: BTreeMap::new(),
                        extra: BTreeMap::new(),
                    }],
                })
                .collect(),
        })
    }
}

#[test]
fn facade_calls_typed_host_batches_through_tree_haver_and_keeps_native_failure() {
    let _scenario = REGISTRY_TEST.lock().unwrap();
    let host = Arc::new(Host { calls: AtomicUsize::new(0), descriptions: AtomicUsize::new(0) });
    register_parser_host(host.clone()).unwrap();
    let inventory = parser_registry_inventory().unwrap();
    assert_eq!(inventory.schema, service::PARSER_REGISTRY_INVENTORY_SCHEMA);
    assert_eq!(inventory.providers.len(), 1);
    assert_eq!(inventory.providers[0].id, "core-test");
    assert_eq!(host.descriptions.load(Ordering::SeqCst), 1);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
    let request = ParseRequest {
        schema: service::PARSE_REQUEST_SCHEMA.into(),
        request_id: "request".into(),
        source: source_input(
            "input".into(),
            SourceRole::Current,
            SourceEncoding::Utf8,
            "é\r\n".as_bytes().to_vec(),
        )
        .unwrap(),
        language: "test".into(),
        dialect: None,
        selection: ParserSelection {
            backend_id: Some("core-test".into()),
            preference: vec![],
            required_capabilities: vec![],
        },
        options: ParseOptions::default(),
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let limits = ParseLimits {
        max_batch_items: 10,
        max_input_bytes: 100,
        max_nodes: 100,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let results = parse_sources(vec![request.clone()], limits.clone()).unwrap();
    let observed =
        parser_selection_report(ParserSelectionRequest::from(&request), limits.clone()).unwrap();
    assert_eq!(observed, results[0].selection);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    let control = OperationControl::new();
    assert!(!control.is_cancelled());
    control.clone().cancel();
    assert!(control.is_cancelled());
    assert_eq!(
        parse_sources_controlled(vec![request.clone()], limits.clone(), &control).unwrap_err().code,
        "execution.cancelled"
    );
    let legacy_limits: ParseLimits = serde_json::from_str(
        r#"{"max_batch_items":10,"max_input_bytes":100,"max_nodes":100,"max_diagnostics":100}"#,
    )
    .unwrap();
    assert_eq!(legacy_limits, limits);
    assert_eq!(
        parse_sources(
            vec![request.clone()],
            ParseLimits { timeout_millis: Some(0), ..limits.clone() }
        )
        .unwrap_err()
        .code,
        "execution.deadline_exceeded"
    );
    let limited = ParseLimits { max_input_bytes: 0, ..limits.clone() };
    assert_eq!(parse_sources(vec![request.clone()], limited).unwrap_err().code, "resource.limit");
    assert_eq!(parse_sources(vec![], limits.clone()).unwrap_err().code, "request.invalid");
    assert_eq!(results[0].parsed.source, request.source.descriptor);
    assert!(!results[0].parsed.ok);
    assert_eq!(results[0].parsed.diagnostics[0].code.as_deref(), Some("test.syntax"));
    assert_eq!(results[0].selection.selected_backend.as_deref(), Some("core-test"));
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.descriptions.load(Ordering::SeqCst), 1);
    assert_eq!(
        merge_yaml_mapping(vec![request.clone()], limits.clone()).unwrap_err().code,
        "invalid_merge_inputs"
    );
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    let mapping_requests: Vec<_> = [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
        .into_iter()
        .enumerate()
        .map(|(index, role)| ParseRequest {
            language: "yaml".into(),
            request_id: format!("mapping-{index}"),
            source: source_input(
                format!("mapping-{index}"),
                role,
                SourceEncoding::Utf8,
                b"a: [\n".to_vec(),
            )
            .unwrap(),
            ..request.clone()
        })
        .collect();
    assert_eq!(
        merge_yaml_mapping(
            mapping_requests.clone(),
            ParseLimits { max_input_bytes: 0, ..limits.clone() },
        )
        .unwrap_err()
        .code,
        "resource.limit"
    );
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    let rejected = merge_yaml_mapping(mapping_requests, limits.clone()).unwrap();
    assert_eq!(rejected.outcome, ThreeWayMergeOutcome::Error);
    assert!(rejected.output.is_none());
    assert_eq!(rejected.input_parses.len(), 3);
    assert_eq!(rejected.diagnostics.len(), 3);
    assert!(
        rejected.diagnostics.iter().all(|diagnostic| diagnostic.severity
            == structuredmerge_core::DiagnosticSeverity::Error
            && diagnostic.category == structuredmerge_core::DiagnosticCategory::ParseError)
    );
    assert!(rejected.input_parses.iter().all(|parsed| !parsed.parsed.ok));
    assert!(
        rejected
            .input_parses
            .iter()
            .all(|parsed| parsed.parsed.diagnostics[0].code.as_deref() == Some("test.syntax"))
    );
    assert_eq!(
        rejected.sources.iter().map(|source| source.role).collect::<Vec<_>>(),
        vec![SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
    );
    let parsed = rejected.rejected_parse.unwrap();
    assert_eq!(parsed.parsed.source.role, SourceRole::Base);
    assert_eq!(parsed.parsed.diagnostics[0].code.as_deref(), Some("test.syntax"));
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
    unregister_parser_host("core-test".into()).unwrap();
    let removed = parser_registry_inventory().unwrap();
    assert!(removed.providers.is_empty());
    assert!(removed.generation > inventory.generation);
    assert_eq!(inventory.providers[0].id, "core-test");
    let unavailable =
        parser_selection_report(ParserSelectionRequest::from(&request), limits.clone()).unwrap();
    assert_eq!(unavailable.selected_backend, None);
    assert!(unavailable.candidates.is_empty());
    assert_eq!(parse_sources(vec![request], limits).unwrap_err().code, "selection.no_parser");
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
}
