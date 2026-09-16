//! Structural profile introspection only; no AST selection or source mutation.
use serde::{Deserialize, Serialize};

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
