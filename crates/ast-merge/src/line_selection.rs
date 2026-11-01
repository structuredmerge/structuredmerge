//! Shared recovery of owner merges whose inter-owner layout the generic engine cannot prove.
//!
//! The generic source-preserving owner engine refuses a clean result when deleting or adding an
//! owner changes layout between owners. Providers with native comment and line facts can instead
//! select physical source lines by owner decision. This module holds the language-neutral parts:
//! line facts remapping, the recoverable-error test, selection rendering and byte-segment proof.
//! It does not reorder owners; any case it cannot render exactly returns `None` and the caller
//! keeps the engine's fail-closed result.
use std::collections::{HashMap, HashSet};

use crate::{
    CommentAugmentation, LayoutAugmentation, NormalizedCommentEvidence, OwnerDecisionKind,
    SourceLineDisposition, SourcePreservingMergeEvidence, SourceRenderResult, SourceRevision,
    ThreeWayMergeOutcome, native_analysis::NativeOwnerAnalysis,
};

/// Per-revision line facts a provider composes from its native parse.
#[derive(Clone, Debug)]
pub struct LineSelectionAnalysis {
    pub owners: NativeOwnerAnalysis,
    pub comments: NormalizedCommentEvidence,
    pub dispositions: Vec<SourceLineDisposition>,
}

/// True when the engine stopped only because it could not prove layout ownership, and every
/// owner decision is a plain select/delete that line selection can reproduce.
pub fn is_recoverable_owner_layout_error(evidence: &SourcePreservingMergeEvidence) -> bool {
    evidence.result.outcome == ThreeWayMergeOutcome::Error
        && evidence.result.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.message.as_str(),
                "source-preserving declaration merge cannot prove ownership of changed inter-owner layout"
                    | "source-preserving declaration merge cannot prove a combined owner membership layout"
            )
        })
        && evidence.classification.as_ref().is_some_and(|classification| {
            classification.decisions.iter().all(|decision| {
                matches!(
                    decision.kind,
                    OwnerDecisionKind::SelectOurs
                        | OwnerDecisionKind::SelectTheirs
                        | OwnerDecisionKind::Delete
                )
            })
        })
}

pub fn source_line_contents(source: &str) -> Vec<String> {
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

fn semantic_owner_id<'a>(
    tree_id: &str,
    tree_to_semantic: &'a HashMap<String, String>,
) -> Result<&'a str, String> {
    tree_to_semantic
        .get(tree_id)
        .map(String::as_str)
        .ok_or_else(|| format!("layout fact refers to unknown native owner {tree_id}"))
}

fn remap_optional_owner(
    owner_id: &mut Option<String>,
    tree_to_semantic: &HashMap<String, String>,
) -> Result<(), String> {
    if let Some(value) = owner_id {
        *value = semantic_owner_id(value, tree_to_semantic)?.to_string();
    }
    Ok(())
}

/// Rewrite native tree ids in comment facts to semantic owner ids.
pub fn remap_comment_owner_ids(
    comments: &mut CommentAugmentation,
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

/// Rewrite native tree ids in layout facts to semantic owner ids.
pub fn remap_layout_owner_ids(
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

fn revision_index(revision: SourceRevision) -> usize {
    match revision {
        SourceRevision::Base => 0,
        SourceRevision::Ours => 1,
        SourceRevision::Theirs => 2,
    }
}

/// Render the engine's owner decisions as ordered physical source lines.
/// Returns `None` when the decisions cannot be reproduced without synthesis or reordering.
pub fn render_line_selected_output(
    analyses: &[LineSelectionAnalysis; 3],
    evidence: &SourcePreservingMergeEvidence,
) -> Result<Option<SourceRenderResult>, String> {
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
        .ok_or_else(|| "owner merge omitted classification evidence".to_string())?;
    let mut owner_revisions = HashMap::new();
    let mut deleted_owner_ids = HashSet::new();
    if let Some(revision) = classification.whole_source_selection {
        for owner_id in &owner_ids[revision_index(revision)] {
            owner_revisions.insert((*owner_id).to_string(), revision);
        }
    } else {
        for decision in &classification.decisions {
            let revision = match decision.kind {
                OwnerDecisionKind::SelectOurs => SourceRevision::Ours,
                OwnerDecisionKind::SelectTheirs => SourceRevision::Theirs,
                OwnerDecisionKind::Delete => {
                    deleted_owner_ids.insert(decision.owner_id.clone());
                    continue;
                }
                _ => return Ok(None),
            };
            owner_revisions.insert(decision.owner_id.clone(), revision);
        }
    }
    // A deleted owner whose neighbours include unattached comment text (a floating comment between
    // blank lines) can lose or gain a separator line; leave those to the fail-closed result.
    if !deleted_owner_ids.is_empty() {
        let base = &analyses[0].owners.document;
        let lines = base.source.split_inclusive('\n').collect::<Vec<_>>();
        let non_blank = |from: usize, to: usize| {
            (from..=to).any(|line| {
                line >= 1 && lines.get(line - 1).is_some_and(|text| !text.trim().is_empty())
            })
        };
        for (index, owner) in base.owners.iter().enumerate() {
            if !deleted_owner_ids.contains(&owner.id) {
                continue;
            }
            if index > 0 {
                let previous = &base.owners[index - 1];
                if owner.start_line > previous.end_line + 1
                    && non_blank(previous.end_line + 1, owner.start_line - 1)
                {
                    return Ok(None);
                }
            }
            let next_start =
                base.owners.get(index + 1).map_or(lines.len() + 1, |next| next.start_line);
            if next_start > owner.end_line + 1 && non_blank(owner.end_line + 1, next_start - 1) {
                return Ok(None);
            }
        }
    }
    let selected_owner_ids = owner_revisions.keys().map(String::as_str).collect::<HashSet<_>>();
    let baseline_revision = if let Some(revision) = classification.whole_source_selection {
        revision
    } else if selected_owner_ids == owner_ids[1] || owner_ids[1].is_subset(&selected_owner_ids) {
        SourceRevision::Ours
    } else if selected_owner_ids == owner_ids[2] || owner_ids[2].is_subset(&selected_owner_ids) {
        SourceRevision::Theirs
    } else if owner_ids[0].is_subset(&selected_owner_ids) {
        SourceRevision::Base
    } else if !deleted_owner_ids.is_empty() {
        let accounted = selected_owner_ids
            .union(&deleted_owner_ids.iter().map(String::as_str).collect())
            .copied()
            .collect::<HashSet<_>>();
        if owner_ids[1].is_subset(&accounted) {
            SourceRevision::Ours
        } else if owner_ids[2].is_subset(&accounted) {
            SourceRevision::Theirs
        } else if owner_ids[0] == accounted {
            SourceRevision::Base
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };
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
    let selections = crate::source_line_selections_for_owner_revisions_preserving_addition_gaps(
        baseline_revision,
        &dispositions,
        &owner_revisions,
        &deleted_owner_ids,
    )
    .map_err(|error| error.to_string())?;
    let plan = crate::source_render_plan_from_ordered_selections_with_rehomes(
        sources,
        dispositions,
        comment_regions,
        &selections,
        &[],
    )
    .map_err(|error| error.to_string())?;
    Ok(Some(crate::render_source_plan(&plan).map_err(|error| error.to_string())?))
}

/// Prove every output byte comes from a whole source line of one revision.
pub fn source_segments_from_line_render(
    rendered: &SourceRenderResult,
    sources: &HashMap<SourceRevision, String>,
) -> Result<Option<Vec<crate::byte_evidence::SourceByteSegment>>, String> {
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
        segments.push(crate::byte_evidence::SourceByteSegment {
            id: format!("line-fragment-{index}"),
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
    if crate::byte_evidence::verify_source_byte_segments(
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

/// Render the engine's decisions by line selection and accept the result only when every output
/// byte is proven against a source line, the output reparses with the ours parser, and the
/// output's owners equal the surviving owners (all owners of any revision minus deletions).
/// On success `evidence` becomes a clean result; otherwise it is left untouched.
pub fn recover_with_line_selection(
    inputs: &[tree_haver::service::ParsedResult],
    analyses: &[LineSelectionAnalysis; 3],
    evidence: &mut SourcePreservingMergeEvidence,
    parse_output: &mut dyn FnMut(&str) -> Result<tree_haver::service::ParsedResult, String>,
    output_owner_ids: &dyn Fn(
        &tree_haver::service::ParsedResult,
    ) -> Result<HashSet<String>, String>,
) -> bool {
    use tree_haver::source::SourceRole;
    let Some(ours) =
        inputs.iter().find(|parsed| parsed.source.descriptor().role == SourceRole::Ours)
    else {
        return false;
    };
    let Ok(Some(rendered)) = render_line_selected_output(analyses, evidence) else {
        return false;
    };
    let source_texts = HashMap::from([
        (SourceRevision::Base, analyses[0].owners.document.source.clone()),
        (SourceRevision::Ours, analyses[1].owners.document.source.clone()),
        (SourceRevision::Theirs, analyses[2].owners.document.source.clone()),
    ]);
    let Ok(Some(segments)) = source_segments_from_line_render(&rendered, &source_texts) else {
        return false;
    };
    let Ok(parsed) = parse_output(&rendered.content) else {
        return false;
    };
    let input_ids = inputs
        .iter()
        .map(|input| input.source.descriptor().source_id.clone())
        .collect::<HashSet<_>>();
    if parsed.source.descriptor().role != SourceRole::Output
        || input_ids.contains(&parsed.source.descriptor().source_id)
        || parsed.source.bytes() != rendered.content.as_bytes()
        || parsed.backend != ours.backend
        || !parsed.document.output().ok
    {
        return false;
    }
    let Ok(actual) = output_owner_ids(&parsed) else {
        return false;
    };
    let deleted = evidence
        .classification
        .as_ref()
        .map(|classification| {
            classification
                .decisions
                .iter()
                .filter(|decision| decision.kind == OwnerDecisionKind::Delete)
                .map(|decision| decision.owner_id.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let expected = analyses
        .iter()
        .flat_map(|analysis| analysis.owners.document.owners.iter().map(|owner| owner.id.clone()))
        .filter(|id| !deleted.contains(id))
        .collect::<HashSet<_>>();
    if expected != actual {
        return false;
    }
    evidence.source_segments = segments;
    evidence.result.outcome = ThreeWayMergeOutcome::Clean;
    evidence.result.diagnostics.clear();
    evidence.result.conflicts.clear();
    evidence.result.output = Some(rendered.content);
    true
}
