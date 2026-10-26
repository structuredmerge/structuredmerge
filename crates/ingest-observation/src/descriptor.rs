//! Byte descriptor verification.

use crate::digest::sha256;
use crate::error::{Error, ErrorCode, Result};
use std::collections::HashMap;

/// Verify a byte descriptor and return the decoded bytes.
///
/// Descriptor must have exactly: domain, byte_length, digest, and exactly one of
/// inline_base64 or ref. The ref must be resolvable via the resources map.
pub fn verify_bytes(
    value: &serde_json::Value,
    expected_domain: &str,
    resources: &HashMap<String, String>,
) -> Result<Vec<u8>> {
    let obj = value.as_object().ok_or_else(|| {
        eprintln!("DEBUG verify_bytes: value is not an object for domain {}", expected_domain);
        Error::new(ErrorCode::Json)
    })?;
    let domain = obj.get("domain").and_then(|v| v.as_str()).ok_or_else(|| {
        eprintln!("DEBUG verify_bytes: missing or invalid domain for expected {}", expected_domain);
        Error::new(ErrorCode::Json)
    })?;
    if domain != expected_domain {
        eprintln!("DEBUG verify_bytes: domain mismatch: got {} expected {}", domain, expected_domain);
        return Err(Error::new(ErrorCode::Json));
    }
    let byte_length = obj.get("byte_length").and_then(|v| v.as_u64()).ok_or_else(|| {
        eprintln!("DEBUG verify_bytes: missing or invalid byte_length for {}", expected_domain);
        Error::new(ErrorCode::Json)
    })?;
    let digest = obj.get("digest").and_then(|v| v.as_str()).ok_or_else(|| {
        eprintln!("DEBUG verify_bytes: missing or invalid digest for {}", expected_domain);
        Error::new(ErrorCode::Json)
    })?;

    // Exactly one transport
    let has_inline = obj.contains_key("inline_base64");
    let has_ref = obj.contains_key("ref");
    if has_inline == has_ref {
        eprintln!("DEBUG verify_bytes: transport mismatch for {} (inline={}, ref={})", expected_domain, has_inline, has_ref);
        return Err(Error::new(ErrorCode::Json));
    }

    let encoded = if has_inline {
        obj["inline_base64"].as_str().ok_or_else(|| {
            eprintln!("DEBUG verify_bytes: missing inline_base64 for {}", expected_domain);
            Error::new(ErrorCode::Json)
        })?
    } else {
        let ref_key = obj["ref"].as_str().ok_or_else(|| {
            eprintln!("DEBUG verify_bytes: missing ref for {}", expected_domain);
            Error::new(ErrorCode::Json)
        })?;
        resources.get(ref_key).ok_or_else(|| {
            eprintln!("DEBUG verify_bytes: ref {} not found in resources for {}", ref_key, expected_domain);
            Error::new(ErrorCode::UnavailableBytes)
        })?
    };

    let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
        .map_err(|e| {
            eprintln!("DEBUG verify_bytes: base64 decode failed for {}: {:?}", expected_domain, e);
            Error::new(ErrorCode::Json)
        })?;
    // Strict round-trip check
    if base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data) != encoded {
        eprintln!("DEBUG verify_bytes: base64 round-trip failed for {}", expected_domain);
        return Err(Error::new(ErrorCode::Json));
    }
    if data.len() as u64 != byte_length || sha256(&data) != digest {
        eprintln!("DEBUG verify_bytes: length/digest mismatch for {}: len={} expected={}, digest={} expected={}", 
            expected_domain, data.len(), byte_length, sha256(&data), digest);
        return Err(Error::new(ErrorCode::Json));
    }
    Ok(data)
}