//! Concrete binding projection of the existing YAML mapping operation.
//! This is not yet the complete portable provider-result envelope.
use crate::{CoreError, CoreParseResult, ParseLimits, ParseRequest};
use serde::{Deserialize, Serialize};
use tree_haver::service::TreeHaverParseService;
use yaml_merge::typed::{MappingMergeError, merge_mapping_sources};

/// All fields from the shared three-way result are retained without recoding
/// its conflicts, diagnostics, policies, or output. Native syntax rejection is
/// separately retained as a typed parse result, including its revision role.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MappingMergeResult {
    pub outcome: ast_merge::ThreeWayMergeOutcome,
    pub diagnostics: Vec<ast_merge::Diagnostic>,
    pub conflicts: Vec<ast_merge::MergeConflict>,
    pub output: Option<String>,
    pub policies: Vec<ast_merge::PolicyReference>,
    pub rejected_parse: Option<CoreParseResult>,
}

/// Experimental block-mapping profile. Native hosts provide syntax only;
/// YAML analysis and shared matching/conflict/render execution remain Rust-owned.
/// No implicit base, parser substitution, textual fallback, or host merge exists.
pub fn merge_yaml_mapping(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<MappingMergeResult, CoreError> {
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    match merge_mapping_sources(
        requests,
        &TreeHaverParseService::default(),
        &snapshot,
        &limits.context(),
    ) {
        Ok(result) => Ok(MappingMergeResult {
            outcome: result.outcome,
            diagnostics: result.diagnostics,
            conflicts: result.conflicts,
            output: result.output,
            policies: result.policies,
            rejected_parse: None,
        }),
        Err(MappingMergeError::NativeParseRejected(parsed)) => Ok(MappingMergeResult {
            outcome: ast_merge::ThreeWayMergeOutcome::Error,
            diagnostics: vec![],
            conflicts: vec![],
            output: None,
            policies: vec![],
            rejected_parse: Some(CoreParseResult {
                schema: parsed.schema,
                selection: parsed.selection,
                backend: parsed.backend,
                parsed: parsed.document.output().clone(),
            }),
        }),
        Err(error) => Err(CoreError {
            code: match &error {
                MappingMergeError::InvalidInputs => "invalid_merge_inputs",
                MappingMergeError::Unsupported(_) => "unsupported_mapping_profile",
                MappingMergeError::Parse(_) => "parse_service",
                MappingMergeError::NativeParseRejected(_) => unreachable!(),
            }
            .into(),
            message: format!("{error:?}"),
        }),
    }
}
