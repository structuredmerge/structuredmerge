//! Existing TypeScript/TSX declaration ownership over validated native facts.
//! Import/comment bytes remain layout; wrapper spans remain whole source owners.
use ast_merge::{
    CommentScope, SourcePreservingOwnerDocument, native_analysis::NativeOwnerAnalysis,
};
use tree_haver::service::ParsedResult;

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    project_analysis(parsed, false)
}

/// Merge3 may treat one top-level multi-declarator statement as an atomic owner.
/// Analyze, diff, and directional merge retain their existing per-operation
/// support boundary.
pub fn merge3_owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    project_analysis(parsed, true).map(|analysis| analysis.document)
}

fn project_analysis(
    parsed: &ParsedResult,
    include_grouped_lexical_owners: bool,
) -> Result<NativeOwnerAnalysis, String> {
    if !parsed
        .backend
        .languages
        .iter()
        .any(|language| matches!(language.as_str(), "typescript" | "tsx"))
    {
        return Err("TypeScript analysis requires a TypeScript or TSX parser".into());
    }
    let root =
        parsed.document.output().root_id.as_deref().ok_or("TypeScript parse omitted its root")?;
    let mut nodes = parsed.normalized_nodes()?;
    super::annotate_lexical_declaration_owner_identities(
        &mut nodes,
        root,
        include_grouped_lexical_owners,
    )?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root_node = index.root(root)?;
    let top_level = index.children(root_node);
    let analysis = ast_merge::project_named_top_level_analysis_with_comment_scope(
        source,
        root,
        &nodes,
        super::owner_policy(),
        |node| {
            let text = node.source_fragment.trim_start();
            let structural_doc = text.starts_with("///") || text.starts_with("/**");
            let precedes_first_owner = top_level
                .iter()
                .filter(|candidate| candidate.span.range.end_byte <= node.span.range.start_byte)
                .all(|candidate| {
                    candidate.role == tree_haver::NodeRole::Comment
                        || candidate.kind == "import_statement"
                });
            if precedes_first_owner && !structural_doc {
                CommentScope::Root
            } else {
                CommentScope::Owner
            }
        },
    )?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
}

pub fn directional_owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    let nodes = parsed.normalized_nodes()?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root = index
        .root(parsed.document.output().root_id.as_deref().ok_or("missing TypeScript root")?)?;
    for node in index.children(root).into_iter().filter(|node| node.kind == "lexical_declaration") {
        let declarator_count = index
            .children(node)
            .into_iter()
            .filter(|child| child.kind == "variable_declarator")
            .count();
        if declarator_count > 1 {
            return Err(
                "TypeScript directional merge does not support multi-declarator statements".into(),
            );
        }
    }
    owners(parsed)
}

pub type TypeScriptMergeExecution = ast_merge::typed_merge::ParsedOwnerMergeExecution;

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<TypeScriptMergeExecution, String> {
    // TypeScript uses the generic owner engine, unlike the Go/Rust family guards.
    ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: merge3_owners,
            merge: |base, ours, theirs, verify| {
                ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
            },
            postprocess: None,
        },
        parse_output,
    )
}
