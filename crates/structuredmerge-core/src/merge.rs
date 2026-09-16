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
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    project_result(merge_native_sources_with_evidence(
        "yaml",
        requests,
        &TreeHaverParseService::default(),
        &snapshot,
        &limits.context(),
        yaml_merge::typed::mapping_owners,
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
    project_result(merge_native_sources_with_evidence(
        "python",
        requests,
        &TreeHaverParseService::default(),
        &snapshot,
        &limits.context(),
        python_merge::declaration_owners,
    ))
}

fn project_result(
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
                outcome: result.outcome,
                diagnostics: result.diagnostics,
                conflicts: result.conflicts,
                output: result.output,
                policies: result.policies,
                rejected_parse: None,
                sources: execution.sources,
                output_source: execution.output_source,
                source_segments: segments,
            })
        }
        Err(MappingMergeError::NativeParseRejected { parsed, sources }) => Ok(NativeMergeResult {
            outcome: ast_merge::ThreeWayMergeOutcome::Error,
            diagnostics: vec![],
            conflicts: vec![],
            output: None,
            policies: vec![],
            sources,
            output_source: None,
            source_segments: vec![],
            rejected_parse: Some(CoreParseResult {
                schema: parsed.schema,
                selection: parsed.selection,
                backend: parsed.backend,
                parsed: parsed.document.output().clone(),
            }),
        }),
        Err(MappingMergeError::Parse(error)) => Err(CoreError::from(error)),
        Err(error) => Err(CoreError {
            code: match &error {
                MappingMergeError::InvalidInputs => "invalid_merge_inputs",
                MappingMergeError::Unsupported(_) => "unsupported_native_profile",
                MappingMergeError::Parse(_) => unreachable!(),
                MappingMergeError::NativeParseRejected { .. } => unreachable!(),
            }
            .into(),
            message: format!("{error:?}"),
        }),
    }
}
