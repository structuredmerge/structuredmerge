//! Structural profile introspection only; no AST selection or source mutation.
use serde::{Deserialize, Serialize};

/// Historical package responsibilities and coordinates, not supported-runtime
/// declarations or a capability/default-authority manifest.
pub fn report_structural_boundary() -> crate::CrisprBoundaryReport {
    ast_crispr::typed_boundary_report()
}

/// Describe and evaluate a conjunction of count constraints, not a selector.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrisprLimitRequest {
    pub constraints: Option<Vec<crate::CrisprLimitConstraint>>,
    pub counts: Vec<usize>,
}

pub fn report_structural_limit(request: CrisprLimitRequest) -> crate::CrisprLimitReport {
    let limit = ast_crispr::Limit::from_constraints(request.constraints);
    let counts = request.counts;
    crate::CrisprLimitReport {
        description: limit.describe(),
        allowed: counts.iter().map(|&count| limit.allows(count)).collect(),
        counts,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrisprMatchRequest {
    pub start_boundary: String,
    pub end_boundary: String,
    pub payload_kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrisprSelectionRequest {
    pub owner_scope: String,
    pub owner_selector: String,
    pub selector_kind: String,
    pub selection_intent: String,
    pub comment_region: Option<String>,
    pub include_trailing_gap: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrisprDestinationRequest {
    pub resolution_kind: String,
    pub resolution_source: String,
    pub anchor_boundary: String,
    pub used_if_missing: bool,
}

/// Describe match vocabulary without matching or selecting any source.
pub fn report_structural_match(request: CrisprMatchRequest) -> crate::CrisprMatchReport {
    ast_crispr::MatchProfile::new(
        &request.start_boundary,
        &request.end_boundary,
        &request.payload_kind,
    )
    .typed_report()
}

/// Describe selection vocabulary without executing a structural selector.
pub fn report_structural_selection(
    request: CrisprSelectionRequest,
) -> crate::CrisprSelectionReport {
    ast_crispr::SelectionProfile::new(
        &request.owner_scope,
        &request.owner_selector,
        &request.selector_kind,
        &request.selection_intent,
        request.comment_region.as_deref(),
        request.include_trailing_gap,
    )
    .typed_report()
}

/// Describe destination policy without resolving or mutating a destination.
pub fn report_structural_destination(
    request: CrisprDestinationRequest,
) -> crate::CrisprDestinationReport {
    ast_crispr::DestinationProfile::new(
        &request.resolution_kind,
        &request.resolution_source,
        &request.anchor_boundary,
        request.used_if_missing,
    )
    .typed_report()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrisprOperationRequest {
    pub operation_kind: String,
    pub source_requirement: String,
    pub destination_requirement: String,
    pub replacement_source: String,
    pub captures_source_text: bool,
    pub supports_if_missing: bool,
}

/// Describe structural operation profiles in input order using ast-crispr's
/// shared classification. Unknown strings remain observable as unknown; reports
/// are not validation that an operation can execute against a document.
pub fn report_structural_operations(
    requests: Vec<CrisprOperationRequest>,
) -> crate::CrisprBatchOperationReport {
    let profiles: Vec<_> = requests
        .into_iter()
        .map(|request| {
            ast_crispr::OperationProfile::new(
                &request.operation_kind,
                &request.source_requirement,
                &request.destination_requirement,
                &request.replacement_source,
                request.captures_source_text,
                request.supports_if_missing,
            )
        })
        .collect();
    ast_crispr::typed_batch_operation_report(&profiles)
}
