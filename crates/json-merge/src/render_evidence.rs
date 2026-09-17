//! Executed source edits and byte-checked retained baseline ranges. Replacement
//! text is intentionally not claimed to be an unchanged donor-source region.
use ast_merge::{SourceEdit, apply_source_edits};
use serde::{Deserialize, Serialize};
use tree_haver::{
    ByteRange,
    source::{SourceDescriptor, SourceDocument},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonRenderEdit {
    pub source_range: ByteRange,
    pub output_range: ByteRange,
    pub replacement: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonRetainedRegion {
    pub source_range: ByteRange,
    pub output_range: ByteRange,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonRenderEvidence {
    pub baseline: SourceDescriptor,
    pub edits: Vec<JsonRenderEdit>,
    pub retained: Vec<JsonRetainedRegion>,
}

impl JsonRenderEvidence {
    pub(crate) fn from_edits(
        baseline: &SourceDocument,
        output: &str,
        edits: &[SourceEdit],
    ) -> Result<Self, String> {
        let source = std::str::from_utf8(baseline.bytes()).map_err(|e| e.to_string())?;
        if apply_source_edits(source, edits).map_err(|e| e.to_string())? != output {
            return Err("JSON render does not match its executed edit plan".into());
        }
        let mut ordered = edits.to_vec();
        ordered.sort_by_key(|edit| (edit.start_byte, edit.end_byte));
        let mut result =
            Self { baseline: baseline.descriptor().clone(), edits: vec![], retained: vec![] };
        let mut source_offset = 0;
        let mut output_offset = 0;
        let mut retain = |start, end, output_offset: &mut usize| -> Result<(), String> {
            if start == end {
                return Ok(());
            }
            let range = ByteRange { start_byte: start, end_byte: end };
            let output_range =
                ByteRange { start_byte: *output_offset, end_byte: *output_offset + end - start };
            if baseline.slice(range.clone()).map_err(|e| e.to_string())?
                != &output.as_bytes()[output_range.start_byte..output_range.end_byte]
            {
                return Err("JSON retained bytes differ from their baseline".into());
            }
            *output_offset = output_range.end_byte;
            result.retained.push(JsonRetainedRegion {
                source_range: range.clone(),
                output_range,
                sha256: baseline.range_digest(range).map_err(|e| e.to_string())?,
            });
            Ok(())
        };
        for edit in ordered {
            retain(source_offset, edit.start_byte, &mut output_offset)?;
            let end = output_offset + edit.replacement.len();
            result.edits.push(JsonRenderEdit {
                source_range: ByteRange { start_byte: edit.start_byte, end_byte: edit.end_byte },
                output_range: ByteRange { start_byte: output_offset, end_byte: end },
                replacement: edit.replacement,
            });
            source_offset = edit.end_byte;
            output_offset = end;
        }
        retain(source_offset, source.len(), &mut output_offset)?;
        if output_offset != output.len() {
            return Err("JSON render evidence does not cover output".into());
        }
        Ok(result)
    }

    /// Proves the reported edit replay and retained-byte partition, not the
    /// semantic authority for those edits. That comes from family execution.
    pub fn validate(&self, baseline: &SourceDocument, output: &str) -> Result<(), String> {
        let edits = self
            .edits
            .iter()
            .map(|edit| {
                SourceEdit::replace(
                    edit.source_range.start_byte,
                    edit.source_range.end_byte,
                    &edit.replacement,
                )
            })
            .collect::<Vec<_>>();
        let expected = Self::from_edits(baseline, output, &edits)?;
        if self != &expected {
            return Err("JSON render evidence differs from checked edit replay".into());
        }
        Ok(())
    }
}
