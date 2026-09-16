//! Projection of executed Rust owner-classification evidence, not reconstruction
//! of decisions from legacy messages, generic categories, or marker text.

use crate::{
    ByteRange, DiagnosticSeverity, Metadata, OperationKind, SourceRole,
    operation::ValidatedOperationRequest,
    operation_result::{ResultPoint, ResultRange, ResultSpan},
    portable_conflict::*,
    portable_diagnostic::*,
};
use ast_merge::{
    ConflictAlternativeState, OwnerDecisionKind, SourceRevision, ThreeWayMergeOutcome,
    typed_merge::NativeMergeExecution,
};

#[derive(Clone, Debug, PartialEq)]
pub struct NativeConflictProjection {
    pub conflicts: Vec<PortableConflict>,
    pub diagnostics: Vec<PortableDiagnostic>,
    pub evidence: ConflictEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictProjectionError {
    Operation,
    Source,
    Classification,
    Provider,
}

impl std::fmt::Display for ConflictProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ConflictProjectionError {}

/// `provider_id` must be the actual selected merge provider, supplied by the
/// executor. This function does not negotiate or invent a provider identity.
pub fn project_native_merge_conflicts(
    execution: &NativeMergeExecution,
    request: &ValidatedOperationRequest,
    provider_id: &str,
) -> Result<NativeConflictProjection, ConflictProjectionError> {
    use ConflictProjectionError as E;
    if request.request().operation.kind() != OperationKind::Merge3
        || execution.rendered.result.outcome != ThreeWayMergeOutcome::Conflict
    {
        return Err(E::Operation);
    }
    if provider_id.is_empty()
        || request
            .request()
            .provider_selection
            .provider_id
            .as_deref()
            .is_some_and(|requested| requested != provider_id)
    {
        return Err(E::Provider);
    }
    let roles = [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs];
    if execution.sources.len() != 3 {
        return Err(E::Source);
    }
    for (source, role) in execution.sources.iter().zip(roles) {
        let expected = request.request().sources.get(&role).ok_or(E::Source)?;
        if source != request.sources().get(&expected.source_id).map_err(|_| E::Source)?.descriptor()
        {
            return Err(E::Source);
        }
    }
    let classification = execution.rendered.classification.as_ref().ok_or(E::Classification)?;
    if classification.whole_source_selection.is_some() {
        return Err(E::Classification);
    }
    let source_bytes = |role| {
        request
            .sources()
            .get(&request.request().sources[&role].source_id)
            .map(|source| source.bytes())
            .map_err(|_| E::Source)
    };
    let base = source_bytes(SourceRole::Base)?;
    let ours = source_bytes(SourceRole::Ours)?;
    let theirs = source_bytes(SourceRole::Theirs)?;
    if classification.base_equals_ours != (base == ours)
        || classification.base_equals_theirs != (base == theirs)
        || classification.ours_equals_theirs != (ours == theirs)
    {
        return Err(E::Classification);
    }
    let mut projected = NativeConflictProjection {
        conflicts: vec![],
        diagnostics: vec![],
        evidence: ConflictEvidence::default(),
    };
    let mut seen_conflicts = std::collections::BTreeSet::new();
    for decision in &classification.decisions {
        if decision.id.is_empty() || !projected.evidence.decisions.insert(decision.id.clone()) {
            return Err(E::Classification);
        }
        let (category, code) = match decision.kind {
            OwnerDecisionKind::ConflictEditEdit => (ConflictCategory::Content, "merge.edit_edit"),
            OwnerDecisionKind::ConflictDeleteModify => {
                (ConflictCategory::DeleteModify, "merge.delete_edit")
            }
            OwnerDecisionKind::ConflictAddAdd => (ConflictCategory::AddAdd, "merge.add_add"),
            _ => {
                if decision.conflict_id.is_some() {
                    return Err(E::Classification);
                }
                continue;
            }
        };
        let id = decision.conflict_id.as_ref().ok_or(E::Classification)?;
        if !seen_conflicts.insert(id) {
            return Err(E::Classification);
        }
        let actual = execution
            .rendered
            .result
            .conflicts
            .iter()
            .find(|conflict| &conflict.conflict_id == id)
            .ok_or(E::Classification)?;
        if actual.alternatives != decision.alternatives || actual.path != decision.path {
            return Err(E::Classification);
        }
        if decision.alternatives.iter().map(|a| a.revision).collect::<Vec<_>>()
            != [SourceRevision::Base, SourceRevision::Ours, SourceRevision::Theirs]
        {
            return Err(E::Classification);
        }
        let mut alternatives = vec![];
        let mut source_refs = vec![];
        for (alternative, role) in decision.alternatives.iter().zip(roles) {
            let input = &request.request().sources[&role];
            let document = request.sources().get(&input.source_id).map_err(|_| E::Source)?;
            let present = alternative.state == ConflictAlternativeState::Present;
            if present == alternative.regions.is_empty() {
                return Err(E::Classification);
            }
            let mut regions = vec![];
            for region in &alternative.regions {
                let range = ByteRange { start_byte: region.start_byte, end_byte: region.end_byte };
                let bytes = document.slice(range.clone()).map_err(|_| E::Source)?;
                let sha256 = document.range_digest(range).map_err(|_| E::Source)?;
                let start = document.point(region.start_byte).map_err(|_| E::Source)?;
                let end = document.point(region.end_byte).map_err(|_| E::Source)?;
                let range = ResultRange {
                    start_byte: region.start_byte,
                    end_byte: region.end_byte,
                    extra: Metadata::new(),
                };
                regions.push(ExactConflictRegion {
                    range: range.clone(),
                    byte_length: bytes.len() as u64,
                    sha256,
                    extra: Metadata::new(),
                });
                source_refs.push(DiagnosticSourceRef {
                    source_id: input.source_id.clone(),
                    role,
                    span: Some(ResultSpan {
                        range,
                        start_point: ResultPoint {
                            row: start.row,
                            column: start.column,
                            extra: Metadata::new(),
                        },
                        end_point: ResultPoint {
                            row: end.row,
                            column: end.column,
                            extra: Metadata::new(),
                        },
                        extra: Metadata::new(),
                    }),
                    extra: Metadata::new(),
                });
            }
            alternatives.push(ConflictSourceAlternative {
                role,
                state: if present { AlternativeState::Present } else { AlternativeState::Absent },
                source_id: present.then(|| input.source_id.clone()),
                regions,
                change_ids: vec![],
                extra: Metadata::new(),
            });
        }
        let diagnostic_id = format!("diagnostic.{}", decision.id);
        projected.diagnostics.push(PortableDiagnostic {
            schema: DIAGNOSTIC_SCHEMA.into(),
            id: diagnostic_id.clone(),
            sequence: projected.diagnostics.len() as u64,
            severity: DiagnosticSeverity::Error,
            category: PortableCategory::MergeConflict,
            code: code.into(),
            message: actual.message.clone(),
            blocking: true,
            operation: Some(OperationKind::Merge3),
            request_id: Some(request.request().request_id.clone()),
            source_refs,
            subject_refs: Some(vec![DiagnosticSubjectRef {
                kind: "conflict".into(),
                id: id.clone(),
                extra: Metadata::new(),
            }]),
            cause_ids: vec![],
            related_ids: vec![],
            origin: DiagnosticOrigin {
                layer: DiagnosticLayer::Provider,
                provider_id: Some(provider_id.into()),
                backend_id: None,
                package: None,
                package_version: None,
                native_code: None,
                extra: Metadata::new(),
            },
            data: Metadata::new(),
            extensions: vec![],
            metadata: Metadata::new(),
            extra: Metadata::new(),
        });
        projected.conflicts.push(PortableConflict {
            schema: CONFLICT_SCHEMA.into(), id: id.clone(), operation: OperationKind::Merge3, category, code: code.into(),
            message: Some(actual.message.clone()), subject: ConflictSubject {
                structural_path: Some(decision.path.clone()), owner_ref: Some(decision.owner_id.clone()), node_ref: None,
                archive_entry: None, binary_region: None, whole_document: Some(false), extra: Metadata::new(),
            }, roles: roles.to_vec(), alternatives,
            classification: ConflictClassification { base_participated: Some(true), change_ids: vec![], decision_ids: vec![decision.id.clone()],
                extra: [("owner_decision".into(), serde_json::to_value(decision).map_err(|_| E::Classification)?),
                    ("source_comparisons".into(), serde_json::json!({"base_equals_ours": classification.base_equals_ours,
                        "base_equals_theirs": classification.base_equals_theirs, "ours_equals_theirs": classification.ours_equals_theirs}))].into(),
            }, localization: ConflictLocalization { status: LocalizationStatus::Owner, verified: true, output_regions: vec![], extra: Metadata::new() },
            resolution: ConflictResolution { status: ResolutionStatus::Unresolved, strategy: ResolutionStrategy::None, selected_roles: vec![],
                decision_id: None, resolver: None, reason: None, extra: Metadata::new() },
            diagnostic_ids: vec![diagnostic_id], change_ids: vec![], decision_ids: vec![decision.id.clone()], render_fragment_ids: vec![],
            extensions: vec![], metadata: Metadata::new(), extra: Metadata::new(),
        });
    }
    if projected.conflicts.is_empty()
        || projected.conflicts.len() != execution.rendered.result.conflicts.len()
    {
        return Err(E::Classification);
    }
    Ok(projected)
}
