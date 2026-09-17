//! Family-owned analysis decisions with exact native references. This is not a
//! render partition: nested owners and comment subjects can overlap.
use crate::{
    JsonDialect,
    typed::{self, JsonOwnerFact},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{SourceSpan, service::ParsedResult};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommentRegion {
    pub id: String,
    pub node_ids: Vec<String>,
    pub span: SourceSpan,
    pub source_sha256: String,
    pub kind: String,
    pub floating: bool,
    pub owner_id: String,
    pub family_region_ids: Vec<String>,
    pub attachment_resolved: bool,
}

/// Original family grouping, with owner IDs translated into this analysis.
/// A group is not necessarily one contiguous comment-only source range.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FamilyCommentRegion {
    pub id: String,
    pub owner_id: String,
    pub kind: String,
    pub floating: bool,
    pub node_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayoutGap {
    pub id: String,
    pub kind: String,
    pub span: SourceSpan,
    pub source_sha256: String,
    pub before_owner_id: Option<String>,
    pub after_owner_id: Option<String>,
    pub controller_side: String,
    pub fallback_controller_side: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Attachment {
    pub owner_id: String,
    pub leading_comment_region_ids: Vec<String>,
    pub inline_comment_region_ids: Vec<String>,
    pub trailing_comment_region_ids: Vec<String>,
    pub orphan_comment_region_ids: Vec<String>,
    pub leading_gap_id: Option<String>,
    pub trailing_gap_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Ownership {
    pub subject_ref: String,
    pub selected_owner_ref: String,
    pub relation: String,
    pub basis: String,
    pub confidence: String,
    pub alternatives: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Analysis {
    pub owners: Vec<JsonOwnerFact>,
    pub comment_regions: Vec<CommentRegion>,
    pub family_comment_regions: Vec<FamilyCommentRegion>,
    pub layout_gaps: Vec<LayoutGap>,
    pub attachments: Vec<Attachment>,
    pub ownership: Vec<Ownership>,
    /// Native nodes with incomplete or conflicting family attachment decisions.
    /// They are retained at document scope, not silently assigned to a sibling.
    pub unresolved_comment_node_ids: Vec<String>,
}

pub fn analyze(parsed: &ParsedResult, dialect: JsonDialect) -> Result<Analysis, String> {
    let facts = typed::owner_analysis(parsed, dialect)?;
    let root = facts.owners.first().ok_or("JSON analysis omitted syntax root")?.id.clone();
    let native_to_owner = facts
        .owners
        .iter()
        .map(|owner| (owner.value_node_id.as_str(), owner.id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let resolve = |id: &str| -> Result<String, String> {
        if matches!(id, "document:preamble" | "document:postlude" | "document:orphan") {
            Ok(root.clone())
        } else {
            native_to_owner
                .get(id)
                .map(|id| id.to_string())
                .ok_or_else(|| "unresolved family owner reference".into())
        }
    };
    let mut attachments = facts
        .owners
        .iter()
        .map(|owner| {
            (
                owner.id.clone(),
                Attachment {
                    owner_id: owner.id.clone(),
                    leading_comment_region_ids: vec![],
                    inline_comment_region_ids: vec![],
                    trailing_comment_region_ids: vec![],
                    orphan_comment_region_ids: vec![],
                    leading_gap_id: None,
                    trailing_gap_id: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut ownership = vec![];
    let family_comment_regions = facts
        .family
        .comment_regions
        .iter()
        .map(|region| {
            Ok(FamilyCommentRegion {
                id: region.id.clone(),
                owner_id: resolve(&region.owner_id)?,
                kind: region.kind.clone(),
                floating: region.floating,
                node_ids: facts
                    .comment_region_node_ids
                    .get(&region.id)
                    .ok_or("unresolved family comment provenance")?
                    .clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut comment_regions = vec![];
    let mut unresolved_comment_node_ids = vec![];
    let mut native_comments = parsed
        .document
        .output()
        .comments
        .iter()
        .map(|comment| parsed.document.node(&comment.node_id).ok_or("unresolved native comment"))
        .collect::<Result<Vec<_>, _>>()?;
    native_comments.sort_by_key(|node| (node.span.range.start_byte, node.span.range.end_byte));
    for (index, node) in native_comments.into_iter().enumerate() {
        let mut candidates = BTreeSet::new();
        let mut relations = BTreeSet::new();
        let mut region_ids = vec![];
        let mut floating = false;
        for region in &facts.family.comment_regions {
            if facts
                .comment_region_node_ids
                .get(&region.id)
                .is_some_and(|ids| ids.contains(&node.id))
            {
                candidates.insert(resolve(&region.owner_id)?);
                relations.insert(region.kind.as_str());
                region_ids.push(region.id.clone());
                floating |= region.floating;
            }
        }
        let resolved = candidates.len() == 1
            && relations.len() == 1
            && !facts.unclaimed_comment_node_ids.contains(&node.id);
        let (selected, kind) = if resolved {
            (candidates.first().unwrap().clone(), relations.first().unwrap().to_string())
        } else {
            unresolved_comment_node_ids.push(node.id.clone());
            (root.clone(), "orphan".into())
        };
        let id = format!("json.comment:{index}");
        let attachment = attachments.get_mut(&selected).ok_or("unresolved comment controller")?;
        match kind.as_str() {
            "leading" | "preamble" => &mut attachment.leading_comment_region_ids,
            "inline" => &mut attachment.inline_comment_region_ids,
            "trailing" | "postlude" => &mut attachment.trailing_comment_region_ids,
            _ => &mut attachment.orphan_comment_region_ids,
        }
        .push(id.clone());
        ownership.push(Ownership {
            subject_ref: id.clone(),
            selected_owner_ref: selected.clone(),
            relation: kind.clone(),
            basis: if resolved {
                "existing-json-family-attachment"
            } else {
                "document-retention-unresolved-attachment"
            }
            .into(),
            confidence: if resolved { "deterministic" } else { "unresolved" }.into(),
            alternatives: if resolved { vec![] } else { candidates.into_iter().collect() },
        });
        comment_regions.push(CommentRegion {
            id,
            node_ids: vec![node.id.clone()],
            span: node.span.clone(),
            source_sha256: parsed
                .source
                .range_digest(node.span.range.clone())
                .map_err(|e| e.to_string())?,
            kind,
            floating,
            owner_id: selected,
            family_region_ids: region_ids,
            attachment_resolved: resolved,
        });
    }
    let mut layout_gaps = vec![];
    for gap in &facts.family.layout_gaps {
        let source = facts.layout_gap_sources.get(&gap.id).ok_or("unresolved layout gap bytes")?;
        let before = gap.before_owner_id.as_deref().map(&resolve).transpose()?;
        let after = gap.after_owner_id.as_deref().map(&resolve).transpose()?;
        let controller = if gap.controller_side == "before" { &before } else { &after };
        let controller = controller.as_ref().ok_or("unresolved gap controller")?;
        if let Some(owner) = &before {
            attachments.get_mut(owner).ok_or("unresolved gap owner")?.trailing_gap_id =
                Some(gap.id.clone());
        }
        if let Some(owner) = &after {
            attachments.get_mut(owner).ok_or("unresolved gap owner")?.leading_gap_id =
                Some(gap.id.clone());
        }
        ownership.push(Ownership {
            subject_ref: gap.id.clone(),
            selected_owner_ref: controller.clone(),
            relation: "controls-output".into(),
            basis: "existing-json-family-blank-gap".into(),
            confidence: "deterministic".into(),
            alternatives: vec![],
        });
        layout_gaps.push(LayoutGap {
            id: gap.id.clone(),
            kind: gap.kind.clone(),
            span: source.span.clone(),
            source_sha256: source.sha256.clone(),
            before_owner_id: before,
            after_owner_id: after,
            controller_side: gap.controller_side.clone(),
            fallback_controller_side: if gap.before_owner_id.is_some()
                && gap.after_owner_id.is_some()
            {
                Some(if gap.controller_side == "before" { "after" } else { "before" }.into())
            } else {
                None
            },
        });
    }
    let attachments = facts
        .owners
        .iter()
        .map(|owner| {
            attachments.remove(&owner.id).ok_or_else(|| "unresolved owner attachment".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Analysis {
        owners: facts.owners,
        comment_regions,
        family_comment_regions,
        layout_gaps,
        attachments,
        ownership,
        unresolved_comment_node_ids,
    })
}
