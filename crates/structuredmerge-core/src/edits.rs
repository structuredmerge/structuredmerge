//! Explicit source projection, not AST selection or filesystem mutation.
use crate::{CoreError, SourceDescriptor, SourceDocument, SourceEncoding, SourceInput, SourceRole};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExplicitSourceEdit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceEditRequest {
    pub request_id: String,
    pub source: SourceInput,
    pub edits: Vec<ExplicitSourceEdit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceEditLimits {
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_edits: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceEditResult {
    pub request_id: String,
    pub source: SourceDescriptor,
    pub output: String,
    pub edit_count: usize,
}

/// Apply caller-selected UTF-8 byte edits atomically using the shared renderer.
/// Invalid, overlapping, ambiguous or non-character-boundary edits fail without
/// output. A successful result does not claim a syntax check or AST selection.
pub fn apply_explicit_source_edits(
    request: SourceEditRequest,
    limits: SourceEditLimits,
) -> Result<SourceEditResult, CoreError> {
    if request.request_id.is_empty()
        || request.source.descriptor.role != SourceRole::Source
        || request.source.descriptor.encoding != SourceEncoding::Utf8
    {
        return Err(CoreError {
            code: "request.invalid".into(),
            message: "explicit edits require request identity and a UTF-8 source-role input".into(),
        });
    }
    if request.edits.len() > limits.max_edits {
        return Err(limit_error());
    }
    let source = SourceDocument::validate(request.source, limits.max_input_bytes)
        .map_err(|error| CoreError::from(crate::service::ServiceError::Source(error)))?;
    // Check the exact projected output size before invoking the allocating
    // renderer. Range validity/overlap and UTF-8 boundaries remain renderer-owned.
    let mut removed = 0_u64;
    let mut added = 0_u64;
    for edit in &request.edits {
        let width = edit.end_byte.checked_sub(edit.start_byte).ok_or_else(|| CoreError {
            code: "source_edit.rejected".into(),
            message: "reversed source edit range".into(),
        })?;
        removed = removed.checked_add(width as u64).ok_or_else(limit_error)?;
        added = added.checked_add(edit.replacement.len() as u64).ok_or_else(limit_error)?;
    }
    let projected = (source.bytes().len() as u64)
        .checked_sub(removed)
        .ok_or_else(|| CoreError {
            code: "source_edit.rejected".into(),
            message: "source edit removal exceeds source length".into(),
        })?
        .checked_add(added)
        .ok_or_else(limit_error)?;
    if projected > limits.max_output_bytes || projected > usize::MAX as u64 {
        return Err(limit_error());
    }
    let text = std::str::from_utf8(source.bytes())
        .map_err(|error| CoreError { code: "source.invalid".into(), message: error.to_string() })?;
    let edits: Vec<_> = request
        .edits
        .into_iter()
        .map(|edit| {
            ast_merge::SourceEdit::replace(edit.start_byte, edit.end_byte, edit.replacement)
        })
        .collect();
    let output = ast_merge::apply_source_edits(text, &edits).map_err(|error| CoreError {
        code: "source_edit.rejected".into(),
        message: error.message().into(),
    })?;
    Ok(SourceEditResult {
        request_id: request.request_id,
        source: source.descriptor().clone(),
        output,
        edit_count: edits.len(),
    })
}

fn limit_error() -> CoreError {
    CoreError {
        code: "resource.limit".into(),
        message: "source edit resource limit exceeded".into(),
    }
}
