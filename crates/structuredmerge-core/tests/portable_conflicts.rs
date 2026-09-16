use serde_json::{Value, json};
use std::collections::BTreeSet;
use structuredmerge_core::{
    OperationKind, SourceDocument, SourceEncoding, SourceMap, SourceRole,
    operation::{OperationRequest, ValidatedOperationRequest},
    operation_result::OperationResult,
    portable_conflict::{
        AlternativeState, ConflictContractError as E, ConflictEvidence, ConflictRecord,
        ConflictValidationContext, PortableConflict, ResolutionAuthorization, ResolutionStrategy,
        validate_conflicts,
    },
    source_input,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-1028-stable-diagnostic-conflict-serialization/contract.json")).unwrap()
}
fn conflict(index: usize) -> PortableConflict {
    serde_json::from_value(fixture()["conflict_examples"][index]["conflicts"][0].clone()).unwrap()
}
fn ids(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).into()).collect()
}
fn evidence() -> ConflictEvidence {
    ConflictEvidence {
        decisions: ids(&["decision.match.a", "decision.classify.a", "decision.resolve.a"]),
        render_fragments: ids(&["render.ours.a"]),
        authorizations: vec![ResolutionAuthorization {
            decision_id: "decision.resolve.a".into(),
            conflict_id: "conflict.a.resolved.001".into(),
            strategy: ResolutionStrategy::SelectRole,
            selected_roles: vec![SourceRole::Ours],
            resolver: "policy.explicit-select-ours".into(),
        }],
    }
}
fn sources() -> SourceMap {
    let fixture = fixture();
    SourceMap::validate(
        ["source.base", "source.ours", "source.theirs"]
            .iter()
            .map(|id| {
                let wire = &fixture["source_catalog"][id];
                let input = source_input(
                    (*id).into(),
                    serde_json::from_value(wire["role"].clone()).unwrap(),
                    SourceEncoding::Utf8,
                    wire["content"].as_str().unwrap().as_bytes().to_vec(),
                )
                .unwrap();
                assert_eq!(input.descriptor.sha256, wire["sha256"].as_str().unwrap());
                assert_eq!(input.descriptor.byte_length, wire["byte_length"].as_u64().unwrap());
                input
            })
            .collect(),
        4096,
    )
    .unwrap()
}
fn output() -> SourceDocument {
    SourceDocument::validate(
        source_input(
            "output".into(),
            SourceRole::Output,
            SourceEncoding::Utf8,
            b"{\"a\":2}\n".to_vec(),
        )
        .unwrap(),
        4096,
    )
    .unwrap()
}
fn check(
    conflicts: &[PortableConflict],
    result_ok: bool,
    evidence: &ConflictEvidence,
) -> Result<(), E> {
    validate_conflicts(
        &conflicts.iter().collect::<Vec<_>>(),
        &ConflictValidationContext {
            operation: OperationKind::Merge3,
            result_ok,
            sources: &sources(),
            output: Some(&output()),
            diagnostic_ids: &ids(&[
                "diagnostic.conflict.a.001",
                "diagnostic.conflict.a.resolved.001",
            ]),
            change_ids: &ids(&["change.ours.a", "change.theirs.a"]),
            evidence,
        },
    )
}
fn reject(mutate: impl FnOnce(&mut PortableConflict), expected: E) {
    let mut record = conflict(0);
    mutate(&mut record);
    assert_eq!(check(&[record], false, &evidence()), Err(expected));
}

#[test]
fn fixture_conflicts_round_trip_with_exact_source_and_output_evidence() {
    for index in 0..2 {
        let record = conflict(index);
        assert_eq!(
            serde_json::to_value(&record).unwrap(),
            fixture()["conflict_examples"][index]["conflicts"][0]
        );
        check(&[record], index == 1, &evidence()).unwrap();
    }
}

#[test]
fn base_participation_role_order_and_structured_outcomes_are_required() {
    reject(|record| record.alternatives.remove(0).regions.clear(), E::Roles);
    reject(|record| record.roles.swap(1, 2), E::Roles);
    reject(|record| record.alternatives.swap(1, 2), E::Roles);
    reject(|record| record.classification.base_participated = Some(false), E::BaseParticipation);
    reject(|record| record.operation = OperationKind::Merge2, E::Operation);
    assert_eq!(check(&[conflict(0)], true, &evidence()), Err(E::Outcome));
    assert_eq!(check(&[conflict(0), conflict(0)], false, &evidence()), Err(E::Identity));
}

#[test]
fn exact_localization_requires_verified_regions_not_just_a_boolean() {
    reject(|record| record.alternatives[1].regions.clear(), E::ExactLocalization);
    reject(|record| record.localization.verified = false, E::ExactLocalization);
    reject(|record| record.alternatives[1].regions[0].sha256 = "0".repeat(64), E::Source);
    reject(|record| record.alternatives[1].regions[0].byte_length = 2, E::Source);
    reject(|record| record.alternatives[1].regions[0].range.end_byte = 1000, E::Source);
    reject(|record| record.alternatives[1].source_id = Some("source.theirs".into()), E::Source);
    reject(|record| record.alternatives[1].source_id = None, E::Source);
    let mut record = conflict(1);
    record.localization.output_regions[0].sha256 = "0".repeat(64);
    assert_eq!(check(&[record], true, &evidence()), Err(E::Source));
}

#[test]
fn absent_alternatives_have_no_fabricated_ranges_and_coarse_conflicts_remain_honest() {
    reject(|record| record.alternatives[1].state = AlternativeState::Absent, E::Source);
    let mut record = conflict(0);
    record.alternatives[1].state = AlternativeState::Absent;
    record.alternatives[1].source_id = None;
    record.alternatives[1].regions.clear();
    check(&[record], false, &evidence()).unwrap();
    let mut record = conflict(0);
    record.localization.status =
        structuredmerge_core::portable_conflict::LocalizationStatus::WholeDocument;
    record.localization.verified = false;
    record.subject.whole_document = Some(true);
    record.subject.structural_path = None;
    record.subject.owner_ref = None;
    for alternative in &mut record.alternatives {
        alternative.regions.clear();
    }
    check(&[record], false, &evidence()).unwrap();
}

#[test]
fn resolution_requires_separate_conflict_scoped_authorization() {
    for mutation in 0..5 {
        let mut record = conflict(1);
        match mutation {
            0 => record.resolution.reason = None,
            1 => record.resolution.decision_id = None,
            2 => record.resolution.strategy = ResolutionStrategy::None,
            3 => record.resolution.selected_roles = vec![SourceRole::Theirs],
            4 => record.resolution.resolver = Some("unapproved-resolver".into()),
            _ => unreachable!(),
        }
        assert_eq!(check(&[record], true, &evidence()), Err(E::ResolutionAuthorization));
    }
    let mut evidence = evidence();
    evidence.authorizations.clear();
    assert_eq!(check(&[conflict(1)], true, &evidence), Err(E::ResolutionAuthorization));
    reject(
        |record| record.resolution.decision_id = Some("decision.resolve.a".into()),
        E::ResolutionAuthorization,
    );
}

#[test]
fn references_and_canonical_schema_are_fail_closed() {
    reject(|record| record.schema = "structuredmerge.conflict/v2".into(), E::Schema);
    reject(|record| record.code = "edit-edit".into(), E::Code);
    reject(|record| record.diagnostic_ids = vec!["missing".into()], E::Reference);
    reject(|record| record.alternatives[0].change_ids = vec!["missing".into()], E::Reference);
    assert_eq!(check(&[conflict(0)], false, &ConflictEvidence::default()), Err(E::Reference));
    let mut wire = fixture()["conflict_examples"][0]["conflicts"][0].clone();
    wire.as_object_mut().unwrap().remove("alternatives");
    assert!(serde_json::from_value::<ConflictRecord>(wire).is_err());
    let encoded = serde_json::to_string(&conflict(0)).unwrap();
    let duplicate = encoded.replacen('{', "{\"schema\":\"structuredmerge.conflict/v2\",", 1);
    assert!(serde_json::from_str::<ConflictRecord>(&duplicate).is_err());
}

#[test]
fn unknown_fields_and_extensions_survive_forwarding_at_each_evidence_layer() {
    let mut wire = fixture()["conflict_examples"][0]["conflicts"][0].clone();
    for pointer in [
        "",
        "/subject",
        "/alternatives/0",
        "/alternatives/0/regions/0",
        "/alternatives/0/regions/0/range",
        "/classification",
        "/localization",
        "/resolution",
    ] {
        wire.pointer_mut(pointer).unwrap()["future"] = json!({"opaque": [null, false, 42]});
    }
    wire["extensions"] = json!([{"schema": "example/v1", "namespace": "example", "capabilities": [], "payload": {"native": true}, "future": null}]);
    let record: ConflictRecord = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(record).unwrap(), wire);
}

fn operation_case(index: usize) -> (ValidatedOperationRequest, OperationResult) {
    let old: Value = serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-1025-versioned-merge-operation-envelopes/contract.json")).unwrap();
    let fixture = fixture();
    let case = &fixture["conflict_examples"][index];
    let mut request = old["operations"][3]["request"].clone();
    request["request_id"] = case["request_id"].clone();
    for (role, id) in
        [("base", "source.base"), ("ours", "source.ours"), ("theirs", "source.theirs")]
    {
        request["sources"][role] = fixture["source_catalog"][id].clone();
        request["sources"][role]["encoding"] = json!("utf-8");
    }
    let request: OperationRequest = serde_json::from_value(request).unwrap();
    let request = request.validate(4096, |_, _| panic!()).unwrap();
    let mut result = old["operations"][3]["result"].clone();
    result["request_id"] = case["request_id"].clone();
    result["ok"] = case["result_ok"].clone();
    result["diagnostics"] = case["diagnostics"].clone();
    result["conflicts"] = case["conflicts"].clone();
    result["changes"][0]["id"] = json!("change.ours.a");
    result["changes"][1]["id"] = json!("change.theirs.a");
    if index == 0 {
        result.as_object_mut().unwrap().remove("output");
    } else {
        result["output"] = json!("{\"a\":2}\n");
    }
    (request, serde_json::from_value(result).unwrap())
}

#[test]
fn canonical_conflicts_are_validated_inside_the_existing_result_envelope() {
    for index in 0..2 {
        let (request, result) = operation_case(index);
        assert!(result.validate_against(&request).is_err()); // No implicit decision evidence.
        result.validate_with_conflict_evidence(&request, &evidence()).unwrap();
        let forwarded: OperationResult =
            serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
        forwarded.validate_with_conflict_evidence(&request, &evidence()).unwrap();
    }
    let (request, mut result) = operation_case(1);
    result.output = Some("{\"a\":3}\n".into());
    assert!(result.validate_with_conflict_evidence(&request, &evidence()).is_err());
}

#[test]
fn directional_conflicts_preserve_incoming_current_without_a_synthetic_base() {
    let mut record = conflict(0);
    record.operation = OperationKind::Merge2;
    record.roles = vec![SourceRole::Incoming, SourceRole::Current];
    record.classification.base_participated = None;
    record.alternatives.remove(0);
    record.alternatives[0].role = SourceRole::Incoming;
    record.alternatives[0].source_id = Some("incoming".into());
    record.alternatives[1].role = SourceRole::Current;
    record.alternatives[1].source_id = Some("current".into());
    let sources = SourceMap::validate(
        vec![
            source_input(
                "incoming".into(),
                SourceRole::Incoming,
                SourceEncoding::Utf8,
                b"{\"a\":2}\n".to_vec(),
            )
            .unwrap(),
            source_input(
                "current".into(),
                SourceRole::Current,
                SourceEncoding::Utf8,
                b"{\"a\":3}\n".to_vec(),
            )
            .unwrap(),
        ],
        4096,
    )
    .unwrap();
    let evidence = evidence();
    let context = ConflictValidationContext {
        operation: OperationKind::Merge2,
        result_ok: false,
        sources: &sources,
        output: None,
        diagnostic_ids: &ids(&["diagnostic.conflict.a.001"]),
        change_ids: &ids(&["change.ours.a", "change.theirs.a"]),
        evidence: &evidence,
    };
    validate_conflicts(&[&record], &context).unwrap();
    record.alternatives.swap(0, 1);
    assert_eq!(validate_conflicts(&[&record], &context), Err(E::Roles));
}

#[test]
fn batch_authorization_evidence_is_request_scoped() {
    use structuredmerge_core::operation_result::validate_operation_results_with_evidence;
    let (first_request, first_result) = operation_case(0);
    let (second_request, second_result) = operation_case(1);
    let mut first_evidence = evidence();
    first_evidence.authorizations.clear();
    let mut evidence_by_id = std::collections::BTreeMap::from([
        (first_request.request().request_id.clone(), first_evidence),
        (second_request.request().request_id.clone(), evidence()),
    ]);
    let requests = vec![first_request, second_request];
    let results = vec![second_result, first_result];
    let ordered =
        validate_operation_results_with_evidence(&requests, results.clone(), &evidence_by_id)
            .unwrap();
    assert_eq!(ordered[0].request_id, requests[0].request().request_id);
    evidence_by_id.get_mut(&requests[1].request().request_id).unwrap().authorizations.clear();
    assert!(
        validate_operation_results_with_evidence(&requests, results.clone(), &evidence_by_id)
            .is_err()
    );
    evidence_by_id.insert("unknown-request".into(), evidence());
    assert!(validate_operation_results_with_evidence(&requests, results, &evidence_by_id).is_err());
}
