//! Conservative Go declaration ownership over validated Tree-sitter facts.
//! Package clauses and supported declarations are owners; import bytes remain unowned layout.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument,
    native_analysis::NativeOwnerAnalysis,
};
use tree_haver::service::ParsedResult;

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    if !parsed.backend.languages.iter().any(|language| language == "go") {
        return Err("Go analysis requires a Go parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis = super::project_go_owners(source, root, &nodes)?;
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
    super::go_membership_change_with_owner_edit(base, ours, theirs)
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

pub type GoMergeExecution = ast_merge::typed_merge::ParsedOwnerMergeExecution;

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<GoMergeExecution, String> {
    ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine { analyze: owners, merge: merge_documents },
        parse_output,
    )
}
