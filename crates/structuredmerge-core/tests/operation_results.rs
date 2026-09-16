use serde_json::{Value, json};
use structuredmerge_core::{
    operation::{OperationRequest, ValidatedOperationRequest},
    operation_result::{OperationResult, ResultContractError as E, validate_operation_results},
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-1025-versioned-merge-operation-envelopes/contract.json")).unwrap()
}

fn validated(value: Value) -> ValidatedOperationRequest {
    let request: OperationRequest = serde_json::from_value(value).unwrap();
    let catalog = fixture()["source_catalog"].clone();
    request
        .validate(4096, |source, _| {
            Ok(catalog[&source.source_id]["content"].as_str().unwrap().as_bytes().to_vec())
        })
        .unwrap()
}

fn supplemental(index: usize) -> (Value, Value) {
    let fixture = fixture();
    let outcome = &fixture["supplemental_outcomes"][index];
    let mut request = fixture["operations"][3]["request"].clone();
    request["request_id"] = outcome["request_id"].clone();
    for (role, source) in outcome["source_overrides"].as_object().unwrap() {
        request["sources"][role] = source.clone();
    }
    (request, outcome["result"].clone())
}

#[test]
fn all_fixture_outcomes_round_trip_and_validate_against_their_verified_requests() {
    let fixture = fixture();
    let mut cases: Vec<_> = fixture["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (pair["request"].clone(), pair["result"].clone()))
        .collect();
    cases.extend([supplemental(0), supplemental(1)]);
    for (request, wire) in cases {
        let result: OperationResult = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&result).unwrap(), wire);
        result.validate_against(&validated(request)).unwrap();
    }
}

fn reject(index: usize, mutate: impl FnOnce(&mut Value), expected: E) {
    let fixture = fixture();
    let request = validated(fixture["operations"][index]["request"].clone());
    let mut wire = fixture["operations"][index]["result"].clone();
    mutate(&mut wire);
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&request), Err(expected));
}

#[test]
fn rejects_contradictions_named_by_the_shared_contract() {
    reject(
        2,
        |wire| wire["verification"]["directional_roles_preserved"] = json!(false),
        E::DirectionalRoleContractViolation,
    );
    reject(
        3,
        |wire| wire["verification"]["base_participated"] = json!(false),
        E::BaseParticipationContractViolation,
    );
    reject(
        3,
        |wire| wire["conflicted_output"] = json!("conflict rendering"),
        E::ContradictoryOutput,
    );
    reject(
        3,
        |wire| wire["conflicts"] = json!([supplemental(1).1["conflicts"][0]]),
        E::ContradictoryOutcome,
    );
    reject(3, |wire| wire["fallbacks"] = json!([{"kind": "textual"}]), E::UndeclaredFallback);
    for status in ["failed", "unverified"] {
        reject(
            2,
            |wire| wire["verification"]["preservation"][1]["status"] = json!(status),
            E::PreservationContractViolation,
        );
    }
}

#[test]
fn correlation_schema_selection_and_role_claims_are_not_interchangeable() {
    reject(3, |wire| wire["request_id"] = json!("another-request"), E::IdentityMismatch);
    reject(3, |wire| wire["operation"] = json!("merge2"), E::IdentityMismatch);
    reject(
        3,
        |wire| {
            wire["schema"] = json!("https://structuredmerge.org/schemas/provider-result/v2.json")
        },
        E::UnsupportedSchema,
    );
    reject(3, |wire| wire["provider"]["provider_id"] = json!("other"), E::SelectionMismatch);
    reject(
        3,
        |wire| wire["profile"]["parser"]["selected_backend"] = json!("other"),
        E::SelectionMismatch,
    );
    reject(3, |wire| wire["profile"]["profile_id"] = json!("formatter"), E::SelectionMismatch);
    reject(
        3,
        |wire| wire["verification"]["consumed_source_roles"] = json!(["ours", "theirs"]),
        E::InvalidSourceRoles,
    );
    reject(
        3,
        |wire| wire["verification"]["classification_reached"] = json!(false),
        E::ContradictoryOutcome,
    );
}

#[test]
fn analyze_and_diff_never_emit_output_or_another_operations_payload() {
    for index in [0, 1] {
        for field in ["output", "conflicted_output"] {
            reject(index, |wire| wire[field] = json!(""), E::ContradictoryOutput);
        }
    }
    reject(
        0,
        |wire| {
            wire.as_object_mut().unwrap().remove("analysis");
        },
        E::MissingPayload,
    );
    reject(
        1,
        |wire| {
            wire.as_object_mut().unwrap().remove("diff");
        },
        E::MissingPayload,
    );
    reject(
        3,
        |wire| {
            wire.as_object_mut().unwrap().remove("output");
        },
        E::MissingPayload,
    );
    reject(
        3,
        |wire| wire["analysis"] = json!({"schema": "structuredmerge.analysis-result/v1"}),
        E::ContradictoryOutcome,
    );
}

#[test]
fn source_evidence_is_verified_not_just_forwarded() {
    let (request, mut wire) = supplemental(0);
    wire["diagnostics"][0].as_object_mut().unwrap().remove("source_role");
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::InvalidSourceEvidence));
    let (request, mut wire) = supplemental(1);
    wire["provider"] = json!({});
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::SelectionMismatch));
    reject(
        2,
        |wire| wire["verification"]["retained_source_regions"][0]["sha256"] = json!("0".repeat(64)),
        E::InvalidSourceEvidence,
    );
    reject(
        2,
        |wire| {
            wire["verification"]["retained_source_regions"][0]["source_role"] = json!("incoming")
        },
        E::InvalidSourceEvidence,
    );
    reject(
        2,
        |wire| {
            wire["verification"]["retained_source_regions"][0]["range"]["end_byte"] = json!(10000)
        },
        E::InvalidSourceEvidence,
    );
    reject(
        2,
        |wire| {
            wire["verification"]["retained_source_regions"][0]
                .as_object_mut()
                .unwrap()
                .remove("sha256");
        },
        E::InvalidSourceEvidence,
    );
    reject(
        2,
        |wire| wire["verification"]["output_reparsed"] = json!(false),
        E::VerificationContractViolation,
    );
    reject(
        2,
        |wire| wire["verification"]["structural_equivalence"] = json!(false),
        E::VerificationContractViolation,
    );
    let (request, mut wire) = supplemental(0);
    wire["diagnostics"][0]["span"]["start_point"]["column"] = json!(0);
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::InvalidSourceEvidence));
    let (request, mut wire) = supplemental(1);
    wire["conflicts"][0]["source_regions"][0]["source_id"] = json!("source:json:a2");
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::InvalidSourceEvidence));
}

#[test]
fn failures_require_blocking_evidence_or_structured_unresolved_conflicts() {
    let (request, mut wire) = supplemental(0);
    wire["diagnostics"][0]["blocking"] = json!(false);
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::ContradictoryOutcome));
    let (request, mut wire) = supplemental(1);
    wire["diagnostics"] = json!([]);
    let result: OperationResult = serde_json::from_value(wire.clone()).unwrap();
    let request = validated(request);
    result.validate_against(&request).unwrap();
    wire["conflicts"] = json!([]);
    wire["conflicted_output"] = json!("<<<<<<< ours\n=======\n>>>>>>> theirs\n");
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&request), Err(E::ContradictoryOutcome));
}

#[test]
fn record_ids_and_diff_references_are_stable_unique_and_ordered() {
    reject(3, |wire| wire["changes"][1]["id"] = wire["changes"][0]["id"].clone(), E::InvalidRecord);
    reject(1, |wire| wire["diff"]["change_ids"] = json!(["missing-change"]), E::InvalidRecord);
    reject(3, |wire| wire["changes"][0]["role_states"]["incoming"] = json!("2"), E::InvalidRecord);
    let (request, mut wire) = supplemental(1);
    wire["conflicts"][0]["resolution"] = json!("unknown");
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::InvalidRecord));
}

#[test]
fn compatible_unknown_fields_are_not_lost_in_evidence_records() {
    let fixture = fixture();
    let mut wire = fixture["operations"][2]["result"].clone();
    for pointer in [
        "",
        "/provider",
        "/profile",
        "/profile/parser",
        "/changes/0",
        "/verification",
        "/verification/preservation/0",
        "/verification/retained_source_regions/0",
        "/verification/retained_source_regions/0/range",
    ] {
        wire.pointer_mut(pointer).unwrap()["future"] = json!({"opaque": [null, false, 42]});
    }
    wire["extensions"] = json!([{"schema": "example/v1", "namespace": "example", "capabilities": [], "payload": {"native": true}, "future": [1, 2]}]);
    let result: OperationResult = serde_json::from_value(wire.clone()).unwrap();
    result.validate_against(&validated(fixture["operations"][2]["request"].clone())).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), wire);
}

#[test]
fn named_fallback_is_not_mistaken_for_verified_fallback() {
    let fixture = fixture();
    let mut request = fixture["operations"][3]["request"].clone();
    request["policy"]["fallback_policy"] = json!("textual");
    let mut wire = fixture["operations"][3]["result"].clone();
    wire["fallbacks"] = json!([{"kind": "textual"}]);
    let result: OperationResult = serde_json::from_value(wire).unwrap();
    assert_eq!(result.validate_against(&validated(request)), Err(E::UnverifiedFallback));
}

#[test]
fn directional_roles_cannot_be_reversed_and_analysis_cannot_change_schema() {
    reject(
        2,
        |wire| {
            wire["verification"].as_object_mut().unwrap().remove("preservation");
        },
        E::PreservationContractViolation,
    );
    reject(
        3,
        |wire| wire["profile"]["parser"]["selection_mode"] = json!("policy"),
        E::SelectionMismatch,
    );
    reject(
        2,
        |wire| wire["verification"]["consumed_source_roles"] = json!(["current", "incoming"]),
        E::DirectionalRoleContractViolation,
    );
    reject(
        0,
        |wire| wire["analysis"]["schema"] = json!("structuredmerge.analysis-result/v2"),
        E::UnsupportedSchema,
    );
}

#[test]
fn batch_results_are_correlated_without_cross_request_state_or_partial_acceptance() {
    let fixture = fixture();
    let requests: Vec<_> = fixture["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| validated(pair["request"].clone()))
        .collect();
    let mut results: Vec<OperationResult> = fixture["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| serde_json::from_value(pair["result"].clone()).unwrap())
        .collect();
    let expected = results.clone();
    results.reverse();
    assert_eq!(validate_operation_results(&requests, results.clone()).unwrap(), expected);
    results[0].request_id = results[1].request_id.clone();
    assert_eq!(validate_operation_results(&requests, results), Err(E::IdentityMismatch));
    let mut missing = expected.clone();
    missing.pop();
    assert_eq!(validate_operation_results(&requests, missing), Err(E::IdentityMismatch));
    let mut unknown = expected.clone();
    unknown[0].request_id = "unknown-request".into();
    assert_eq!(validate_operation_results(&requests, unknown), Err(E::IdentityMismatch));
    let mut failed = expected;
    failed[3].verification.base_participated = Some(false);
    assert_eq!(
        validate_operation_results(&requests, failed),
        Err(E::BaseParticipationContractViolation)
    );
    assert!(validate_operation_results(&[], vec![]).unwrap().is_empty());
}
