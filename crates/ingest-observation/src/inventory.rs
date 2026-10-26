//! Inventory types and validation.

use crate::error::{Error, ErrorCode, Result as ValidationResult};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChunkRef {
    pub id: String,
    pub ordinal: u64,
    pub content_hash: String,
}

/// Compact a chunks array to canonical form and validate uniqueness.
pub fn compact_inventory(chunks: &Value) -> ValidationResult<Vec<ChunkRef>> {
    let arr = chunks.as_array().ok_or(Error::new(ErrorCode::Json))?;
    let mut result = Vec::with_capacity(arr.len());
    for row in arr {
        let obj = row.as_object().ok_or(Error::new(ErrorCode::Json))?;
        let id = obj.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Json))?;
        let ordinal = obj.get("ordinal").and_then(|v| v.as_u64()).ok_or(Error::new(ErrorCode::Json))?;
        let content_hash = obj.get("content_hash").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Json))?;
        result.push(ChunkRef { id: id.to_string(), ordinal, content_hash: content_hash.to_string() });
    }
    // Unique IDs and ordinals
    if result.iter().map(|c| &c.id).collect::<std::collections::HashSet<_>>().len() != result.len() ||
       result.iter().map(|c| c.ordinal).collect::<std::collections::HashSet<_>>().len() != result.len() {
        return Err(Error::new(ErrorCode::Json));
    }
    Ok(result)
}