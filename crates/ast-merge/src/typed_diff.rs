//! TreeHaver-backed diff orchestration. Hosts supply syntax, family analyzers
//! supply ownership, and the shared Rust diff primitive classifies changes.
use crate::{
    SourcePreservingOwnerDocument,
    owner_diff::{OwnerDiff, diff_owner_documents},
    typed_merge::{NativeAnalysisFailure, NativeMergeError},
};
use std::collections::BTreeSet;
use tree_haver::{
    service::{
        ExecutionContext, ParseRequest, ParseService, ParsedResult, ParserRegistrySnapshot,
        ServiceError,
    },
    source::{SourceMap, SourceRole},
};

#[derive(Debug)]
pub struct NativeDiffExecution {
    pub diff: OwnerDiff,
    pub input_parses: Vec<ParsedResult>,
}

/// Internal operation building block; shares the existing typed-operation
/// failure variants. It neither renders output nor performs verification parses.
pub fn diff_native_sources_with_evidence(
    language: &str,
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyze: fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>,
) -> Result<NativeDiffExecution, NativeMergeError> {
    diff_native_sources_with_engine(language, requests, service, snapshot, context, analyze, None)
}

pub fn diff_native_sources_with_engine(
    language: &str,
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyze: fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>,
    analyze_with_diagnostics: Option<
        fn(
            &ParsedResult,
        )
            -> Result<SourcePreservingOwnerDocument, crate::typed_merge::NativeAnalysisError>,
    >,
) -> Result<NativeDiffExecution, NativeMergeError> {
    let roles: BTreeSet<_> =
        requests.iter().map(|request| request.source.descriptor.role).collect();
    if requests.len() != 2
        || roles != BTreeSet::from([SourceRole::Before, SourceRole::After])
        || requests.iter().any(|request| request.language != language)
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    context.check().map_err(NativeMergeError::Parse)?;
    if requests.len() > context.max_batch_items {
        return Err(NativeMergeError::Parse(ServiceError::LimitExceeded));
    }
    let map = SourceMap::validate(
        requests.iter().map(|request| request.source.clone()).collect(),
        context.max_input_bytes,
    )
    .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))?;
    let mut sources = requests
        .iter()
        .map(|request| {
            map.get(&request.source.descriptor.source_id)
                .map(|source| source.descriptor().clone())
                .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    sources.sort_by_key(|source| source.role);
    let parsed = service.parse_batch(requests, snapshot, context);
    // Late callback results or failures cannot override cancellation/deadlines.
    context.check().map_err(NativeMergeError::Parse)?;
    let mut parsed = parsed
        .map_err(|error| NativeMergeError::InputParseFailed { error, sources: sources.clone() })?;
    parsed.sort_by_key(|result| result.source.descriptor().role);
    if parsed.len() != 2
        || parsed.iter().zip(&sources).any(|(result, source)| result.source.descriptor() != source)
        || parsed[0].backend.id != parsed[1].backend.id
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    if parsed.iter().any(|result| !result.document.output().ok) {
        return Err(NativeMergeError::NativeParseRejected { parses: parsed, sources });
    }
    let mut documents = Vec::new();
    let mut failures = Vec::new();
    for result in &parsed {
        let analysis = if let Some(analyzer) = analyze_with_diagnostics {
            analyzer(result).map_err(|error| (error.kind, error.message))
        } else {
            analyze(result).map_err(|message| {
                (crate::typed_merge::NativeAnalysisFailureKind::UnsupportedFeature, message)
            })
        }
        .and_then(|document| {
            if document.source.as_bytes() != result.source.bytes() {
                return Err((
                    crate::typed_merge::NativeAnalysisFailureKind::UnsupportedFeature,
                    "analysis source differs from validated source".into(),
                ));
            }
            document.validate("diff input").map_err(|message| {
                (crate::typed_merge::NativeAnalysisFailureKind::UnsupportedFeature, message)
            })?;
            Ok(document)
        });
        context.check().map_err(NativeMergeError::Parse)?;
        match analysis {
            Ok(document) => documents.push(document),
            Err((kind, message)) => failures.push(NativeAnalysisFailure {
                kind,
                source_id: result.source.descriptor().source_id.clone(),
                source_role: result.source.descriptor().role,
                message,
            }),
        }
    }
    if !failures.is_empty() {
        return Err(NativeMergeError::AnalysisRejected { failures, parses: parsed, sources });
    }
    let diff =
        diff_owner_documents(&parsed[0].source, &documents[0], &parsed[1].source, &documents[1])
            .map_err(NativeMergeError::Unsupported)?;
    context.check().map_err(NativeMergeError::Parse)?;
    Ok(NativeDiffExecution { diff, input_parses: parsed })
}
