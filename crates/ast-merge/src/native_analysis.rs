//! Family-owned structural analysis with references to the actual native tree.
//! This building block is not the complete portable analysis-result contract:
//! comment attachment and layout-controller decisions require further analysis.
use crate::SourcePreservingOwnerDocument;
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::service::ParsedResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeOwnerAnalysis {
    pub document: SourcePreservingOwnerDocument,
    /// Ordered native nodes used by family analysis to establish each owner.
    /// A YAML mapping entry has key/value nodes, not a fabricated pair node.
    pub owner_node_ids: BTreeMap<String, Vec<String>>,
    /// Parser comment nodes assigned to each owner by shared ast-merge rules.
    pub attached_comment_node_ids: BTreeMap<String, Vec<String>>,
}

impl NativeOwnerAnalysis {
    /// The same byte-gap plan consumed by the owner renderer. This is not
    /// semantic comment attachment or permission to transfer a deleted gap.
    pub fn layout_gaps(&self) -> Result<Vec<crate::layout::SourceLayoutGap>, String> {
        crate::layout::source_layout_gaps(&self.document)
    }

    /// Adjacency references only: sharing a gap does not give both owners
    /// emission control. The gap's explicit controller remains authoritative.
    pub fn layout_attachments(&self) -> Result<Vec<crate::layout::LayoutAttachment>, String> {
        let gaps = self.layout_gaps()?;
        Ok(self
            .document
            .owners
            .iter()
            .enumerate()
            .map(|(index, owner)| {
                let nonempty_id = |gap: &crate::layout::SourceLayoutGap| {
                    (!gap.range.is_empty()).then(|| gap.id.clone())
                };
                crate::layout::LayoutAttachment {
                    owner_id: owner.id.clone(),
                    leading_gap_id: nonempty_id(&gaps[index]),
                    trailing_gap_id: nonempty_id(&gaps[index + 1]),
                }
            })
            .collect())
    }

    pub fn validate(&self, parsed: &ParsedResult) -> Result<(), String> {
        if !parsed.document.output().ok
            || parsed.source.descriptor() != &parsed.document.output().source
            || self.document.source.as_bytes() != parsed.source.bytes()
        {
            return Err("analysis does not describe the validated native parse".into());
        }
        self.document.validate("analysis")?;
        if self.owner_node_ids.len() != self.document.owners.len() {
            return Err("analysis node-reference catalog differs from owner catalog".into());
        }
        if self.attached_comment_node_ids.keys().any(|id| !self.owner_node_ids.contains_key(id)) {
            return Err("comment attachment references an unknown owner".into());
        }
        for owner in &self.document.owners {
            let ids = self.owner_node_ids.get(&owner.id).ok_or("missing owner node references")?;
            if ids.is_empty() {
                return Err("owner has no native node references".into());
            }
            let mut seen = BTreeSet::new();
            let mut start = usize::MAX;
            let mut end = 0;
            let mut previous_start = 0;
            for id in ids {
                if !seen.insert(id) {
                    return Err("duplicate owner node reference".into());
                }
                let node = parsed.document.node(id).ok_or("unresolved owner node reference")?;
                let range = &node.span.range;
                if range.start_byte < previous_start {
                    return Err("owner node references have invalid ranges or order".into());
                }
                previous_start = range.start_byte;
                start = start.min(range.start_byte);
                end = end.max(range.end_byte);
            }
            let mut previous_comment_start = 0;
            if let Some(comment_ids) = self.attached_comment_node_ids.get(&owner.id) {
                for id in comment_ids {
                    if !seen.insert(id) {
                        return Err("duplicate structural/comment owner node reference".into());
                    }
                    let node =
                        parsed.document.node(id).ok_or("unresolved attached comment node")?;
                    if node.role != tree_haver::NodeRole::Comment
                        || node.span.range.start_byte < previous_comment_start
                    {
                        return Err(
                            "attached comment references have invalid roles or order".into()
                        );
                    }
                    previous_comment_start = node.span.range.start_byte;
                    start = start.min(node.span.range.start_byte);
                    end = end.max(node.span.range.end_byte);
                }
            }
            if start != owner.start_byte || end != owner.end_byte {
                return Err("native syntax and attached comment references do not establish owner boundaries".into());
            }
        }
        Ok(())
    }
}
