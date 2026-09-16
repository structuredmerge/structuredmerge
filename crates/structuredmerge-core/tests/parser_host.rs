use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use structuredmerge_core::*;

struct Host {
    calls: AtomicUsize,
    descriptions: AtomicUsize,
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
    let host = Arc::new(Host { calls: AtomicUsize::new(0), descriptions: AtomicUsize::new(0) });
    register_parser_host(host.clone()).unwrap();
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
    };
    let results = parse_sources(vec![request.clone()], limits.clone()).unwrap();
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
    assert_eq!(parse_sources(vec![request], limits).unwrap_err().code, "selection.no_parser");
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
}
