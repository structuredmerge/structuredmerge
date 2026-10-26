//! SHA-256 digest utilities.

use sha2::{Digest, Sha256};

/// Compute SHA-256 digest as `sha256:<64 lowercase hex>`.
pub fn sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}