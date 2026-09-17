//! Current-preferred TypeScript declaration insertion over native ownership.
//! Import declarations stay at module scope. Byte scans
//! locate newline framing only; syntax and comment kinds come from native nodes.
use ast_merge::{SourcePreservingOwnerDocument, directional_render::DirectionalInsertion};
use std::collections::BTreeMap;
use tree_haver::{ByteRange, NodeRole, service::ParsedResult};

/// Empty/import-only documents are directional endpoints, not wider merge3 support.
pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    if !parsed
        .backend
        .languages
        .iter()
        .any(|language| matches!(language.as_str(), "typescript" | "tsx"))
    {
        return Err("TypeScript directional analysis requires a TypeScript parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root = index
        .root(parsed.document.output().root_id.as_deref().ok_or("missing TypeScript root")?)?;
    if index
        .children(root)
        .iter()
        .any(|node| node.role != NodeRole::Comment && node.kind != "import_statement")
    {
        return crate::typed::owners(parsed);
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    Ok(SourcePreservingOwnerDocument { source: source.into(), owners: vec![] })
}

struct Layout {
    imports: Vec<String>,
    ranges: BTreeMap<String, ByteRange>,
    empty_offset: usize,
}

fn layout(
    parsed: &ParsedResult,
    document: &SourcePreservingOwnerDocument,
) -> Result<Layout, String> {
    let nodes = parsed.normalized_nodes()?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root = index
        .root(parsed.document.output().root_id.as_deref().ok_or("missing TypeScript root")?)?;
    let children = index.children(root);
    let bytes = parsed.source.bytes();
    let comments: BTreeMap<_, _> = children
        .iter()
        .filter(|node| node.role == NodeRole::Comment)
        .map(|node| (node.span.range.start_byte, &node.span.range))
        .collect();
    let owners_by_span: BTreeMap<_, _> =
        document.owners.iter().map(|owner| ((owner.start_byte, owner.end_byte), owner)).collect();
    let trivia = |start: usize, end: usize| {
        if start > end || end > bytes.len() {
            return false;
        }
        let mut cursor = start;
        for range in comments.range(start..end).map(|(_, range)| range) {
            if range.end_byte > end {
                continue;
            }
            if range.start_byte < cursor
                || !bytes[cursor..range.start_byte].iter().all(u8::is_ascii_whitespace)
            {
                return false;
            }
            cursor = range.end_byte;
        }
        bytes[cursor..end].iter().all(u8::is_ascii_whitespace)
    };
    let mut result = Layout { imports: vec![], ranges: BTreeMap::new(), empty_offset: bytes.len() };
    let mut previous_end = 0;
    for node in &children {
        let is_import = node.kind == "import_statement";
        let owner = owners_by_span.get(&(node.span.range.start_byte, node.span.range.end_byte));
        if !is_import && owner.is_none() {
            continue;
        }
        let start = node.span.range.start_byte;
        let end = node.span.range.end_byte;
        let line_start =
            bytes[..start].iter().rposition(|byte| *byte == b'\n').map_or(0, |offset| offset + 1);
        let content_end = end - usize::from(end > start && bytes[end - 1] == b'\n');
        let line_end = bytes[content_end..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| content_end + offset + 1);
        if previous_end > line_start
            || !bytes[line_start..start].iter().all(u8::is_ascii_whitespace)
            || !trivia(previous_end, start)
            || !trivia(end, line_end)
        {
            return Err(
                "TypeScript directional insertion requires standalone native declaration lines"
                    .into(),
            );
        }
        if is_import {
            result.imports.push(node.source_fragment.clone());
        }
        if is_import {
            // Module-scoped barriers never travel with an inserted declaration.
            result.empty_offset = line_end;
        } else if let Some(owner) = owner {
            result.ranges.insert(
                owner.id.clone(),
                ByteRange {
                    // Before the first declaration/import, comments are a module
                    // header. Do not move current directives after new statements.
                    // After imports/owners, native leading trivia travels with additions.
                    start_byte: if previous_end == 0 { line_start } else { previous_end },
                    end_byte: line_end,
                },
            );
        }
        previous_end = line_end;
    }
    Ok(result)
}

/// Preserve every current byte and shared declaration; copy incoming-only owners
/// with their leading trivia before the next shared anchor, or before the footer.
/// Exact import declarations must agree when adding owners. This is not import/name
/// resolution, compiler directive interpretation or semantic dependency reconciliation.
pub fn plan_insertions(
    incoming_parse: &ParsedResult,
    incoming: &SourcePreservingOwnerDocument,
    current_parse: &ParsedResult,
    current: &SourcePreservingOwnerDocument,
) -> Result<Vec<DirectionalInsertion>, String> {
    if owners(incoming_parse)? != *incoming || owners(current_parse)? != *current {
        return Err("TypeScript directional ownership differs from native facts".into());
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
    let incoming_layout = layout(incoming_parse, incoming)?;
    let current_layout = layout(current_parse, current)?;
    if incoming_layout.imports != current_layout.imports {
        return Err(
            "TypeScript declaration insertion requires identical import declarations".into()
        );
    }
    let shared: Vec<_> =
        incoming.owners.iter().filter_map(|owner| positions.get(owner.id.as_str())).collect();
    if shared.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("TypeScript shared anchors are reordered".into());
    }
    let mut result = vec![];
    for (index, owner) in incoming.owners.iter().enumerate() {
        if positions.contains_key(owner.id.as_str()) {
            continue;
        }
        let next = incoming.owners[index + 1..]
            .iter()
            .find(|owner| positions.contains_key(owner.id.as_str()));
        let offset = match next {
            Some(anchor) => current_layout.ranges[&anchor.id].start_byte,
            None => current.owners.last().map_or(current_layout.empty_offset, |owner| {
                current_layout.ranges[&owner.id].end_byte
            }),
        };
        let source_range = incoming_layout.ranges[&owner.id].clone();
        if !incoming_parse.source.bytes()[source_range.start_byte..source_range.end_byte]
            .ends_with(b"\n")
            || (offset > 0 && current_parse.source.bytes()[offset - 1] != b'\n')
        {
            return Err(
                "TypeScript directional insertion requires explicit newline boundaries".into()
            );
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
