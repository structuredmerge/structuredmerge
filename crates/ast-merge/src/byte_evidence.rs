//! Exact byte-origin records emitted by the owner renderer. These records prove
//! the output partition, not the complete policy/disposition envelope of Slice 1027.
use crate::SourceRevision;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tree_haver::ByteRange;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceByteSegment {
    pub id: String,
    pub revision: SourceRevision,
    pub source_range: ByteRange,
    pub output_range: ByteRange,
    pub sha256: String,
    pub owner_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteEvidenceError {
    InvalidIdentity,
    InvalidPartition,
    MissingSource,
    InvalidRange,
    ByteMismatch,
    DigestMismatch,
}

pub fn verify_source_byte_segments(
    output: &[u8],
    sources: &HashMap<SourceRevision, &[u8]>,
    segments: &[SourceByteSegment],
) -> Result<(), ByteEvidenceError> {
    let mut ids = std::collections::HashSet::new();
    let mut cursor = 0;
    for segment in segments {
        if segment.id.is_empty() || !ids.insert(&segment.id) {
            return Err(ByteEvidenceError::InvalidIdentity);
        }
        let out = &segment.output_range;
        if out.start_byte != cursor || out.end_byte <= out.start_byte {
            return Err(ByteEvidenceError::InvalidPartition);
        }
        let output_bytes =
            output.get(out.start_byte..out.end_byte).ok_or(ByteEvidenceError::InvalidRange)?;
        let source = sources.get(&segment.revision).ok_or(ByteEvidenceError::MissingSource)?;
        let range = &segment.source_range;
        let source_bytes =
            source.get(range.start_byte..range.end_byte).ok_or(ByteEvidenceError::InvalidRange)?;
        if source_bytes != output_bytes {
            return Err(ByteEvidenceError::ByteMismatch);
        }
        if format!("{:x}", Sha256::digest(source_bytes)) != segment.sha256 {
            return Err(ByteEvidenceError::DigestMismatch);
        }
        cursor = out.end_byte;
    }
    if cursor != output.len() {
        return Err(ByteEvidenceError::InvalidPartition);
    }
    Ok(())
}

pub(crate) fn append_source_segment(
    output: &mut String,
    segments: &mut Vec<SourceByteSegment>,
    source: &str,
    revision: SourceRevision,
    source_range: ByteRange,
    owner_id: Option<String>,
) {
    if source_range.start_byte == source_range.end_byte {
        return;
    }
    let bytes = &source[source_range.start_byte..source_range.end_byte];
    let start_byte = output.len();
    output.push_str(bytes);
    segments.push(SourceByteSegment {
        id: format!("source-fragment-{}", segments.len()),
        revision,
        source_range,
        output_range: ByteRange { start_byte, end_byte: output.len() },
        sha256: format!("{:x}", Sha256::digest(bytes.as_bytes())),
        owner_id,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_partition_includes_bom_unicode_crlf_and_no_final_newline() {
        let ours = "\u{feff}é\r\n";
        let theirs = "beta";
        let sources = HashMap::from([
            (SourceRevision::Ours, ours.as_bytes()),
            (SourceRevision::Theirs, theirs.as_bytes()),
        ]);
        let mut output = String::new();
        let mut segments = Vec::new();
        append_source_segment(
            &mut output,
            &mut segments,
            ours,
            SourceRevision::Ours,
            ByteRange { start_byte: 0, end_byte: ours.len() },
            None,
        );
        append_source_segment(
            &mut output,
            &mut segments,
            theirs,
            SourceRevision::Theirs,
            ByteRange { start_byte: 0, end_byte: theirs.len() },
            Some("beta".into()),
        );
        assert_eq!(output, "\u{feff}é\r\nbeta");
        assert_eq!(verify_source_byte_segments(output.as_bytes(), &sources, &segments), Ok(()));
        let original = segments.clone();
        segments[1].output_range.start_byte += 1;
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::InvalidPartition)
        );
        segments = original.clone();
        segments[1].output_range.start_byte -= 1;
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::InvalidPartition)
        );
        segments = original.clone();
        segments[0].sha256 = "wrong".into();
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::DigestMismatch)
        );
        segments = original.clone();
        segments[0].source_range.end_byte = usize::MAX;
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::InvalidRange)
        );
        segments = original.clone();
        segments[1].revision = SourceRevision::Base;
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::MissingSource)
        );
        segments = original.clone();
        segments[1].id = segments[0].id.clone();
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &segments),
            Err(ByteEvidenceError::InvalidIdentity)
        );
        let changed = output.replace("beta", "BETA");
        assert_eq!(
            verify_source_byte_segments(changed.as_bytes(), &sources, &original),
            Err(ByteEvidenceError::ByteMismatch)
        );
        assert_eq!(
            verify_source_byte_segments(output.as_bytes(), &sources, &original[..1]),
            Err(ByteEvidenceError::InvalidPartition)
        );
        assert_eq!(verify_source_byte_segments(b"", &sources, &[]), Ok(()));
    }
}
