//! Ingest observation validation — shared, stdlib-only, no I/O.
//!
//! Canonical verification logic for the
//! `structuredmerge.ingest-observation/v2` envelope.
//! No legacy/dialect baggage — this is the native observation format.

#![forbid(unsafe_code)]

pub mod deletion;
pub mod envelope;
pub mod error;
pub mod processing;
pub mod time;

pub use envelope::{
    AcceptedEntry, Context, Decision, Envelope, RetainedEvidence, RetainedRevision, State,
    Tombstone,
};
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
pub fn verify(
    envelope: &Envelope,
    context: &Context,
    state: &mut State,
) -> ValidationResult<Decision> {
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

    // Exact top-level key set: required fields plus known optional fields;
    // unknown fields reject rather than being silently dropped.
    let required = [
        "producer",
        "scope",
        "observation_id",
        "revision_ref",
        "source_revision_ref",
        "expected_prior_revision_ref",
        "prior_inventory",
        "current_inventory",
        "added",
        "updated",
        "removed",
        "original",
        "normalized",
        "processing",
        "times",
        "scan",
        "deletion",
        "evidence_refs",
        "extensions",
    ];
    let payload_map = payload.as_object().ok_or(Error::new(ErrorCode::Fields))?;
    let optional: &[&str] = &[
        "revision_parents",
        "source_time_start",
        "source_time_end",
        "embedding_contract",
        "lineage",
        "incorporation",
    ];
    let has_start = payload_map.contains_key("source_time_start");
    let has_end = payload_map.contains_key("source_time_end");
    if has_start != has_end {
        return Err(Error::new(ErrorCode::Time));
    }
    for key in payload_map.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(Error::new(ErrorCode::Fields));
        }
    }
    for field in required {
        if payload_map.get(field).is_none() {
            return Err(Error::new(ErrorCode::Fields));
        }
    }
    // revision_parents validation (slice 1047)
    if let Some(parents) = payload_map.get("revision_parents") {
        let parents = parents.as_array().ok_or(Error::new(ErrorCode::Ancestry))?;
        let mut seen: HashSet<&str> = HashSet::new();
        for parent in parents {
            let value =
                parent.as_str().filter(|s| !s.is_empty()).ok_or(Error::new(ErrorCode::Ancestry))?;
            if !seen.insert(value) || value == revision_ref_for(&payload)? {
                return Err(Error::new(ErrorCode::Ancestry));
            }
        }
        let expected_prior = payload["expected_prior_revision_ref"].as_str();
        if parents.is_empty() {
            if expected_prior.is_some() {
                return Err(Error::new(ErrorCode::Ancestry));
            }
        } else if let Some(prior) = expected_prior {
            let current_revision = revision_ref_for(&payload)?;
            let chained =
                prior == current_revision || parents.iter().any(|v| v.as_str() == Some(prior));
            if !chained {
                return Err(Error::new(ErrorCode::Ancestry));
            }
        }
    }
    // source time interval (slice 1048)
    if let (Some(start), Some(end)) =
        (payload_map.get("source_time_start"), payload_map.get("source_time_end"))
    {
        let (start, end) = (
            start.as_str().ok_or(Error::new(ErrorCode::Time))?,
            end.as_str().ok_or(Error::new(ErrorCode::Time))?,
        );
        if !time::is_valid_rfc3339_utc(start) || !time::is_valid_rfc3339_utc(end) || start > end {
            return Err(Error::new(ErrorCode::Time));
        }
    }
    // embedding contract handoff (slice 1049)
    if let Some(contract) = payload_map.get("embedding_contract") {
        let contract = contract.as_object().ok_or(Error::new(ErrorCode::Embedding))?;
        let exact = [
            "owner",
            "model_ref",
            "model_revision",
            "dimensions",
            "normalization",
            "instruction_profile",
        ];
        if contract.len() != exact.len() || !exact.iter().all(|k| contract.contains_key(*k)) {
            return Err(Error::new(ErrorCode::Embedding));
        }
        for key in ["owner", "model_ref"] {
            if !contract[key].as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                return Err(Error::new(ErrorCode::Embedding));
            }
        }
        for key in ["model_revision", "normalization", "instruction_profile"] {
            match &contract[key] {
                serde_json::Value::Null => {}
                serde_json::Value::String(s) if !s.is_empty() => {}
                _ => return Err(Error::new(ErrorCode::Embedding)),
            }
        }
        match &contract["dimensions"] {
            serde_json::Value::Null => {}
            serde_json::Value::Number(n) if n.as_u64().map(|d| d > 0).unwrap_or(false) => {}
            _ => return Err(Error::new(ErrorCode::Embedding)),
        }
    }

    let producer = payload["producer"].as_object().ok_or(Error::new(ErrorCode::Producer))?;
    let scope = payload["scope"].as_object().ok_or(Error::new(ErrorCode::Scope))?;

    // Producer fields
    if !producer.get("id").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false)
        || !producer.get("version").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false)
        || !producer
            .get("contract_ref")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    {
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
    let observation_id = payload["observation_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(Error::new(ErrorCode::Identity))?;
    let revision_ref = payload["revision_ref"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(Error::new(ErrorCode::Identity))?;
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
            tombstones: prev.decision.tombstones.clone(),
            retained_evidence: prev.decision.retained_evidence.clone(),
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
        (true, None) => true,                           // no prior on either side
        (false, Some(p)) if p == head.as_str() => true, // both have same value
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
        let prior_bytes =
            verify_bytes(&payload["prior_inventory"], "prior-inventory-json", &context.resources)?;
        let prior_obj: serde_json::Value = strict_json::parse(&prior_bytes)?;
        if serde_json::to_string(&prior_obj["scope"]).unwrap()
            != serde_json::to_string(scope).unwrap()
            || prior_obj["revision_ref"].as_str() != Some(head)
        {
            return Err(Error::new(ErrorCode::Prior));
        }
        if digest::sha256(&prior_bytes) != context.prior_inventory_digest {
            return Err(Error::new(ErrorCode::Prior));
        }
        compact_inventory(&prior_obj["chunks"])?
    };

    // Current inventory (replaces legacy_result)
    let current_bytes =
        verify_bytes(&payload["current_inventory"], "current-inventory-json", &context.resources)?;
    let current_obj: serde_json::Value = strict_json::parse(&current_bytes)?;
    if serde_json::to_string(&current_obj["scope"]).unwrap()
        != serde_json::to_string(scope).unwrap()
        || current_obj["revision_ref"].as_str() != Some(revision_ref)
    {
        return Err(Error::new(ErrorCode::Fields));
    }
    let current = compact_inventory(&current_obj["chunks"])?;

    // Lineage assertions (slice 1053)
    if let Some(lineage) = payload.get("lineage") {
        verify_lineage(lineage, &prior, &current)?;
    }

    // Added / updated / removed arrays (explicit, not inferred)
    let added: HashSet<String> = payload["added"]
        .as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::new(ErrorCode::Fields))
                .map(String::from)
        })
        .collect::<ValidationResult<_>>()?;
    let updated: HashSet<String> = payload["updated"]
        .as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::new(ErrorCode::Fields))
                .map(String::from)
        })
        .collect::<ValidationResult<_>>()?;
    let removed: HashSet<String> = payload["removed"]
        .as_array()
        .ok_or(Error::new(ErrorCode::Fields))?
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::new(ErrorCode::Fields))
                .map(String::from)
        })
        .collect::<ValidationResult<_>>()?;

    // Validate added/updated/removed against prior and current
    let prior_ids: HashSet<String> = prior.iter().map(|c| c.id.clone()).collect();
    let current_ids: HashSet<String> = current.iter().map(|c| c.id.clone()).collect();
    if !added.is_subset(&current_ids)
        || !updated.is_subset(&current_ids)
        || !removed.is_subset(&prior_ids)
    {
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
    let _original =
        verify_bytes(&payload["original"], "original-source-bytes", &context.resources)?;
    let normalized =
        verify_bytes(&payload["normalized"], "normalized-input-utf8", &context.resources)?;
    std::str::from_utf8(&normalized).map_err(|_| Error::new(ErrorCode::Utf8))?;

    // Processing fingerprint
    let processing = &payload["processing"];
    let descriptor_bytes =
        verify_bytes(&processing["descriptor"], "processing-json", &context.resources)?;
    if processing["fingerprint"].as_str() != Some(&digest::sha256(&descriptor_bytes)) {
        return Err(Error::new(ErrorCode::Processing));
    }
    let proc_config: serde_json::Value = strict_json::parse(&descriptor_bytes)?;
    if proc_config["schema"].as_str() != Some("structuredmerge.ingest-processing/v1") {
        return Err(Error::new(ErrorCode::Processing));
    }
    for name in ["normalizer", "chunker"] {
        let obj = &proc_config[name];
        if obj["id"].as_str().filter(|s| !s.is_empty()).is_none()
            || obj["version"].as_str().filter(|s| !s.is_empty()).is_none()
            || obj["config"].as_object().is_none()
        {
            return Err(Error::new(ErrorCode::Processing));
        }
    }
    if proc_config["hash"]
        != serde_json::json!({"algorithm": "sha256", "domain": "chunk-text-utf8"})
    {
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
        if name == "source_at" && value.is_null() {
            continue;
        }
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
    if serde_json::to_string(&scan["enumerated_scope"]).unwrap()
        != serde_json::to_string(scope).unwrap()
    {
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
        if !ref_
            .as_str()
            .map(|s| !s.is_empty() && context.evidence_refs.contains(s))
            .unwrap_or(false)
        {
            return Err(Error::new(ErrorCode::Evidence));
        }
    }

    // Incorporation decisions (slice 1054)
    if let Some(incorporation) = payload.get("incorporation") {
        let declared: Vec<&str> = payload["evidence_refs"]
            .as_array()
            .ok_or(Error::new(ErrorCode::Evidence))?
            .iter()
            .map(|v| v.as_str().ok_or(Error::new(ErrorCode::Evidence)))
            .collect::<ValidationResult<_>>()?;
        verify_incorporation(incorporation, &prior, &current, &declared)?;
    }

    // Deletion authorization (uses explicit added/updated/removed)
    let deletion = &payload["deletion"];
    let mode = deletion["mode"].as_str().ok_or(Error::new(ErrorCode::Deletion))?;
    let targets: HashSet<String> = deletion["targets"]
        .as_array()
        .ok_or(Error::new(ErrorCode::Deletion))?
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::new(ErrorCode::Deletion))
                .map(String::from)
        })
        .collect::<ValidationResult<_>>()?;

    match mode {
        "none" => {
            if !(deletion["authority_ref"].is_null() && targets.is_empty()) {
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
            check_authority(
                context,
                deletion,
                mode,
                finalization,
                &targets,
                producer["id"].as_str().unwrap(),
                scope,
            )?;
        }
        "explicit_source" => {
            if !current.is_empty() || targets != prior_ids {
                return Err(Error::new(ErrorCode::Deletion));
            }
            check_authority(
                context,
                deletion,
                mode,
                finalization,
                &targets,
                producer["id"].as_str().unwrap(),
                scope,
            )?;
        }
        _ => return Err(Error::new(ErrorCode::Deletion)),
    }

    // Revision retention: re-referencing is valid only as a fresh observation
    // of the same projection (identical projection evidence, no advance).
    let revision_key = (
        producer["id"].as_str().unwrap().to_string(),
        serde_json::to_string(scope).unwrap(),
        revision_ref.to_string(),
    );
    let projection_evidence = digest::sha256(
        [
            payload["current_inventory"]["digest"].as_str().unwrap_or(""),
            payload["original"]["digest"].as_str().unwrap_or(""),
            payload["normalized"]["digest"].as_str().unwrap_or(""),
            payload["processing"]["fingerprint"].as_str().unwrap_or(""),
        ]
        .concat()
        .as_bytes(),
    );
    match state.revisions.get(&revision_key) {
        None => {
            state
                .revisions
                .insert(revision_key, RetainedRevision { projection: projection_evidence });
        }
        Some(retained) => {
            if retained.projection != projection_evidence {
                return Err(Error::new(ErrorCode::Revision));
            }
        }
    }

    // Derived tombstones (slice 1050) and retained-evidence manifest (slice 1051)
    let head_for_tombstones = if head.is_empty() { "" } else { head.as_str() };
    let tombstones: Vec<Tombstone> = targets
        .iter()
        .map(|target| Tombstone {
            target: target.clone(),
            prior_revision_ref: head_for_tombstones.to_string(),
            cause: mode.to_string(),
            observation_id: observation_id.to_string(),
            revision_ref: revision_ref.to_string(),
            authority_ref: deletion["authority_ref"].as_str().map(String::from),
        })
        .collect();
    let mut retained_evidence = Vec::new();
    for item in [
        payload.get("prior_inventory").filter(|v| !v.is_null()),
        Some(&payload["current_inventory"]),
        Some(&payload["original"]),
        Some(&payload["normalized"]),
        Some(&payload["processing"]["descriptor"]),
    ]
    .into_iter()
    .flatten()
    {
        let obj = item.as_object().ok_or(Error::new(ErrorCode::Fields))?;
        retained_evidence.push(RetainedEvidence {
            domain: obj.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            digest: obj.get("digest").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            via: if obj.contains_key("ref") {
                "ref".to_string()
            } else {
                "inline_base64".to_string()
            },
            ref_: obj.get("ref").and_then(|v| v.as_str()).map(String::from),
        });
    }

    // Commit state
    let decision = Decision {
        authorized_delete_ids: targets.into_iter().collect(),
        tombstones,
        retained_evidence,
        replay: false,
    };
    state.accepted.insert(
        dedupe_key,
        AcceptedEntry { digest: envelope.payload_digest.clone(), decision: decision.clone() },
    );
    state.head = Some(revision_ref.to_string());

    Ok(decision)
}

fn revision_ref_for(payload: &serde_json::Value) -> ValidationResult<&str> {
    payload["revision_ref"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(Error::new(ErrorCode::Identity))
}

fn occurrence_entries(
    value: &serde_json::Value,
    code: ErrorCode,
) -> ValidationResult<Vec<(String, u64)>> {
    let refs = value.as_array().ok_or(Error::new(code))?;
    let mut entries = Vec::with_capacity(refs.len());
    for ref_ in refs {
        let obj = ref_.as_object().ok_or(Error::new(code))?;
        if obj.len() != 2 {
            return Err(Error::new(code));
        }
        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or(Error::new(code))?;
        let ordinal = obj.get("ordinal").and_then(|v| v.as_u64()).ok_or(Error::new(code))?;
        let entry = (id.to_string(), ordinal);
        if entries.contains(&entry) {
            return Err(Error::new(code));
        }
        entries.push(entry);
    }
    Ok(entries)
}

fn verify_lineage(
    lineage: &serde_json::Value,
    prior: &[inventory::ChunkRef],
    current: &[inventory::ChunkRef],
) -> ValidationResult<()> {
    let code = ErrorCode::Lineage;
    let assertions = lineage.as_array().ok_or(Error::new(code))?;
    let prior_refs: std::collections::HashMap<(&str, u64), &str> =
        prior.iter().map(|c| ((c.id.as_str(), c.ordinal), c.content_hash.as_str())).collect();
    let current_refs: std::collections::HashMap<(&str, u64), &str> =
        current.iter().map(|c| ((c.id.as_str(), c.ordinal), c.content_hash.as_str())).collect();
    for assertion in assertions {
        let obj = assertion.as_object().ok_or(Error::new(code))?;
        let exact = ["kind", "from", "to", "provenance", "status"];
        if obj.len() != exact.len() || !exact.iter().all(|k| obj.contains_key(*k)) {
            return Err(Error::new(code));
        }
        let kind = assertion["kind"].as_str().ok_or(Error::new(code))?;
        if !matches!(kind, "move" | "copy" | "split" | "join" | "reorder") {
            return Err(Error::new(code));
        }
        let status = assertion["status"].as_str().ok_or(Error::new(code))?;
        if !matches!(status, "asserted" | "ambiguous" | "unknown") {
            return Err(Error::new(code));
        }
        let from = occurrence_entries(&assertion["from"], code)?;
        let to = occurrence_entries(&assertion["to"], code)?;
        let provenance = assertion["provenance"].as_object().ok_or(Error::new(code))?;
        let exact = ["basis", "authority_ref", "algorithm", "confidence"];
        if provenance.len() != exact.len() || !exact.iter().all(|k| provenance.contains_key(*k)) {
            return Err(Error::new(code));
        }
        let basis = assertion["provenance"]["basis"].as_str().ok_or(Error::new(code))?;
        match basis {
            "authored" => {
                let authority = provenance["authority_ref"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or(Error::new(code))?;
                let _ = authority;
                if !provenance["algorithm"].is_null() {
                    return Err(Error::new(code));
                }
            }
            "algorithmic" => {
                if !provenance["authority_ref"].is_null() {
                    return Err(Error::new(code));
                }
                let algorithm = provenance["algorithm"].as_object().ok_or(Error::new(code))?;
                if algorithm.len() != 2
                    || !algorithm
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.is_empty())
                        .unwrap_or(false)
                    || !algorithm
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.is_empty())
                        .unwrap_or(false)
                {
                    return Err(Error::new(code));
                }
            }
            "unknown" => {
                if !provenance["authority_ref"].is_null() || !provenance["algorithm"].is_null() {
                    return Err(Error::new(code));
                }
            }
            _ => return Err(Error::new(code)),
        }
        match &provenance["confidence"] {
            serde_json::Value::Null => {}
            serde_json::Value::Number(n) => {
                let ok = n.as_f64().map(|c| (0.0..=1.0).contains(&c)).unwrap_or(false);
                if !ok {
                    return Err(Error::new(code));
                }
            }
            _ => return Err(Error::new(code)),
        }
        if status == "asserted" {
            if from.is_empty() || to.is_empty() {
                return Err(Error::new(code));
            }
            for (id, ordinal) in &from {
                if !prior_refs.contains_key(&(id.as_str(), *ordinal)) {
                    return Err(Error::new(code));
                }
            }
            for (id, ordinal) in &to {
                if !current_refs.contains_key(&(id.as_str(), *ordinal)) {
                    return Err(Error::new(code));
                }
            }
            if kind == "reorder" {
                let mut from_hashes: Vec<&str> =
                    from.iter().map(|(id, ord)| prior_refs[&(id.as_str(), *ord)]).collect();
                let mut to_hashes: Vec<&str> =
                    to.iter().map(|(id, ord)| current_refs[&(id.as_str(), *ord)]).collect();
                from_hashes.sort();
                to_hashes.sort();
                if from_hashes != to_hashes {
                    return Err(Error::new(code));
                }
            }
        }
    }
    Ok(())
}

fn verify_incorporation(
    incorporation: &serde_json::Value,
    prior: &[inventory::ChunkRef],
    current: &[inventory::ChunkRef],
    payload_evidence: &[&str],
) -> ValidationResult<()> {
    let code = ErrorCode::Incorporation;
    let records = incorporation.as_array().ok_or(Error::new(code))?;
    let prior_refs: HashSet<(&str, u64)> =
        prior.iter().map(|c| (c.id.as_str(), c.ordinal)).collect();
    let current_refs: HashSet<(&str, u64)> =
        current.iter().map(|c| (c.id.as_str(), c.ordinal)).collect();
    let mut decision_ids: HashSet<&str> = HashSet::new();
    for record in records {
        let obj = record.as_object().ok_or(Error::new(code))?;
        let exact = [
            "decision_id",
            "authority_ref",
            "source_refs",
            "subject_refs",
            "outcome",
            "evidence_refs",
        ];
        if obj.len() != exact.len() || !exact.iter().all(|k| obj.contains_key(*k)) {
            return Err(Error::new(code));
        }
        for key in ["decision_id", "authority_ref"] {
            if !record[key].as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                return Err(Error::new(code));
            }
        }
        let decision_id = record["decision_id"].as_str().unwrap();
        if !decision_ids.insert(decision_id) {
            return Err(Error::new(code));
        }
        let outcome = record["outcome"].as_str().ok_or(Error::new(code))?;
        if !matches!(outcome, "accepted" | "rejected" | "edited" | "ignored" | "unknown") {
            return Err(Error::new(code));
        }
        let source = occurrence_entries(&record["source_refs"], code)?;
        let subject = occurrence_entries(&record["subject_refs"], code)?;
        for (id, ordinal) in &source {
            if !prior_refs.contains(&(id.as_str(), *ordinal)) {
                return Err(Error::new(code));
            }
        }
        for (id, ordinal) in &subject {
            if !current_refs.contains(&(id.as_str(), *ordinal)) {
                return Err(Error::new(code));
            }
        }
        if matches!(outcome, "accepted" | "rejected" | "edited") && subject.is_empty() {
            return Err(Error::new(code));
        }
        let evidence = record["evidence_refs"].as_array().ok_or(Error::new(code))?;
        for ref_ in evidence {
            let value = ref_.as_str().ok_or(Error::new(code))?;
            if !payload_evidence.contains(&value) {
                return Err(Error::new(code));
            }
        }
    }
    Ok(())
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
    if authority.get("producer_id").and_then(|v| v.as_str()) != Some(producer_id)
        || authority.get("scope") != Some(&scope_val)
        || authority.get("mode").and_then(|v| v.as_str()) != Some(mode)
        || authority.get("finalization_ref").and_then(|v| v.as_str()) != finalization
    {
        return Err(Error::new(ErrorCode::Authority));
    }
    let auth_targets: HashSet<String> = authority["targets"]
        .as_array()
        .ok_or(Error::new(ErrorCode::Authority))?
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::new(ErrorCode::Authority))
                .map(String::from)
        })
        .collect::<ValidationResult<_>>()?;
    if auth_targets != *targets {
        return Err(Error::new(ErrorCode::Authority));
    }
    Ok(())
}
