use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEdit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

impl SourceEdit {
    pub fn replace(start_byte: usize, end_byte: usize, replacement: impl Into<String>) -> Self {
        Self { start_byte, end_byte, replacement: replacement.into() }
    }

    pub fn insert(at_byte: usize, replacement: impl Into<String>) -> Self {
        Self::replace(at_byte, at_byte, replacement)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRenderError {
    message: String,
}

impl SourceRenderError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SourceRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SourceRenderError {}

pub fn apply_source_edits(source: &str, edits: &[SourceEdit]) -> Result<String, SourceRenderError> {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| (edit.start_byte, edit.end_byte));

    let mut previous_end = 0;
    let mut previous_start = None;
    for edit in &ordered {
        if edit.start_byte > edit.end_byte || edit.end_byte > source.len() {
            return Err(SourceRenderError::new(format!(
                "invalid source edit range [{}, {}) for source length {}",
                edit.start_byte,
                edit.end_byte,
                source.len()
            )));
        }
        if !source.is_char_boundary(edit.start_byte) || !source.is_char_boundary(edit.end_byte) {
            return Err(SourceRenderError::new(format!(
                "source edit range [{}, {}) is not on UTF-8 boundaries",
                edit.start_byte, edit.end_byte
            )));
        }
        if edit.start_byte < previous_end {
            return Err(SourceRenderError::new(format!(
                "source edit range [{}, {}) overlaps a prior edit ending at {}",
                edit.start_byte, edit.end_byte, previous_end
            )));
        }
        if previous_start == Some(edit.start_byte) {
            return Err(SourceRenderError::new(format!(
                "multiple source edits start at byte {}; their order is ambiguous",
                edit.start_byte
            )));
        }
        previous_start = Some(edit.start_byte);
        previous_end = edit.end_byte;
    }

    let replacement_bytes = ordered.iter().map(|edit| edit.replacement.len()).sum::<usize>();
    let removed_bytes = ordered.iter().map(|edit| edit.end_byte - edit.start_byte).sum::<usize>();
    let mut output = String::with_capacity(source.len() - removed_bytes + replacement_bytes);
    let mut cursor = 0;
    for edit in ordered {
        output.push_str(&source[cursor..edit.start_byte]);
        output.push_str(&edit.replacement);
        cursor = edit.end_byte;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_untouched_bytes_across_ordered_edits() {
        let source = "{\r\n  \"left\": 1,\r\n  \"right\": 1\r\n}\r\n";
        let left = source.find("1,").unwrap();
        let right = source.rfind('1').unwrap();
        let output = apply_source_edits(
            source,
            &[SourceEdit::replace(right, right + 1, "3"), SourceEdit::replace(left, left + 1, "2")],
        )
        .unwrap();

        assert_eq!(output, "{\r\n  \"left\": 2,\r\n  \"right\": 3\r\n}\r\n");
    }

    #[test]
    fn supports_unambiguous_insertions() {
        assert_eq!(
            apply_source_edits("{}\n", &[SourceEdit::insert(1, "\n  \"key\": 1\n")]).unwrap(),
            "{\n  \"key\": 1\n}\n"
        );
    }

    #[test]
    fn rejects_overlapping_or_ambiguous_edits() {
        let overlap = apply_source_edits(
            "abcdef",
            &[SourceEdit::replace(1, 4, "x"), SourceEdit::replace(3, 5, "y")],
        )
        .unwrap_err();
        assert!(overlap.message().contains("overlaps"));

        let ambiguous = apply_source_edits(
            "abcdef",
            &[SourceEdit::insert(2, "x"), SourceEdit::replace(2, 3, "y")],
        )
        .unwrap_err();
        assert!(ambiguous.message().contains("ambiguous"));
    }

    #[test]
    fn rejects_ranges_that_split_utf8_or_exceed_source() {
        let utf8 = apply_source_edits("aéz", &[SourceEdit::replace(2, 3, "e")]).unwrap_err();
        assert!(utf8.message().contains("UTF-8 boundaries"));

        let outside = apply_source_edits("abc", &[SourceEdit::replace(0, 4, "x")]).unwrap_err();
        assert!(outside.message().contains("source length 3"));
    }
}
