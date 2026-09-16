//! Development typed diff boundary, not yet the complete portable operation
//! envelope. No output, synthetic base or host-owned matching is introduced.
use crate::{
    CoreError, CoreParseResult, NativeAnalysisRejection, ParseLimits, ParseRequest, ParserFailure,
    SourceDescriptor,
};
use serde::{Deserialize, Serialize};
use tree_haver::service::TreeHaverParseService;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeDiffRequest {
    pub request_id: String,
    pub profile_id: String,
    pub parses: Vec<ParseRequest>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeDiffResult {
    pub request_id: String,
    pub profile_id: String,
    pub ok: bool,
    pub sources: Vec<SourceDescriptor>,
    pub diff: Option<crate::OwnerDiff>,
    pub diagnostics: Vec<crate::Diagnostic>,
    pub input_parses: Vec<CoreParseResult>,
    pub input_failure: Option<ParserFailure>,
    pub analysis_rejections: Vec<NativeAnalysisRejection>,
}

pub fn diff_native_owners(
    request: NativeDiffRequest,
    limits: ParseLimits,
) -> Result<NativeDiffResult, CoreError> {
    diff_native_owners_controlled(request, limits, &crate::OperationControl::new())
}

pub fn diff_native_owners_controlled(
    request: NativeDiffRequest,
    limits: ParseLimits,
    control: &crate::OperationControl,
) -> Result<NativeDiffResult, CoreError> {
    if request.request_id.is_empty() {
        return Err(CoreError {
            code: "request.invalid".into(),
            message: "diff requires request identity".into(),
        });
    }
    type Analyzer = fn(
        &tree_haver::service::ParsedResult,
    ) -> Result<ast_merge::SourcePreservingOwnerDocument, String>;
    let (language, analyze): (&str, Analyzer) = match request.profile_id.as_str() {
        crate::profiles::YAML_MAPPING => ("yaml", yaml_merge::typed::mapping_owners),
        crate::profiles::PYTHON_DECLARATIONS => ("python", python_merge::declaration_owners),
        _ => {
            return Err(CoreError {
                code: "unsupported_native_profile".into(),
                message: "no diff implementation for requested profile".into(),
            });
        }
    };
    let context = limits.controlled_context(control)?;
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    let result = ast_merge::typed_diff::diff_native_sources_with_evidence(
        language,
        request.parses,
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        analyze,
    );
    match result {
        Ok(execution) => Ok(NativeDiffResult {
            request_id: request.request_id,
            profile_id: request.profile_id,
            ok: true,
            sources: execution.diff.sources.clone(),
            diff: Some(execution.diff),
            diagnostics: vec![],
            input_parses: execution.input_parses.into_iter().map(CoreParseResult::from).collect(),
            input_failure: None,
            analysis_rejections: vec![],
        }),
        Err(ast_merge::typed_merge::NativeMergeError::InvalidInputs) => Err(CoreError {
            code: "invalid_diff_inputs".into(),
            message: "diff requires compatible before/after parse inputs".into(),
        }),
        Err(error) => {
            // Reuse failure projection only; no merge operation is executed.
            // This preserves existing service/control categorization across
            // boundaries while the complete portable result is being built.
            let failure = crate::merge::project_result(&request.profile_id, Err(error))?;
            Ok(NativeDiffResult {
                request_id: request.request_id,
                profile_id: request.profile_id,
                ok: false,
                sources: failure.sources,
                diff: None,
                diagnostics: failure.diagnostics,
                input_parses: failure.input_parses,
                input_failure: failure.input_failure,
                analysis_rejections: failure.analysis_rejections,
            })
        }
    }
}
