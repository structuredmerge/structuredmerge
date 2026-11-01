//! Rust declaration ownership uses native syn item identities and spans when that
//! provider is selected, then applies shared comment and line ownership rules.
//! Nested owner/member semantics stay out of scope.
use ast_merge::{
    CommentScope, LayoutAugmentation, LayoutOwner, NormalizedCommentEvidence,
    SourceLineDisposition, SourcePreservingMergeEvidence, SourcePreservingOwnerDocument,
    augment_layout, augment_normalized_comments_with_evidence_scope,
    native_analysis::NativeOwnerAnalysis, source_line_dispositions_from_augmentations,
};
use std::collections::{HashMap, HashSet};
use tree_haver::service::ParsedResult;

#[derive(Clone, Debug, PartialEq)]
pub struct RustSourceLineAnalysis {
    pub owners: NativeOwnerAnalysis,
    pub comments: NormalizedCommentEvidence,
    pub layout: LayoutAugmentation,
    pub dispositions: Vec<SourceLineDisposition>,
}

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    analysis_with_diagnostics(parsed).map_err(|error| error.message)
}

pub fn analysis_with_diagnostics(
    parsed: &ParsedResult,
) -> Result<NativeOwnerAnalysis, ast_merge::typed_merge::NativeAnalysisError> {
    if !parsed.backend.languages.iter().any(|language| language == "rust") {
        return Err("Rust analysis requires a Rust parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Rust parse omitted its root")?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root_node = index.root(root)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let parser_is_syn = parsed.backend.parser == "syn";
    if index.children(root_node).iter().all(|node| node.role == tree_haver::NodeRole::Comment) {
        let empty_analysis = NativeOwnerAnalysis {
            document: SourcePreservingOwnerDocument { source: source.to_string(), owners: vec![] },
            owner_node_ids: Default::default(),
            attached_comment_node_ids: Default::default(),
        };
        empty_analysis.validate(parsed)?;
        return Ok(empty_analysis);
    }
    let mut analysis = ast_merge::project_named_top_level_analysis_with_classified_errors(
        source,
        root,
        &nodes,
        if parser_is_syn { super::owner_policy_for_syn() } else { super::owner_policy() },
        |node| {
            if parser_is_syn && node.semantic_roles.iter().any(|role| role == "inner_doc_comment") {
                CommentScope::Root
            } else {
                CommentScope::Owner
            }
        },
    )
    .map_err(|error| match error {
        ast_merge::NamedOwnerProjectionError::Unsupported(message) => {
            ast_merge::typed_merge::NativeAnalysisError::unsupported(message)
        }
        ast_merge::NamedOwnerProjectionError::AmbiguousOwner(message) => {
            ast_merge::typed_merge::NativeAnalysisError::ambiguous_owner(message)
        }
    })?;
    // A native `syn` provider already supplies authoritative Rust item spans.
    // Keep its projected owners and comment nodes as the source of truth instead
    // of reparsing through Tree-sitter as an acceptance gate. The legacy
    // Tree-sitter provider still gets the native identity cross-check.
    if parsed.backend.parser != "syn" {
        let native_document =
            super::parse_source_preserving_rust_native_with_treesitter_comments(source)?;
        let projected_ids =
            analysis.document.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>();
        let native_ids =
            native_document.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>();
        if projected_ids != native_ids {
            return Err("syn and TreeHaver disagree on supported Rust owner identity/order".into());
        }
        analysis.document = native_document;
    }
    analysis.validate(parsed)?;
    Ok(analysis)
}

/// Compose Rust owners, comments, gaps, and line dispositions using the shared
/// ast-merge rubric. The provider contributes syntax/comment facts only.
pub fn source_line_analysis(parsed: &ParsedResult) -> Result<RustSourceLineAnalysis, String> {
    let owners = analysis(parsed)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let nodes = parsed.normalized_nodes()?;
    let parser_is_syn = parsed.backend.parser == "syn";
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
        let mut start_line = structural_nodes
            .iter()
            .map(|node| node.span.start_point.row + 1)
            .min()
            .ok_or_else(|| format!("Rust owner {} has no starting line", owner.id))?;
        // syn item spans include outer doc comments because Rust parses them as
        // doc attributes. Those lines are also emitted as comment facts, so the
        // structural line range starts after the leading doc-comment run.
        let source_line_text = source_lines(source);
        loop {
            while source_line_text
                .get(start_line.saturating_sub(1))
                .is_some_and(|line| line.trim().is_empty())
            {
                start_line += 1;
            }
            let leading_doc_end = nodes
                .iter()
                .filter(|node| {
                    node.role == tree_haver::NodeRole::Comment
                        && owner.start_byte <= node.span.range.start_byte
                        && node.span.range.end_byte <= owner.end_byte
                })
                .filter_map(|node| {
                    if node.span.start_point.row + 1 != start_line {
                        return None;
                    }
                    let text = node.source_fragment.trim_start();
                    let outer_doc = if parser_is_syn {
                        node.semantic_roles.iter().any(|role| role == "doc_comment")
                    } else {
                        // Tree-sitter does not expose Rust's doc-attribute semantics
                        // consistently, so keep this parser path's lexical projection
                        // separate from the native syn facts.
                        (text.starts_with("///") && !text.starts_with("////"))
                            || (text.starts_with("/**") && !text.starts_with("/***"))
                    };
                    outer_doc.then_some(node.span.end_point.row + 1)
                })
                .max();
            match leading_doc_end {
                Some(end_line) => start_line = end_line + 1,
                None => break,
            }
        }
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
    let comments = augment_normalized_comments_with_evidence_scope(
        source,
        &native_owners,
        &nodes,
        "rust_comment",
        super::owner_policy().comment_attachment_policy,
        |node| {
            matches!(node.kind.as_str(), "line_comment" | "block_comment").then(|| {
                if node.source_fragment.trim_start().starts_with("//!")
                    || node.source_fragment.trim_start().starts_with("/*!")
                {
                    CommentScope::Root
                } else {
                    CommentScope::Owner
                }
            })
        },
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

#[derive(Clone, Debug)]
pub struct RustMergeExecution {
    pub evidence: SourcePreservingMergeEvidence,
    /// True when shared ordered source-line selection reproduced the verified output.
    pub ordered_line_render_applied: bool,
    pub output_parse: Option<ParsedResult>,
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<RustMergeExecution, String> {
    let execution = ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: owners,
            merge: merge_documents,
            postprocess: Some(line_selection_postprocessor),
        },
        parse_output,
    )?;
    Ok(RustMergeExecution {
        evidence: execution.evidence,
        ordered_line_render_applied: execution.postprocess_applied,
        output_parse: execution.output_parse,
    })
}

/// Select provider line ownership after common classification, then reparse any
/// recovered render through the parser service supplied by the common engine.
pub fn line_selection_postprocessor(
    inputs: &[ParsedResult],
    evidence: &mut SourcePreservingMergeEvidence,
    parse_output: &mut dyn FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<bool, String> {
    if inputs.len() != 3 {
        return Err("Rust line selection requires base, ours, and theirs parses".into());
    }
    let input_for = |role| {
        inputs
            .iter()
            .find(|parsed| parsed.source.descriptor().role == role)
            .ok_or_else(|| format!("Rust line selection omitted {role:?} parse"))
    };
    let base = input_for(tree_haver::source::SourceRole::Base)?;
    let ours = input_for(tree_haver::source::SourceRole::Ours)?;
    let theirs = input_for(tree_haver::source::SourceRole::Theirs)?;
    if evidence.result.outcome != ast_merge::ThreeWayMergeOutcome::Clean
        && !is_recoverable_owner_layout_error(evidence)
    {
        return Ok(false);
    }
    let analyses =
        [source_line_analysis(base)?, source_line_analysis(ours)?, source_line_analysis(theirs)?];
    let order_error = evidence.result.diagnostics.iter().any(|diagnostic| {
        diagnostic.message
            == "source-preserving declaration merge cannot prove an agreed owner order"
    });
    let merged_order = compatible_stable_owner_order(&analyses);
    if order_error && merged_order.is_none() && one_sided_owner_order_revision(&analyses).is_none()
    {
        record_divergent_owner_order_conflict(&analyses, evidence);
        return Ok(true);
    }
    let rendered = match render_line_selected_output(&analyses, evidence, merged_order.as_deref()) {
        Ok(Some(rendered)) => rendered,
        Ok(None) => return Ok(false),
        Err(message) if message == "both tips changed the same document preamble differently" => {
            let path = "<crate-preamble>".to_string();
            let conflict = ast_merge::MergeConflict {
                conflict_id: "rust-crate-preamble-divergence".to_string(),
                category: "unmanaged_source_change".to_string(),
                path: path.clone(),
                fallback_scope: "full_file".to_string(),
                message: "both tips changed the same document preamble differently".to_string(),
                alternatives: vec![],
            };
            evidence.source_segments.clear();
            evidence.classification = None;
            evidence.result = ast_merge::ThreeWayMergeResult {
                outcome: ast_merge::ThreeWayMergeOutcome::Conflict,
                diagnostics: vec![ast_merge::Diagnostic {
                    severity: ast_merge::DiagnosticSeverity::Error,
                    category: ast_merge::DiagnosticCategory::MergeConflict,
                    message: conflict.message.clone(),
                    path: Some(path.clone()),
                    review: None,
                }],
                conflicts: vec![conflict],
                output: None,
                policies: vec![],
            };
            return Ok(true);
        }
        Err(message) => return Err(message),
    };
    if evidence.result.outcome == ast_merge::ThreeWayMergeOutcome::Clean {
        let previous = evidence
            .result
            .output
            .as_deref()
            .ok_or_else(|| "clean Rust owner merge omitted output".to_string())?;
        if rendered.content != previous {
            return Err("Rust source-line render differs from the verified owner merge".into());
        }
        evidence.result.output = Some(rendered.content);
        return Ok(true);
    }

    let source_texts = HashMap::from([
        (ast_merge::SourceRevision::Base, analyses[0].owners.document.source.clone()),
        (ast_merge::SourceRevision::Ours, analyses[1].owners.document.source.clone()),
        (ast_merge::SourceRevision::Theirs, analyses[2].owners.document.source.clone()),
    ]);
    let Some(segments) = source_segments_from_line_render(&rendered, &source_texts)? else {
        return Ok(false);
    };
    let Ok(parsed) = parse_output(&rendered.content) else {
        return Ok(false);
    };
    let input_ids = inputs
        .iter()
        .map(|input| input.source.descriptor().source_id.clone())
        .collect::<HashSet<_>>();
    if parsed.source.descriptor().role != tree_haver::source::SourceRole::Output
        || input_ids.contains(&parsed.source.descriptor().source_id)
        || parsed.source.bytes() != rendered.content.as_bytes()
        || parsed.backend != ours.backend
        || analysis(&parsed).is_err()
    {
        return Ok(false);
    }
    evidence.source_segments = segments;
    evidence.result.outcome = ast_merge::ThreeWayMergeOutcome::Clean;
    evidence.result.diagnostics.clear();
    evidence.result.conflicts.clear();
    evidence.result.output = Some(rendered.content);
    Ok(true)
}

fn record_divergent_owner_order_conflict(
    analyses: &[RustSourceLineAnalysis; 3],
    evidence: &mut SourcePreservingMergeEvidence,
) {
    let conflict_id = "rust-owner-order-divergence".to_string();
    let path = "<owner-order>".to_string();
    let message = "Rust tips impose incompatible pairwise owner orders.".to_string();
    let revisions = [
        ast_merge::SourceRevision::Base,
        ast_merge::SourceRevision::Ours,
        ast_merge::SourceRevision::Theirs,
    ];
    let alternatives = revisions
        .iter()
        .zip(analyses)
        .map(|(revision, analysis)| ast_merge::ConflictAlternative {
            revision: *revision,
            state: ast_merge::ConflictAlternativeState::Present,
            regions: analysis
                .owners
                .document
                .owners
                .iter()
                .map(|owner| ast_merge::OwnedSourceRegion {
                    node_id: owner.id.clone(),
                    region_kind: "structural_owner".to_string(),
                    start_byte: owner.start_byte,
                    end_byte: owner.end_byte,
                    start_line: owner.start_line,
                    end_line: owner.end_line,
                })
                .collect(),
        })
        .collect();
    let conflict = ast_merge::MergeConflict {
        conflict_id: conflict_id.clone(),
        category: "owner_order_conflict".to_string(),
        path: path.clone(),
        fallback_scope: "full_file".to_string(),
        message: message.clone(),
        alternatives,
    };
    evidence.result = ast_merge::ThreeWayMergeResult {
        outcome: ast_merge::ThreeWayMergeOutcome::Conflict,
        diagnostics: vec![ast_merge::Diagnostic {
            severity: ast_merge::DiagnosticSeverity::Error,
            category: ast_merge::DiagnosticCategory::MergeConflict,
            message,
            path: Some(path.clone()),
            review: None,
        }],
        conflicts: vec![conflict],
        output: None,
        policies: vec![],
    };
    let base_source = &analyses[0].owners.document.source;
    let ours_source = &analyses[1].owners.document.source;
    let theirs_source = &analyses[2].owners.document.source;
    evidence.source_segments.clear();
    evidence.classification = Some(ast_merge::OwnerMergeClassification {
        base_equals_ours: base_source == ours_source,
        base_equals_theirs: base_source == theirs_source,
        ours_equals_theirs: ours_source == theirs_source,
        whole_source_selection: None,
        decisions: vec![ast_merge::OwnerDecisionEvidence {
            id: "decision.rust-owner-order-divergence".to_string(),
            owner_id: "<owner-order>".to_string(),
            path: path.clone(),
            kind: ast_merge::OwnerDecisionKind::ConflictEditEdit,
            alternatives: evidence.result.conflicts[0].alternatives.clone(),
            conflict_id: Some(conflict_id),
        }],
    });
}

fn is_recoverable_owner_layout_error(evidence: &SourcePreservingMergeEvidence) -> bool {
    evidence.result.outcome == ast_merge::ThreeWayMergeOutcome::Error
        && evidence.result.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.message.as_str(),
                "source-preserving declaration merge cannot prove ownership of changed inter-owner layout"
                    | "source-preserving declaration merge cannot prove a combined owner membership layout"
                    | "source-preserving declaration merge cannot prove an agreed owner order"
            )
        })
        && evidence.classification.as_ref().is_some_and(|classification| {
            classification.decisions.iter().all(|decision| {
                matches!(
                    decision.kind,
                    ast_merge::OwnerDecisionKind::SelectOurs
                        | ast_merge::OwnerDecisionKind::SelectTheirs
                        | ast_merge::OwnerDecisionKind::Delete
                )
            })
        })
}

fn compatible_stable_owner_order(analyses: &[RustSourceLineAnalysis; 3]) -> Option<Vec<String>> {
    let orders: [Vec<String>; 3] = std::array::from_fn(|index| {
        analyses[index].owners.document.owners.iter().map(|owner| owner.id.clone()).collect()
    });
    if orders[0].is_empty()
        || orders[0].len() != orders[1].len()
        || orders[0].len() != orders[2].len()
    {
        return None;
    }
    let base_set = orders[0].iter().cloned().collect::<HashSet<_>>();
    if orders[1].iter().cloned().collect::<HashSet<_>>() != base_set
        || orders[2].iter().cloned().collect::<HashSet<_>>() != base_set
    {
        return None;
    }
    if orders[1] == orders[0] || orders[2] == orders[0] {
        return None;
    }
    let positions: [HashMap<String, usize>; 3] = std::array::from_fn(|index| {
        orders[index].iter().enumerate().map(|(position, id)| (id.clone(), position)).collect()
    });
    let mut outgoing = orders[0]
        .iter()
        .cloned()
        .map(|id| (id, HashSet::<String>::new()))
        .collect::<HashMap<_, _>>();
    let mut indegree = orders[0].iter().cloned().map(|id| (id, 0usize)).collect::<HashMap<_, _>>();
    for left_index in 0..orders[0].len() {
        for right_index in left_index + 1..orders[0].len() {
            let left = &orders[0][left_index];
            let right = &orders[0][right_index];
            let ours_before = positions[1][left] < positions[1][right];
            let theirs_before = positions[2][left] < positions[2][right];
            // Base order is `left` before `right`; any tip reversal wins this
            // pairwise relation. A cycle across pairs fails closed below.
            let merged_before = ours_before && theirs_before;
            let (before, after) = if merged_before { (left, right) } else { (right, left) };
            if outgoing.get_mut(before)?.insert(after.clone()) {
                *indegree.get_mut(after)? += 1;
            }
        }
    }
    let mut remaining = indegree;
    let mut merged = Vec::with_capacity(orders[0].len());
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .min_by_key(|id| positions[0][id]);
        let id = next?;
        remaining.remove(&id);
        merged.push(id.clone());
        for successor in outgoing.get(&id)? {
            if let Some(degree) = remaining.get_mut(successor) {
                *degree = degree.checked_sub(1)?;
            }
        }
    }
    Some(merged)
}

fn one_sided_owner_order_revision(
    analyses: &[RustSourceLineAnalysis; 3],
) -> Option<ast_merge::SourceRevision> {
    let base_ids = analyses[0]
        .owners
        .document
        .owners
        .iter()
        .map(|owner| owner.id.clone())
        .collect::<HashSet<_>>();
    let ours_ids = analyses[1]
        .owners
        .document
        .owners
        .iter()
        .map(|owner| owner.id.clone())
        .collect::<HashSet<_>>();
    let theirs_ids = analyses[2]
        .owners
        .document
        .owners
        .iter()
        .map(|owner| owner.id.clone())
        .collect::<HashSet<_>>();
    let stable_ids = base_ids
        .intersection(&ours_ids)
        .cloned()
        .filter(|id| theirs_ids.contains(id))
        .collect::<HashSet<_>>();
    if stable_ids.len() < 2 {
        return None;
    }
    let order = |analysis: &RustSourceLineAnalysis| {
        analysis
            .owners
            .document
            .owners
            .iter()
            .map(|owner| owner.id.clone())
            .filter(|id| stable_ids.contains(id))
            .collect::<Vec<_>>()
    };
    let base_order = order(&analyses[0]);
    let ours_order = order(&analyses[1]);
    let theirs_order = order(&analyses[2]);
    if ours_order == base_order && theirs_order != base_order {
        Some(ast_merge::SourceRevision::Theirs)
    } else if theirs_order == base_order && ours_order != base_order {
        Some(ast_merge::SourceRevision::Ours)
    } else {
        None
    }
}

/// Build a shared ordered source-line render from executed Rust owner decisions.
/// Additions follow selected tip order; deletions use shared structural rehome plans.
fn render_line_selected_output(
    analyses: &[RustSourceLineAnalysis; 3],
    evidence: &SourcePreservingMergeEvidence,
    merged_owner_order: Option<&[String]>,
) -> Result<Option<ast_merge::SourceRenderResult>, String> {
    if evidence.result.outcome != ast_merge::ThreeWayMergeOutcome::Clean
        && !is_recoverable_owner_layout_error(evidence)
    {
        return Ok(None);
    }
    let owner_ids = analyses
        .iter()
        .map(|analysis| {
            analysis
                .owners
                .document
                .owners
                .iter()
                .map(|owner| owner.id.as_str())
                .collect::<HashSet<_>>()
        })
        .collect::<Vec<_>>();
    let classification = evidence
        .classification
        .as_ref()
        .ok_or_else(|| "Rust owner merge omitted classification evidence".to_string())?;
    let mut owner_revisions = HashMap::new();
    let mut deleted_owner_ids = HashSet::new();
    if classification.whole_source_selection.is_some() {
        let revision = classification.whole_source_selection.unwrap();
        let revision_index = match revision {
            ast_merge::SourceRevision::Base => 0,
            ast_merge::SourceRevision::Ours => 1,
            ast_merge::SourceRevision::Theirs => 2,
        };
        for owner_id in &owner_ids[revision_index] {
            owner_revisions.insert((*owner_id).to_string(), revision);
        }
    } else {
        for decision in &classification.decisions {
            let revision = match decision.kind {
                ast_merge::OwnerDecisionKind::SelectOurs => ast_merge::SourceRevision::Ours,
                ast_merge::OwnerDecisionKind::SelectTheirs => ast_merge::SourceRevision::Theirs,
                ast_merge::OwnerDecisionKind::Delete => {
                    deleted_owner_ids.insert(decision.owner_id.clone());
                    continue;
                }
                _ => return Ok(None),
            };
            owner_revisions.insert(decision.owner_id.clone(), revision);
        }
    }
    let selected_owner_ids = owner_revisions.keys().map(String::as_str).collect::<HashSet<_>>();
    let baseline_revision = if let Some(revision) = classification.whole_source_selection {
        revision
    } else if selected_owner_ids == owner_ids[1] {
        ast_merge::SourceRevision::Ours
    } else if selected_owner_ids == owner_ids[2] {
        ast_merge::SourceRevision::Theirs
    } else if owner_ids[1].is_subset(&selected_owner_ids) {
        ast_merge::SourceRevision::Ours
    } else if owner_ids[2].is_subset(&selected_owner_ids) {
        ast_merge::SourceRevision::Theirs
    } else if owner_ids[0].is_subset(&selected_owner_ids) {
        ast_merge::SourceRevision::Base
    } else if !deleted_owner_ids.is_empty() {
        let accounted_owner_ids = selected_owner_ids
            .union(&deleted_owner_ids.iter().map(String::as_str).collect())
            .copied()
            .collect::<HashSet<_>>();
        if owner_ids[1].is_subset(&accounted_owner_ids) {
            ast_merge::SourceRevision::Ours
        } else if owner_ids[2].is_subset(&accounted_owner_ids) {
            ast_merge::SourceRevision::Theirs
        } else if owner_ids[0] == accounted_owner_ids {
            ast_merge::SourceRevision::Base
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };
    let baseline_revision = one_sided_owner_order_revision(analyses).unwrap_or(baseline_revision);
    let revisions = [
        ast_merge::SourceRevision::Base,
        ast_merge::SourceRevision::Ours,
        ast_merge::SourceRevision::Theirs,
    ];
    let sources = revisions
        .iter()
        .zip(analyses)
        .map(|(revision, analysis)| (*revision, analysis.owners.document.source.clone()))
        .collect::<HashMap<_, _>>();
    let dispositions = revisions
        .iter()
        .zip(analyses)
        .map(|(revision, analysis)| (*revision, analysis.dispositions.clone()))
        .collect::<HashMap<_, _>>();
    let comment_regions = revisions
        .iter()
        .zip(analyses)
        .map(|(revision, analysis)| (*revision, analysis.comments.augmentation.regions.clone()))
        .collect::<HashMap<_, _>>();
    let mut selections =
        ast_merge::source_line_selections_for_owner_revisions_preserving_addition_gaps(
            baseline_revision,
            &dispositions,
            &owner_revisions,
            &deleted_owner_ids,
        )
        .map_err(|error| error.to_string())?;
    if let Some(owner_order) = merged_owner_order {
        let Some(ordered) = ast_merge::source_line_selections_for_owner_order(
            selections,
            &dispositions,
            &owner_revisions,
            owner_order,
        )
        .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        selections = ordered;
    }
    let plan = ast_merge::source_render_plan_from_ordered_selections_with_rehomes(
        sources,
        dispositions,
        comment_regions,
        &selections,
        &[],
    )
    .map_err(|error| error.to_string())?;
    Ok(Some(ast_merge::render_source_plan(&plan).map_err(|error| error.to_string())?))
}

fn source_segments_from_line_render(
    rendered: &ast_merge::SourceRenderResult,
    sources: &HashMap<ast_merge::SourceRevision, String>,
) -> Result<Option<Vec<ast_merge::byte_evidence::SourceByteSegment>>, String> {
    if rendered.verification_input.synthesized_fragment_count != 0 || !rendered.conflicts.is_empty()
    {
        return Ok(None);
    }
    let mut cursor = 0usize;
    let mut segments = Vec::with_capacity(rendered.verification_input.source_fragments.len());
    for (index, fragment) in rendered.verification_input.source_fragments.iter().enumerate() {
        let Some(source) = sources.get(&fragment.revision) else {
            return Ok(None);
        };
        if fragment.start_line == 0 || fragment.end_line < fragment.start_line {
            return Ok(None);
        }
        let source_lines = source.split_inclusive('\n').collect::<Vec<_>>();
        if fragment.end_line > source_lines.len() {
            return Ok(None);
        }
        let source_start =
            source_lines[..fragment.start_line - 1].iter().map(|line| line.len()).sum::<usize>();
        let source_end =
            source_lines[..fragment.end_line].iter().map(|line| line.len()).sum::<usize>();
        let output_end = cursor + (source_end - source_start);
        let Some(source_bytes) = source.as_bytes().get(source_start..source_end) else {
            return Ok(None);
        };
        if rendered.content.as_bytes().get(cursor..output_end) != Some(source_bytes) {
            return Ok(None);
        }
        segments.push(ast_merge::byte_evidence::SourceByteSegment {
            id: format!("rust-line-fragment-{index}"),
            revision: fragment.revision,
            source_range: tree_haver::ByteRange { start_byte: source_start, end_byte: source_end },
            output_range: tree_haver::ByteRange { start_byte: cursor, end_byte: output_end },
            sha256: fragment.sha256.clone(),
            owner_id: None,
        });
        cursor = output_end;
    }
    if cursor != rendered.content.len() {
        return Ok(None);
    }
    let byte_sources = sources
        .iter()
        .map(|(revision, source)| (*revision, source.as_bytes()))
        .collect::<HashMap<_, _>>();
    if ast_merge::byte_evidence::verify_source_byte_segments(
        rendered.content.as_bytes(),
        &byte_sources,
        &segments,
    )
    .is_err()
    {
        return Ok(None);
    }
    Ok(Some(segments))
}
