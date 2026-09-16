//! Exact-source structural diff. Family analyzers supply validated ownership;
//! this module owns identity matching and change classification, never parsing.
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
pub enum OwnerChangeKind {
    Added,
    Deleted,
    Edited,
}

/// A present source region. Absence is represented by None, never a fake range.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffSourceRegion {
    pub source_id: String,
    pub source_role: SourceRole,
    pub range: ByteRange,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnerChange {
    pub id: String,
    pub owner_id: String,
    pub before_path: Option<String>,
    pub after_path: Option<String>,
    pub kind: OwnerChangeKind,
    pub before: Option<DiffSourceRegion>,
    pub after: Option<DiffSourceRegion>,
}

/// Partial diff building block, not the portable provider-result envelope.
/// Owner order and unowned regions remain observable independently of edits.
/// An empty changes list does not imply equal documents or semantic equivalence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnerDiff {
    pub sources: Vec<SourceDescriptor>,
    pub changes: Vec<OwnerChange>,
    pub before_owner_order: Vec<String>,
    pub after_owner_order: Vec<String>,
    pub before_layout: Vec<DiffSourceRegion>,
    pub after_layout: Vec<DiffSourceRegion>,
}

fn region(source: &SourceDocument, start_byte: usize, end_byte: usize) -> DiffSourceRegion {
    DiffSourceRegion {
        source_id: source.descriptor().source_id.clone(),
        source_role: source.descriptor().role,
        range: ByteRange { start_byte, end_byte },
        sha256: format!("{:x}", Sha256::digest(&source.bytes()[start_byte..end_byte])),
    }
}

fn owner_region(source: &SourceDocument, owner: &SourcePreservingOwner) -> DiffSourceRegion {
    region(source, owner.start_byte, owner.end_byte)
}

fn layout(
    source: &SourceDocument,
    document: &SourcePreservingOwnerDocument,
) -> Vec<DiffSourceRegion> {
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

/// Match unique family-owned identities and compare exact owned bytes. Changes
/// are ordered by before-source owner order, then new after-source owners.
/// Do not pass arbitrary host-supplied merge decisions as ownership analysis.
pub fn diff_owner_documents(
    before_source: &SourceDocument,
    before: &SourcePreservingOwnerDocument,
    after_source: &SourceDocument,
    after: &SourcePreservingOwnerDocument,
) -> Result<OwnerDiff, String> {
    if before_source.descriptor().role != SourceRole::Before
        || after_source.descriptor().role != SourceRole::After
        || before_source.descriptor().source_id == after_source.descriptor().source_id
    {
        return Err("diff requires distinct before/after source identities".into());
    }
    for (source, document, role) in
        [(before_source, before, "before"), (after_source, after, "after")]
    {
        if source.bytes() != document.source.as_bytes() {
            return Err(format!("{role} analysis does not describe the validated source bytes"));
        }
        document.validate(role)?;
    }
    let before_map: BTreeMap<_, _> =
        before.owners.iter().map(|owner| (owner.id.as_str(), owner)).collect();
    let after_map: BTreeMap<_, _> =
        after.owners.iter().map(|owner| (owner.id.as_str(), owner)).collect();
    let mut seen = BTreeSet::new();
    let mut changes = Vec::new();
    for owner in before.owners.iter().chain(&after.owners) {
        if !seen.insert(owner.id.as_str()) {
            continue;
        }
        let left = before_map.get(owner.id.as_str()).copied();
        let right = after_map.get(owner.id.as_str()).copied();
        let kind = match (left, right) {
            (Some(left), Some(right)) => {
                if before.source[left.start_byte..left.end_byte]
                    == after.source[right.start_byte..right.end_byte]
                    && left.path == right.path
                {
                    continue;
                }
                OwnerChangeKind::Edited
            }
            (Some(_), None) => OwnerChangeKind::Deleted,
            (None, Some(_)) => OwnerChangeKind::Added,
            (None, None) => unreachable!("identity came from one input"),
        };
        changes.push(OwnerChange {
            id: format!("change-{}", changes.len()),
            owner_id: owner.id.clone(),
            before_path: left.map(|owner| owner.path.clone()),
            after_path: right.map(|owner| owner.path.clone()),
            kind,
            before: left.map(|owner| owner_region(before_source, owner)),
            after: right.map(|owner| owner_region(after_source, owner)),
        });
    }
    Ok(OwnerDiff {
        sources: vec![before_source.descriptor().clone(), after_source.descriptor().clone()],
        changes,
        before_owner_order: before.owners.iter().map(|owner| owner.id.clone()).collect(),
        after_owner_order: after.owners.iter().map(|owner| owner.id.clone()).collect(),
        before_layout: layout(before_source, before),
        after_layout: layout(after_source, after),
    })
}
