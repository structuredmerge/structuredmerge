use serde_json::json;
use structuredmerge_core::{
    OperationKind, SourceEncoding, SourceRole,
    ast_merge::{
        self, SourcePreservingOwner, SourcePreservingOwnerDocument,
        typed_merge::NativeMergeExecution,
    },
    native_conflict_projection::*,
    operation::{OperationRequest, ValidatedOperationRequest},
    portable_conflict::{ConflictCategory, ConflictValidationContext, validate_conflicts},
    portable_diagnostic::validate_diagnostics,
    source_input,
};

fn document(source: &str) -> SourcePreservingOwnerDocument {
    SourcePreservingOwnerDocument {
        source: source.into(),
        owners: if source.is_empty() {
            vec![]
        } else {
            vec![SourcePreservingOwner {
                id: "value".into(),
                path: "/value".into(),
                fingerprint: source.into(),
                start_byte: 0,
                end_byte: source.len(),
                start_line: 1,
                end_line: 1,
            }]
        },
    }
}

fn execute(texts: [&str; 3]) -> (ValidatedOperationRequest, NativeMergeExecution) {
    let mut sources = serde_json::Map::new();
    let mut descriptors = vec![];
    for ((role, name), text) in
        [(SourceRole::Base, "base"), (SourceRole::Ours, "ours"), (SourceRole::Theirs, "theirs")]
            .into_iter()
            .zip(texts)
    {
        let input = source_input(name.into(), role, SourceEncoding::Utf8, text.as_bytes().to_vec())
            .unwrap();
        sources.insert(name.into(), json!({"source_id": name, "role": name, "content": text,
            "byte_length": input.descriptor.byte_length, "sha256": input.descriptor.sha256, "encoding": "utf-8"}));
        descriptors.push(input.descriptor);
    }
    let request: OperationRequest = serde_json::from_value(json!({
        "schema": "structuredmerge.operation-request/v1", "request_id": "execution-1", "operation": "merge3",
        "provider_selection": {"provider_id": "kernel.owners", "family": "test", "required_capabilities": []},
        "parser_selection": {"preference": [], "required_capabilities": []}, "sources": sources,
        "policy": {"render_policy": "source-preserving"}, "extensions": [], "metadata": {}
    })).unwrap();
    let rendered = ast_merge::merge_source_preserving_owners_with_evidence(
        document(texts[0]),
        document(texts[1]),
        document(texts[2]),
        |source| Ok(document(source)),
    );
    // Exercise the actual shared Rust classifier with explicit owner facts.
    // Native parser execution is covered separately by the Psych integration gate.
    (
        request.validate(4096, |_, _| panic!()).unwrap(),
        NativeMergeExecution {
            rendered,
            sources: descriptors,
            output_source: None,
            input_parses: vec![],
            output_parse: None,
            verification_error: None,
        },
    )
}

#[test]
fn real_owner_classifier_projects_distinct_portable_conflicts_and_verified_bytes() {
    for (texts, category, code) in [
        (["one", "two", "three"], ConflictCategory::Content, "merge.edit_edit"),
        (["one", "two", ""], ConflictCategory::DeleteModify, "merge.delete_edit"),
        (["one", "", "three"], ConflictCategory::DeleteModify, "merge.delete_edit"),
        (["", "two", "three"], ConflictCategory::AddAdd, "merge.add_add"),
    ] {
        let (request, execution) = execute(texts);
        let projected =
            project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap();
        assert_eq!(projected.conflicts.len(), 1);
        assert_eq!(projected.conflicts[0].category, category);
        assert_eq!(projected.conflicts[0].code, code);
        assert_eq!(
            projected.evidence.decisions.iter().collect::<Vec<_>>(),
            vec!["decision.owner.0"]
        );
        assert_eq!(projected.conflicts[0].classification.base_participated, Some(true));
        assert!(projected.conflicts[0].localization.output_regions.is_empty());
        let result: structuredmerge_core::operation_result::OperationResult = serde_json::from_value(json!({
            "schema": structuredmerge_core::operation_result::OPERATION_RESULT_SCHEMA,
            "request_id": request.request().request_id, "operation": "merge3", "ok": false,
            "provider": {"provider_id": "kernel.owners", "family": "test"}, "profile": {},
            "diagnostics": projected.diagnostics, "conflicts": projected.conflicts, "changes": [], "fallbacks": [],
            "render_report": {}, "verification": {"base_participated": true, "classification_reached": true,
                "consumed_source_roles": ["base", "ours", "theirs"], "preservation": []},
            "extensions": [], "metadata": {}
        })).unwrap();
        result.validate_with_conflict_evidence(&request, &projected.evidence).unwrap();
        validate_conflicts(
            &projected.conflicts.iter().collect::<Vec<_>>(),
            &ConflictValidationContext {
                operation: OperationKind::Merge3,
                result_ok: false,
                sources: request.sources(),
                output: None,
                diagnostic_ids: &projected
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.id.clone())
                    .collect(),
                change_ids: &Default::default(),
                evidence: &projected.evidence,
            },
        )
        .unwrap();
        validate_diagnostics(
            &projected.diagnostics.iter().collect::<Vec<_>>(),
            &request.request().request_id,
            OperationKind::Merge3,
            request.sources(),
            |subject| {
                subject.kind == "conflict"
                    && projected.conflicts.iter().any(|conflict| conflict.id == subject.id)
            },
        )
        .unwrap();
        let (request2, execution2) = execute(texts);
        assert_eq!(
            projected,
            project_native_merge_conflicts(&execution2, &request2, "kernel.owners").unwrap()
        );
    }
}

#[test]
fn projection_uses_executed_decisions_not_legacy_messages_or_generic_categories() {
    let (request, mut execution) = execute(["one", "", "three"]);
    execution.rendered.result.conflicts[0].message = "different human wording".into();
    execution.rendered.result.conflicts[0].category = "generic historical category".into();
    let projected = project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap();
    assert_eq!(projected.conflicts[0].code, "merge.delete_edit");
    execution.rendered.classification = None;
    assert_eq!(
        project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap_err(),
        ConflictProjectionError::Classification
    );
}

#[test]
fn missing_decisions_source_mismatch_and_wrong_provider_fail_closed() {
    let (request, mut execution) = execute(["one", "two", "three"]);
    assert_eq!(
        project_native_merge_conflicts(&execution, &request, "other").unwrap_err(),
        ConflictProjectionError::Provider
    );
    execution.sources[1].sha256 = "0".repeat(64);
    assert_eq!(
        project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap_err(),
        ConflictProjectionError::Source
    );
    let (request, mut execution) = execute(["one", "two", "three"]);
    execution.rendered.classification.as_mut().unwrap().decisions.clear();
    assert_eq!(
        project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap_err(),
        ConflictProjectionError::Classification
    );
    let (request, execution) = execute(["one", "two", "one"]);
    assert_eq!(
        project_native_merge_conflicts(&execution, &request, "kernel.owners").unwrap_err(),
        ConflictProjectionError::Operation
    );
}

#[test]
fn whole_source_shortcuts_record_base_comparisons_and_preclassification_errors_do_not() {
    let (_, execution) = execute(["base", "same", "same"]);
    let evidence = execution.rendered.classification.unwrap();
    assert!(!evidence.base_equals_ours);
    assert!(!evidence.base_equals_theirs);
    assert!(evidence.ours_equals_theirs);
    assert_eq!(evidence.whole_source_selection, Some(ast_merge::SourceRevision::Ours));
    let mut invalid = document("one");
    invalid.owners[0].end_byte = 100;
    let evidence = ast_merge::merge_source_preserving_owners_with_evidence(
        invalid,
        document("two"),
        document("three"),
        |_| panic!(),
    );
    assert!(evidence.classification.is_none());
}
