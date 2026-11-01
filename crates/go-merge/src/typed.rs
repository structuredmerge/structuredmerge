//! Conservative Go declaration ownership over validated Tree-sitter facts.
//! Package clauses, imports, and supported declarations are whole owners.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument,
    native_analysis::NativeOwnerAnalysis,
};
use tree_haver::service::ParsedResult;

pub fn validate_package_clause(parsed: &ParsedResult) -> Result<(), String> {
    if !parsed.backend.languages.iter().any(|language| language == "go") {
        return Err("Go analysis requires a Go parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    super::validate_go_package_clause(&index, root)
}

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    let analysis = project(parsed)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    validate_native_source_ranges(source, &analysis.document)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    project(parsed).map(|analysis| analysis.document)
}

fn project(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    validate_package_clause(parsed)?;
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis = super::project_go_owners(source, root, &nodes)?;
    analysis.validate(parsed)?;
    Ok(analysis)
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
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: owners,
            merge: merge_documents,
            postprocess: None,
        },
        parse_output,
    )
}

fn validate_native_source_ranges(
    source: &str,
    document: &SourcePreservingOwnerDocument,
) -> Result<(), String> {
    let bytes = source.as_bytes();
    let horizontal_whitespace =
        |slice: &[u8]| slice.iter().all(|byte| matches!(byte, b'\t' | b'\r' | b' '));
    for (index, owner) in document.owners.iter().enumerate() {
        let start = owner.start_byte;
        let end = owner.end_byte;
        if start > end || end > bytes.len() {
            return Err(format!("Go owner {:?} has an invalid source range", owner.id));
        }
        let line_start =
            bytes[..start].iter().rposition(|byte| *byte == b'\n').map_or(0, |i| i + 1);
        let line_end =
            bytes[end..].iter().position(|byte| *byte == b'\n').map_or(bytes.len(), |i| end + i);
        if !horizontal_whitespace(&bytes[line_start..start])
            || !horizontal_whitespace(&bytes[end..line_end])
            || (index > 0 && document.owners[index - 1].end_line >= owner.start_line)
        {
            return Err(format!("Go owner {:?} has an unsafe source range", owner.id));
        }
    }
    Ok(())
}
