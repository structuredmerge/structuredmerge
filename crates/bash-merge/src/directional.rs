//! Current-preferred Bash owner insertion. Syntax identifies owners/comments;
//! byte scanning below locates newline framing only, never language constructs.
use ast_merge::{SourcePreservingOwnerDocument, directional_render::DirectionalInsertion};
use std::collections::BTreeMap;
use tree_haver::{ByteRange, NodeRole, service::ParsedResult};

fn trivia(parsed: &ParsedResult, start: usize, end: usize) -> bool {
    let bytes = parsed.source.bytes();
    if start > end || end > bytes.len() {
        return false;
    }
    let mut comments: Vec<_> = parsed
        .document
        .output()
        .nodes
        .iter()
        .filter(|node| {
            node.role == NodeRole::Comment
                && node.span.range.start_byte >= start
                && node.span.range.end_byte <= end
        })
        .map(|node| &node.span.range)
        .collect();
    comments.sort_by_key(|range| range.start_byte);
    let mut cursor = start;
    for range in comments {
        if range.start_byte < cursor
            || !bytes[cursor..range.start_byte].iter().all(u8::is_ascii_whitespace)
        {
            return false;
        }
        cursor = range.end_byte;
    }
    bytes[cursor..end].iter().all(u8::is_ascii_whitespace)
}

fn ranges(
    parsed: &ParsedResult,
    document: &SourcePreservingOwnerDocument,
) -> Result<BTreeMap<String, ByteRange>, String> {
    let bytes = parsed.source.bytes();
    let mut result = BTreeMap::new();
    let mut previous_end = None;
    for owner in &document.owners {
        let line_start = bytes[..owner.start_byte]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |offset| offset + 1);
        let content_end = owner.end_byte - usize::from(bytes[owner.end_byte - 1] == b'\n');
        let line_end = bytes[content_end..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| content_end + offset + 1);
        if !bytes[line_start..owner.start_byte].iter().all(u8::is_ascii_whitespace)
            || !trivia(parsed, owner.end_byte, line_end)
        {
            return Err("Bash directional insertion requires standalone owner lines".into());
        }
        // Keep the first document header (including a shebang) at document scope.
        // Later leading gaps belong to the following owner, after the previous
        // owner's complete trailing line, so inline comments stay with it.
        let start = previous_end.unwrap_or(line_start);
        if start > line_start || !trivia(parsed, start, owner.start_byte) {
            return Err("Bash directional owner lines overlap or contain unowned code".into());
        }
        result.insert(owner.id.clone(), ByteRange { start_byte: start, end_byte: line_end });
        previous_end = Some(line_end);
    }
    Ok(result)
}

/// Retain every current byte and shared owner. Insert incoming-only owners before
/// the next shared anchor, or before the current footer. Header/footer transfer,
/// reordered anchors with additions and ambiguous same-line placement are not
/// silently synthesized. The shared executor reparses and checks owner order.
pub fn plan_insertions(
    incoming_parse: &ParsedResult,
    incoming: &SourcePreservingOwnerDocument,
    current_parse: &ParsedResult,
    current: &SourcePreservingOwnerDocument,
) -> Result<Vec<DirectionalInsertion>, String> {
    if crate::typed::owners(incoming_parse)? != *incoming
        || crate::typed::owners(current_parse)? != *current
    {
        return Err("Bash directional ownership differs from native facts".into());
    }
    let positions: BTreeMap<_, _> = current
        .owners
        .iter()
        .enumerate()
        .map(|(index, owner)| (owner.id.as_str(), index))
        .collect();
    if incoming.owners.iter().all(|owner| positions.contains_key(owner.id.as_str())) {
        return Ok(vec![]);
    }
    let shared: Vec<_> =
        incoming.owners.iter().filter_map(|owner| positions.get(owner.id.as_str())).collect();
    if shared.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("Bash shared anchors are reordered".into());
    }
    let incoming_ranges = ranges(incoming_parse, incoming)?;
    let current_ranges = ranges(current_parse, current)?;
    let mut result = vec![];
    for (index, owner) in incoming.owners.iter().enumerate() {
        if positions.contains_key(owner.id.as_str()) {
            continue;
        }
        let next = incoming.owners[index + 1..]
            .iter()
            .find(|owner| positions.contains_key(owner.id.as_str()));
        let offset = match next {
            Some(anchor) => current_ranges[&anchor.id].start_byte,
            None => current
                .owners
                .last()
                .map_or(current.source.len(), |owner| current_ranges[&owner.id].end_byte),
        };
        let source_range = incoming_ranges[&owner.id].clone();
        // Without a terminating newline, joining whole statements could change
        // shell semantics. Never fabricate a separator and claim source retention.
        if !incoming_parse.source.bytes()[source_range.start_byte..source_range.end_byte]
            .ends_with(b"\n")
            || (offset > 0 && current_parse.source.bytes()[offset - 1] != b'\n')
        {
            return Err("Bash directional insertion requires explicit newline boundaries".into());
        }
        result.push(DirectionalInsertion {
            owner_id: owner.id.clone(),
            before_current_owner_id: next.map(|owner| owner.id.clone()),
            current_offset: offset,
            source_range,
        });
    }
    Ok(result)
}
