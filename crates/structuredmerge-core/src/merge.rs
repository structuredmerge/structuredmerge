//! Concrete binding projection of native-parser family operations.
//! This is not yet the complete portable provider-result envelope.
use crate::{CoreError, CoreParseResult, ParseLimits, ParseRequest};
use ast_merge::typed_merge::{
    NativeMergeError as MappingMergeError, NativeMergeExecution, merge_native_sources_with_evidence,
};
use serde::{Deserialize, Serialize};
use tree_haver::service::TreeHaverParseService;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RetainedSourceSegment {
    pub id: String,
    pub source_id: String,
    pub source_role: crate::SourceRole,
    pub source_range: crate::ByteRange,
    pub output_range: crate::ByteRange,
    pub sha256: String,
    pub owner_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeAnalysisRejection {
    pub code: String,
    pub source_id: String,
    pub source_role: crate::SourceRole,
    pub message: String,
}

/// All fields from the shared three-way result are retained without recoding
/// its conflicts, diagnostics, policies, or output. Native syntax rejection is
/// separately retained as a typed parse result, including its revision role.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeMergeResult {
    pub profile_id: String,
    pub outcome: ast_merge::ThreeWayMergeOutcome,
    pub diagnostics: Vec<ast_merge::Diagnostic>,
    pub conflicts: Vec<ast_merge::MergeConflict>,
    pub output: Option<String>,
    pub policies: Vec<ast_merge::PolicyReference>,
    pub rejected_parse: Option<CoreParseResult>,
    /// Complete input parse results in semantic role order, including warnings
    /// and every rejected revision. `rejected_parse` is the primary shorthand.
    pub input_parses: Vec<CoreParseResult>,
    /// Actual reparse of synthesized output, including native rejection. None
    /// when rendering was not attempted or an already-parsed input was selected.
    pub output_parse: Option<CoreParseResult>,
    pub verification_failure: Option<crate::ParserFailure>,
    /// Selection/provider failure before a complete validated input parse batch.
    pub input_failure: Option<crate::ParserFailure>,
    pub analysis_rejections: Vec<NativeAnalysisRejection>,
    pub sources: Vec<crate::SourceDescriptor>,
    pub output_source: Option<crate::SourceDescriptor>,
    pub source_segments: Vec<RetainedSourceSegment>,
}

/// Experimental block-mapping profile. Native hosts provide syntax only;
/// YAML analysis and shared matching/conflict/render execution remain Rust-owned.
/// No implicit base, parser substitution, textual fallback, or host merge exists.
pub fn merge_yaml_mapping(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<NativeMergeResult, CoreError> {
    merge_yaml_mapping_controlled(requests, limits, &crate::OperationControl::new())
}

pub fn merge_yaml_mapping_controlled(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
    control: &crate::OperationControl,
) -> Result<NativeMergeResult, CoreError> {
    let context = limits.controlled_context(control)?;
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    project_result(
        crate::profiles::YAML_MAPPING,
        merge_native_sources_with_evidence(
            "yaml",
            requests,
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            yaml_merge::typed::mapping_owners,
        ),
    )
}

/// Rust derives Python owners from LibCST facts and uses the same shared merge
/// engine as YAML. This initial profile supports whole top-level declarations.
pub fn merge_python_declarations(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<NativeMergeResult, CoreError> {
    merge_python_declarations_controlled(requests, limits, &crate::OperationControl::new())
}

pub fn merge_python_declarations_controlled(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
    control: &crate::OperationControl,
) -> Result<NativeMergeResult, CoreError> {
    let context = limits.controlled_context(control)?;
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    project_result(
        crate::profiles::PYTHON_DECLARATIONS,
        merge_native_sources_with_evidence(
            "python",
            requests,
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            python_merge::declaration_owners,
        ),
    )
}

fn project_result(
    profile_id: &str,
    result: Result<NativeMergeExecution, MappingMergeError>,
) -> Result<NativeMergeResult, CoreError> {
    match result {
        Ok(execution) => {
            let mut segments = Vec::new();
            for segment in execution.rendered.source_segments {
                let role = match segment.revision {
                    ast_merge::SourceRevision::Base => crate::SourceRole::Base,
                    ast_merge::SourceRevision::Ours => crate::SourceRole::Ours,
                    ast_merge::SourceRevision::Theirs => crate::SourceRole::Theirs,
                };
                let source =
                    execution.sources.iter().find(|source| source.role == role).ok_or_else(
                        || CoreError {
                            code: "invalid_byte_evidence".into(),
                            message: "missing source descriptor".into(),
                        },
                    )?;
                segments.push(RetainedSourceSegment {
                    id: segment.id,
                    source_id: source.source_id.clone(),
                    source_role: role,
                    source_range: segment.source_range,
                    output_range: segment.output_range,
                    sha256: segment.sha256,
                    owner_id: segment.owner_id,
                });
            }
            let result = execution.rendered.result;
            Ok(NativeMergeResult {
                profile_id: profile_id.into(),
                outcome: result.outcome,
                diagnostics: result.diagnostics,
                conflicts: result.conflicts,
                output: result.output,
                policies: result.policies,
                rejected_parse: None,
                analysis_rejections: vec![],
                input_failure: None,
                output_parse: execution.output_parse.map(CoreParseResult::from),
                verification_failure: execution.verification_error.map(crate::ParserFailure::from),
                input_parses: execution
                    .input_parses
                    .into_iter()
                    .map(CoreParseResult::from)
                    .collect(),
                sources: execution.sources,
                output_source: execution.output_source,
                source_segments: segments,
            })
        }
        Err(MappingMergeError::NativeParseRejected { parses, sources }) => {
            let input_parses: Vec<_> = parses.into_iter().map(CoreParseResult::from).collect();
            let rejected_parse = input_parses.iter().find(|result| !result.parsed.ok).cloned();
            Ok(NativeMergeResult {
                profile_id: profile_id.into(),
                outcome: ast_merge::ThreeWayMergeOutcome::Error,
                diagnostics: vec![],
                conflicts: vec![],
                output: None,
                policies: vec![],
                sources,
                output_source: None,
                source_segments: vec![],
                rejected_parse,
                analysis_rejections: vec![],
                input_failure: None,
                output_parse: None,
                verification_failure: None,
                input_parses,
            })
        }
        Err(MappingMergeError::AnalysisRejected { failures, parses, sources }) => {
            Ok(NativeMergeResult {
                profile_id: profile_id.into(),
                outcome: ast_merge::ThreeWayMergeOutcome::Error,
                diagnostics: vec![],
                conflicts: vec![],
                output: None,
                policies: vec![],
                rejected_parse: None,
                output_parse: None,
                verification_failure: None,
                output_source: None,
                source_segments: vec![],
                sources,
                input_failure: None,
                input_parses: parses.into_iter().map(CoreParseResult::from).collect(),
                analysis_rejections: failures
                    .into_iter()
                    .map(|failure| NativeAnalysisRejection {
                        code: "analysis.unsupported_profile".into(),
                        source_id: failure.source_id,
                        source_role: failure.source_role,
                        message: failure.message,
                    })
                    .collect(),
            })
        }
        Err(MappingMergeError::InputParseFailed { error, sources }) => {
            use tree_haver::service::ServiceError;
            match &error {
                ServiceError::Selection(_)
                | ServiceError::Provider { .. }
                | ServiceError::ProviderPanic { .. }
                | ServiceError::InvalidBatch { .. }
                | ServiceError::InvalidResult { .. }
                    if CoreError::from(error.clone()).code != "resource.limit" =>
                {
                    Ok(NativeMergeResult {
                        profile_id: profile_id.into(),
                        outcome: ast_merge::ThreeWayMergeOutcome::Error,
                        diagnostics: vec![],
                        conflicts: vec![],
                        output: None,
                        policies: vec![],
                        rejected_parse: None,
                        input_parses: vec![],
                        output_parse: None,
                        verification_failure: None,
                        analysis_rejections: vec![],
                        sources,
                        output_source: None,
                        source_segments: vec![],
                        input_failure: Some(crate::ParserFailure::from(error)),
                    })
                }
                _ => Err(CoreError::from(error)),
            }
        }
        Err(MappingMergeError::Parse(error)) => Err(CoreError::from(error)),
        Err(error) => Err(CoreError {
            code: match &error {
                MappingMergeError::InvalidInputs => "invalid_merge_inputs",
                MappingMergeError::Unsupported(_) => "unsupported_native_profile",
                MappingMergeError::Parse(_) => unreachable!(),
                MappingMergeError::InputParseFailed { .. } => unreachable!(),
                MappingMergeError::NativeParseRejected { .. } => unreachable!(),
                MappingMergeError::AnalysisRejected { .. } => unreachable!(),
            }
            .into(),
            message: format!("{error:?}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_haver::{parsed::ParseValidationError, service::ServiceError};

    #[test]
    fn controls_remain_errors_while_provider_faults_become_evidence() {
        for (error, code) in [
            (ServiceError::InvalidRequest, "request.invalid"),
            (ServiceError::LimitExceeded, "resource.limit"),
            (ServiceError::Cancelled, "execution.cancelled"),
            (ServiceError::DeadlineExceeded, "execution.deadline_exceeded"),
            (
                ServiceError::InvalidResult {
                    backend_id: "native".into(),
                    error: ParseValidationError::LimitExceeded,
                },
                "resource.limit",
            ),
        ] {
            for failure in [
                MappingMergeError::Parse(error.clone()),
                MappingMergeError::InputParseFailed { error, sources: vec![] },
            ] {
                assert_eq!(project_result("test", Err(failure)).unwrap_err().code, code);
            }
        }
        for (error, code) in [
            (ServiceError::ProviderPanic { backend_id: "native".into() }, "parser.provider_panic"),
            (
                ServiceError::InvalidResult {
                    backend_id: "native".into(),
                    error: ParseValidationError::IdentityMismatch,
                },
                "parser.invalid_result",
            ),
        ] {
            let result = project_result(
                "test",
                Err(MappingMergeError::InputParseFailed { error, sources: vec![] }),
            )
            .unwrap();
            assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Error);
            assert_eq!(result.input_failure.unwrap().code, code);
            assert!(result.output.is_none());
            assert!(result.sources.is_empty());
            assert!(result.input_parses.is_empty());
        }
    }
}
