//! Current-preferred Go function insertion with exact package/import compatibility.
//! Byte scanning below locates newline framing only, never language constructs.
use ast_merge::{SourcePreservingOwnerDocument, directional_render::DirectionalInsertion};
use std::collections::BTreeMap;
use tree_haver::{ByteRange, NodeRole, service::ParsedResult};

struct Header {
    declarations: Vec<(String, String)>,
    end: usize,
}

fn header(parsed: &ParsedResult) -> Result<Header, String> {
    if !parsed.backend.languages.iter().any(|language| language == "go") {
        return Err("Go directional analysis requires a Go parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root = index.root(parsed.document.output().root_id.as_deref().ok_or("missing Go root")?)?;
    let mut declarations = vec![];
    let mut end = 0;
    let mut seen_function = false;
    for node in index.children(root) {
        match node.kind.as_str() {
            "package_clause" | "import_declaration" if !seen_function => {
                declarations.push((node.kind.clone(), node.source_fragment.clone()));
                // Newline framing only: syntax and comments come from native nodes.
                let bytes = parsed.source.bytes();
                end = bytes[node.span.range.end_byte..]
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(bytes.len(), |offset| node.span.range.end_byte + offset + 1);
                if !trivia(parsed, node.span.range.end_byte, end) {
                    return Err("Go header requires standalone declaration lines".into());
                }
            }
            "function_declaration" => seen_function = true,
            _ if node.role == NodeRole::Comment => {}
            _ => return Err("unsupported Go directional top-level declaration".into()),
        }
    }
    if declarations.first().is_none_or(|(kind, _)| kind != "package_clause")
        || declarations.iter().filter(|(kind, _)| kind == "package_clause").count() != 1
    {
        return Err("Go directional input requires exactly one leading package clause".into());
    }
    Ok(Header { declarations, end })
}

/// Package-only documents are valid directional endpoints, not new merge3 or
/// analysis authority. Reject unsupported nodes before recognizing empty owners.
pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    header(parsed)?;
    let nodes = parsed.normalized_nodes()?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root = index.root(parsed.document.output().root_id.as_deref().ok_or("missing Go root")?)?;
    if index.children(root).iter().any(|node| node.kind == "function_declaration") {
        return crate::typed::owners(parsed);
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    Ok(SourcePreservingOwnerDocument { source: source.into(), owners: vec![] })
}

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
    let mut previous_end = header(parsed)?.end;
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
            return Err("Go directional insertion requires standalone owner lines".into());
        }
        // Package/import declarations stay in current. Leading comments after
        // that native header travel with the first incoming function; later
        // gaps start after the preceding function's complete trailing line.
        let start = previous_end;
        if start > line_start || !trivia(parsed, start, owner.start_byte) {
            return Err("Go directional owner lines overlap or contain unowned code".into());
        }
        result.insert(owner.id.clone(), ByteRange { start_byte: start, end_byte: line_end });
        previous_end = line_end;
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
    if owners(incoming_parse)? != *incoming || owners(current_parse)? != *current {
        return Err("Go directional ownership differs from native facts".into());
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
    let incoming_header = header(incoming_parse)?;
    let current_header = header(current_parse)?;
    if incoming_header.declarations != current_header.declarations {
        return Err("Go function insertion requires identical package/import declarations".into());
    }
    let shared: Vec<_> =
        incoming.owners.iter().filter_map(|owner| positions.get(owner.id.as_str())).collect();
    if shared.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("Go shared anchors are reordered".into());
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
                .map_or(current_header.end, |owner| current_ranges[&owner.id].end_byte),
        };
        let source_range = incoming_ranges[&owner.id].clone();
        // Without a terminating newline, joining whole statements could change
        // Go statement boundaries. Never fabricate a separator and claim source retention.
        if !incoming_parse.source.bytes()[source_range.start_byte..source_range.end_byte]
            .ends_with(b"\n")
            || (offset > 0 && current_parse.source.bytes()[offset - 1] != b'\n')
        {
            return Err("Go directional insertion requires explicit newline boundaries".into());
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
