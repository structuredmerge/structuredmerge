//! Extended contract-surface tests: optional fields from slices 1047–1054.

use ingest_observation::{Context, Envelope, State, verify};
use serde_json::{Value, json};

fn sha256_hex(data: &str) -> String {
    format!("sha256:{}", {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    })
}

fn descriptor(bytes: &str, domain: &str) -> Value {
    use base64::Engine;
    json!({
        "domain": domain,
        "byte_length": bytes.len(),
        "digest": sha256_hex(bytes),
        "inline_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

fn context_for() -> Context {
    serde_json::from_value(json!({
        "scope": {
            "workspace_id": "workspace-a",
            "source_id": "source-a",
            "document_id": "doc-a",
            "enumeration": "document-chunks"
        },
        "producer_id": "designed-producer",
        "head_revision_ref": "",
        "prior_inventory_digest": "",
        "evidence_refs": ["evidence:final"],
        "authorities": {},
        "resources": {}
    }))
    .expect("context parses")
}

fn base_payload(extra: Value) -> Value {
    let mut payload = json!({
        "producer": {"id": "designed-producer", "version": "1", "contract_ref": "fixture:extended"},
        "scope": {
            "workspace_id": "workspace-a",
            "source_id": "source-a",
            "document_id": "doc-a",
            "enumeration": "document-chunks"
        },
        "observation_id": "obs:extended:1",
        "revision_ref": "rev:1",
        "source_revision_ref": null,
        "expected_prior_revision_ref": null,
        "prior_inventory": null,
        "current_inventory": descriptor(
            "{\"scope\":{\"workspace_id\":\"workspace-a\",\"source_id\":\"source-a\",\"document_id\":\"doc-a\",\"enumeration\":\"document-chunks\"},\"revision_ref\":\"rev:1\",\"chunks\":[]}",
            "current-inventory-json"
        ),
        "added": [],
        "updated": [],
        "removed": [],
        "original": descriptor("Alpha\n\nBeta\n", "original-source-bytes"),
        "normalized": descriptor("Alpha\n\nBeta\n", "normalized-input-utf8"),
        "processing": {
            "descriptor": descriptor(
                "{\"schema\":\"structuredmerge.ingest-processing/v1\",\"normalizer\":{\"id\":\"identity\",\"version\":\"1\",\"config\":{}},\"chunker\":{\"id\":\"plain\",\"version\":\"1\",\"config\":{}},\"hash\":{\"algorithm\":\"sha256\",\"domain\":\"chunk-text-utf8\"},\"parser\":{\"status\":\"not_applicable\"}}",
                "processing-json"
            ),
            "fingerprint": sha256_hex("{\"schema\":\"structuredmerge.ingest-processing/v1\",\"normalizer\":{\"id\":\"identity\",\"version\":\"1\",\"config\":{}},\"chunker\":{\"id\":\"plain\",\"version\":\"1\",\"config\":{}},\"hash\":{\"algorithm\":\"sha256\",\"domain\":\"chunk-text-utf8\"},\"parser\":{\"status\":\"not_applicable\"}}"),
        },
        "times": {"source_at": null, "observed_at": "2026-10-26T10:00:00Z", "processed_at": "2026-10-26T10:01:00Z"},
        "scan": {
            "status": "complete",
            "enumerated_scope": {
                "workspace_id": "workspace-a",
                "source_id": "source-a",
                "document_id": "doc-a",
                "enumeration": "document-chunks"
            },
            "checkpoint_ref": null,
            "finalization_ref": "evidence:final"
        },
        "deletion": {"mode": "none", "authority_ref": null, "targets": []},
        "evidence_refs": [],
        "extensions": {}
    });
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            payload[key.as_str()] = value.clone();
        }
    }
    payload
}

fn envelope_for(payload: &Value) -> Envelope {
    let payload_json = serde_json::to_string(payload).expect("serializes");
    Envelope {
        schema: "structuredmerge.ingest-observation/v2".to_string(),
        payload_digest: sha256_hex(&payload_json),
        payload_json,
    }
}

fn run(extra: Value) -> Result<ingest_observation::Decision, ingest_observation::Error> {
    verify(&envelope_for(&base_payload(extra)), &context_for(), &mut State::default())
}

#[test]
fn unknown_top_level_field_rejects() {
    let error = run(json!({"bogus": true})).expect_err("must reject");
    assert_eq!(error.code().to_string(), "fields");
}

#[test]
fn declared_ancestry_with_vacuous_prior_verifies() {
    // Root declaration with no prior is a valid initial observation.
    let decision = run(json!({"revision_parents": []})).expect("verifies");
    assert!(!decision.replay);
}

#[test]
fn embedding_contract_valid_and_invalid() {
    let valid = run(json!({
        "embedding_contract": {
            "owner": "ur-brain-pgvector", "model_ref": "model:embed-a",
            "model_revision": null, "dimensions": 1536,
            "normalization": null, "instruction_profile": null
        }
    }))
    .expect("verifies");
    assert!(!valid.replay);

    let error = run(json!({
        "embedding_contract": {
            "owner": "ur-brain-pgvector", "model_ref": "model:embed-a",
            "model_revision": null, "dimensions": -1,
            "normalization": null, "instruction_profile": null
        }
    }))
    .expect_err("must reject");
    assert_eq!(error.code().to_string(), "embedding");
}

#[test]
fn source_interval_rules() {
    run(json!({
        "source_time_start": "2026-10-26T09:00:00Z",
        "source_time_end": "2026-10-26T09:05:00Z"
    }))
    .expect("valid interval verifies");

    let inverted = run(json!({
        "source_time_start": "2026-10-26T09:05:00Z",
        "source_time_end": "2026-10-26T09:00:00Z"
    }))
    .expect_err("must reject");
    assert_eq!(inverted.code().to_string(), "time");

    let half = run(json!({"source_time_start": "2026-10-26T09:00:00Z"})).expect_err("must reject");
    assert_eq!(half.code().to_string(), "time");
}

#[test]
fn revision_reuse_with_same_projection_evidence_verifies() {
    let payload = base_payload(Value::Null);
    let envelope = envelope_for(&payload);
    let context = context_for();
    let mut state = State::default();
    verify(&envelope, &context, &mut state).expect("first accepts");

    // Re-observation: expected prior equals the re-referenced revision, with
    // the trusted context advanced to that head (as a real consumer would).
    let inventory_json = "{\"scope\":{\"workspace_id\":\"workspace-a\",\"source_id\":\"source-a\",\"document_id\":\"doc-a\",\"enumeration\":\"document-chunks\"},\"revision_ref\":\"rev:1\",\"chunks\":[]}";
    let mut advanced = serde_json::to_value(&context).expect("context serializes");
    advanced["head_revision_ref"] = json!("rev:1");
    advanced["prior_inventory_digest"] = json!(sha256_hex(inventory_json));
    let advanced: Context = serde_json::from_value(advanced).expect("advanced context parses");

    let mut reobserved = payload.clone();
    reobserved["observation_id"] = json!("obs:extended:2");
    reobserved["expected_prior_revision_ref"] = json!("rev:1");
    reobserved["prior_inventory"] = descriptor(inventory_json, "prior-inventory-json");
    let second = envelope_for(&reobserved);
    let decision = verify(&second, &advanced, &mut state).expect("re-observation verifies");
    assert!(!decision.replay);

    // Same revision with changed projection evidence rejects.
    let mut contradictory = reobserved.clone();
    contradictory["observation_id"] = json!("obs:extended:3");
    contradictory["original"] = descriptor("Changed original bytes\n", "original-source-bytes");
    let third = envelope_for(&contradictory);
    let error = verify(&third, &advanced, &mut state).expect_err("must reject");
    assert_eq!(error.code().to_string(), "revision");
}

#[test]
fn decision_carries_retained_evidence_manifest() {
    let decision = run(Value::Null).expect("verifies");
    assert_eq!(decision.retained_evidence.len(), 4);
    assert!(decision.retained_evidence.iter().all(|e| e.via == "inline_base64"));
    assert!(decision.tombstones.is_empty());
}

#[test]
fn incorporation_rules_enforced() {
    let valid = run(json!({
        "incorporation": [{
            "decision_id": "dec:1",
            "authority_ref": "authority:user:review",
            "source_refs": [],
            "subject_refs": [],
            "outcome": "unknown",
            "evidence_refs": []
        }]
    }))
    .expect("unknown outcome with empty sides verifies");
    assert!(!valid.replay);

    let error = run(json!({
        "incorporation": [{
            "decision_id": "dec:1",
            "authority_ref": "authority:user:review",
            "source_refs": [],
            "subject_refs": [],
            "outcome": "accepted",
            "evidence_refs": []
        }]
    }))
    .expect_err("accepted without subject must reject");
    assert_eq!(error.code().to_string(), "incorporation");
}

#[test]
fn lineage_rules_enforced() {
    let ambiguous = run(json!({
        "lineage": [{
            "kind": "move", "from": [], "to": [],
            "provenance": {"basis": "unknown", "authority_ref": null,
                            "algorithm": null, "confidence": null},
            "status": "ambiguous"
        }]
    }))
    .expect("ambiguous assertion verifies");
    assert!(!ambiguous.replay);

    let error = run(json!({
        "lineage": [{
            "kind": "move", "from": [], "to": [],
            "provenance": {"basis": "unknown", "authority_ref": null,
                            "algorithm": null, "confidence": null},
            "status": "asserted"
        }]
    }))
    .expect_err("asserted with empty sides must reject");
    assert_eq!(error.code().to_string(), "lineage");
}
