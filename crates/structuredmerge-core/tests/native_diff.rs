use structuredmerge_core::*;

fn request() -> NativeDiffRequest {
    NativeDiffRequest {
        request_id: "diff-request".into(),
        profile_id: "kernel.yaml.native_mapping.v1".into(),
        parses: [SourceRole::Before, SourceRole::After]
            .into_iter()
            .map(|role| ParseRequest {
                schema: service::PARSE_REQUEST_SCHEMA.into(),
                request_id: format!("{role:?}"),
                source: source_input(
                    format!("{role:?}"),
                    role,
                    SourceEncoding::Utf8,
                    b"a: one\n".to_vec(),
                )
                .unwrap(),
                language: "yaml".into(),
                dialect: None,
                selection: ParserSelection {
                    backend_id: Some("missing.diff.test".into()),
                    preference: vec![],
                    required_capabilities: vec![],
                },
                options: ParseOptions { native_extensions: true, ..ParseOptions::default() },
                metadata: Default::default(),
                extra: Default::default(),
            })
            .collect(),
    }
}
fn limits() -> ParseLimits {
    ParseLimits {
        max_batch_items: 2,
        max_input_bytes: 10000,
        max_nodes: 1000,
        max_diagnostics: 20,
        timeout_millis: None,
    }
}

#[test]
fn missing_parser_preserves_identity_and_sources_without_diff() {
    let result = diff_native_owners(request(), limits()).unwrap();
    assert!(!result.ok);
    assert_eq!(result.request_id, "diff-request");
    assert_eq!(
        result.sources.iter().map(|s| s.role).collect::<Vec<_>>(),
        vec![SourceRole::Before, SourceRole::After]
    );
    assert!(result.diff.is_none());
    assert!(result.input_parses.is_empty());
    assert_eq!(result.input_failure.unwrap().code, "selection.no_parser");
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn rejects_unknown_profile_missing_identity_and_incorrect_roles() {
    let mut input = request();
    input.profile_id = "unimplemented".into();
    assert_eq!(diff_native_owners(input, limits()).unwrap_err().code, "unsupported_native_profile");
    let mut input = request();
    input.request_id.clear();
    assert_eq!(diff_native_owners(input, limits()).unwrap_err().code, "request.invalid");
    let mut input = request();
    input.parses[0].source.descriptor.role = SourceRole::Incoming;
    assert_eq!(diff_native_owners(input, limits()).unwrap_err().code, "invalid_diff_inputs");
}

#[test]
fn budgets_and_control_failures_remain_errors_not_partial_diff_results() {
    let control = create_operation_control();
    control.cancel();
    assert_eq!(
        diff_native_owners_controlled(request(), limits(), &control).unwrap_err().code,
        "execution.cancelled"
    );
    let mut budget = limits();
    budget.timeout_millis = Some(0);
    assert_eq!(
        diff_native_owners(request(), budget).unwrap_err().code,
        "execution.deadline_exceeded"
    );
    let mut budget = limits();
    budget.max_input_bytes = 1;
    assert_eq!(diff_native_owners(request(), budget).unwrap_err().code, "resource.limit");
}
