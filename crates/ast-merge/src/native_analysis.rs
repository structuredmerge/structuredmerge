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
}

impl NativeOwnerAnalysis {
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
                if range.start_byte < owner.start_byte
                    || range.end_byte > owner.end_byte
                    || range.start_byte < previous_start
                {
                    return Err("owner node references have invalid ranges or order".into());
                }
                previous_start = range.start_byte;
                start = start.min(range.start_byte);
                end = end.max(range.end_byte);
            }
            if start != owner.start_byte || end != owner.end_byte {
                return Err("native node references do not establish the owner boundaries".into());
            }
        }
        Ok(())
    }
}
