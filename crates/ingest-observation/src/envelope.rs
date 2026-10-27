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
    pub revisions: HashMap<(String, String, String), RetainedRevision>,
    pub head: Option<String>,
}

/// Projection evidence retained for a revision: re-referencing the revision
/// is valid only when this evidence matches (fresh observation, no advance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainedRevision {
    pub projection: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tombstones: Vec<Tombstone>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_evidence: Vec<RetainedEvidence>,
    pub replay: bool,
}

/// Derived tombstone record for one authorized deletion target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tombstone {
    pub target: String,
    pub prior_revision_ref: String,
    pub cause: String,
    pub observation_id: String,
    pub revision_ref: String,
    pub authority_ref: Option<String>,
}

/// Retained-evidence manifest entry for one consumed byte descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetainedEvidence {
    pub domain: String,
    pub digest: String,
    pub via: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,
}