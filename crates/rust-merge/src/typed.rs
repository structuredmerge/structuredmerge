//! Existing conservative Rust declaration ownership over validated TreeHaver facts.
//! Use declarations are whole owners; nested owner/member semantics stay out of scope.
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
    let mut parse_output = parse_output;
    let mut execution = ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: owners,
            merge: merge_documents,
        },
        |text| parse_output(text),
    )?;
    let can_select_lines = execution.evidence.result.outcome
        == ast_merge::ThreeWayMergeOutcome::Clean
        || is_changed_inter_owner_layout_error(&execution.evidence);
    let ordered_line_render_applied = if can_select_lines {
        let analyses = [
            source_line_analysis(base)?,
            source_line_analysis(ours)?,
            source_line_analysis(theirs)?,
        ];
        let Some(rendered) = render_line_selected_output(&analyses, &execution.evidence)? else {
            return Ok(RustMergeExecution {
                evidence: execution.evidence,
                ordered_line_render_applied: false,
                output_parse: execution.output_parse,
            });
        };
        if execution.evidence.result.outcome == ast_merge::ThreeWayMergeOutcome::Clean {
            let previous = execution
                .evidence
                .result
                .output
                .as_deref()
                .ok_or_else(|| "clean Rust owner merge omitted output".to_string())?;
            if rendered.content != previous {
                return Err("Rust source-line render differs from the verified owner merge".into());
            }
            execution.evidence.result.output = Some(rendered.content);
            true
        } else {
            let source_texts = HashMap::from([
                (ast_merge::SourceRevision::Base, analyses[0].owners.document.source.clone()),
                (ast_merge::SourceRevision::Ours, analyses[1].owners.document.source.clone()),
                (ast_merge::SourceRevision::Theirs, analyses[2].owners.document.source.clone()),
            ]);
            let Some(segments) = source_segments_from_line_render(&rendered, &source_texts)? else {
                return Ok(RustMergeExecution {
                    evidence: execution.evidence,
                    ordered_line_render_applied: false,
                    output_parse: execution.output_parse,
                });
            };
            let Ok(parsed) = parse_output(&rendered.content) else {
                return Ok(RustMergeExecution {
                    evidence: execution.evidence,
                    ordered_line_render_applied: false,
                    output_parse: execution.output_parse,
                });
            };
            let input_ids = [base, ours, theirs]
                .into_iter()
                .map(|input| input.source.descriptor().source_id.clone())
                .collect::<HashSet<_>>();
            if parsed.source.descriptor().role != tree_haver::source::SourceRole::Output
                || input_ids.contains(&parsed.source.descriptor().source_id)
                || parsed.source.bytes() != rendered.content.as_bytes()
                || parsed.backend != ours.backend
                || analysis(&parsed).is_err()
            {
                return Ok(RustMergeExecution {
                    evidence: execution.evidence,
                    ordered_line_render_applied: false,
                    output_parse: execution.output_parse,
                });
            }
            execution.evidence.source_segments = segments;
            execution.evidence.result.outcome = ast_merge::ThreeWayMergeOutcome::Clean;
            execution.evidence.result.diagnostics.clear();
            execution.evidence.result.conflicts.clear();
            execution.evidence.result.output = Some(rendered.content);
            execution.output_parse = Some(parsed);
            true
        }
    } else {
        false
    };
    Ok(RustMergeExecution {
        evidence: execution.evidence,
        ordered_line_render_applied,
        output_parse: execution.output_parse,
    })
}

fn is_changed_inter_owner_layout_error(evidence: &SourcePreservingMergeEvidence) -> bool {
    evidence.result.outcome == ast_merge::ThreeWayMergeOutcome::Error
        && evidence.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.message
                == "source-preserving declaration merge cannot prove ownership of changed inter-owner layout"
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

/// Build a shared ordered source-line render from executed Rust owner decisions.
/// Additions are appended in selected tip order; deletions await shared rehome planning.
fn render_line_selected_output(
    analyses: &[RustSourceLineAnalysis; 3],
    evidence: &SourcePreservingMergeEvidence,
) -> Result<Option<ast_merge::SourceRenderResult>, String> {
    if evidence.result.outcome != ast_merge::ThreeWayMergeOutcome::Clean
        && !is_changed_inter_owner_layout_error(evidence)
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
                ast_merge::OwnerDecisionKind::Delete => return Ok(None),
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
    } else {
        return Ok(None);
    };
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
    let mut selections = ast_merge::source_line_selections_for_owner_revisions(
        baseline_revision,
        &dispositions,
        &owner_revisions,
    )
    .map_err(|error| error.to_string())?;
    let baseline_index = match baseline_revision {
        ast_merge::SourceRevision::Base => 0,
        ast_merge::SourceRevision::Ours => 1,
        ast_merge::SourceRevision::Theirs => 2,
    };
    let baseline_owner_ids = &owner_ids[baseline_index];
    let mut selected_line_keys =
        selections.iter().map(|selection| selection.selection_key.clone()).collect::<HashSet<_>>();
    for (revision, analysis) in revisions.iter().zip(analyses) {
        for owner in &analysis.owners.document.owners {
            if baseline_owner_ids.contains(owner.id.as_str())
                || owner_revisions.get(&owner.id) != Some(revision)
            {
                continue;
            }
            for record in analysis.dispositions.iter().filter(|record| {
                record.owner_id.as_deref() == Some(owner.id.as_str())
                    && matches!(record.kind.as_str(), "comment_owner" | "structural_owner")
            }) {
                if selected_line_keys.insert(record.selection_key.clone()) {
                    selections.push(ast_merge::SourceLineSelection {
                        selection_key: record.selection_key.clone(),
                        revision: *revision,
                    });
                }
            }
        }
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
