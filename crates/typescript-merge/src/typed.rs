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

/// Generic owner merge; TypeScript has no family-specific guard before line-selection recovery.
pub fn merge_documents(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: &mut dyn FnMut(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> ast_merge::SourcePreservingMergeEvidence {
    ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
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
            postprocess: Some(line_selection_postprocessor),
        },
        parse_output,
    )
}

/// Line-level ownership facts for one validated TypeScript/TSX parse.
pub fn source_line_analysis(
    parsed: &ParsedResult,
) -> Result<ast_merge::line_selection::LineSelectionAnalysis, String> {
    use ast_merge::{
        LayoutOwner, augment_layout, augment_normalized_comments_with_evidence_scope,
        line_selection::{remap_comment_owner_ids, remap_layout_owner_ids, source_line_contents},
        source_line_dispositions_from_augmentations,
    };
    use std::collections::HashMap;
    let owners = project_analysis(parsed, true)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let nodes = parsed.normalized_nodes()?;
    let root_id =
        parsed.document.output().root_id.as_deref().ok_or("TypeScript parse omitted its root")?;
    let mut tree_to_semantic = HashMap::<String, String>::new();
    let mut native_owners = Vec::with_capacity(owners.document.owners.len());
    let mut semantic_owners = Vec::with_capacity(owners.document.owners.len());
    for owner in &owners.document.owners {
        let node_ids = owners.owner_node_ids.get(&owner.id).ok_or_else(|| {
            format!("TypeScript owner {} has no native node references", owner.id)
        })?;
        let mut structural = Vec::new();
        for node_id in node_ids {
            let node = nodes.iter().find(|node| node.id == *node_id).ok_or_else(|| {
                format!("TypeScript owner {} references missing native node {node_id}", owner.id)
            })?;
            if node.role == tree_haver::NodeRole::Comment {
                continue;
            }
            if tree_to_semantic.insert(node_id.clone(), owner.id.clone()).is_some() {
                return Err(format!("native TypeScript node {node_id} maps to multiple owners"));
            }
            structural.push(node);
        }
        let start_line = structural
            .iter()
            .map(|node| node.span.start_point.row + 1)
            .min()
            .ok_or_else(|| format!("TypeScript owner {} has no structural nodes", owner.id))?;
        let end_line = structural
            .iter()
            .map(|node| node.span.end_point.row + 1)
            .max()
            .ok_or_else(|| format!("TypeScript owner {} has no ending line", owner.id))?;
        let primary = node_ids
            .last()
            .ok_or_else(|| format!("TypeScript owner {} has no primary node", owner.id))?;
        native_owners.push(LayoutOwner { owner_id: primary.clone(), start_line, end_line });
        semantic_owners.push(LayoutOwner { owner_id: owner.id.clone(), start_line, end_line });
    }
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let top_level = index.children(index.root(root_id)?);
    let mut comments = augment_normalized_comments_with_evidence_scope(
        source,
        &native_owners,
        &nodes,
        "ts_comment",
        super::owner_policy().comment_attachment_policy,
        |node| {
            if node.role != tree_haver::NodeRole::Comment {
                return None;
            }
            Some(comment_scope(node, &top_level))
        },
        normalize_typescript_comment,
    )?;
    remap_comment_owner_ids(&mut comments.augmentation, &tree_to_semantic)?;
    let lines = source_line_contents(source);
    let mut layout = augment_layout(&lines, &native_owners)?;
    remap_layout_owner_ids(&mut layout, &tree_to_semantic)?;
    let dispositions = source_line_dispositions_from_augmentations(
        source,
        &semantic_owners,
        &comments.augmentation,
        &layout,
    )?;
    Ok(ast_merge::line_selection::LineSelectionAnalysis { owners, comments, dispositions })
}

fn comment_scope(
    node: &tree_haver::NormalizedTreeNode,
    top_level: &[&tree_haver::NormalizedTreeNode],
) -> CommentScope {
    let text = node.source_fragment.trim_start();
    let structural_doc = text.starts_with("///") || text.starts_with("/**");
    let precedes_first_owner = top_level
        .iter()
        .filter(|candidate| candidate.span.range.end_byte <= node.span.range.start_byte)
        .all(|candidate| {
            candidate.role == tree_haver::NodeRole::Comment || candidate.kind == "import_statement"
        });
    if precedes_first_owner && !structural_doc { CommentScope::Root } else { CommentScope::Owner }
}

fn normalize_typescript_comment(text: &str) -> String {
    let text = text.trim();
    let body = if let Some(rest) = text.strip_prefix("/*") {
        rest.strip_suffix("*/").unwrap_or(rest).trim_start_matches('*')
    } else {
        text.trim_start_matches('/')
    };
    body.trim().to_string()
}

/// Recover a merge the generic engine refused only because deleting or adding an owner changed
/// inter-owner layout, by selecting whole physical source lines per owner decision. The result
/// must reparse with the same parser, and its owners must be exactly the surviving owners.
pub fn line_selection_postprocessor(
    inputs: &[ParsedResult],
    evidence: &mut ast_merge::SourcePreservingMergeEvidence,
    parse_output: &mut dyn FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<bool, String> {
    use ast_merge::line_selection::is_recoverable_owner_layout_error;
    use tree_haver::source::SourceRole;

    if inputs.len() != 3 || !is_recoverable_owner_layout_error(evidence) {
        return Ok(false);
    }
    let find = |role| inputs.iter().find(|parsed| parsed.source.descriptor().role == role);
    let (Some(base), Some(ours), Some(theirs)) =
        (find(SourceRole::Base), find(SourceRole::Ours), find(SourceRole::Theirs))
    else {
        return Ok(false);
    };
    let (Ok(base_lines), Ok(ours_lines), Ok(theirs_lines)) =
        (source_line_analysis(base), source_line_analysis(ours), source_line_analysis(theirs))
    else {
        return Ok(false);
    };
    let analyses = [base_lines, ours_lines, theirs_lines];
    Ok(ast_merge::line_selection::recover_with_line_selection(
        inputs,
        &analyses,
        evidence,
        parse_output,
        &|parsed| {
            project_analysis(parsed, true).map(|analysis| {
                analysis.document.owners.iter().map(|owner| owner.id.clone()).collect()
            })
        },
    ))
}
