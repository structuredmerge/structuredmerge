//! Slice 1025 result contract and request-correlated evidence checks.
//! Accepts migration records or canonical Slice 1028 diagnostics and conflicts.
//! Validation cannot prove that a provider actually ran a semantic algorithm.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    ByteRange, Metadata, NativeExtension, OperationKind, SourceRole,
    operation::ValidatedOperationRequest,
    portable_conflict::{
        ConflictEvidence, ConflictRecord, ConflictValidationContext, validate_conflicts,
    },
    portable_diagnostic::{DiagnosticRecord, PortableCategory, validate_diagnostics},
};

pub const OPERATION_RESULT_SCHEMA: &str =
    "https://structuredmerge.org/schemas/provider-result/v1.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultProvider {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<Vec<ResultProvider>>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultParserSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_mode: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parser: Option<ResultParserSelection>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultRange {
    pub start_byte: usize,
    pub end_byte: usize,
    #[serde(flatten)]
    pub extra: Metadata,
}

impl ResultRange {
    fn bytes(&self) -> ByteRange {
        ByteRange { start_byte: self.start_byte, end_byte: self.end_byte }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultPoint {
    pub row: usize,
    pub column: usize,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultSpan {
    pub range: ResultRange,
    pub start_point: ResultPoint,
    pub end_point: ResultPoint,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultSourceRegion {
    pub source_id: String,
    pub source_role: SourceRole,
    pub range: ResultRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultDiagnostic {
    pub id: String,
    pub severity: String,
    pub category: String,
    pub code: String,
    pub message: String,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_role: Option<SourceRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<ResultSpan>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultChange {
    pub id: String,
    pub classification: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub role_states: Metadata,
    pub source_spans: BTreeMap<SourceRole, ResultSpan>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultConflict {
    pub id: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub roles: Vec<SourceRole>,
    pub source_regions: Vec<ResultSourceRegion>,
    pub localized: bool,
    pub resolution: String,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PreservationProperty {
    pub property: String,
    pub required: bool,
    pub status: String,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultVerification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed_source_roles: Option<Vec<SourceRole>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directional_roles_preserved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_participated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_reached: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_reparsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structural_equivalence: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preservation: Option<Vec<PreservationProperty>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_source_regions: Option<Vec<ResultSourceRegion>>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultDiff {
    pub change_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

/// Analysis is an embedded Slice 1024 contract, not an unversioned payload.
/// Its full owner/tree evidence is preserved for the analysis validator.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultAnalysis {
    pub schema: String,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationResult {
    pub schema: String,
    pub request_id: String,
    pub operation: OperationKind,
    pub ok: bool,
    pub provider: ResultProvider,
    pub profile: ResultProfile,
    pub diagnostics: Vec<DiagnosticRecord>,
    pub changes: Vec<ResultChange>,
    pub conflicts: Vec<ConflictRecord>,
    // Fallback records have family-specific evidence. None is implicitly
    // authorized; a registry-aware executor must validate named alternatives.
    pub fallbacks: Vec<Metadata>,
    pub render_report: Metadata,
    pub verification: ResultVerification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis: Option<ResultAnalysis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<ResultDiff>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflicted_output: Option<String>,
    pub extensions: Vec<NativeExtension>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultContractError {
    UnsupportedSchema,
    IdentityMismatch,
    SelectionMismatch,
    InvalidSourceRoles,
    DirectionalRoleContractViolation,
    BaseParticipationContractViolation,
    ContradictoryOutput,
    ContradictoryOutcome,
    MissingPayload,
    InvalidRecord,
    InvalidSourceEvidence,
    UndeclaredFallback,
    UnverifiedFallback,
    PreservationContractViolation,
    VerificationContractViolation,
}

impl fmt::Display for ResultContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ResultContractError {}

fn unique_nonempty<'a>(ids: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    ids.into_iter().all(|id| !id.is_empty() && seen.insert(id))
}

/// Correlate an out-of-order transport batch by identity, validate every member,
/// and return caller request order. Missing, extra or duplicate results reject
/// the entire batch instead of leaking partially accepted results.
pub fn validate_operation_results(
    requests: &[ValidatedOperationRequest],
    results: Vec<OperationResult>,
) -> Result<Vec<OperationResult>, ResultContractError> {
    validate_operation_results_with_evidence(requests, results, &BTreeMap::new())
}

/// As above, with executor evidence scoped by request ID. Missing evidence is
/// not borrowed from another batch item or from a conflict's own declarations.
pub fn validate_operation_results_with_evidence(
    requests: &[ValidatedOperationRequest],
    results: Vec<OperationResult>,
    evidence: &BTreeMap<String, ConflictEvidence>,
) -> Result<Vec<OperationResult>, ResultContractError> {
    if requests.len() != results.len()
        || !unique_nonempty(requests.iter().map(|request| request.request().request_id.as_str()))
        || !unique_nonempty(results.iter().map(|result| result.request_id.as_str()))
    {
        return Err(ResultContractError::IdentityMismatch);
    }
    if evidence.keys().any(|id| !requests.iter().any(|request| request.request().request_id == *id))
    {
        return Err(ResultContractError::IdentityMismatch);
    }
    let mut pending: BTreeMap<_, _> =
        results.into_iter().map(|result| (result.request_id.clone(), result)).collect();
    let mut ordered = Vec::with_capacity(requests.len());
    for request in requests {
        let result = pending
            .remove(&request.request().request_id)
            .ok_or(ResultContractError::IdentityMismatch)?;
        result.validate_with_conflict_evidence(
            request,
            evidence.get(&request.request().request_id).unwrap_or(&ConflictEvidence::default()),
        )?;
        ordered.push(result);
    }
    Ok(ordered)
}

fn role_name(role: SourceRole) -> &'static str {
    match role {
        SourceRole::Source => "source",
        SourceRole::Before => "before",
        SourceRole::After => "after",
        SourceRole::Incoming => "incoming",
        SourceRole::Current => "current",
        SourceRole::Base => "base",
        SourceRole::Ours => "ours",
        SourceRole::Theirs => "theirs",
        SourceRole::Output => "output",
    }
}

fn check_span(
    role: SourceRole,
    span: &ResultSpan,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let source = request.request().sources.get(&role).ok_or(Invalid)?;
    let document = request.sources().get(&source.source_id).map_err(|_| Invalid)?;
    document.slice(span.range.bytes()).map_err(|_| Invalid)?;
    for (offset, point) in
        [(span.range.start_byte, &span.start_point), (span.range.end_byte, &span.end_point)]
    {
        let actual = document.point(offset).map_err(|_| Invalid)?;
        if actual.row != point.row || actual.column != point.column {
            return Err(Invalid);
        }
    }
    Ok(())
}

fn check_region(
    region: &ResultSourceRegion,
    request: &ValidatedOperationRequest,
    digest_required: bool,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let source = request.request().sources.get(&region.source_role).ok_or(Invalid)?;
    if source.source_id != region.source_id || (digest_required && region.sha256.is_none()) {
        return Err(Invalid);
    }
    let document = request.sources().get(&region.source_id).map_err(|_| Invalid)?;
    let digest = document.range_digest(region.range.bytes()).map_err(|_| Invalid)?;
    if region.sha256.as_ref().is_some_and(|expected| *expected != digest) {
        return Err(Invalid);
    }
    Ok(())
}

impl OperationResult {
    /// Check observable contract invariants against verified request bytes.
    /// This is not a semantic verifier, registry negotiation, render-plan
    /// verifier, or Slice 1028 canonicalization. Nonempty fallbacks currently
    /// fail closed even when named, until their verification is implemented.
    pub fn validate_against(
        &self,
        request: &ValidatedOperationRequest,
    ) -> Result<(), ResultContractError> {
        self.validate_with_conflict_evidence(request, &ConflictEvidence::default())
    }

    /// Decision/render references and authorizations must come from the trusted
    /// executor, not be inferred from this result's own conflict declarations.
    pub fn validate_with_conflict_evidence(
        &self,
        request: &ValidatedOperationRequest,
        conflict_evidence: &ConflictEvidence,
    ) -> Result<(), ResultContractError> {
        use ResultContractError as E;
        let input = request.request();
        if self.schema != OPERATION_RESULT_SCHEMA {
            return Err(E::UnsupportedSchema);
        }
        if self.request_id != input.request_id || self.operation != input.operation.kind() {
            return Err(E::IdentityMismatch);
        }
        if self.output.is_some() && self.conflicted_output.is_some() {
            return Err(E::ContradictoryOutput);
        }
        let is_merge = matches!(self.operation, OperationKind::Merge2 | OperationKind::Merge3);
        if !is_merge && (self.output.is_some() || self.conflicted_output.is_some()) {
            return Err(E::ContradictoryOutput);
        }
        if !self.fallbacks.is_empty() {
            return Err(if input.operation.fallback_policy() == "none" {
                E::UndeclaredFallback
            } else {
                E::UnverifiedFallback
            });
        }
        let unresolved = self.conflicts.iter().any(ConflictRecord::unresolved);
        let blocking = self.diagnostics.iter().any(DiagnosticRecord::blocking);
        if (self.ok && (unresolved || blocking || self.conflicted_output.is_some()))
            || (!self.ok && !unresolved && !blocking)
            || (!self.ok && self.output.is_some())
            || (self.conflicted_output.is_some() && !unresolved)
        {
            return Err(E::ContradictoryOutcome);
        }
        let classified = self.ok
            || !self.changes.is_empty()
            || !self.conflicts.is_empty()
            || self.verification.classification_reached == Some(true);
        if classified && self.verification.classification_reached == Some(false) {
            return Err(E::ContradictoryOutcome);
        }
        let roles = self.operation.source_roles();
        if classified && self.verification.consumed_source_roles.as_deref() != Some(roles) {
            return Err(if self.operation == OperationKind::Merge2 {
                E::DirectionalRoleContractViolation
            } else {
                E::InvalidSourceRoles
            });
        }
        if classified
            && self.operation == OperationKind::Merge2
            && self.verification.directional_roles_preserved != Some(true)
        {
            return Err(E::DirectionalRoleContractViolation);
        }
        if classified
            && self.operation == OperationKind::Merge3
            && self.verification.base_participated != Some(true)
        {
            return Err(E::BaseParticipationContractViolation);
        }
        if let Some(consumed) = &self.verification.consumed_source_roles {
            if !consumed.iter().all(|role| roles.contains(role))
                || consumed.iter().collect::<std::collections::BTreeSet<_>>().len()
                    != consumed.len()
            {
                return Err(E::InvalidSourceRoles);
            }
        }
        if classified
            && (self.provider.provider_id.as_ref().is_none_or(String::is_empty)
                || self.provider.family.as_ref().is_none_or(String::is_empty))
        {
            return Err(E::SelectionMismatch);
        }
        if self.ok {
            match self.operation {
                OperationKind::Analyze if self.analysis.is_none() || self.diff.is_some() => {
                    return Err(E::MissingPayload);
                }
                OperationKind::Diff2 if self.diff.is_none() || self.analysis.is_some() => {
                    return Err(E::MissingPayload);
                }
                OperationKind::Merge2 | OperationKind::Merge3 if self.output.is_none() => {
                    return Err(E::MissingPayload);
                }
                _ => {}
            }
            if is_merge
                && (self.verification.output_reparsed != Some(true)
                    || self.verification.structural_equivalence != Some(true))
            {
                return Err(E::VerificationContractViolation);
            }
            if is_merge && self.verification.preservation.is_none() {
                return Err(E::PreservationContractViolation);
            }
        }
        if (self.operation != OperationKind::Analyze && self.analysis.is_some())
            || (self.operation != OperationKind::Diff2 && self.diff.is_some())
        {
            return Err(E::ContradictoryOutcome);
        }
        if self
            .analysis
            .as_ref()
            .is_some_and(|analysis| analysis.schema != "structuredmerge.analysis-result/v1")
        {
            return Err(E::UnsupportedSchema);
        }
        for (requested, actual) in [
            (&input.provider_selection.provider_id, &self.provider.provider_id),
            (&input.provider_selection.family, &self.provider.family),
            (&input.provider_selection.profile_id, &self.profile.profile_id),
        ] {
            if requested.is_some() && (classified || actual.is_some()) && requested != actual {
                return Err(E::SelectionMismatch);
            }
        }
        if let Some(backend) = &input.parser_selection.backend {
            if let Some(parser) = &self.profile.parser {
                if parser.requested_backend.as_ref() != Some(backend)
                    || parser.selection_mode.as_deref() != Some("explicit")
                    || parser.selected_backend.as_ref().is_some_and(|selected| selected != backend)
                    || (classified && parser.selected_backend.as_ref() != Some(backend))
                {
                    return Err(E::SelectionMismatch);
                }
            } else if classified {
                return Err(E::SelectionMismatch);
            }
        }
        if !unique_nonempty(self.diagnostics.iter().map(DiagnosticRecord::id))
            || !unique_nonempty(self.changes.iter().map(|record| record.id.as_str()))
            || !unique_nonempty(self.conflicts.iter().map(ConflictRecord::id))
        {
            return Err(E::InvalidRecord);
        }
        let canonical: Vec<_> = self
            .diagnostics
            .iter()
            .filter_map(|record| match record {
                DiagnosticRecord::Canonical(diagnostic) => Some(diagnostic),
                DiagnosticRecord::Migration(_) => None,
            })
            .collect();
        if !canonical.is_empty() && canonical.len() != self.diagnostics.len() {
            return Err(E::InvalidRecord);
        }
        if canonical.iter().any(|diagnostic| diagnostic.category == PortableCategory::MergeConflict)
            && self.conflicts.is_empty()
        {
            return Err(E::ContradictoryOutcome);
        }
        validate_diagnostics(
            &canonical,
            &input.request_id,
            self.operation,
            request.sources(),
            |subject| match subject.kind.as_str() {
                "conflict" => self.conflicts.iter().any(|conflict| conflict.id() == subject.id),
                "change" => self.changes.iter().any(|change| change.id == subject.id),
                "structural_path" => {
                    self.conflicts.iter().any(|conflict| conflict.matches_path(&subject.id))
                        || self.changes.iter().any(|change| {
                            change.subject_ref.as_ref() == Some(&subject.id)
                                || change.path.as_ref() == Some(&subject.id)
                        })
                }
                "operation" => match self.operation {
                    OperationKind::Merge3 => subject.id == "merge3.classification",
                    OperationKind::Merge2 => subject.id == "merge2.classification",
                    _ => false,
                },
                _ => false,
            },
        )
        .map_err(|_| E::InvalidRecord)?;
        for record in &self.diagnostics {
            let DiagnosticRecord::Migration(diagnostic) = record else { continue };
            if matches!(
                diagnostic.category.as_str(),
                "parse-error" | "parse_error" | "destination_parse_error"
            ) && diagnostic.source_role.is_none()
            {
                return Err(E::InvalidSourceEvidence);
            }
            if !matches!(diagnostic.severity.as_str(), "info" | "warning" | "error")
                || diagnostic.category.is_empty()
                || diagnostic.code.is_empty()
                || diagnostic.source_role.is_some_and(|role| !roles.contains(&role))
            {
                return Err(E::InvalidRecord);
            }
            if let Some(span) = &diagnostic.span {
                check_span(diagnostic.source_role.ok_or(E::InvalidSourceEvidence)?, span, request)?;
            }
        }
        for change in &self.changes {
            if change.classification.is_empty()
                || (change.subject_ref.as_ref().is_none_or(String::is_empty)
                    && change.path.as_ref().is_none_or(String::is_empty))
                || !change
                    .role_states
                    .keys()
                    .all(|name| roles.iter().any(|role| role_name(*role) == name))
            {
                return Err(E::InvalidRecord);
            }
            for (&role, span) in &change.source_spans {
                check_span(role, span, request)?;
            }
        }
        if let Some(diff) = &self.diff {
            if diff.change_ids
                != self.changes.iter().map(|change| change.id.clone()).collect::<Vec<_>>()
            {
                return Err(E::InvalidRecord);
            }
        }
        let canonical_conflicts: Vec<_> = self
            .conflicts
            .iter()
            .filter_map(|conflict| match conflict {
                ConflictRecord::Canonical(record) => Some(record.as_ref()),
                ConflictRecord::Migration(_) => None,
            })
            .collect();
        if !canonical_conflicts.is_empty() && canonical_conflicts.len() != self.conflicts.len() {
            return Err(E::InvalidRecord);
        }
        let output = self
            .output
            .as_ref()
            .or(self.conflicted_output.as_ref())
            .map(|text| {
                let input = crate::source_input(
                    "operation-output".into(),
                    SourceRole::Output,
                    crate::SourceEncoding::Utf8,
                    text.as_bytes().to_vec(),
                )?;
                crate::SourceDocument::validate(input, text.len() as u64)
            })
            .transpose()
            .map_err(|_| E::InvalidSourceEvidence)?;
        validate_conflicts(
            &canonical_conflicts,
            &ConflictValidationContext {
                operation: self.operation,
                result_ok: self.ok,
                sources: request.sources(),
                output: output.as_ref(),
                diagnostic_ids: &self
                    .diagnostics
                    .iter()
                    .map(|record| record.id().to_owned())
                    .collect(),
                change_ids: &self.changes.iter().map(|record| record.id.clone()).collect(),
                evidence: conflict_evidence,
            },
        )
        .map_err(|_| E::InvalidRecord)?;
        for record in &self.conflicts {
            let ConflictRecord::Migration(conflict) = record else { continue };
            if !is_merge
                || conflict.category.is_empty()
                || (conflict.subject_ref.as_ref().is_none_or(String::is_empty)
                    && conflict.path.as_ref().is_none_or(String::is_empty))
                || conflict.roles.is_empty()
                || !unique_nonempty(conflict.roles.iter().map(|role| role_name(*role)))
                || !conflict.roles.iter().all(|role| roles.contains(role))
                || !matches!(conflict.resolution.as_str(), "resolved" | "unresolved")
            {
                return Err(E::InvalidRecord);
            }
            for region in &conflict.source_regions {
                if !conflict.roles.contains(&region.source_role) {
                    return Err(E::InvalidSourceEvidence);
                }
                check_region(region, request, false)?;
            }
        }
        if let Some(properties) = &self.verification.preservation {
            if !unique_nonempty(properties.iter().map(|property| property.property.as_str())) {
                return Err(E::InvalidRecord);
            }
            for property in properties {
                if !matches!(property.status.as_str(), "passed" | "failed" | "unverified")
                    || (self.ok && property.required && property.status != "passed")
                {
                    return Err(E::PreservationContractViolation);
                }
            }
        }
        for region in self.verification.retained_source_regions.iter().flatten() {
            check_region(region, request, true)?;
        }
        Ok(())
    }
}
