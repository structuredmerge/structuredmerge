//! Existing Bash ownership/merge semantics over validated TreeHaver facts.
//! Parsing and output verification are supplied by the shared service caller.
use ast_merge::{
    BoundaryEdge, LayoutAugmentation, LayoutOwner, NormalizedCommentEvidence, RemovePlan,
    RemovedOwnerAttachments, SourceLineDisposition, SourceLineOwner, SourcePreservingMergeEvidence,
    SourcePreservingOwnerDocument, SourceRevision, StructuralBoundary, ThreeWayMergeOutcome,
    augment_layout, augment_normalized_comments_with_evidence_filter, render_source_plan,
    source_line_dispositions_from_augmentations, source_line_fragments_for_rehome_plan,
    source_line_selections_for_owner_revisions,
    source_render_plan_from_ordered_selections_with_rehomes,
};
use std::collections::{HashMap, HashSet};
use tree_haver::{service::ParsedResult, source::SourceRole};

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
}

pub fn directional_owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis_with_mode(parsed, false)
        .map(|analysis| analysis.document)
        .map_err(|error| error.message)
}

pub fn analysis(
    parsed: &ParsedResult,
) -> Result<ast_merge::native_analysis::NativeOwnerAnalysis, String> {
    analysis_with_diagnostics(parsed).map_err(|error| error.message)
}

pub fn analysis_document_with_diagnostics(
    parsed: &ParsedResult,
) -> Result<SourcePreservingOwnerDocument, ast_merge::typed_merge::NativeAnalysisError> {
    analysis_with_diagnostics(parsed).map(|analysis| analysis.document)
}

pub fn analysis_with_diagnostics(
    parsed: &ParsedResult,
) -> Result<
    ast_merge::native_analysis::NativeOwnerAnalysis,
    ast_merge::typed_merge::NativeAnalysisError,
> {
    analysis_with_mode(parsed, true)
}

fn analysis_with_mode(
    parsed: &ParsedResult,
    reject_unsafe_source_ranges: bool,
) -> Result<
    ast_merge::native_analysis::NativeOwnerAnalysis,
    ast_merge::typed_merge::NativeAnalysisError,
> {
    if !parsed.backend.languages.iter().any(|language| language == "bash") {
        return Err(ast_merge::typed_merge::NativeAnalysisError::unsupported(
            "Bash analysis requires a Bash parser",
        ));
    }
    let nodes = parsed
        .normalized_nodes()
        .map_err(ast_merge::typed_merge::NativeAnalysisError::unsupported)?;
    let root = parsed.document.output().root_id.as_deref().ok_or_else(|| {
        ast_merge::typed_merge::NativeAnalysisError::unsupported("Bash parse omitted its root")
    })?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| {
        ast_merge::typed_merge::NativeAnalysisError::unsupported(error.to_string())
    })?;
    // Empty/comment-only documents are valid directional endpoints. Derive this
    // from the native root's children, never from textual source classification.
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)
        .map_err(ast_merge::typed_merge::NativeAnalysisError::unsupported)?;
    let root_node =
        index.root(root).map_err(ast_merge::typed_merge::NativeAnalysisError::unsupported)?;
    let analysis = if index
        .children(root_node)
        .iter()
        .all(|node| node.role == tree_haver::NodeRole::Comment)
    {
        ast_merge::native_analysis::NativeOwnerAnalysis {
            document: SourcePreservingOwnerDocument { source: source.into(), owners: vec![] },
            owner_node_ids: Default::default(),
            attached_comment_node_ids: Default::default(),
        }
    } else {
        super::project_bash_analysis(source, root, &nodes, reject_unsafe_source_ranges).map_err(
            |error| match error {
                super::BashAnalysisError::Unsupported(message) => {
                    ast_merge::typed_merge::NativeAnalysisError::unsupported(message)
                }
                super::BashAnalysisError::UnsafeSourceRange(message) => {
                    ast_merge::typed_merge::NativeAnalysisError {
                        kind: ast_merge::typed_merge::NativeAnalysisFailureKind::UnsafeSourceRange,
                        message,
                    }
                }
            },
        )?
    };
    analysis.validate(parsed).map_err(ast_merge::typed_merge::NativeAnalysisError::unsupported)?;
    Ok(analysis)
}

/// Line-level Ruby-compatible ownership facts for one validated Bash parse.
#[derive(Clone, Debug, PartialEq)]
pub struct BashSourceLineAnalysis {
    pub owners: ast_merge::native_analysis::NativeOwnerAnalysis,
    pub comments: NormalizedCommentEvidence,
    pub layout: LayoutAugmentation,
    pub dispositions: Vec<SourceLineDisposition>,
    /// Provider-verified single-line owner kinds eligible for Ruby-compatible
    /// inline-comment promotion during removal/rehome planning.
    pub inline_promotion_owner_ids: HashSet<String>,
}

/// Compose the native Bash owner projection with TreeHaver comments and the
/// shared comment/layout augmenters before a merge chooses source regions.
pub fn source_line_analysis(parsed: &ParsedResult) -> Result<BashSourceLineAnalysis, String> {
    let owners = analysis(parsed)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let nodes = parsed.normalized_nodes()?;
    let mut tree_to_semantic = HashMap::<String, String>::new();
    let mut inline_promotion_owner_ids = HashSet::new();
    let mut native_layout_owners = Vec::with_capacity(owners.document.owners.len());
    let mut semantic_layout_owners = Vec::with_capacity(owners.document.owners.len());
    for owner in &owners.document.owners {
        let node_ids = owners
            .owner_node_ids
            .get(&owner.id)
            .ok_or_else(|| format!("Bash owner {} has no native node references", owner.id))?;
        if node_ids.len() != 1 {
            return Err(format!("Bash owner {} does not map to exactly one native node", owner.id));
        }
        let node_id = &node_ids[0];
        let native_node = nodes
            .iter()
            .find(|node| node.id == *node_id)
            .ok_or_else(|| format!("Bash owner {} references a missing native node", owner.id))?;
        let ruby_inline_safe_kind = native_node.kind == "variable_assignment"
            || (native_node.kind == "command" && owner.id.starts_with("/test_harness_call:"));
        if owner.start_line == owner.end_line && ruby_inline_safe_kind {
            inline_promotion_owner_ids.insert(owner.id.clone());
        }
        if tree_to_semantic.insert(node_id.clone(), owner.id.clone()).is_some() {
            return Err(format!("native Bash node {node_id} maps to multiple owners"));
        }
        native_layout_owners.push(LayoutOwner {
            owner_id: node_id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        });
        semantic_layout_owners.push(LayoutOwner {
            owner_id: owner.id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        });
    }

    let first_line = source.lines().next().unwrap_or_default();
    let comments = augment_normalized_comments_with_evidence_filter(
        source,
        &native_layout_owners,
        &nodes,
        "hash_comment",
        ast_merge::CommentAttachmentPolicy::PrecedingOwner,
        |node| !(node.span.start_point.row == 0 && first_line.starts_with("#!")),
        normalize_bash_comment,
    )?;
    let mut comments = comments;
    remap_comment_owner_ids(&mut comments.augmentation, &tree_to_semantic)?;
    let lines = source_line_contents(source);
    let mut layout = augment_layout(&lines, &native_layout_owners)?;
    remap_layout_owner_ids(&mut layout, &tree_to_semantic)?;
    let dispositions = source_line_dispositions_from_augmentations(
        source,
        &semantic_layout_owners,
        &comments.augmentation,
        &layout,
    )?;

    Ok(BashSourceLineAnalysis {
        owners,
        comments,
        layout,
        dispositions,
        inline_promotion_owner_ids,
    })
}

fn normalize_bash_comment(text: &str) -> String {
    text.trim_start().strip_prefix('#').unwrap_or(text.trim_start()).trim_start().to_string()
}

fn source_line_contents(source: &str) -> Vec<String> {
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

fn remap_comment_owner_ids(
    comments: &mut ast_merge::CommentAugmentation,
    tree_to_semantic: &HashMap<String, String>,
) -> Result<(), String> {
    for attachment in &mut comments.attachments {
        attachment.owner_id =
            semantic_owner_id(&attachment.owner_id, tree_to_semantic)?.to_string();
    }
    for region in &mut comments.regions {
        if !region.owner_id.starts_with("document:") {
            region.owner_id = semantic_owner_id(&region.owner_id, tree_to_semantic)?.to_string();
        }
    }
    for gap in &mut comments.gaps {
        remap_optional_owner(&mut gap.before_owner_id, tree_to_semantic)?;
        remap_optional_owner(&mut gap.after_owner_id, tree_to_semantic)?;
    }
    Ok(())
}

fn remap_layout_owner_ids(
    layout: &mut LayoutAugmentation,
    tree_to_semantic: &HashMap<String, String>,
) -> Result<(), String> {
    for gap in &mut layout.gaps {
        remap_optional_owner(&mut gap.before_owner_id, tree_to_semantic)?;
        remap_optional_owner(&mut gap.after_owner_id, tree_to_semantic)?;
    }
    for attachment in &mut layout.attachments {
        attachment.owner_id =
            semantic_owner_id(&attachment.owner_id, tree_to_semantic)?.to_string();
    }
    Ok(())
}

fn remap_optional_owner(
    owner_id: &mut Option<String>,
    tree_to_semantic: &HashMap<String, String>,
) -> Result<(), String> {
    if let Some(owner_id_value) = owner_id {
        *owner_id_value = semantic_owner_id(owner_id_value, tree_to_semantic)?.to_string();
    }
    Ok(())
}

fn semantic_owner_id<'a>(
    tree_id: &str,
    tree_to_semantic: &'a HashMap<String, String>,
) -> Result<&'a str, String> {
    tree_to_semantic
        .get(tree_id)
        .map(String::as_str)
        .ok_or_else(|| format!("layout fact refers to unknown native Bash owner {tree_id}"))
}

#[derive(Clone, Debug)]
pub struct BashMergeExecution {
    pub evidence: SourcePreservingMergeEvidence,
    /// True only when the shared ordered physical-line plan produced the
    /// verified owner-merge output.
    pub ordered_line_render_applied: bool,
    /// Present only for a clean, freshly verified result. No input parse is
    /// relabeled as an output parse, including whole-source/no-op selections.
    pub output_parse: Option<ParsedResult>,
}

fn verified_document(
    parsed: &ParsedResult,
    text: &str,
    ours: &ParsedResult,
    ids: &std::collections::HashSet<&String>,
) -> Result<SourcePreservingOwnerDocument, String> {
    if parsed.source.descriptor().role != SourceRole::Output
        || ids.contains(&parsed.source.descriptor().source_id)
        || parsed.source.bytes() != text.as_bytes()
        || parsed.backend != ours.backend
    {
        return Err(
            "Bash output verification changed source bytes, identity or selected parser".into()
        );
    }
    owners(parsed)
}

/// Return the Ruby-compatible whole-document guard when unmanaged source differs.
pub fn unmanaged_source_conflict(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> Option<ast_merge::ThreeWayMergeResult<String>> {
    if ours.source == theirs.source || base.source == theirs.source || base.source == ours.source {
        return None;
    }
    (ast_merge::unmanaged_source_fingerprint(ours)
        != ast_merge::unmanaged_source_fingerprint(theirs))
    .then(crate::unmanaged_source_conflict_result)
}

/// Shared-core merge callback with Bash's Ruby-compatible unmanaged-source guard.
pub fn merge_documents(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: &mut dyn FnMut(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> SourcePreservingMergeEvidence {
    if let Some(result) = unmanaged_source_conflict(&base, &ours, &theirs) {
        return SourcePreservingMergeEvidence {
            result,
            source_segments: vec![],
            classification: None,
        };
    }
    ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<BashMergeExecution, String> {
    let inputs = [base, ours, theirs];
    let mut ids = std::collections::HashSet::new();
    for (parsed, role) in
        inputs.into_iter().zip([SourceRole::Base, SourceRole::Ours, SourceRole::Theirs])
    {
        if parsed.source.descriptor().role != role
            || !ids.insert(&parsed.source.descriptor().source_id)
        {
            return Err("Bash merge requires distinct base/ours/theirs source identities".into());
        }
    }
    let source_line_analyses =
        [source_line_analysis(base)?, source_line_analysis(ours)?, source_line_analysis(theirs)?];
    let [base_lines, ours_lines, theirs_lines] = source_line_analyses;
    let [base_document, ours_document, theirs_document] = [
        base_lines.owners.document.clone(),
        ours_lines.owners.document.clone(),
        theirs_lines.owners.document.clone(),
    ];
    if let Some(result) =
        unmanaged_source_conflict(&base_document, &ours_document, &theirs_document)
    {
        return Ok(BashMergeExecution {
            evidence: SourcePreservingMergeEvidence {
                result,
                source_segments: vec![],
                classification: None,
            },
            ordered_line_render_applied: false,
            output_parse: None,
        });
    }
    let mut output_parse = None;
    let mut verify = |text: &str| {
        let parsed = parse_output(text)?;
        let document = verified_document(&parsed, text, ours, &ids)?;
        output_parse = Some(parsed);
        Ok(document)
    };
    let mut evidence = ast_merge::merge_source_preserving_owners_with_evidence(
        base_document,
        ours_document,
        theirs_document,
        &mut verify,
    );
    let source_line_analyses = [base_lines, ours_lines, theirs_lines];
    let ordered_line_render_applied =
        apply_line_selected_output(&source_line_analyses, &mut evidence)?;
    // Existing whole-source shortcuts do not call verify. Typed execution must
    // verify these too, while preserving the engine's decisions and segments.
    if evidence.result.outcome == ThreeWayMergeOutcome::Clean {
        let text = evidence.result.output.as_deref().ok_or("clean Bash merge omitted output")?;
        if output_parse.is_none() {
            let parsed = parse_output(text)?;
            verified_document(&parsed, text, ours, &ids)?;
            output_parse = Some(parsed);
        }
    } else {
        output_parse = None;
    }
    Ok(BashMergeExecution { evidence, ordered_line_render_applied, output_parse })
}

/// Use the shared ordered source-line renderer for clean merges with stable
/// selected layout. Additive membership uses the selected layout's owner order;
/// deletion remains on the existing path until rehome selections are composed.
fn apply_line_selected_output(
    analyses: &[BashSourceLineAnalysis; 3],
    evidence: &mut SourcePreservingMergeEvidence,
) -> Result<bool, String> {
    if evidence.result.outcome != ThreeWayMergeOutcome::Clean {
        return Ok(false);
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
                .collect::<std::collections::HashSet<_>>()
        })
        .collect::<Vec<_>>();
    let classification = evidence
        .classification
        .as_ref()
        .ok_or_else(|| "clean Bash owner merge omitted classification evidence".to_string())?;
    let membership_changed = owner_ids[0] != owner_ids[1] || owner_ids[0] != owner_ids[2];
    let baseline_revision = if let Some(revision) = classification.whole_source_selection {
        revision
    } else {
        let mut owner_revisions = HashMap::new();
        for decision in &classification.decisions {
            let revision = match &decision.kind {
                ast_merge::OwnerDecisionKind::SelectOurs => SourceRevision::Ours,
                ast_merge::OwnerDecisionKind::SelectTheirs => SourceRevision::Theirs,
                ast_merge::OwnerDecisionKind::Delete => continue,
                _ => return Err("clean Bash owner merge contains a conflict decision".into()),
            };
            owner_revisions.insert(decision.owner_id.clone(), revision);
        }
        let selected_ids =
            owner_revisions.keys().map(String::as_str).collect::<std::collections::HashSet<_>>();
        if selected_ids == owner_ids[1] {
            // The source-preserving kernel uses ours as the layout baseline
            // when the selected owner set matches ours.
            SourceRevision::Ours
        } else if selected_ids == owner_ids[2] {
            SourceRevision::Theirs
        } else if owner_ids[1].is_subset(&selected_ids) {
            // Match Ruby's declaration composite: retain ours' layout and
            // append independently selected declarations from theirs.
            SourceRevision::Ours
        } else {
            return Err(
                "Bash owner selections do not match either selected layout membership".into()
            );
        }
    };
    let mut owner_revisions = HashMap::new();
    if classification.whole_source_selection.is_some() {
        let baseline_owner_ids = match baseline_revision {
            SourceRevision::Base => &owner_ids[0],
            SourceRevision::Ours => &owner_ids[1],
            SourceRevision::Theirs => &owner_ids[2],
        };
        for owner_id in baseline_owner_ids {
            owner_revisions.insert((*owner_id).to_string(), baseline_revision);
        }
    } else {
        for decision in &classification.decisions {
            let revision = match &decision.kind {
                ast_merge::OwnerDecisionKind::SelectOurs => SourceRevision::Ours,
                ast_merge::OwnerDecisionKind::SelectTheirs => SourceRevision::Theirs,
                ast_merge::OwnerDecisionKind::Delete => continue,
                _ => return Err("clean Bash owner merge contains a non-selection decision".into()),
            };
            owner_revisions.insert(decision.owner_id.clone(), revision);
        }
    }
    let revisions = [SourceRevision::Base, SourceRevision::Ours, SourceRevision::Theirs];
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
    let selections = source_line_selections_for_owner_revisions(
        baseline_revision,
        &dispositions,
        &owner_revisions,
    )
    .map_err(|error| error.to_string())?;
    let rehomes = if membership_changed {
        match removed_owner_rehome_selections(analyses, classification, &owner_revisions) {
            Ok(rehomes) => rehomes,
            Err(error) => return Err(format!("removed-owner rehome: {error}")),
        }
    } else {
        Vec::new()
    };
    let plan = match source_render_plan_from_ordered_selections_with_rehomes(
        sources,
        dispositions,
        comment_regions,
        &selections,
        &rehomes,
    ) {
        Ok(plan) => plan,
        Err(error) if membership_changed => return Err(format!("composite line plan: {error}")),
        Err(error) => return Err(error.to_string()),
    };
    let rendered = match render_source_plan(&plan) {
        Ok(rendered) => rendered,
        Err(error) if membership_changed => return Err(format!("composite render: {error}")),
        Err(error) => return Err(error.to_string()),
    };
    let previous = evidence
        .result
        .output
        .as_deref()
        .ok_or_else(|| "clean Bash owner merge omitted output".to_string())?;
    if rendered.content != previous {
        if membership_changed {
            return Err(format!(
                "composite output mismatch: expected {previous:?}, got {:?}",
                rendered.content
            ));
        }
        return Err("Bash source-line render differs from the verified owner merge".into());
    }
    evidence.result.output = Some(rendered.content);
    Ok(true)
}

fn removed_owner_rehome_selections(
    analyses: &[BashSourceLineAnalysis; 3],
    classification: &ast_merge::OwnerMergeClassification,
    retained_revisions: &HashMap<String, SourceRevision>,
) -> Result<Vec<ast_merge::RehomeSourceLineSelections>, String> {
    let revisions = [SourceRevision::Base, SourceRevision::Ours, SourceRevision::Theirs];
    let retained_ids = retained_revisions.keys().map(String::as_str).collect::<HashSet<_>>();
    let mut rehomes = Vec::new();
    for decision in classification
        .decisions
        .iter()
        .filter(|decision| decision.kind == ast_merge::OwnerDecisionKind::Delete)
    {
        let (source_revision, source) = revisions
            .iter()
            .zip(analyses)
            .find(|(_, analysis)| {
                analysis.owners.document.owners.iter().any(|owner| owner.id == decision.owner_id)
            })
            .ok_or_else(|| format!("deleted owner {} has no source revision", decision.owner_id))?;
        let owner_index = source
            .owners
            .document
            .owners
            .iter()
            .position(|owner| owner.id == decision.owner_id)
            .ok_or_else(|| format!("deleted owner {} has no source range", decision.owner_id))?;
        let owner = &source.owners.document.owners[owner_index];
        let leading_boundary = source.owners.document.owners[..owner_index]
            .iter()
            .rev()
            .find(|candidate| retained_ids.contains(candidate.id.as_str()))
            .map(|candidate| StructuralBoundary {
                edge: BoundaryEdge::Leading,
                owner_id: Some(candidate.id.clone()),
            });
        let trailing_boundary = source.owners.document.owners[owner_index + 1..]
            .iter()
            .find(|candidate| retained_ids.contains(candidate.id.as_str()))
            .map(|candidate| StructuralBoundary {
                edge: BoundaryEdge::Trailing,
                owner_id: Some(candidate.id.clone()),
            });
        let comment_attachment = source
            .comments
            .augmentation
            .attachments
            .iter()
            .find(|attachment| attachment.owner_id == decision.owner_id);
        let layout_attachment = source
            .layout
            .attachments
            .iter()
            .find(|attachment| attachment.owner_id == decision.owner_id);
        let removed_attachment = RemovedOwnerAttachments {
            owner_id: Some(decision.owner_id.clone()),
            leading_region_id: comment_attachment
                .and_then(|attachment| attachment.leading_region_id.clone()),
            inline_region_id: comment_attachment
                .and_then(|attachment| attachment.inline_region_id.clone()),
            orphan_region_ids: comment_attachment
                .map(|attachment| attachment.orphan_region_ids.clone())
                .unwrap_or_default(),
            trailing_region_id: comment_attachment
                .and_then(|attachment| attachment.trailing_region_id.clone()),
            leading_gap_id: layout_attachment
                .and_then(|attachment| attachment.leading_gap_id.clone()),
            trailing_gap_id: layout_attachment
                .and_then(|attachment| attachment.trailing_gap_id.clone()),
        };
        let remove_plan = RemovePlan::new(
            source.owners.document.source.clone(),
            owner.start_line,
            owner.end_line,
            leading_boundary,
            trailing_boundary,
            vec![removed_attachment],
        )?;
        let source_owners = source
            .owners
            .document
            .owners
            .iter()
            .map(|owner| SourceLineOwner {
                id: owner.id.clone(),
                start_line: owner.start_line,
                end_line: owner.end_line,
            })
            .collect::<Vec<_>>();
        for rehome in &remove_plan.rehome_plans {
            rehomes.push(
                source_line_fragments_for_rehome_plan(
                    *source_revision,
                    &source.dispositions,
                    &source_owners,
                    &source.comments.augmentation.regions,
                    &source.inline_promotion_owner_ids,
                    rehome,
                )
                .map_err(|error| error.to_string())?,
            );
        }
    }
    Ok(rehomes)
}
