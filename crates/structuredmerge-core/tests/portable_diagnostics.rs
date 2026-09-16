use serde_json::{Value, json};
use structuredmerge_core::{
    OperationKind, SourceEncoding, SourceMap, SourceRole,
    operation::{OperationRequest, ValidatedOperationRequest},
    operation_result::{OperationResult, ResultDiagnostic},
    portable_diagnostic::{
        DiagnosticContractError as E, DiagnosticOrigin, DiagnosticRecord, PortableDiagnostic,
        migrate_diagnostic, validate_diagnostics,
    },
    source_input,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-1028-stable-diagnostic-conflict-serialization/contract.json")).unwrap()
}

fn source_map() -> SourceMap {
    let sources = fixture()["source_catalog"]
        .as_object()
        .unwrap()
        .values()
        .map(|source| {
            let input = source_input(
                source["source_id"].as_str().unwrap().into(),
                serde_json::from_value(source["role"].clone()).unwrap(),
                SourceEncoding::Utf8,
                source["content"].as_str().unwrap().as_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(input.descriptor.sha256, source["sha256"].as_str().unwrap());
            assert_eq!(input.descriptor.byte_length, source["byte_length"].as_u64().unwrap());
            input
        })
        .collect();
    SourceMap::validate(sources, 4096).unwrap()
}

fn diagnostics() -> Vec<PortableDiagnostic> {
    serde_json::from_value(fixture()["diagnostic_examples"][0]["diagnostics"].clone()).unwrap()
}

fn check(diagnostics: &[PortableDiagnostic]) -> Result<(), E> {
    validate_diagnostics(
        &diagnostics.iter().collect::<Vec<_>>(),
        "request.merge3.parse-failure.001",
        OperationKind::Merge3,
        &source_map(),
        |subject| subject.kind == "operation" && subject.id == "merge3.classification",
    )
}

#[test]
fn all_canonical_fixture_diagnostics_round_trip_with_native_origin_and_exact_sources() {
    let fixture = fixture();
    let examples = fixture["diagnostic_examples"]
        .as_array()
        .unwrap()
        .iter()
        .chain(fixture["conflict_examples"].as_array().unwrap());
    for example in examples {
        let records: Vec<PortableDiagnostic> =
            serde_json::from_value(example["diagnostics"].clone()).unwrap();
        assert_eq!(serde_json::to_value(&records).unwrap(), example["diagnostics"]);
        validate_diagnostics(
            &records.iter().collect::<Vec<_>>(),
            example["request_id"].as_str().unwrap(),
            OperationKind::Merge3,
            &source_map(),
            |subject| match subject.kind.as_str() {
                "operation" => subject.id == "merge3.classification",
                "structural_path" => subject.id == "json-pointer:/a",
                "conflict" => example["conflicts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|conflict| conflict["id"] == subject.id),
                _ => false,
            },
        )
        .unwrap();
    }
    assert_eq!(diagnostics()[0].origin.native_code.as_deref(), Some("ERROR"));
    assert_eq!(diagnostics()[0].code, "parse.unexpected_eof");
}

#[test]
fn sequences_identity_and_causal_graph_fail_closed() {
    let mut records = diagnostics();
    records.reverse();
    for (index, record) in records.iter_mut().enumerate() {
        record.sequence = index as u64;
        record.cause_ids.clear();
    }
    assert_eq!(check(&records), Err(E::SemanticOrder));
    let mut records = diagnostics();
    records[1].sequence = 2;
    assert_eq!(check(&records), Err(E::Sequence));
    let mut records = diagnostics();
    records[1].id = records[0].id.clone();
    assert_eq!(check(&records), Err(E::Identity));
    let mut records = diagnostics();
    records[0].cause_ids = vec![records[1].id.clone()];
    assert_eq!(check(&records), Err(E::Causality));
    let mut records = diagnostics();
    records[0].cause_ids = vec![records[0].id.clone()];
    assert_eq!(check(&records), Err(E::Causality));
    let mut records = diagnostics();
    records[1].cause_ids = vec!["unknown".into()];
    assert_eq!(check(&records), Err(E::Causality));
    let mut records = diagnostics();
    records[0].related_ids = vec![records[1].id.clone()];
    check(&records).unwrap();
    records[0].related_ids.push("other-result:diagnostic".into());
    assert_eq!(check(&records), Err(E::RelatedReference));
}

#[test]
fn canonical_schema_and_machine_keys_never_come_from_native_codes_or_messages() {
    let wire =
        serde_json::to_string(&fixture()["diagnostic_examples"][0]["diagnostics"][0]).unwrap();
    let duplicate = wire.replacen('{', "{\"schema\":\"structuredmerge.diagnostic/v2\",", 1);
    assert!(serde_json::from_str::<DiagnosticRecord>(&duplicate).is_err());
    for code in ["ERROR", "unexpected-eof", "parse.Bad", "parse..eof", "parse"] {
        let mut records = diagnostics();
        records[0].code = code.into();
        assert_eq!(check(&records), Err(E::Code));
    }
    let mut records = diagnostics();
    records[0].schema = "structuredmerge.diagnostic/v2".into();
    assert_eq!(check(&records), Err(E::Schema));
    let mut records = diagnostics();
    records[0].message = "Totally different message: this is not a merge conflict".into();
    records[0].origin.native_code = Some("implementation-specific".into());
    check(&records).unwrap();
    let mut wire = fixture()["diagnostic_examples"][0]["diagnostics"][0].clone();
    wire["category"] = json!("parse-error");
    assert!(serde_json::from_value::<DiagnosticRecord>(wire).is_err());
    let mut wire = fixture()["diagnostic_examples"][0]["diagnostics"][0].clone();
    wire.as_object_mut().unwrap().remove("sequence");
    // It still has every required migration field; schema prevents downgrade.
    assert!(serde_json::from_value::<DiagnosticRecord>(wire).is_err());
}

#[test]
fn source_identity_role_range_points_and_subjects_must_resolve() {
    for mutation in 0..5 {
        let mut records = diagnostics();
        let reference = &mut records[0].source_refs[0];
        match mutation {
            0 => reference.role = SourceRole::Theirs,
            1 => reference.source_id = "unknown".into(),
            2 => reference.span.as_mut().unwrap().range.end_byte = 100,
            3 => reference.span.as_mut().unwrap().start_point.column = 0,
            4 => records[0].source_refs.clear(),
            _ => unreachable!(),
        }
        assert_eq!(check(&records), Err(E::SourceReference));
    }
    let mut records = diagnostics();
    records[0].subject_refs.as_mut().unwrap()[0].id = "unknown-subject".into();
    assert_eq!(check(&records), Err(E::SubjectReference));
    let mut records = diagnostics();
    records[0].request_id = Some("another-request".into());
    assert_eq!(check(&records), Err(E::RequestIdentity));
}

#[test]
fn unknown_fields_and_extensions_survive_canonical_forwarding() {
    let mut wire = fixture()["diagnostic_examples"][0]["diagnostics"][0].clone();
    for pointer in [
        "",
        "/origin",
        "/source_refs/0",
        "/source_refs/0/span",
        "/source_refs/0/span/range",
        "/source_refs/0/span/start_point",
        "/subject_refs/0",
    ] {
        wire.pointer_mut(pointer).unwrap()["future"] = json!({"array": [null, 42, false]});
    }
    wire["extensions"] = json!([{"schema": "native/v1", "namespace": "example", "capabilities": [], "payload": {"opaque": "data"}, "future": null}]);
    let record: DiagnosticRecord = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(record).unwrap(), wire);
}

fn legacy_case(index: usize) -> (ValidatedOperationRequest, OperationResult) {
    let fixture: Value = serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-1025-versioned-merge-operation-envelopes/contract.json")).unwrap();
    let outcome = &fixture["supplemental_outcomes"][index];
    let mut request = fixture["operations"][3]["request"].clone();
    request["request_id"] = outcome["request_id"].clone();
    for (role, source) in outcome["source_overrides"].as_object().unwrap() {
        request["sources"][role] = source.clone();
    }
    let request: OperationRequest = serde_json::from_value(request).unwrap();
    let validated = request
        .validate(4096, |source, _| {
            Ok(fixture["source_catalog"][&source.source_id]["content"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec())
        })
        .unwrap();
    (validated, serde_json::from_value(outcome["result"].clone()).unwrap())
}

fn origin() -> DiagnosticOrigin {
    serde_json::from_value(fixture()["diagnostic_examples"][0]["diagnostics"][0]["origin"].clone())
        .unwrap()
}

#[test]
fn explicit_migration_preserves_evidence_and_is_checked_by_the_same_result_envelope() {
    let (request, mut result) = legacy_case(0);
    let DiagnosticRecord::Migration(legacy) = result.diagnostics[0].clone() else { panic!() };
    let original = serde_json::to_value(&legacy).unwrap();
    let migrated = migrate_diagnostic(legacy, 0, &request, origin()).unwrap();
    assert_eq!(migrated.code, "parse.unexpected_eof");
    assert_eq!(migrated.origin.native_code.as_deref(), Some("ERROR"));
    assert_eq!(migrated.source_refs[0].source_id, "source:merge3:malformed-ours");
    assert_eq!(migrated.extensions[0].payload, original);
    result.diagnostics = vec![DiagnosticRecord::Canonical(migrated.clone())];
    result.validate_against(&request).unwrap();
    let wire = serde_json::to_value(&result).unwrap();
    assert_eq!(wire["diagnostics"][0]["schema"], "structuredmerge.diagnostic/v1");
    let forwarded: OperationResult = serde_json::from_value(wire).unwrap();
    forwarded.validate_against(&request).unwrap();
    let mut missing_conflict = forwarded.clone();
    let DiagnosticRecord::Canonical(diagnostic) = &mut missing_conflict.diagnostics[0] else {
        panic!()
    };
    diagnostic.category =
        structuredmerge_core::portable_diagnostic::PortableCategory::MergeConflict;
    diagnostic.code = "merge.edit_edit".into();
    assert!(missing_conflict.validate_against(&request).is_err());
    let DiagnosticRecord::Canonical(mut bad) = forwarded.diagnostics[0].clone() else { panic!() };
    bad.sequence = 1;
    result.diagnostics = vec![DiagnosticRecord::Canonical(bad)];
    assert!(result.validate_against(&request).is_err());
    let mut unsupported: ResultDiagnostic = serde_json::from_value(original).unwrap();
    unsupported.code = "ERROR".into();
    assert_eq!(
        migrate_diagnostic(unsupported, 0, &request, origin()).unwrap_err(),
        E::UnmappedMigration
    );
}

#[test]
fn conflict_diagnostic_migration_does_not_invent_alternatives_or_render_markers() {
    let (request, mut result) = legacy_case(1);
    let original_conflicts = result.conflicts.clone();
    let DiagnosticRecord::Migration(legacy) = result.diagnostics[0].clone() else { panic!() };
    let origin: DiagnosticOrigin = serde_json::from_value(
        fixture()["conflict_examples"][0]["diagnostics"][0]["origin"].clone(),
    )
    .unwrap();
    let canonical = migrate_diagnostic(legacy, 0, &request, origin).unwrap();
    assert_eq!(canonical.code, "merge.edit_edit");
    assert_eq!(canonical.source_refs.len(), 1);
    assert_eq!(canonical.source_refs[0].role, SourceRole::Ours);
    result.diagnostics = vec![DiagnosticRecord::Canonical(canonical)];
    result.validate_against(&request).unwrap();
    assert_eq!(result.conflicts, original_conflicts);
    assert!(result.conflicted_output.is_none());
}
