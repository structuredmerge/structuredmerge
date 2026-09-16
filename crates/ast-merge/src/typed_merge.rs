//! Shared orchestration over validated native syntax facts. Family analysis
//! supplies owners; hosts cannot supply matching, conflict, or render decisions.
use crate::{SourcePreservingOwnerDocument, ThreeWayMergeResult, merge_source_preserving_owners};
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{
    service::{
        ExecutionContext, ParseRequest, ParseService, ParsedResult, ParserRegistrySnapshot,
        ServiceError,
    },
    source::{SourceEncoding, SourceRole, source_input},
};

#[derive(Debug)]
pub enum NativeMergeError {
    InvalidInputs,
    Parse(ServiceError),
    NativeParseRejected(Box<ParsedResult>),
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
    let roles: BTreeSet<_> =
        requests.iter().map(|request| request.source.descriptor.role).collect();
    if requests.len() != 3
        || roles != BTreeSet::from([SourceRole::Base, SourceRole::Ours, SourceRole::Theirs])
        || requests.iter().any(|request| request.language != language)
    {
        return Err(NativeMergeError::InvalidInputs);
    }
    let mut verification = requests[0].clone();
    let parsed =
        service.parse_batch(requests, snapshot, context).map_err(NativeMergeError::Parse)?;
    let backend = parsed.first().ok_or(NativeMergeError::InvalidInputs)?.backend.id.clone();
    if parsed.iter().any(|result| result.backend.id != backend) {
        return Err(NativeMergeError::InvalidInputs);
    }
    verification.selection.backend_id = Some(backend);
    let mut documents = BTreeMap::new();
    for result in parsed {
        if !result.document.output().ok {
            return Err(NativeMergeError::NativeParseRejected(Box::new(result)));
        }
        documents.insert(
            result.source.descriptor().role,
            analyze(&result).map_err(NativeMergeError::Unsupported)?,
        );
    }
    let mut verify = |output: &str| -> Result<SourcePreservingOwnerDocument, String> {
        verification.request_id = "merge-verification".into();
        verification.source = source_input(
            "merge-output".into(),
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
    let result = merge_source_preserving_owners(
        documents.remove(&SourceRole::Base).ok_or(NativeMergeError::InvalidInputs)?,
        documents.remove(&SourceRole::Ours).ok_or(NativeMergeError::InvalidInputs)?,
        documents.remove(&SourceRole::Theirs).ok_or(NativeMergeError::InvalidInputs)?,
        &mut verify,
    );
    context.check().map_err(NativeMergeError::Parse)?;
    Ok(result)
}
