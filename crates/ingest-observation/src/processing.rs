//! Processing fingerprint types.

use serde::{Deserialize, Serialize};

/// Processing configuration descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingDescriptor {
    pub schema: String,
    pub normalizer: ComponentConfig,
    pub chunker: ComponentConfig,
    pub hash: HashConfig,
    pub parser: ParserConfig,
    pub legacy_schema_ref: String,
}

/// Component configuration (normalizer/chunker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub id: String,
    pub version: String,
    pub config: serde_json::Map<String, serde_json::Value>,
}

/// Hash configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashConfig {
    pub algorithm: String,
    pub domain: String,
}

/// Parser configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ParserConfig {
    #[serde(rename = "used")]
    Used {
        id: String,
        version: String,
        backend: String,
        grammar: String,
        grammar_version: String,
        contract_ref: String,
    },
    #[serde(rename = "not_applicable")]
    NotApplicable,
    #[serde(rename = "unknown")]
    Unknown,
}

impl ParserConfig {
    pub fn is_used(&self) -> bool {
        matches!(self, ParserConfig::Used { .. })
    }
}