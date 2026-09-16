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
    source::{SourceDescriptor, SourceEncoding, SourceMap, SourceRole, source_input},
};

#[derive(Debug)]
pub enum NativeMergeError {
    InvalidInputs,
    Parse(ServiceError),
    InputParseFailed {
        error: ServiceError,
        sources: Vec<SourceDescriptor>,
    },
    NativeParseRejected {
        parses: Vec<ParsedResult>,
        sources: Vec<SourceDescriptor>,
    },
    AnalysisRejected {
        failures: Vec<NativeAnalysisFailure>,
        parses: Vec<ParsedResult>,
        sources: Vec<SourceDescriptor>,
    },
    Unsupported(String),
}

#[derive(Debug)]
pub struct NativeAnalysisFailure {
    pub source_id: String,
    pub source_role: SourceRole,
    pub message: String,
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
        .map_err(|error| match error {
            // Preserve the older internal family entry point's error contract.
            NativeMergeError::AnalysisRejected { failures, .. } => NativeMergeError::Unsupported(
                failures.into_iter().map(|failure| failure.message).collect::<Vec<_>>().join("; "),
            ),
            NativeMergeError::InputParseFailed { error, .. } => NativeMergeError::Parse(error),
            error => error,
        })
}

pub struct NativeMergeExecution {
    pub rendered: SourcePreservingMergeEvidence,
    pub sources: Vec<SourceDescriptor>,
    pub output_source: Option<SourceDescriptor>,
    pub input_parses: Vec<ParsedResult>,
    pub output_parse: Option<ParsedResult>,
    pub verification_error: Option<ServiceError>,
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
    context.check().map_err(NativeMergeError::Parse)?;
    if requests.len() > context.max_batch_items {
        return Err(NativeMergeError::Parse(ServiceError::LimitExceeded));
    }
    // Retain only independently validated identities on a later service failure.
    // Reuse the operation source-map contract, not provider-returned descriptors.
    let mut sources = {
        let map = SourceMap::validate(
            requests.iter().map(|request| request.source.clone()).collect(),
            context.max_input_bytes,
        )
        .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))?;
        requests
            .iter()
            .map(|request| {
                map.get(&request.source.descriptor.source_id)
                    .map(|source| source.descriptor().clone())
                    .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    sources.sort_by_key(|source| source.role);
    let mut parsed = match service.parse_batch(requests, snapshot, context) {
        Ok(parsed) => parsed,
        Err(error) => return Err(NativeMergeError::InputParseFailed { error, sources }),
    };
    let backend = parsed.first().ok_or(NativeMergeError::InvalidInputs)?.backend.id.clone();
    if parsed.iter().any(|result| result.backend.id != backend) {
        return Err(NativeMergeError::InvalidInputs);
    }
    verification.selection.backend_id = Some(backend);
    // Failure identity must not depend on request order, and a family-analysis
    // rejection must not hide native syntax errors in another revision.
    parsed.sort_by_key(|result| result.source.descriptor().role);
    if parsed.iter().any(|result| !result.document.output().ok) {
        return Err(NativeMergeError::NativeParseRejected { parses: parsed, sources });
    }
    let mut documents = BTreeMap::new();
    let mut failures = Vec::new();
    for result in &parsed {
        match analyze(result) {
            Ok(document) => {
                let checked = if document.source.as_bytes() != result.source.bytes() {
                    Err("family analysis changed the validated source bytes".into())
                } else {
                    document.validate("input")
                };
                match checked {
                    Ok(()) => {
                        documents.insert(result.source.descriptor().role, document);
                    }
                    Err(message) => failures.push(NativeAnalysisFailure {
                        source_id: result.source.descriptor().source_id.clone(),
                        source_role: result.source.descriptor().role,
                        message,
                    }),
                }
            }
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
    let mut output_id = "merge-output".to_owned();
    while sources.iter().any(|source| source.source_id == output_id) {
        output_id.push('_');
    }
    let mut output_parse = None;
    let mut verification_error = None;
    let mut verify = |output: &str| -> Result<SourcePreservingOwnerDocument, String> {
        verification.request_id = "merge-verification".into();
        verification.source = source_input(
            output_id.clone(),
            SourceRole::Output,
            SourceEncoding::Utf8,
            output.as_bytes().to_vec(),
        )
        .map_err(|error| error.to_string())?;
        let mut parsed = match service.parse_batch(vec![verification.clone()], snapshot, context) {
            Ok(parsed) => parsed,
            Err(error) => {
                let message = format!("{error:?}");
                verification_error = Some(error);
                return Err(message);
            }
        };
        if parsed.len() != 1 {
            return Err("expected one verification parse".into());
        }
        let parsed = parsed.remove(0);
        let analysis = if parsed.document.output().ok {
            analyze(&parsed)
        } else {
            Err("native parser rejected rendered output".into())
        };
        output_parse = Some(parsed);
        analysis
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
    Ok(NativeMergeExecution {
        rendered,
        sources,
        output_source,
        input_parses: parsed,
        output_parse,
        verification_error,
    })
}
