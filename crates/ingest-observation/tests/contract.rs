//! Contract test runner — exercises v2 fixture vectors against Rust implementation.

use ingest_observation::{Context, Decision, Envelope, State, verify};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct FixtureEnvelope {
    schema: String,
    payload_json: String,
    payload_digest: String,
}

#[derive(Debug, Deserialize)]
struct FixtureContext {
    scope: serde_json::Map<String, serde_json::Value>,
    producer_id: String,
    head_revision_ref: String,
    prior_inventory_digest: String,
    evidence_refs: HashSet<String>,
    authorities: HashMap<String, serde_json::Value>,
    resources: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Expectation {
    Success { status: String, authorized_delete_ids: Vec<String>, replay: bool },
    Error { error: String },
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    name: String,
    envelope: FixtureEnvelope,
    context: FixtureContext,
    expect: Expectation,
}

#[derive(Debug, Deserialize)]
struct FixtureSequence {
    name: String,
    context: FixtureContext,
    steps: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct ContractFixture {
    schema: String,
    kind: String,
    cases: Vec<FixtureCase>,
    sequences: Vec<FixtureSequence>,
}

fn fixture_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../../fixtures/diagnostics/slice-1045-ingest-observation-deletion-authority/contract-v2.json")
}

fn load_contract() -> ContractFixture {
    let path = fixture_path();
    let content =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", path));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse fixture: {}", e))
}

fn to_envelope(f: &FixtureEnvelope) -> Envelope {
    Envelope {
        schema: f.schema.clone(),
        payload_json: f.payload_json.clone(),
        payload_digest: f.payload_digest.clone(),
    }
}

fn to_context(f: &FixtureContext) -> Context {
    Context {
        scope: f.scope.clone(),
        producer_id: f.producer_id.clone(),
        head_revision_ref: f.head_revision_ref.clone(),
        prior_inventory_digest: f.prior_inventory_digest.clone(),
        evidence_refs: f.evidence_refs.clone(),
        authorities: f.authorities.clone(),
        resources: f.resources.clone(),
    }
}

fn check_case(case: &FixtureCase, state: &mut State) {
    let envelope = to_envelope(&case.envelope);
    let context = to_context(&case.context);
    match &case.expect {
        Expectation::Error { error } => {
            let result = verify(&envelope, &context, state);
            assert!(result.is_err(), "Expected error '{}' but got Ok: {:?}", error, result);
            let err = result.unwrap_err();
            assert_eq!(
                err.code().to_string(),
                *error,
                "Wrong error code for {}: got {}",
                case.name,
                err.code()
            );
        }
        Expectation::Success { status, authorized_delete_ids, replay } => {
            let decision = verify(&envelope, &context, state)
                .unwrap_or_else(|e| panic!("{} failed: {}", case.name, e));
            assert_eq!(
                decision.authorized_delete_ids, *authorized_delete_ids,
                "Delete IDs mismatch for {}",
                case.name
            );
            assert_eq!(decision.replay, *replay, "Replay flag mismatch for {}", case.name);
            assert_eq!(status, "verified", "Unexpected status for {}", case.name);
        }
    }
}

#[test]
fn shared_positive_and_negative_cases() {
    let contract = load_contract();
    assert!(!contract.cases.is_empty(), "No cases in contract");

    for case in &contract.cases {
        let mut state = State::default();
        check_case(case, &mut state);
    }

    for sequence in &contract.sequences {
        let mut state = State::default();
        for step in &sequence.steps {
            check_case(step, &mut state);
        }
    }
}
