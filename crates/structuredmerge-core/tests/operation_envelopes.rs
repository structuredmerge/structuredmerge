use serde_json::{Value, json};
use structuredmerge_core::{
    SourceEncoding, SourceRole,
    operation::{OperationRequest, OperationRequestErrorCode as Code, validate_operation_batch},
    source_input,
};

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/diagnostics/slice-1025-versioned-merge-operation-envelopes/contract.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn request(index: usize) -> OperationRequest {
    serde_json::from_value(fixture()["operations"][index]["request"].clone()).unwrap()
}

fn inline(mut request: OperationRequest) -> OperationRequest {
    let fixture = fixture();
    for source in request.sources.values_mut() {
        source.reference = None;
        source.content =
            Some(fixture["source_catalog"][&source.source_id]["content"].as_str().unwrap().into());
    }
    request
}

#[test]
fn all_four_fixture_requests_round_trip_and_resolve_verified_sources_in_semantic_order() {
    let fixture = fixture();
    for pair in fixture["operations"].as_array().unwrap() {
        let request: OperationRequest = serde_json::from_value(pair["request"].clone()).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), pair["request"]);
        let original = request.clone();
        let mut resolved_roles = vec![];
        let validated = request
            .validate(1024, |source, budget| {
                assert!(budget >= source.byte_length);
                assert_eq!(
                    source.reference.as_deref().unwrap(),
                    format!("fixture-source:{}", source.source_id)
                );
                resolved_roles.push(source.role);
                Ok(fixture["source_catalog"][&source.source_id]["content"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec())
            })
            .unwrap();
        assert_eq!(validated.request(), &original);
        assert_eq!(resolved_roles, original.operation.kind().source_roles());
        for source in original.sources.values() {
            let document = validated.sources().get(&source.source_id).unwrap();
            assert_eq!(document.descriptor().role, source.role);
            assert_eq!(document.descriptor().sha256, source.sha256);
            assert_eq!(
                document.bytes(),
                fixture["source_catalog"][&source.source_id]["content"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
            );
        }
    }
}

#[test]
fn compatible_fields_and_namespaced_extensions_survive_normalization() {
    let mut value = fixture()["operations"][3]["request"].clone();
    for key in ["provider_selection", "parser_selection", "policy", "metadata"] {
        value[key]["future"] = json!({"ordered": [2, 1], "null": null});
    }
    value["future"] = json!([false, {"opaque": "retained"}]);
    value["sources"]["base"]["future"] = json!(42);
    value["extensions"] = json!([{
        "schema": "example.native/v1", "namespace": "example.native",
        "capabilities": [], "payload": {"bytes": [0, 255]}, "future": true
    }]);
    let request: OperationRequest = serde_json::from_value(value.clone()).unwrap();
    let fixture = fixture();
    let validated = request
        .validate(1024, |source, _| {
            Ok(fixture["source_catalog"][&source.source_id]["content"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec())
        })
        .unwrap();
    assert_eq!(serde_json::to_value(validated.request()).unwrap(), value);
}

fn rejects_without_resolution(request: OperationRequest, budget: u64, expected: Code) {
    let error = request
        .validate(budget, |_, _| panic!("invalid request reached source resolver"))
        .unwrap_err();
    assert_eq!(error.code, expected, "{error}");
}

#[test]
fn exact_roles_and_identity_are_checked_before_resolution() {
    for index in 0..4 {
        let original = request(index);
        let role = original.operation.kind().source_roles()[0];
        let mut missing = original.clone();
        missing.sources.remove(&role);
        rejects_without_resolution(missing, 1024, Code::InvalidRoles);
        let mut extra = original.clone();
        extra.sources.insert(SourceRole::Output, original.sources[&role].clone());
        rejects_without_resolution(extra, 1024, Code::InvalidRoles);
        let mut mismatch = original.clone();
        mismatch.sources.get_mut(&role).unwrap().role = SourceRole::Output;
        rejects_without_resolution(mismatch, 1024, Code::InvalidRoles);
        let mut version = original.clone();
        version.schema = "structuredmerge.operation-request/v2".into();
        rejects_without_resolution(version, 1024, Code::UnsupportedSchema);
        let mut identity = original;
        identity.request_id.clear();
        rejects_without_resolution(identity, 1024, Code::InvalidIdentity);
    }
    let mut duplicate = request(3);
    duplicate.sources.get_mut(&SourceRole::Theirs).unwrap().source_id =
        duplicate.sources[&SourceRole::Base].source_id.clone();
    rejects_without_resolution(duplicate, 1024, Code::InvalidSource);
}

#[test]
fn selection_layers_remain_independent_and_provider_id_does_not_require_family() {
    let mut selected = inline(request(0));
    selected.provider_selection.family = None;
    selected.parser_selection.profile_id = Some("native-profile/v1".into());
    selected.parser_selection.language_version = Some("3.14".into());
    let normalized = selected.clone().validate(1024, |_, _| panic!()).unwrap();
    assert_eq!(normalized.request(), &selected);
    selected.provider_selection.provider_id = None;
    rejects_without_resolution(selected, 1024, Code::InvalidSelection);
    let mut automatic = inline(request(0));
    automatic.provider_selection.provider_id = None;
    automatic.parser_selection.backend = None;
    automatic.parser_selection.preference = vec!["psych".into(), "tslp".into()];
    assert_eq!(automatic.clone().validate(1024, |_, _| panic!()).unwrap().request(), &automatic);
    let mut missing_layer = fixture()["operations"][0]["request"].clone();
    missing_layer.as_object_mut().unwrap().remove("parser_selection");
    assert!(serde_json::from_value::<OperationRequest>(missing_layer).is_err());
    let mut invalid = request(0);
    invalid.parser_selection.required_capabilities = vec!["z".into(), "a".into()];
    rejects_without_resolution(invalid, 1024, Code::InvalidSelection);
}

#[test]
fn content_is_exclusive_and_hash_length_encoding_and_layout_are_verified() {
    let mut ambiguous = request(0);
    ambiguous.sources.get_mut(&SourceRole::Source).unwrap().content = Some("{}".into());
    rejects_without_resolution(ambiguous, 1024, Code::InvalidSource);
    let mut missing = request(0);
    missing.sources.get_mut(&SourceRole::Source).unwrap().reference = None;
    rejects_without_resolution(missing, 1024, Code::InvalidSource);
    for mutation in 0..5 {
        let mut input = inline(request(0));
        let source = input.sources.get_mut(&SourceRole::Source).unwrap();
        match mutation {
            0 => source.sha256 = "0".repeat(64),
            1 => source.byte_length += 1,
            2 => source.content = Some("{\"a\":2}\n".into()),
            3 => source.bom = Some(true),
            4 => source.final_newline = Some(false),
            _ => unreachable!(),
        }
        let error = input.validate(1024, |_, _| panic!()).unwrap_err();
        assert_eq!(error.code, Code::InvalidSource);
        assert_eq!(error.source_role, Some(SourceRole::Source));
        assert_eq!(error.request_id, "operation:analyze:001");
    }
    let mut encoding = request(0);
    encoding.sources.get_mut(&SourceRole::Source).unwrap().encoding = "utf-16".into();
    rejects_without_resolution(encoding, 1024, Code::InvalidSource);
}

#[test]
fn source_resolution_is_explicit_bounded_and_untrusted() {
    rejects_without_resolution(request(3), 1, Code::ResourceLimit);
    let failure =
        request(0).validate(1024, |_, _| Err("reference not in local store".into())).unwrap_err();
    assert_eq!(failure.code, Code::SourceResolution);
    assert_eq!(failure.source_role, Some(SourceRole::Source));
    let corrupt = request(0).validate(1024, |_, _| Ok(b"{\"a\":2}\n".to_vec())).unwrap_err();
    assert_eq!(corrupt.code, Code::InvalidSource);
    let oversized = request(0)
        .validate(8, |_, limit| {
            assert_eq!(limit, 8);
            Ok(vec![0; 9])
        })
        .unwrap_err();
    assert_eq!(oversized.code, Code::ResourceLimit);
}

#[test]
fn binary_and_multibyte_layout_survive_without_string_normalization() {
    for (encoding, bytes, wire_encoding) in [
        (SourceEncoding::Binary, vec![0, 255, 13, 10], "binary"),
        (SourceEncoding::Utf8, "\u{feff}é\r\n\r".as_bytes().to_vec(), "utf-8"),
    ] {
        let input =
            source_input("bytes".into(), SourceRole::Source, encoding, bytes.clone()).unwrap();
        let mut request = request(0);
        let source = request.sources.get_mut(&SourceRole::Source).unwrap();
        source.source_id = "bytes".into();
        source.reference = None;
        source.bytes = Some(bytes.clone());
        source.encoding = wire_encoding.into();
        source.byte_length = input.descriptor.byte_length;
        source.sha256 = input.descriptor.sha256.clone();
        source.bom = Some(input.descriptor.bom);
        source.line_endings = Some(input.descriptor.line_endings.clone());
        source.final_newline = Some(input.descriptor.final_newline);
        let validated = request.validate(1024, |_, _| panic!()).unwrap();
        let document = validated.sources().get("bytes").unwrap();
        assert_eq!(document.bytes(), bytes);
        assert_eq!(document.descriptor(), &input.descriptor);
    }
}

#[test]
fn policy_defaults_do_not_activate_fallback_or_mutate_the_wire_request() {
    for index in 0..4 {
        let mut value = fixture()["operations"][index]["request"].clone();
        value["policy"].as_object_mut().unwrap().remove("fallback_policy");
        let request: OperationRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(request.operation.fallback_policy(), "none");
        assert_eq!(serde_json::to_value(request).unwrap(), value);
    }
    let mut invalid = fixture()["operations"][2]["request"].clone();
    invalid["policy"]["directional_merge"] = json!(false);
    assert!(serde_json::from_value::<OperationRequest>(invalid).is_err());
}

#[test]
fn batches_preserve_order_isolation_and_have_shared_limits_and_unique_ids() {
    let requests: Vec<_> = (0..4).rev().map(|index| inline(request(index))).collect();
    let bytes = requests
        .iter()
        .flat_map(|request| request.sources.values())
        .map(|source| source.byte_length)
        .sum();
    let results = validate_operation_batch(requests.clone(), 4, bytes, |_, _| panic!()).unwrap();
    assert_eq!(results.len(), 4);
    for (result, request) in results.iter().zip(&requests) {
        assert_eq!(result.request(), request);
        for source in request.sources.values() {
            assert_eq!(
                result.sources().get(&source.source_id).unwrap().descriptor().role,
                source.role
            );
        }
    }
    let insufficient =
        validate_operation_batch(requests.clone(), 4, bytes - 1, |_, _| panic!()).unwrap_err();
    assert_eq!(insufficient.code, Code::ResourceLimit);
    let count = validate_operation_batch(requests.clone(), 3, bytes, |_, _| panic!()).unwrap_err();
    assert_eq!(count.code, Code::ResourceLimit);
    let mut duplicate = requests.clone();
    duplicate[3].request_id = duplicate[0].request_id.clone();
    let error = validate_operation_batch(duplicate, 4, bytes, |_, _| panic!()).unwrap_err();
    assert_eq!(error.code, Code::InvalidIdentity);
    assert!(validate_operation_batch(vec![], 0, 0, |_, _| panic!()).unwrap().is_empty());
    let mut isolation = requests;
    isolation[0].metadata.insert("only-first".into(), json!(true));
    let results = validate_operation_batch(isolation, 4, bytes, |_, _| panic!()).unwrap();
    assert!(!results[1].request().metadata.contains_key("only-first"));
}

#[test]
fn duplicate_wire_role_keys_cannot_silently_overwrite_a_source() {
    let value = fixture()["operations"][0]["request"].clone();
    let source = serde_json::to_string(&value["sources"]["source"]).unwrap();
    let encoded = serde_json::to_string(&value).unwrap();
    let duplicate = encoded.replace(
        &format!("\"sources\":{{\"source\":{source}}}"),
        &format!("\"sources\":{{\"source\":{source},\"source\":{source}}}"),
    );
    assert_ne!(duplicate, encoded);
    let error = serde_json::from_str::<OperationRequest>(&duplicate).unwrap_err();
    assert!(error.to_string().contains("duplicate source role key"));
}
