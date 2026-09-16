//! Typed public kernel facade, implemented incrementally under Slices 1024–1030.
//! This crate does not depend on the discarded host-prototype facade.
//! Validation is not a claim of implemented merge or parser capabilities.

use std::{collections::BTreeSet, error::Error, fmt};

pub use tree_haver::source::{
    LineEndings, SourceDescriptor, SourceDocument, SourceEncoding, SourceError, SourceErrorCode,
    SourceInput, SourceMap, SourceRole, source_input,
};

pub const OPERATION_SCHEMA: &str = "structuredmerge.operation-request/v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSelection {
    pub provider_id: Option<String>,
    pub family: String,
    pub required_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserSelection {
    pub backend_id: Option<String>,
    pub preference: Vec<String>,
    pub required_capabilities: Vec<String>,
}

/// Typed request inputs. No argument-position role inference or implicit base.
#[derive(Clone, Debug, Eq, PartialEq)]
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
