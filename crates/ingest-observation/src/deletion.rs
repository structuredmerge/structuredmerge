//! Deletion mode and authority types.

use serde::{Deserialize, Serialize};

/// Deletion authorization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionMode {
    None,
    Absence,
    ExplicitSource,
}

/// Deletion specification in the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionSpec {
    pub mode: DeletionMode,
    pub authority_ref: Option<String>,
    pub targets: Vec<String>,
}
