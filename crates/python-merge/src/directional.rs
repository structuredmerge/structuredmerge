//! Current-preferred whole-declaration insertion. LibCST supplies whitespace-
//! inclusive statement spans; Rust selects identities, attachment and placement.
use ast_merge::{SourcePreservingOwnerDocument, directional_render::DirectionalInsertion};
use std::collections::BTreeMap;
use tree_haver::{ByteRange, service::ParsedResult};

fn full_ranges(
    parsed: &ParsedResult,
    document: &SourcePreservingOwnerDocument,
) -> Result<BTreeMap<String, ByteRange>, String> {
    let analysis = crate::declaration_analysis(parsed)?;
    if &analysis.document != document {
        return Err("directional analysis differs from native ownership".into());
    }
    let mut result = BTreeMap::new();
    let mut previous_end = 0;
    for owner in &document.owners {
        let node = parsed
            .document
            .node(&analysis.owner_node_ids[&owner.id][0])
            .ok_or("missing statement")?;
        let full = node
            .extensions
            .iter()
            .find(|e| {
                e.schema == "structuredmerge.extension/python-libcst/v1"
                    && e.namespace == "python-libcst"
            })
            .and_then(|e| e.payload.get("full_span"))
            .ok_or("directional Python requires whitespace-inclusive native spans")?;
        let offset = |key| {
            full.get(key)
                .and_then(|v| v.as_u64())
                .and_then(|v| usize::try_from(v).ok())
                .ok_or("invalid native full-span offset")
        };
        let range = ByteRange { start_byte: offset("start_byte")?, end_byte: offset("end_byte")? };
        if range.start_byte < previous_end
            || range.start_byte > owner.start_byte
            || range.end_byte < owner.end_byte
            || range.end_byte > document.source.len()
            || !document.source.is_char_boundary(range.start_byte)
            || !document.source.is_char_boundary(range.end_byte)
        {
            return Err("invalid or overlapping native full statement span".into());
        }
        previous_end = range.end_byte;
        result.insert(owner.id.clone(), range);
    }
    Ok(result)
}

/// Preserve current declarations/order/header/footer. Insert incoming-only
/// declarations before the next shared identity, after the previous current
/// statement's trailing trivia and before the anchor's leading trivia. A tail
/// insertion goes before the current module footer. Incoming module header and
/// footer do not replace current module layout. Nested bodies remain opaque.
/// Reordered shared anchors with additions are ambiguous and fail explicitly.
pub fn plan_insertions(
    incoming_parse: &ParsedResult,
    incoming: &SourcePreservingOwnerDocument,
    current_parse: &ParsedResult,
    current: &SourcePreservingOwnerDocument,
) -> Result<Vec<DirectionalInsertion>, String> {
    let incoming_ranges = full_ranges(incoming_parse, incoming)?;
    let current_ranges = full_ranges(current_parse, current)?;
    let current_positions: BTreeMap<_, _> =
        current.owners.iter().enumerate().map(|(i, o)| (o.id.as_str(), i)).collect();
    let has_additions =
        incoming.owners.iter().any(|o| !current_positions.contains_key(o.id.as_str()));
    let shared: Vec<_> =
        incoming.owners.iter().filter_map(|o| current_positions.get(o.id.as_str())).collect();
    if has_additions && shared.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("incoming and current shared declaration anchors are reordered".into());
    }
    let mut result = Vec::new();
    for (index, owner) in incoming.owners.iter().enumerate() {
        if current_positions.contains_key(owner.id.as_str()) {
            continue;
        }
        let next = incoming.owners[index + 1..]
            .iter()
            .find(|o| current_positions.contains_key(o.id.as_str()));
        let offset = match next {
            Some(anchor) => current_ranges[&anchor.id].start_byte,
            None => current
                .owners
                .last()
                .map_or(current.source.len(), |last| current_ranges[&last.id].end_byte),
        };
        result.push(DirectionalInsertion {
            owner_id: owner.id.clone(),
            before_current_owner_id: next.map(|o| o.id.clone()),
            current_offset: offset,
            source_range: incoming_ranges[&owner.id].clone(),
        });
    }
    Ok(result)
}
