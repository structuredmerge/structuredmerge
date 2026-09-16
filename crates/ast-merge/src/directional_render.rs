//! Byte-preserving execution of a Rust family insertion plan. This module does
//! not infer comment attachment or insertion placement from source text.
use crate::{
    SourcePreservingOwnerDocument,
    owner_merge2::{
        DirectionalOwnerAction, DirectionalOwnerClassification, classify_directional_owners,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{
    ByteRange,
    source::{SourceDocument, SourceRole},
};

/// A family-derived incoming-only insertion. The range must contain the entire
/// owner and may include its explicitly selected unowned layout, but never
/// another owner. None places it after all current bytes; Some places it before
/// the named current owner's leading gap. Order within a slot is explicit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectionalInsertion {
    pub owner_id: String,
    pub before_current_owner_id: Option<String>,
    pub source_range: ByteRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectionalByteSegment {
    pub id: String,
    pub source_id: String,
    pub source_role: SourceRole,
    pub source_range: ByteRange,
    pub output_range: ByteRange,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectionalRender {
    pub output: String,
    pub classification: DirectionalOwnerClassification,
    pub segments: Vec<DirectionalByteSegment>,
    pub owner_order: Vec<String>,
}

/// Independently verify output partition, exact origins and complete, ordered
/// retention of current bytes. Owner semantics still require a native reparse.
pub fn verify_directional_segments(
    output: &[u8],
    incoming: &SourceDocument,
    current: &SourceDocument,
    segments: &[DirectionalByteSegment],
) -> Result<(), String> {
    if incoming.descriptor().role != SourceRole::Incoming
        || current.descriptor().role != SourceRole::Current
        || incoming.descriptor().source_id == current.descriptor().source_id
    {
        return Err("expected distinct incoming/current sources".into());
    }
    let mut ids = BTreeSet::new();
    let mut cursor = 0;
    let mut current_cursor = 0;
    for segment in segments {
        if segment.id.is_empty()
            || !ids.insert(&segment.id)
            || segment.output_range.start_byte != cursor
            || segment.output_range.end_byte <= cursor
            || segment.source_range.start_byte >= segment.source_range.end_byte
        {
            return Err("invalid directional output partition".into());
        }
        let source = match segment.source_role {
            SourceRole::Incoming => incoming,
            SourceRole::Current => current,
            _ => return Err("non-directional source role".into()),
        };
        if segment.source_id != source.descriptor().source_id {
            return Err("directional source identity mismatch".into());
        }
        let original = source
            .bytes()
            .get(segment.source_range.start_byte..segment.source_range.end_byte)
            .ok_or("invalid directional source range")?;
        let rendered = output
            .get(segment.output_range.start_byte..segment.output_range.end_byte)
            .ok_or("invalid directional output range")?;
        if original != rendered || segment.sha256 != format!("{:x}", Sha256::digest(original)) {
            return Err("directional source bytes or digest mismatch".into());
        }
        if segment.source_role == SourceRole::Current {
            if segment.source_range.start_byte != current_cursor {
                return Err("current bytes were omitted, duplicated or reordered".into());
            }
            current_cursor = segment.source_range.end_byte;
        }
        cursor = segment.output_range.end_byte;
    }
    if cursor != output.len() || current_cursor != current.bytes().len() {
        return Err("incomplete directional byte evidence".into());
    }
    Ok(())
}

fn append(
    output: &mut String,
    segments: &mut Vec<DirectionalByteSegment>,
    source: &SourceDocument,
    range: ByteRange,
) {
    if range.start_byte == range.end_byte {
        return;
    }
    let bytes = &source.bytes()[range.start_byte..range.end_byte];
    let start_byte = output.len();
    output.push_str(std::str::from_utf8(bytes).expect("ranges validated before rendering"));
    segments.push(DirectionalByteSegment {
        id: format!("directional-fragment-{}", segments.len()),
        source_id: source.descriptor().source_id.clone(),
        source_role: source.descriptor().role,
        source_range: range,
        output_range: ByteRange { start_byte, end_byte: output.len() },
        sha256: format!("{:x}", Sha256::digest(bytes)),
    });
}

/// Render all current bytes plus every incoming-only owner exactly once using
/// family-provided placements. Reject incomplete/ambiguous plans before calling
/// verify. The verification callback must reparse through TreeHaver and return
/// Rust-derived ownership; it must not echo a constructed expected document.
/// Failure returns no output. This does not advertise merge2 facade support.
pub fn render_directional_owners(
    incoming_source: &SourceDocument,
    incoming: &SourcePreservingOwnerDocument,
    current_source: &SourceDocument,
    current: &SourcePreservingOwnerDocument,
    insertions: &[DirectionalInsertion],
    verify: impl FnOnce(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> Result<DirectionalRender, String> {
    let classification =
        classify_directional_owners(incoming_source, incoming, current_source, current)?;
    let additions: BTreeSet<_> = classification
        .decisions
        .iter()
        .filter(|d| d.action == DirectionalOwnerAction::AddIncomingOnly)
        .map(|d| d.owner_id.as_str())
        .collect();
    let current_ids: BTreeSet<_> = current.owners.iter().map(|o| o.id.as_str()).collect();
    let incoming_map: BTreeMap<_, _> = incoming.owners.iter().map(|o| (o.id.as_str(), o)).collect();
    let mut planned = BTreeSet::new();
    let mut ranges = Vec::new();
    for insertion in insertions {
        if !additions.contains(insertion.owner_id.as_str())
            || !planned.insert(insertion.owner_id.as_str())
            || insertion
                .before_current_owner_id
                .as_deref()
                .is_some_and(|id| !current_ids.contains(id))
        {
            return Err("invalid or duplicate directional insertion owner/anchor".into());
        }
        let owner = incoming_map[insertion.owner_id.as_str()];
        let range = &insertion.source_range;
        if range.start_byte > owner.start_byte
            || range.end_byte < owner.end_byte
            || range.end_byte > incoming.source.len()
            || !incoming.source.is_char_boundary(range.start_byte)
            || !incoming.source.is_char_boundary(range.end_byte)
            || incoming.owners.iter().any(|other| {
                other.id != owner.id
                    && range.start_byte < other.end_byte
                    && other.start_byte < range.end_byte
            })
        {
            return Err("insertion must contain exactly one complete incoming owner".into());
        }
        ranges.push(range.clone());
    }
    if planned != additions {
        return Err("missing incoming-only insertion".into());
    }
    ranges.sort_by_key(|r| r.start_byte);
    if ranges.windows(2).any(|pair| pair[0].end_byte > pair[1].start_byte) {
        return Err("overlapping incoming insertion ranges".into());
    }
    let mut output = String::new();
    let mut segments = Vec::new();
    let mut selected = Vec::new();
    let mut cursor = 0;
    for owner in current.owners.iter().map(Some).chain(std::iter::once(None)) {
        // For the final slot retain the current suffix before appending new owners.
        if owner.is_none() {
            append(
                &mut output,
                &mut segments,
                current_source,
                ByteRange { start_byte: cursor, end_byte: current.source.len() },
            );
        }
        for insertion in insertions
            .iter()
            .filter(|i| i.before_current_owner_id.as_deref() == owner.map(|o| o.id.as_str()))
        {
            append(&mut output, &mut segments, incoming_source, insertion.source_range.clone());
            selected.push((incoming_map[insertion.owner_id.as_str()], incoming));
        }
        if let Some(owner) = owner {
            append(
                &mut output,
                &mut segments,
                current_source,
                ByteRange { start_byte: cursor, end_byte: owner.end_byte },
            );
            cursor = owner.end_byte;
            selected.push((owner, current));
        }
    }
    verify_directional_segments(output.as_bytes(), incoming_source, current_source, &segments)?;
    let reparsed = verify(&output)?;
    reparsed.validate("directional output")?;
    if reparsed.source != output
        || reparsed.owners.len() != selected.len()
        || reparsed.owners.iter().zip(&selected).any(|(actual, (expected, source))| {
            actual.id != expected.id
                || actual.path != expected.path
                || actual.fingerprint != expected.fingerprint
                || reparsed.source[actual.start_byte..actual.end_byte]
                    != source.source[expected.start_byte..expected.end_byte]
        })
    {
        return Err("directional output ownership verification failed".into());
    }
    Ok(DirectionalRender {
        output,
        classification,
        segments,
        owner_order: selected.iter().map(|(owner, _)| owner.id.clone()).collect(),
    })
}
