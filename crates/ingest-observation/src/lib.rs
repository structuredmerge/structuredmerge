//! Ingest observation validation — shared, stdlib-only, no I/O.
//!
//! Canonical verification logic for the
//! `structuredmerge.ingest-observation/v2` envelope.
//! No legacy/dialect baggage — this is the native observation format.

#![forbid(unsafe_code)]

pub mod envelope;
pub mod error;
pub mod processing;
pub mod deletion;
pub mod time;

pub use envelope::{Envelope, Context, State, Decision, AcceptedEntry};
pub use error::{Error, ErrorCode, Result as ValidationResult};

mod descriptor;
mod digest;
mod inventory;
mod strict_json;

use descriptor::verify_bytes;
use inventory::compact_inventory;
use std::collections::HashSet;

/// Verify an observation envelope against trusted local context.
///
/// On success, returns the authorization decision and mutates `state` to
/// record the accepted observation and updated head. On failure, `state` is
/// unchanged (the function fails before any effects are committed).
pub fn verify(envelope: &Envelope, context: &Context, state: &mut State) -> ValidationResult<Decision> {
    // Schema and payload digest
    if envelope.schema != "structuredmerge.ingest-observation/v2" {
        return Err(Error::new(ErrorCode::Schema));
    }
    let payload_bytes = envelope.payload_json.as_bytes();
    if digest::sha256(payload_bytes) != envelope.payload_digest {
        return Err(Error::new(ErrorCode::PayloadDigest));
    }

    // Strict JSON parse (rejects duplicate keys, non-finite numbers, surrogates)
    let payload: serde_json::Value = strict_json::parse(payload_bytes)?;

    // Required top-level fields
    let required = [
        "producer", "scope", "observation_id", "revision_ref",
        "source_revision_ref", "expected_prior_revision_ref",
        "prior_inventory", "current_inventory", "added", "updated", "removed",
        "original", "normalized", "processing", "times", "scan",
        "deletion", "evidence_refs", "extensions",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(Error::new(ErrorCode::Fields));
        }
    }

    let producer = payload["producer"].as_object().ok_or(Error::new(ErrorCode::Producer))?;
    let scope = payload["scope"].as_object().ok_or(Error::new(ErrorCode::Scope))?;

    // Producer fields
    if !producer.get("id").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) ||
       !producer.get("version").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) ||
       !producer.get("contract_ref").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        return Err(Error::new(ErrorCode::Producer));
    }

    // Scope fields
    let scope_keys = ["workspace_id", "source_id", "document_id", "enumeration"];
    for key in scope_keys {
        if !scope.get(key).and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
            return Err(Error::new(ErrorCode::Scope));
        }
    }
    if scope.get("enumeration").and_then(|v| v.as_str()) != Some("document-chunks") {
        return Err(Error::new(ErrorCode::Scope));
    }
    if scope != &context.scope || producer["id"] != context.producer_id {
        return Err(Error::new(ErrorCode::Scope));
    }

    // Identity fields
    let observation_id = payload["observation_id"].as_str()
        .filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Identity))?;
    let revision_ref = payload["revision_ref"].as_str()
        .filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Identity))?;
    for key in ["source_revision_ref", "expected_prior_revision_ref"] {
        if let Some(v) = payload[key].as_str() {
            if v.is_empty() {
                return Err(Error::new(ErrorCode::Identity));
            }
        }
    }

    // Extensions: must be object with namespaced keys
    if let Some(ext) = payload["extensions"].as_object() {
        if !ext.keys().all(|k| k.contains(':')) {
            return Err(Error::new(ErrorCode::Fields));
        }
    } else {
        return Err(Error::new(ErrorCode::Fields));
    }

    // Replay / idempotency key
    let dedupe_key = (
        producer["id"].as_str().unwrap().to_string(),
        serde_json::to_string(scope).unwrap(),
        observation_id.to_string(),
    );
    if let Some(prev) = state.accepted.get(&dedupe_key) {
        if prev.digest != envelope.payload_digest {
            return Err(Error::new(ErrorCode::ConflictingRetry));
        }
        return Ok(Decision {
            authorized_delete_ids: prev.decision.authorized_delete_ids.clone(),
            replay: true,
        });
    }

    // Precondition: expected prior revision must match trusted head
    let head = state.head.as_ref().unwrap_or(&context.head_revision_ref);
    let expected_prior = payload["expected_prior_revision_ref"].as_str();
    // Both empty/no-prior: head empty and expected_prior null -> OK
    // Both have values -> must match exactly
    // Mismatch -> error
    let matches = match (head.is_empty(), expected_prior) {
        (true, None) => true,   // no prior on either side
        (false, Some(p)) if p == head.as_str() => true,  // both have same value
        _ => false,
    };
    if !matches {
        return Err(Error::new(ErrorCode::Precondition));
    }

    // Prior inventory
    let prior = if head.is_empty() {
        if payload["prior_inventory"].is_null() {
            Vec::new()
        } else {
            return Err(Error::new(ErrorCode::Prior));
        }
    } else {
        let prior_bytes = verify_bytes(&payload["prior_inventory"], "prior-inventory-json", &context.resources)?;
        let prior_obj: serde_json::Value = strict_json::parse(&prior_bytes)?;
        if serde_json::to_string(&prior_obj["scope"]).unwrap() != serde_json::to_string(scope).unwrap() ||
           prior_obj["revision_ref"].as_str() != Some(head) {
            return Err(Error::new(ErrorCode::Prior));
        }
        if digest::sha256(&prior_bytes) != context.prior_inventory_digest {
            return Err(Error::new(ErrorCode::Prior));
        }
        compact_inventory(&prior_obj["chunks"])?
    };

    // Current inventory (replaces legacy_result)
    let current_bytes = verify_bytes(&payload["current_inventory"], "current-inventory-json", &context.resources)?;
    let current_obj: serde_json::Value = strict_json::parse(&current_bytes)?;
    if serde_json::to_string(&current_obj["scope"]).unwrap() != serde_json::to_string(scope).unwrap() ||
       current_obj["revision_ref"].as_str() != Some(revision_ref) {
        return Err(Error::new(ErrorCode::Fields));
    }
    let current = compact_inventory(&current_obj["chunks"])?;

    // Added / updated / removed arrays (explicit, not inferred)
    let added: HashSet<String> = payload["added"].as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Fields)).map(String::from))
        .collect::<ValidationResult<_>>()?;
    let updated: HashSet<String> = payload["updated"].as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Fields)).map(String::from))
        .collect::<ValidationResult<_>>()?;
    let removed: HashSet<String> = payload["removed"].as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Fields)).map(String::from))
        .collect::<ValidationResult<_>>()?;

    // Validate added/updated/removed against prior and current
    let prior_ids: HashSet<String> = prior.iter().map(|c| c.id.clone()).collect();
    let current_ids: HashSet<String> = current.iter().map(|c| c.id.clone()).collect();
    if !added.is_subset(&current_ids) || !updated.is_subset(&current_ids) || !removed.is_subset(&prior_ids) {
        return Err(Error::new(ErrorCode::Fields));
    }
    if added.iter().any(|id| prior_ids.contains(id)) {
        return Err(Error::new(ErrorCode::Fields)); // added must be new
    }
    if updated.iter().any(|id| !prior_ids.contains(id)) {
        return Err(Error::new(ErrorCode::Fields)); // updated must exist in prior
    }
    if removed.iter().any(|id| current_ids.contains(id)) {
        return Err(Error::new(ErrorCode::Fields)); // removed must not be in current
    }

    // Original and normalized bytes
    let _original = verify_bytes(&payload["original"], "original-source-bytes", &context.resources)?;
    let normalized = verify_bytes(&payload["normalized"], "normalized-input-utf8", &context.resources)?;
    std::str::from_utf8(&normalized).map_err(|_| Error::new(ErrorCode::Utf8))?;

    // Processing fingerprint
    let processing = &payload["processing"];
    let descriptor_bytes = verify_bytes(&processing["descriptor"], "processing-json", &context.resources)?;
    if processing["fingerprint"].as_str() != Some(&digest::sha256(&descriptor_bytes)) {
        return Err(Error::new(ErrorCode::Processing));
    }
    let proc_config: serde_json::Value = strict_json::parse(&descriptor_bytes)?;
    if proc_config["schema"].as_str() != Some("structuredmerge.ingest-processing/v1") {
        return Err(Error::new(ErrorCode::Processing));
    }
    for name in ["normalizer", "chunker"] {
        let obj = &proc_config[name];
        if obj["id"].as_str().filter(|s| !s.is_empty()).is_none() ||
           obj["version"].as_str().filter(|s| !s.is_empty()).is_none() ||
           obj["config"].as_object().is_none() {
            return Err(Error::new(ErrorCode::Processing));
        }
    }
    if proc_config["hash"] != serde_json::json!({"algorithm": "sha256", "domain": "chunk-text-utf8"}) {
        return Err(Error::new(ErrorCode::Processing));
    }
    let parser = &proc_config["parser"];
    if parser.get("status") == Some(&serde_json::Value::String("used".to_string())) {
        for key in ["id", "version", "backend", "grammar", "grammar_version", "contract_ref"] {
            if parser[key].as_str().filter(|s| !s.is_empty()).is_none() {
                return Err(Error::new(ErrorCode::Processing));
            }
        }
    } else if parser != &serde_json::json!({"status": "not_applicable"}) {
        return Err(Error::new(ErrorCode::Processing));
    }

    // Times
    let times = &payload["times"];
    for (name, value) in times.as_object().ok_or(Error::new(ErrorCode::Time))? {
        if name == "source_at" && value.is_null() { continue; }
        if !time::is_valid_rfc3339_utc(value.as_str().unwrap_or("")) {
            return Err(Error::new(ErrorCode::Time));
        }
    }

    // Scan
    let scan = &payload["scan"];
    let status = scan["status"].as_str().ok_or(Error::new(ErrorCode::Scan))?;
    if !matches!(status, "complete" | "partial" | "interrupted" | "unknown") {
        return Err(Error::new(ErrorCode::Scan));
    }
    if serde_json::to_string(&scan["enumerated_scope"]).unwrap() != serde_json::to_string(scope).unwrap() {
        return Err(Error::new(ErrorCode::Scope));
    }
    let finalization = scan["finalization_ref"].as_str();
    if status == "complete" {
        if finalization.is_none() || !context.evidence_refs.contains(finalization.unwrap()) {
            return Err(Error::new(ErrorCode::Evidence));
        }
    } else if finalization.is_some() {
        return Err(Error::new(ErrorCode::Scan));
    }
    for ref_ in [scan["checkpoint_ref"].as_str(), finalization].into_iter().flatten() {
        if !context.evidence_refs.contains(ref_) {
            return Err(Error::new(ErrorCode::Evidence));
        }
    }
    for ref_ in payload["evidence_refs"].as_array().ok_or(Error::new(ErrorCode::Evidence))? {
        if !ref_.as_str().map(|s| !s.is_empty() && context.evidence_refs.contains(s)).unwrap_or(false) {
            return Err(Error::new(ErrorCode::Evidence));
        }
    }

    // Deletion authorization (uses explicit added/updated/removed)
    let deletion = &payload["deletion"];
    let mode = deletion["mode"].as_str().ok_or(Error::new(ErrorCode::Deletion))?;
    let targets: HashSet<String> = deletion["targets"].as_array()
        .ok_or(Error::new(ErrorCode::Deletion))?
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Deletion)).map(String::from))
        .collect::<ValidationResult<_>>()?;

    match mode {
        "none" => {
            if deletion["authority_ref"].is_null() && targets.is_empty() {
                // ok
            } else {
                return Err(Error::new(ErrorCode::Deletion));
            }
        }
        "absence" => {
            if status != "complete" {
                return Err(Error::new(ErrorCode::Deletion));
            }
            if targets != removed || !targets.iter().all(|t| prior_ids.contains(t)) {
                return Err(Error::new(ErrorCode::Deletion));
            }
            check_authority(context, deletion, mode, finalization, &targets, producer["id"].as_str().unwrap(), scope)?;
        }
        "explicit_source" => {
            if !current.is_empty() || targets != prior_ids {
                return Err(Error::new(ErrorCode::Deletion));
            }
            check_authority(context, deletion, mode, finalization, &targets, producer["id"].as_str().unwrap(), scope)?;
        }
        _ => return Err(Error::new(ErrorCode::Deletion)),
    }

    // Revision uniqueness
    let revision_key = (
        producer["id"].as_str().unwrap().to_string(),
        serde_json::to_string(scope).unwrap(),
        revision_ref.to_string(),
    );
    if state.revisions.contains_key(&revision_key) {
        return Err(Error::new(ErrorCode::Revision));
    }

    // Commit state
    let decision = Decision {
        authorized_delete_ids: targets.into_iter().collect(),
        replay: false,
    };
    state.accepted.insert(dedupe_key, AcceptedEntry {
        digest: envelope.payload_digest.clone(),
        decision: decision.clone(),
    });
    state.revisions.insert(revision_key, envelope.payload_digest.clone());
    state.head = Some(revision_ref.to_string());

    Ok(decision)
}

fn check_authority(
    context: &Context,
    deletion: &serde_json::Value,
    mode: &str,
    finalization: Option<&str>,
    targets: &HashSet<String>,
    producer_id: &str,
    scope: &serde_json::Map<String, serde_json::Value>,
) -> ValidationResult<()> {
    let auth_ref = deletion["authority_ref"].as_str().ok_or(Error::new(ErrorCode::Authority))?;
    let authority = context.authorities.get(auth_ref).ok_or(Error::new(ErrorCode::Authority))?;

    let scope_val = serde_json::Value::Object(scope.clone());
    if authority.get("producer_id").and_then(|v| v.as_str()) != Some(producer_id) ||
       authority.get("scope") != Some(&scope_val) ||
       authority.get("mode").and_then(|v| v.as_str()) != Some(mode) ||
       authority.get("finalization_ref").and_then(|v| v.as_str()) != finalization {
        return Err(Error::new(ErrorCode::Authority));
    }
    let auth_targets: HashSet<String> = authority["targets"].as_array()
        .ok_or(Error::new(ErrorCode::Authority))?
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Authority)).map(String::from))
        .collect::<ValidationResult<_>>()?;
    if auth_targets != *targets {
        return Err(Error::new(ErrorCode::Authority));
    }
    Ok(())
}