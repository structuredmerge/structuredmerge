use structuredmerge_core::{
    OPERATION_SCHEMA, OperationInputs, OperationKind, ParserSelection, ProviderSelection,
    RequestError, SourceEncoding, SourceErrorCode, SourceRole, source_input,
    validate_operation_inputs,
};

fn request(operation: OperationKind) -> OperationInputs {
    OperationInputs {
        schema: OPERATION_SCHEMA.into(),
        request_id: "request-1".into(),
        operation,
        provider_selection: ProviderSelection {
            provider_id: Some("kernel.yaml".into()),
            family: "yaml".into(),
            required_capabilities: vec![],
        },
        parser_selection: ParserSelection {
            backend_id: Some("psych".into()),
            preference: vec![],
            required_capabilities: vec![],
        },
        sources: operation
            .source_roles()
            .iter()
            .enumerate()
            .map(|(index, &role)| {
                source_input(
                    format!("source-{index}"),
                    role,
                    SourceEncoding::Utf8,
                    b"x\r\n".to_vec(),
                )
                .unwrap()
            })
            .collect(),
    }
}

#[test]
fn all_operations_accept_only_their_semantic_source_roles_in_any_order() {
    for operation in
        [OperationKind::Analyze, OperationKind::Diff2, OperationKind::Merge2, OperationKind::Merge3]
    {
        let mut valid = request(operation);
        valid.sources.reverse();
        assert!(validate_operation_inputs(valid.clone(), 100).is_ok());
        let mut missing = valid.clone();
        missing.sources.pop();
        assert_eq!(
            validate_operation_inputs(missing, 100).unwrap_err(),
            RequestError::InvalidSourceRoles
        );
        let mut extra = valid.clone();
        extra.sources.push(extra.sources[0].clone());
        assert_eq!(
            validate_operation_inputs(extra, 100).unwrap_err(),
            RequestError::InvalidSourceRoles
        );
        valid.sources[0].descriptor.role = SourceRole::Output;
        assert_eq!(
            validate_operation_inputs(valid, 100).unwrap_err(),
            RequestError::InvalidSourceRoles
        );
    }
}

#[test]
fn two_way_cannot_masquerade_as_three_way() {
    let mut input = request(OperationKind::Merge2);
    input.operation = OperationKind::Merge3;
    assert_eq!(
        validate_operation_inputs(input, 100).unwrap_err(),
        RequestError::InvalidSourceRoles
    );
    let mut input = request(OperationKind::Merge3);
    input.sources[1].descriptor.role = SourceRole::Base;
    assert_eq!(
        validate_operation_inputs(input, 100).unwrap_err(),
        RequestError::InvalidSourceRoles
    );
}

#[test]
fn refuses_unknown_schema_and_missing_identity() {
    let mut input = request(OperationKind::Analyze);
    input.schema = "structuredmerge.operation-request/v2".into();
    assert_eq!(validate_operation_inputs(input, 100).unwrap_err(), RequestError::UnsupportedSchema);
    let mut input = request(OperationKind::Analyze);
    input.request_id.clear();
    assert_eq!(validate_operation_inputs(input, 100).unwrap_err(), RequestError::EmptyRequestId);
}

#[test]
fn rejects_source_corruption_before_provider_dispatch() {
    let mut input = request(OperationKind::Merge3);
    input.sources[0].bytes[0] = b'z';
    let error = validate_operation_inputs(input, 100).unwrap_err();
    assert!(
        matches!(error, RequestError::Source(error) if error.code == SourceErrorCode::DescriptorMismatch)
    );
}

#[test]
fn enforces_total_batch_bytes_without_changing_source_data() {
    assert!(matches!(validate_operation_inputs(request(OperationKind::Merge3), 8),
        Err(RequestError::Source(error)) if error.code == SourceErrorCode::LimitExceeded));
    let map = validate_operation_inputs(request(OperationKind::Merge3), 9).unwrap();
    assert_eq!(map.get("source-0").unwrap().descriptor().role, SourceRole::Base);
    assert_eq!(map.get("source-0").unwrap().bytes(), b"x\r\n");
}
