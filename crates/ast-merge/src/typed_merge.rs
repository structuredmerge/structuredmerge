//! Shared orchestration over validated native syntax facts. Family analysis
//! supplies owners; hosts cannot supply matching, conflict, or render decisions.
use crate::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument, ThreeWayMergeResult,
    merge_source_preserving_owners_with_evidence,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{
    service::{
        ExecutionContext, ParseRequest, ParseService, ParsedResult, ParserRegistrySnapshot,
        ServiceError,
    },
    source::{SourceDescriptor, SourceEncoding, SourceRole, source_input},
};

#[derive(Debug)]
pub enum NativeMergeError {
    InvalidInputs,
    Parse(ServiceError),
    NativeParseRejected { parses: Vec<ParsedResult>, sources: Vec<SourceDescriptor> },
    Unsupported(String),
}

pub fn merge_native_sources(
    language: &str,
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyze: fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>,
) -> Result<ThreeWayMergeResult<String>, NativeMergeError> {
    merge_native_sources_with_evidence(language, requests, service, snapshot, context, analyze)
        .map(|execution| execution.rendered.result)
}

pub struct NativeMergeExecution {
    pub rendered: SourcePreservingMergeEvidence,
    pub sources: Vec<SourceDescriptor>,
    pub output_source: Option<SourceDescriptor>,
    pub input_parses: Vec<ParsedResult>,
}

pub fn merge_native_sources_with_evidence(
    language: &str,
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyze: fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>,
) -> Result<NativeMergeExecution, NativeMergeError> {
    let roles: BTreeSet<_> =
        requests.iter().map(|request| request.source.descriptor.role).collect();
    if requests.len() != 3
        || roles != BTreeSet::from([SourceRole::Base, SourceRole::Ours, SourceRole::Theirs])
        || requests.iter().any(|request| request.language != language)
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    let mut verification = requests[0].clone();
    let mut parsed =
        service.parse_batch(requests, snapshot, context).map_err(NativeMergeError::Parse)?;
    let backend = parsed.first().ok_or(NativeMergeError::InvalidInputs)?.backend.id.clone();
    if parsed.iter().any(|result| result.backend.id != backend) {
        return Err(NativeMergeError::InvalidInputs);
    }
    verification.selection.backend_id = Some(backend);
    // Failure identity must not depend on request order, and a family-analysis
    // rejection must not hide native syntax errors in another revision.
    parsed.sort_by_key(|result| result.source.descriptor().role);
    let sources: Vec<_> = parsed.iter().map(|result| result.source.descriptor().clone()).collect();
    if parsed.iter().any(|result| !result.document.output().ok) {
        return Err(NativeMergeError::NativeParseRejected { parses: parsed, sources });
    }
    let mut documents = BTreeMap::new();
    for result in &parsed {
        documents.insert(
            result.source.descriptor().role,
            analyze(result).map_err(NativeMergeError::Unsupported)?,
        );
    }
    let mut output_id = "merge-output".to_owned();
    while sources.iter().any(|source| source.source_id == output_id) {
        output_id.push('_');
    }
    let mut verify = |output: &str| -> Result<SourcePreservingOwnerDocument, String> {
        verification.request_id = "merge-verification".into();
        verification.source = source_input(
            output_id.clone(),
            SourceRole::Output,
            SourceEncoding::Utf8,
            output.as_bytes().to_vec(),
        )
        .map_err(|error| error.to_string())?;
        let parsed = service
            .parse_batch(vec![verification.clone()], snapshot, context)
            .map_err(|error| format!("{error:?}"))?;
        analyze(parsed.first().ok_or("missing verification parse")?)
    };
    let rendered = merge_source_preserving_owners_with_evidence(
        documents.remove(&SourceRole::Base).ok_or(NativeMergeError::InvalidInputs)?,
        documents.remove(&SourceRole::Ours).ok_or(NativeMergeError::InvalidInputs)?,
        documents.remove(&SourceRole::Theirs).ok_or(NativeMergeError::InvalidInputs)?,
        &mut verify,
    );
    context.check().map_err(NativeMergeError::Parse)?;
    let output_source = rendered
        .result
        .output
        .as_ref()
        .map(|output| {
            source_input(
                output_id.clone(),
                SourceRole::Output,
                SourceEncoding::Utf8,
                output.as_bytes().to_vec(),
            )
            .map(|input| input.descriptor)
            .map_err(|error| NativeMergeError::Unsupported(error.to_string()))
        })
        .transpose()?;
    Ok(NativeMergeExecution { rendered, sources, output_source, input_parses: parsed })
}
