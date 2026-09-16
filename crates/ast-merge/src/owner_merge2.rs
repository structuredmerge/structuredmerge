//! Directional classification over Rust-derived ownership. This is not a
//! renderer or a complete merge2 operation. In particular, decision order is
//! not an insertion plan: family layout/attachment rules still decide placement.
use crate::{SourcePreservingOwner, SourcePreservingOwnerDocument};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{
    ByteRange,
    source::{SourceDescriptor, SourceDocument, SourceRole},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionalOwnerAction {
    RetainCurrentOnly,
    RetainIdenticalCurrent,
    PreferCurrent,
    AddIncomingOnly,
}

/// Exact input evidence, with real incoming/current roles and source identity.
/// A missing owner is None, never an empty range or synthetic base revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectionalSourceRegion {
    pub source_id: String,
    pub source_role: SourceRole,
    pub range: ByteRange,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectionalOwnerDecision {
    pub id: String,
    pub owner_id: String,
    pub path: String,
    pub action: DirectionalOwnerAction,
    pub incoming: Option<DirectionalSourceRegion>,
    pub current: Option<DirectionalSourceRegion>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectionalOwnerClassification {
    pub sources: Vec<SourceDescriptor>,
    pub decisions: Vec<DirectionalOwnerDecision>,
    pub incoming_owner_order: Vec<String>,
    pub current_owner_order: Vec<String>,
    /// Unowned bytes are not discarded or silently attached to a new owner.
    /// The renderer must account for these separately from owner selection.
    pub incoming_layout: Vec<DirectionalSourceRegion>,
    pub current_layout: Vec<DirectionalSourceRegion>,
}

fn region(source: &SourceDocument, start_byte: usize, end_byte: usize) -> DirectionalSourceRegion {
    DirectionalSourceRegion {
        source_id: source.descriptor().source_id.clone(),
        source_role: source.descriptor().role,
        range: ByteRange { start_byte, end_byte },
        sha256: format!("{:x}", Sha256::digest(&source.bytes()[start_byte..end_byte])),
    }
}

fn owned(source: &SourceDocument, owner: &SourcePreservingOwner) -> DirectionalSourceRegion {
    region(source, owner.start_byte, owner.end_byte)
}

fn layout(
    source: &SourceDocument,
    document: &SourcePreservingOwnerDocument,
) -> Vec<DirectionalSourceRegion> {
    let mut cursor = 0;
    let mut regions = Vec::new();
    for owner in &document.owners {
        if cursor < owner.start_byte {
            regions.push(region(source, cursor, owner.start_byte));
        }
        cursor = owner.end_byte;
    }
    if cursor < source.bytes().len() {
        regions.push(region(source, cursor, source.bytes().len()));
    }
    regions
}

/// Classify current-preferred, add-incoming-only whole-owner decisions.
/// Current-only owners are retained, not treated as deletions. Differing matched
/// owners prefer current by explicit policy, not by a fabricated merge3 base.
/// This does not implement recursive YAML merging or advertise provider support.
/// Callers must derive owners in Rust from validated native syntax facts.
pub fn classify_directional_owners(
    incoming_source: &SourceDocument,
    incoming: &SourcePreservingOwnerDocument,
    current_source: &SourceDocument,
    current: &SourcePreservingOwnerDocument,
) -> Result<DirectionalOwnerClassification, String> {
    if incoming_source.descriptor().role != SourceRole::Incoming
        || current_source.descriptor().role != SourceRole::Current
        || incoming_source.descriptor().source_id == current_source.descriptor().source_id
    {
        return Err("directional merge requires distinct incoming/current source identities".into());
    }
    for (source, document, role) in
        [(incoming_source, incoming, "incoming"), (current_source, current, "current")]
    {
        if source.bytes() != document.source.as_bytes() {
            return Err(format!("{role} analysis differs from the validated source bytes"));
        }
        document.validate(role)?;
    }
    let incoming_map: BTreeMap<_, _> =
        incoming.owners.iter().map(|owner| (owner.id.as_str(), owner)).collect();
    let current_map: BTreeMap<_, _> =
        current.owners.iter().map(|owner| (owner.id.as_str(), owner)).collect();
    let mut seen = BTreeSet::new();
    let mut decisions = Vec::new();
    // Stable classification IDs: current order followed by incoming-only order.
    // Keep both original orders below; this is not a rendering order decision.
    for owner in current.owners.iter().chain(&incoming.owners) {
        if !seen.insert(owner.id.as_str()) {
            continue;
        }
        let candidate = incoming_map.get(owner.id.as_str()).copied();
        let existing = current_map.get(owner.id.as_str()).copied();
        let action = match (candidate, existing) {
            (Some(candidate), Some(existing)) => {
                if candidate.path != existing.path {
                    return Err(
                        "matched owner identities have incompatible structural paths".into()
                    );
                }
                if incoming.source[candidate.start_byte..candidate.end_byte]
                    == current.source[existing.start_byte..existing.end_byte]
                {
                    DirectionalOwnerAction::RetainIdenticalCurrent
                } else {
                    DirectionalOwnerAction::PreferCurrent
                }
            }
            (None, Some(_)) => DirectionalOwnerAction::RetainCurrentOnly,
            (Some(_), None) => DirectionalOwnerAction::AddIncomingOnly,
            (None, None) => unreachable!("identity is from one input"),
        };
        decisions.push(DirectionalOwnerDecision {
            id: format!("directional-decision-{}", decisions.len()),
            owner_id: owner.id.clone(),
            path: owner.path.clone(),
            action,
            incoming: candidate.map(|owner| owned(incoming_source, owner)),
            current: existing.map(|owner| owned(current_source, owner)),
        });
    }
    Ok(DirectionalOwnerClassification {
        sources: vec![incoming_source.descriptor().clone(), current_source.descriptor().clone()],
        decisions,
        incoming_owner_order: incoming.owners.iter().map(|owner| owner.id.clone()).collect(),
        current_owner_order: current.owners.iter().map(|owner| owner.id.clone()).collect(),
        incoming_layout: layout(incoming_source, incoming),
        current_layout: layout(current_source, current),
    })
}
