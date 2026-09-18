//! Source-free capability observations, never source-specific merge approval.
use crate::{
    CoreError, OperationControl, OperationKind, OperationProfileCatalog, ParseLimits,
    ParserRegistryInventory, ParserSelection, ParserSelectionRequest, SelectionReport,
    operation_profile_catalog,
};
use serde::{Deserialize, Serialize};
use tree_haver::service::TreeHaverParseService;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityQuery {
    pub profile_id: String,
    pub operation: OperationKind,
    pub dialect: Option<String>,
    pub parser_selection: ParserSelection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityObservation {
    pub query: CapabilityQuery,
    pub operation_declared: bool,
    pub dialect_declared: bool,
    /// Absent when the profile does not declare this operation/dialect.
    pub parser_request: Option<ParserSelectionRequest>,
    pub parser_report: Option<SelectionReport>,
    /// Eligibility under the recorded parser requirements, not merge support.
    pub parser_eligible: Option<bool>,
    pub approved_as_default: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CapabilityManifest {
    pub schema: String,
    pub kernel_version: String,
    pub profiles: OperationProfileCatalog,
    pub parsers: ParserRegistryInventory,
    pub observations: Vec<CapabilityObservation>,
}

/// An empty query list only inventories declarations. Explicit queries may load
/// parsers/grammars while probing, but never parse source or execute a merge.
pub fn capability_manifest(
    queries: Vec<CapabilityQuery>,
    limits: ParseLimits,
) -> Result<CapabilityManifest, CoreError> {
    capability_manifest_controlled(queries, limits, &OperationControl::new())
}

/// All observations and declarations use one immutable registry snapshot even
/// if a callback mutates registration. Results are observations, not leases.
pub fn capability_manifest_controlled(
    queries: Vec<CapabilityQuery>,
    limits: ParseLimits,
    control: &OperationControl,
) -> Result<CapabilityManifest, CoreError> {
    let context = limits.controlled_context(control)?;
    context.check().map_err(CoreError::from)?;
    if queries.len() > context.max_batch_items {
        return Err(CoreError {
            code: "resource.limit".into(),
            message: "capability queries exceed max_batch_items".into(),
        });
    }
    let profiles = operation_profile_catalog();
    // Reject the whole invalid batch before any provider callbacks.
    if queries.iter().any(|query| !profiles.profiles.iter().any(|p| p.id == query.profile_id)) {
        return Err(CoreError {
            code: "capability.unknown_profile".into(),
            message: "capability query names an unknown operation profile".into(),
        });
    }
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    let parsers = snapshot.inventory();
    let mut observations = Vec::with_capacity(queries.len());
    for query in queries {
        context.check().map_err(CoreError::from)?;
        let profile = profiles.profiles.iter().find(|p| p.id == query.profile_id).unwrap();
        let operation_declared = profile.operations.contains(&query.operation);
        let dialect_declared =
            query.dialect.as_ref().is_none_or(|d| profile.explicit_dialects.contains(d));
        let parser_request = if operation_declared && dialect_declared {
            let language =
                crate::profiles::profile_parser_language(&profile.family, query.dialect.as_deref())
                    .expect("declared profile dialect");
            Some(ParserSelectionRequest {
                language: language.into(),
                // Dialects select a parser language, as in execute_operation.
                dialect: None,
                selection: query.parser_selection.clone(),
                options: crate::profiles::operation_parse_options(&profile.id, query.operation),
            })
        } else {
            None
        };
        let parser_report = parser_request
            .as_ref()
            .map(|request| {
                TreeHaverParseService::default()
                    .selection_report(request, &snapshot, &context)
                    .map_err(CoreError::from)
            })
            .transpose()?;
        let parser_eligible =
            parser_report.as_ref().map(|report| report.selected_backend.is_some());
        observations.push(CapabilityObservation {
            query,
            operation_declared,
            dialect_declared,
            parser_request,
            parser_report,
            parser_eligible,
            approved_as_default: profile.approved_as_default,
        });
    }
    context.check().map_err(CoreError::from)?;
    Ok(CapabilityManifest {
        schema: "structuredmerge.typed-capability-manifest/v1".into(),
        kernel_version: env!("CARGO_PKG_VERSION").into(),
        profiles,
        parsers,
        observations,
    })
}
