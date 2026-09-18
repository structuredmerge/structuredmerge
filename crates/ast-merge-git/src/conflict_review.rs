//! Read-only Git conflict-marker framing, not language parsing or resolution.
//! Git's marker protocol has no language AST: an explicit byte/line state
//! machine recognizes framing without interpreting or classifying source syntax.
use serde::{Deserialize, Serialize};
use tree_haver::{
    ByteRange,
    source::{SourceDescriptor, SourceEncoding, SourceRole, source_input},
};

pub const MAX_REVIEW_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REVIEW_REGIONS: usize = 10_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictReviewRegion {
    pub id: String,
    pub range: ByteRange,
    pub ours: ByteRange,
    pub base: Option<ByteRange>,
    pub theirs: ByteRange,
    pub start_line: usize,
    pub separator_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictReview {
    pub schema: String,
    pub source: SourceDescriptor,
    pub marker_size: usize,
    pub regions: Vec<ConflictReviewRegion>,
    pub semantic_conflicts_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewError {
    pub code: &'static str,
    pub message: &'static str,
    pub line: Option<usize>,
}

fn error(code: &'static str, message: &'static str, line: Option<usize>) -> ReviewError {
    ReviewError { code, message, line }
}

fn range(start: usize, end: usize) -> ByteRange {
    ByteRange { start_byte: start, end_byte: end }
}

struct Open {
    start: usize,
    start_line: usize,
    ours: usize,
    base: Option<(usize, usize)>,
    separator: Option<(usize, usize, usize)>,
}

fn marker(line: &[u8], size: usize) -> Option<u8> {
    let first = *line.first()?;
    if !matches!(first, b'<' | b'|' | b'=' | b'>')
        || line.len() < size
        || !line[..size].iter().all(|byte| *byte == first)
    {
        return None;
    }
    let suffix = &line[size..];
    if suffix.is_empty() || (first != b'=' && matches!(suffix.first(), Some(b' ' | b'\t'))) {
        Some(first)
    } else {
        None
    }
}

/// Preserve original bytes (including CRLF and absent final newline). Ranges
/// exclude framing from alternatives and include framing in the whole region.
/// Incomplete, nested or misordered recognized markers reject the whole review;
/// no partial regions are returned as a successful report.
pub fn review_conflicts(bytes: Vec<u8>, marker_size: usize) -> Result<ConflictReview, ReviewError> {
    if bytes.len() > MAX_REVIEW_BYTES {
        return Err(error("conflict.input_limit", "conflict review exceeds 8 MiB", None));
    }
    if !(1..=128).contains(&marker_size) {
        return Err(error("conflict.marker_size", "marker size must be between 1 and 128", None));
    }
    let input =
        source_input("conflict-review".into(), SourceRole::Source, SourceEncoding::Utf8, bytes)
            .map_err(|_| {
                error("conflict.invalid_utf8", "conflict review requires UTF-8 source", None)
            })?;
    let mut regions = vec![];
    let mut open: Option<Open> = None;
    let mut offset = 0;
    for (index, raw) in input.bytes.split_inclusive(|byte| *byte == b'\n').enumerate() {
        let line_number = index + 1;
        let end = offset + raw.len();
        let content = raw.strip_suffix(b"\n").unwrap_or(raw);
        let content = content.strip_suffix(b"\r").unwrap_or(content);
        // A UTF-8 BOM belongs to the source, not the first marker or its range.
        let bom = if offset == 0 && content.starts_with(&[0xef, 0xbb, 0xbf]) { 3 } else { 0 };
        let content = &content[bom..];
        let invalid = || {
            error(
                "conflict.invalid_markers",
                "incomplete, nested or misordered conflict markers",
                Some(line_number),
            )
        };
        match marker(content, marker_size) {
            Some(b'<') => {
                if open.is_some() {
                    return Err(invalid());
                }
                if regions.len() >= MAX_REVIEW_REGIONS {
                    return Err(error(
                        "conflict.region_limit",
                        "too many conflict regions",
                        Some(line_number),
                    ));
                }
                open = Some(Open {
                    start: offset + bom,
                    start_line: line_number,
                    ours: end,
                    base: None,
                    separator: None,
                });
            }
            Some(b'|') => {
                let current = open.as_mut().ok_or_else(invalid)?;
                if current.base.is_some() || current.separator.is_some() {
                    return Err(invalid());
                }
                current.base = Some((offset, end));
            }
            Some(b'=') => {
                let current = open.as_mut().ok_or_else(invalid)?;
                if current.separator.is_some() {
                    return Err(invalid());
                }
                current.separator = Some((offset, end, line_number));
            }
            Some(b'>') => {
                let current = open.take().ok_or_else(invalid)?;
                let (separator_start, theirs_start, separator_line) =
                    current.separator.ok_or_else(invalid)?;
                regions.push(ConflictReviewRegion {
                    id: format!("conflict.{}", regions.len()),
                    range: range(current.start, end),
                    ours: range(current.ours, current.base.map_or(separator_start, |base| base.0)),
                    base: current.base.map(|base| range(base.1, separator_start)),
                    theirs: range(theirs_start, offset),
                    start_line: current.start_line,
                    separator_line,
                    end_line: line_number,
                });
            }
            _ => {}
        }
        offset = end;
    }
    if let Some(current) = open {
        return Err(error(
            "conflict.invalid_markers",
            "unterminated conflict region",
            Some(current.start_line),
        ));
    }
    Ok(ConflictReview {
        schema: "structuredmerge.conflict-review/v1".into(),
        source: input.descriptor,
        marker_size,
        regions,
        semantic_conflicts_verified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff3_ranges_preserve_unicode_crlf_and_empty_sides() {
        let bytes = "前\r\n<<<<<<< ours\r\n雪\r\n||||||| base\r\n=======\r\n後\r\n>>>>>>> theirs"
            .as_bytes();
        let review = review_conflicts(bytes.to_vec(), 7).unwrap();
        let region = &review.regions[0];
        let slice = |range: &ByteRange| &bytes[range.start_byte as usize..range.end_byte as usize];
        assert_eq!(slice(&region.ours), "雪\r\n".as_bytes());
        assert_eq!(slice(region.base.as_ref().unwrap()), b"");
        assert_eq!(slice(&region.theirs), "後\r\n".as_bytes());
        assert_eq!(region.range.start_byte, 5);
        assert_eq!(region.range.end_byte, bytes.len());
        assert_eq!((region.start_line, region.separator_line, region.end_line), (2, 5, 7));
        assert!(!review.source.final_newline);
        assert_eq!(review.source.byte_length, bytes.len() as u64);
        assert!(!review.semantic_conflicts_verified);
    }

    #[test]
    fn ordered_regions_and_exact_marker_width() {
        let block = b"<<<<<<<<< ours\n=========\nx\n>>>>>>>>> theirs\n";
        let bytes = [block.as_slice(), block.as_slice()].concat();
        let review = review_conflicts(bytes.clone(), 9).unwrap();
        assert_eq!(review.regions.len(), 2);
        assert!(review.regions[0].base.is_none());
        assert_eq!(review.regions[0].ours.start_byte, review.regions[0].ours.end_byte);
        assert_eq!(review.regions[0].range.end_byte, review.regions[1].range.start_byte);
        assert!(review_conflicts(bytes, 7).unwrap().regions.is_empty());
        assert!(
            review_conflicts(b"<<<<<<<not-a-marker\n=======not-a-marker\n".to_vec(), 7)
                .unwrap()
                .regions
                .is_empty()
        );
    }

    #[test]
    fn malformed_framing_rejects_without_partial_review() {
        for source in [
            "<<<<<<< ours\nx\n",
            "=======\n",
            ">>>>>>> theirs\n",
            "||||||| base\n",
            "<<<<<<< ours\n>>>>>>> theirs\n",
            "<<<<<<< ours\n<<<<<<< nested\n",
            "<<<<<<< ours\n=======\n=======\n",
            "<<<<<<< ours\n=======\n||||||| base\n",
            "<<<<<<< ours\n||||||| base\n||||||| second\n",
        ] {
            assert_eq!(
                review_conflicts(source.as_bytes().to_vec(), 7).unwrap_err().code,
                "conflict.invalid_markers"
            );
        }
    }

    #[test]
    fn invalid_encoding_and_resource_limits_reject() {
        assert_eq!(review_conflicts(vec![255], 7).unwrap_err().code, "conflict.invalid_utf8");
        assert_eq!(
            review_conflicts(vec![b'x'; MAX_REVIEW_BYTES + 1], 7).unwrap_err().code,
            "conflict.input_limit"
        );
        for size in [0, 129, usize::MAX] {
            assert_eq!(review_conflicts(vec![], size).unwrap_err().code, "conflict.marker_size");
        }
        let bytes = b"<<<<<<<\n=======\n>>>>>>>\n".repeat(MAX_REVIEW_REGIONS + 1);
        assert_eq!(review_conflicts(bytes, 7).unwrap_err().code, "conflict.region_limit");
    }
}
