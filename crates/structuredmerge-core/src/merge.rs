//! Concrete binding projection of native-parser family operations.
//! This is not yet the complete portable provider-result envelope.
use crate::{CoreError, CoreParseResult, ParseLimits, ParseRequest};
use serde::{Deserialize, Serialize};
use tree_haver::service::TreeHaverParseService;
use yaml_merge::typed::{MappingMergeError, merge_mapping_sources};

/// All fields from the shared three-way result are retained without recoding
/// its conflicts, diagnostics, policies, or output. Native syntax rejection is
/// separately retained as a typed parse result, including its revision role.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeMergeResult {
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
) -> Result<NativeMergeResult, CoreError> {
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    project_result(merge_mapping_sources(
        requests,
        &TreeHaverParseService::default(),
        &snapshot,
        &limits.context(),
    ))
}

/// Rust derives Python owners from LibCST facts and uses the same shared merge
/// engine as YAML. This initial profile supports whole top-level declarations.
pub fn merge_python_declarations(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<NativeMergeResult, CoreError> {
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    project_result(ast_merge::typed_merge::merge_native_sources(
        "python",
        requests,
        &TreeHaverParseService::default(),
        &snapshot,
        &limits.context(),
        python_merge::declaration_owners,
    ))
}

fn project_result(
    result: Result<ast_merge::ThreeWayMergeResult<String>, MappingMergeError>,
) -> Result<NativeMergeResult, CoreError> {
    match result {
        Ok(result) => Ok(NativeMergeResult {
            outcome: result.outcome,
            diagnostics: result.diagnostics,
            conflicts: result.conflicts,
            output: result.output,
            policies: result.policies,
            rejected_parse: None,
        }),
        Err(MappingMergeError::NativeParseRejected(parsed)) => Ok(NativeMergeResult {
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
                MappingMergeError::Unsupported(_) => "unsupported_native_profile",
                MappingMergeError::Parse(_) => "parse_service",
                MappingMergeError::NativeParseRejected(_) => unreachable!(),
            }
            .into(),
            message: format!("{error:?}"),
        }),
    }
}
