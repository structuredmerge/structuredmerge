//! Existing conservative Rust declaration ownership over validated TreeHaver facts.
//! Use-declaration bytes remain unowned layout, not newly supported merge owners.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument,
    native_analysis::NativeOwnerAnalysis,
};
use tree_haver::service::ParsedResult;

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    if !parsed.backend.languages.iter().any(|language| language == "rust") {
        return Err("Rust analysis requires a Rust parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Rust parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis =
        ast_merge::project_named_top_level_analysis(source, root, &nodes, super::owner_policy())?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
}

/// Preserve the family's conservative boundary independently of transport.
pub fn membership_conflict(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> Option<ast_merge::ThreeWayMergeResult<String>> {
    super::rust_membership_change_with_owner_edit(base, ours, theirs)
        .then(super::conservative_membership_conflict)
}

pub fn merge_documents(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: &mut dyn FnMut(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> SourcePreservingMergeEvidence {
    if let Some(result) = membership_conflict(&base, &ours, &theirs) {
        return SourcePreservingMergeEvidence {
            result,
            source_segments: vec![],
            classification: None,
        };
    }
    ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
}

pub type RustMergeExecution = ast_merge::typed_merge::ParsedOwnerMergeExecution;

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<RustMergeExecution, String> {
    ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine { analyze: owners, merge: merge_documents },
        parse_output,
    )
}
