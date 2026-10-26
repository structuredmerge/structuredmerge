//! Envelope and context types for ingest observation verification.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The observation companion envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub schema: String,
    pub payload_json: String,
    pub payload_digest: String,
}

/// Trusted local context supplied by the harness / adapter.
///
/// This is never obtained from the envelope itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub scope: serde_json::Map<String, serde_json::Value>,
    pub producer_id: String,
    pub head_revision_ref: String,
    pub prior_inventory_digest: String,
    pub evidence_refs: HashSet<String>,
    pub authorities: HashMap<String, serde_json::Value>,
    pub resources: HashMap<String, String>,
}

/// Mutable verification state.
///
/// - `accepted`: dedupe key -> (payload digest, decision)
/// - `revisions`: (producer_id, scope, revision_ref) -> payload digest
/// - `head`: current trusted head revision reference
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub accepted: HashMap<(String, String, String), AcceptedEntry>,
    #[serde(default)]
    pub revisions: HashMap<(String, String, String), String>,
    pub head: Option<String>,
}

/// Stored entry for an accepted observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedEntry {
    pub digest: String,
    pub decision: Decision,
}

/// Authorization decision returned by verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub authorized_delete_ids: Vec<String>,
    pub replay: bool,
}