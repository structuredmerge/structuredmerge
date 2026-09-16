//! Directional native orchestration. Both input roles and output verification
//! use one TreeHaver registry snapshot. Family planning remains Rust-owned.
use crate::{
    SourcePreservingOwnerDocument,
    directional_render::{DirectionalInsertion, DirectionalRender, render_directional_owners},
    typed_merge::{NativeAnalysisFailure, NativeMergeError},
};
use std::collections::BTreeSet;
use tree_haver::{
    service::{
        ExecutionContext, ParseRequest, ParseService, ParsedResult, ParserRegistrySnapshot,
        ServiceError,
    },
    source::{SourceEncoding, SourceMap, SourceRole, source_input},
};

pub type DirectionalPlanner = fn(
    &ParsedResult,
    &SourcePreservingOwnerDocument,
    &ParsedResult,
    &SourcePreservingOwnerDocument,
) -> Result<Vec<DirectionalInsertion>, String>;

#[derive(Debug)]
pub enum DirectionalFailureStage {
    Planning,
    Rendering,
    Verification,
}

#[derive(Debug)]
pub struct NativeDirectionalExecution {
    /// Err retains input/output parse evidence but never exposes rejected output.
    pub rendered: Result<DirectionalRender, String>,
    pub input_parses: Vec<ParsedResult>,
    pub output_parse: Option<ParsedResult>,
    pub verification_error: Option<ServiceError>,
    pub failure_stage: Option<DirectionalFailureStage>,
}

/// Internal building block, not advertised provider support. The Rust family
/// planner selects placement/attachments from analyzed ownership; native hosts
/// supply syntax facts only. Current parser selection/options are retained for
/// verification, with the actual input backend pinned explicitly.
pub fn merge_directional_native_sources(
    language: &str,
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyze: fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>,
    plan: DirectionalPlanner,
) -> Result<NativeDirectionalExecution, NativeMergeError> {
    let roles: BTreeSet<_> = requests.iter().map(|r| r.source.descriptor.role).collect();
    if requests.len() != 2
        || roles != BTreeSet::from([SourceRole::Incoming, SourceRole::Current])
        || requests.iter().any(|r| r.language != language)
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    context.check().map_err(NativeMergeError::Parse)?;
    if requests.len() > context.max_batch_items {
        return Err(NativeMergeError::Parse(ServiceError::LimitExceeded));
    }
    let mut verification =
        requests.iter().find(|r| r.source.descriptor.role == SourceRole::Current).unwrap().clone();
    let sources_map = SourceMap::validate(
        requests.iter().map(|r| r.source.clone()).collect(),
        context.max_input_bytes,
    )
    .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))?;
    let mut sources = requests
        .iter()
        .map(|r| sources_map.get(&r.source.descriptor.source_id).unwrap().descriptor().clone())
        .collect::<Vec<_>>();
    sources.sort_by_key(|s| s.role);
    let parsed = service.parse_batch(requests, snapshot, context);
    context.check().map_err(NativeMergeError::Parse)?;
    let mut parsed = parsed
        .map_err(|error| NativeMergeError::InputParseFailed { error, sources: sources.clone() })?;
    parsed.sort_by_key(|p| p.source.descriptor().role);
    if parsed.len() != 2
        || parsed.iter().zip(&sources).any(|(p, s)| p.source.descriptor() != s)
        || parsed[0].backend.id != parsed[1].backend.id
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    if parsed.iter().any(|p| !p.document.output().ok) {
        return Err(NativeMergeError::NativeParseRejected { parses: parsed, sources });
    }
    let mut documents = Vec::new();
    let mut failures = Vec::new();
    for result in &parsed {
        let analysis = analyze(result).and_then(|doc| {
            if doc.source.as_bytes() != result.source.bytes() {
                return Err("directional analysis changed source bytes".into());
            }
            doc.validate("directional input")?;
            Ok(doc)
        });
        context.check().map_err(NativeMergeError::Parse)?;
        match analysis {
            Ok(doc) => documents.push(doc),
            Err(message) => failures.push(NativeAnalysisFailure {
                source_id: result.source.descriptor().source_id.clone(),
                source_role: result.source.descriptor().role,
                message,
            }),
        }
    }
    if !failures.is_empty() {
        return Err(NativeMergeError::AnalysisRejected { failures, parses: parsed, sources });
    }
    let incoming =
        parsed.iter().position(|p| p.source.descriptor().role == SourceRole::Incoming).unwrap();
    let current = 1 - incoming;
    let insertions =
        plan(&parsed[incoming], &documents[incoming], &parsed[current], &documents[current]);
    context.check().map_err(NativeMergeError::Parse)?;
    let insertions = match insertions {
        Ok(insertions) => insertions,
        Err(message) => {
            return Ok(NativeDirectionalExecution {
                rendered: Err(message),
                input_parses: parsed,
                output_parse: None,
                verification_error: None,
                failure_stage: Some(DirectionalFailureStage::Planning),
            });
        }
    };
    verification.selection.backend_id = Some(parsed[current].backend.id.clone());
    let mut output_id = "merge2-output".to_owned();
    while sources.iter().any(|s| s.source_id == output_id) {
        output_id.push('_');
    }
    let mut output_parse = None;
    let mut verification_error = None;
    let mut verification_attempted = false;
    let rendered = render_directional_owners(
        &parsed[incoming].source,
        &documents[incoming],
        &parsed[current].source,
        &documents[current],
        &insertions,
        |output| {
            verification_attempted = true;
            verification.request_id = format!("{}:verification", verification.request_id);
            verification.source = source_input(
                output_id,
                SourceRole::Output,
                SourceEncoding::Utf8,
                output.as_bytes().to_vec(),
            )
            .map_err(|e| e.to_string())?;
            let result = service.parse_batch(vec![verification.clone()], snapshot, context);
            let result = context.check().and(result);
            let mut result = match result {
                Ok(result) => result,
                Err(error) => {
                    verification_error = Some(error);
                    return Err("directional output parser service failed".into());
                }
            };
            if result.len() != 1
                || result[0].source.descriptor() != &verification.source.descriptor
                || result[0].backend.id != parsed[current].backend.id
            {
                return Err("directional output parse identity mismatch".into());
            }
            let result = result.remove(0);
            let analysis = if result.document.output().ok {
                analyze(&result)
            } else {
                Err("native parser rejected directional output".into())
            };
            output_parse = Some(result);
            analysis
        },
    );
    context.check().map_err(NativeMergeError::Parse)?;
    let failure_stage = rendered.is_err().then_some(if verification_attempted {
        DirectionalFailureStage::Verification
    } else {
        DirectionalFailureStage::Rendering
    });
    Ok(NativeDirectionalExecution {
        rendered,
        input_parses: parsed,
        output_parse,
        verification_error,
        failure_stage,
    })
}
