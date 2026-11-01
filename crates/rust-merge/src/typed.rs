//! Existing conservative Rust declaration ownership over validated TreeHaver facts.
//! Use declarations are whole owners; nested owner/member semantics stay out of scope.
use ast_merge::{
    LayoutAugmentation, LayoutOwner, NormalizedCommentEvidence, SourceLineDisposition,
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument, augment_layout,
    augment_normalized_comments_with_evidence_filter, native_analysis::NativeOwnerAnalysis,
    source_line_dispositions_from_augmentations,
};
use std::collections::HashMap;
use tree_haver::service::ParsedResult;

#[derive(Clone, Debug, PartialEq)]
pub struct RustSourceLineAnalysis {
    pub owners: NativeOwnerAnalysis,
    pub comments: NormalizedCommentEvidence,
    pub layout: LayoutAugmentation,
    pub dispositions: Vec<SourceLineDisposition>,
}

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    if !parsed.backend.languages.iter().any(|language| language == "rust") {
        return Err("Rust analysis requires a Rust parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Rust parse omitted its root")?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root_node = index.root(root)?;
    for node in index.children(root_node) {
        if node.kind == "attribute_item" {
            let attribute =
                parsed.source.slice(node.span.range.clone()).map_err(|error| error.to_string())?;
            let attribute = std::str::from_utf8(attribute).map_err(|error| error.to_string())?;
            if attribute.trim() == "#[test]" || attribute.trim_start().starts_with("#[test(") {
                return Err(
                    "Rust test-selector attributes are unsupported by this owner profile".into()
                );
            }
        }
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis =
        ast_merge::project_named_top_level_analysis(source, root, &nodes, super::owner_policy())?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

/// Compose Rust owners, comments, gaps, and line dispositions using the shared
/// ast-merge rubric. The provider contributes syntax/comment facts only.
pub fn source_line_analysis(parsed: &ParsedResult) -> Result<RustSourceLineAnalysis, String> {
    let owners = analysis(parsed)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let nodes = parsed.normalized_nodes()?;
    let mut tree_to_semantic = HashMap::<String, String>::new();
    let mut native_owners = Vec::with_capacity(owners.document.owners.len());
    let mut semantic_owners = Vec::with_capacity(owners.document.owners.len());
    for owner in &owners.document.owners {
        let node_ids = owners
            .owner_node_ids
            .get(&owner.id)
            .ok_or_else(|| format!("Rust owner {} has no native node references", owner.id))?;
        let mut structural_nodes = Vec::new();
        for node_id in node_ids {
            let node = nodes.iter().find(|node| node.id == *node_id).ok_or_else(|| {
                format!("Rust owner {} references missing native node {node_id}", owner.id)
            })?;
            if node.role == tree_haver::NodeRole::Comment {
                continue;
            }
            if tree_to_semantic.insert(node_id.clone(), owner.id.clone()).is_some() {
                return Err(format!("native Rust node {node_id} maps to multiple owners"));
            }
            structural_nodes.push(node);
        }
        if structural_nodes.is_empty() {
            return Err(format!("Rust owner {} has no structural native nodes", owner.id));
        }
        let start_line = structural_nodes
            .iter()
            .map(|node| node.span.start_point.row + 1)
            .min()
            .ok_or_else(|| format!("Rust owner {} has no starting line", owner.id))?;
        let end_line = structural_nodes
            .iter()
            .map(|node| node.span.end_point.row + 1)
            .max()
            .ok_or_else(|| format!("Rust owner {} has no ending line", owner.id))?;
        let primary_node_id = node_ids
            .last()
            .ok_or_else(|| format!("Rust owner {} has no primary node", owner.id))?;
        native_owners.push(LayoutOwner { owner_id: primary_node_id.clone(), start_line, end_line });
        semantic_owners.push(LayoutOwner { owner_id: owner.id.clone(), start_line, end_line });
    }
    let comments = augment_normalized_comments_with_evidence_filter(
        source,
        &native_owners,
        &nodes,
        "rust_comment",
        super::owner_policy().comment_attachment_policy,
        |node| matches!(node.kind.as_str(), "line_comment" | "block_comment"),
        normalize_rust_comment,
    )?;
    let mut comments = comments;
    for attachment in &mut comments.augmentation.attachments {
        attachment.owner_id = semantic_owner_id(&attachment.owner_id, &tree_to_semantic)?;
    }
    for region in &mut comments.augmentation.regions {
        if !region.owner_id.starts_with("document:") {
            region.owner_id = semantic_owner_id(&region.owner_id, &tree_to_semantic)?;
        }
    }
    for gap in &mut comments.augmentation.gaps {
        remap_optional_owner(&mut gap.before_owner_id, &tree_to_semantic)?;
        remap_optional_owner(&mut gap.after_owner_id, &tree_to_semantic)?;
    }
    let mut layout = augment_layout(&source_lines(source), &native_owners)?;
    for gap in &mut layout.gaps {
        remap_optional_owner(&mut gap.before_owner_id, &tree_to_semantic)?;
        remap_optional_owner(&mut gap.after_owner_id, &tree_to_semantic)?;
    }
    for attachment in &mut layout.attachments {
        attachment.owner_id = semantic_owner_id(&attachment.owner_id, &tree_to_semantic)?;
    }
    let dispositions = source_line_dispositions_from_augmentations(
        source,
        &semantic_owners,
        &comments.augmentation,
        &layout,
    )?;
    Ok(RustSourceLineAnalysis { owners, comments, layout, dispositions })
}

fn semantic_owner_id(id: &str, mapping: &HashMap<String, String>) -> Result<String, String> {
    mapping
        .get(id)
        .cloned()
        .or_else(|| mapping.values().any(|value| value == id).then(|| id.to_string()))
        .ok_or_else(|| format!("line fact refers to unknown native Rust owner {id}"))
}

fn remap_optional_owner(
    id: &mut Option<String>,
    mapping: &HashMap<String, String>,
) -> Result<(), String> {
    if let Some(value) = id {
        *value = semantic_owner_id(value, mapping)?;
    }
    Ok(())
}

fn source_lines(source: &str) -> Vec<String> {
    let mut lines = source
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect::<Vec<_>>();
    if source.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn normalize_rust_comment(text: &str) -> String {
    let text = text.trim();
    if let Some(body) = text.strip_prefix("///").or_else(|| text.strip_prefix("//!")) {
        body.trim_start().to_string()
    } else if let Some(body) = text.strip_prefix("//") {
        body.trim_start().to_string()
    } else if let Some(body) = text
        .strip_prefix("/**")
        .or_else(|| text.strip_prefix("/*!"))
        .or_else(|| text.strip_prefix("/*"))
    {
        let content = body.strip_suffix("*/").unwrap_or(body).trim();
        content.strip_prefix('*').unwrap_or(content).trim_start().to_string()
    } else {
        text.to_string()
    }
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
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: owners,
            merge: merge_documents,
        },
        parse_output,
    )
}
