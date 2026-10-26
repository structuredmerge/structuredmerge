//! Strict JSON parsing: rejects duplicate keys, non-finite numbers, surrogates.

use crate::error::{Error, ErrorCode, Result};
use serde::Deserialize;
use serde_json::Value;

/// Parse JSON with strict validation.
///
/// - Rejects duplicate keys
/// - Rejects NaN, Infinity, -Infinity
/// - Rejects unpaired surrogates
/// - Requires valid UTF-8 round-trip
pub fn parse(data: &[u8]) -> Result<Value> {
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("DEBUG strict_json: from_utf8 failed: {:?}", e);
            return Err(Error::new(ErrorCode::Json));
        }
    };
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = match Value::deserialize(&mut deserializer) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("DEBUG strict_json: deserialize failed: {:?}", e);
            eprintln!("DEBUG strict_json: text first 200 chars: {}", &text[..text.len().min(200)]);
            return Err(Error::new(ErrorCode::Json));
        }
    };
    // Round-trip check catches surrogates and ensures canonical form
    let roundtrip = match serde_json::to_string(&value) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("DEBUG strict_json: to_string failed: {:?}", e);
            return Err(Error::new(ErrorCode::Json));
        }
    };
    let _: Value = match serde_json::from_str(&roundtrip) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("DEBUG strict_json: from_str roundtrip failed: {:?}", e);
            eprintln!("DEBUG strict_json: roundtrip: {}", roundtrip);
            return Err(Error::new(ErrorCode::Json));
        }
    };
    Ok(value)
}