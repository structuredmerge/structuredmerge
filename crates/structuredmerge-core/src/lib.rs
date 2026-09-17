//! Typed public kernel facade, implemented incrementally under Slices 1024–1030.
//! This crate does not depend on the discarded host-prototype facade.
//! Validation is not a claim of implemented merge or parser capabilities.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};

pub mod host;
pub use host::*;
pub mod merge;
pub use ast_merge;
pub use ast_merge::{
    ConflictAlternative, ConflictAlternativeState, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeConflict, OwnedSourceRegion, PolicyReference, PolicySurface,
    ReviewDecisionAction, ReviewDiagnosticDetail, ReviewDiagnosticReason, SourceRevision,
    ThreeWayMergeOutcome,
};
pub use merge::*;
mod diff;
pub use ast_merge::owner_diff;
pub use ast_merge::owner_diff::{DiffSourceRegion, OwnerChange, OwnerChangeKind, OwnerDiff};
pub use diff::*;
mod edits;
mod json_diff;
mod json_operation;
mod native_analysis_projection;
pub mod native_conflict_projection;
pub mod native_operation;
pub mod operation;
pub mod operation_result;
mod operations;
pub use operation::*;
pub use operation_result::*;
pub use operations::*;
pub use portable_conflict::*;
pub use portable_diagnostic::*;
pub mod portable_conflict;
pub mod portable_diagnostic;
pub use edits::*;
mod structural;
pub use ast_crispr::{CrisprBatchOperationReport, CrisprOperationReport};
pub use ast_crispr::{
    CrisprBoundaryImplementation, CrisprBoundaryMetadata, CrisprBoundaryRelationships,
    CrisprBoundaryReport,
};
pub use ast_crispr::{CrisprDestinationReport, CrisprMatchReport, CrisprSelectionReport};
pub use ast_crispr::{CrisprLimitConstraint, CrisprLimitOperator, CrisprLimitReport};
pub use structural::*;
mod templates;
pub use templates::*;
mod profiles;
pub use profiles::{NativeMergeProfile, native_merge_profiles};
pub use tree_haver::parsed::{
    AttachmentHint, ChildEdge, Metadata, NativeExtension, ParseComment, ParseDiagnostic, ParseNode,
    ParseOutput, ParseSeverity,
};
pub use tree_haver::service::{
    ParseOptions, ParseRequest, ParserCandidate, ParserProbeRequest, ParserProbeResult,
    ParserProviderDescriptor, SelectionReport,
};
pub use tree_haver::{ByteRange, NodeRole, SourcePoint, SourceSpan};
pub use tree_haver::{parsed, service, source};

pub use tree_haver::service::ParserSelection;
pub use tree_haver::source::{
    LineEndings, SourceDescriptor, SourceDocument, SourceEncoding, SourceError, SourceErrorCode,
    SourceInput, SourceMap, SourceRole, source_input,
};

pub const OPERATION_SCHEMA: &str = "structuredmerge.operation-request/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Analyze,
    Diff2,
    Merge2,
    Merge3,
}

impl OperationKind {
    pub fn source_roles(self) -> &'static [SourceRole] {
        match self {
            Self::Analyze => &[SourceRole::Source],
            Self::Diff2 => &[SourceRole::Before, SourceRole::After],
            Self::Merge2 => &[SourceRole::Incoming, SourceRole::Current],
            Self::Merge3 => &[SourceRole::Base, SourceRole::Ours, SourceRole::Theirs],
        }
    }
}

/// Selection stays separate: merge provider identity never substitutes for a parser ID.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderSelection {
    pub provider_id: Option<String>,
    pub family: String,
    pub required_capabilities: Vec<String>,
}

/// Typed request inputs. No argument-position role inference or implicit base.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationInputs {
    pub schema: String,
    pub request_id: String,
    pub operation: OperationKind,
    pub provider_selection: ProviderSelection,
    pub parser_selection: ParserSelection,
    pub sources: Vec<SourceInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestError {
    UnsupportedSchema,
    EmptyRequestId,
    InvalidSelection,
    InvalidSourceRoles,
    Source(SourceError),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RequestError {}

/// Validate before any parser/provider dispatch. This does not select a provider.
pub fn validate_operation_inputs(
    request: OperationInputs,
    max_total_bytes: u64,
) -> Result<SourceMap, RequestError> {
    if request.schema != OPERATION_SCHEMA {
        return Err(RequestError::UnsupportedSchema);
    }
    if request.request_id.is_empty() {
        return Err(RequestError::EmptyRequestId);
    }
    if request.provider_selection.family.is_empty()
        || request.provider_selection.provider_id.as_ref().is_some_and(String::is_empty)
        || request.parser_selection.backend_id.as_ref().is_some_and(String::is_empty)
    {
        return Err(RequestError::InvalidSelection);
    }
    let expected = request.operation.source_roles();
    let roles: BTreeSet<_> = request.sources.iter().map(|input| input.descriptor.role).collect();
    if request.sources.len() != expected.len()
        || roles.len() != expected.len()
        || !expected.iter().all(|role| roles.contains(role))
    {
        return Err(RequestError::InvalidSourceRoles);
    }
    SourceMap::validate(request.sources, max_total_bytes).map_err(RequestError::Source)
}
