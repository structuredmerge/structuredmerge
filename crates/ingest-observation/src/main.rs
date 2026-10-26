//! Single-shot verification CLI for ingest-observation v2 envelopes.
//!
//! File-based contract matching the merge-driver style: reads envelope and
//! trusted context from files, writes the decision (or error) to stdout.
//! Exit 0 = verified, exit 1 = verification failed, exit 2 = usage/IO error.
//! Stateful replay and head management belong to the calling consumer.

use std::collections::HashMap;
use std::process::ExitCode;

use ingest_observation::{verify, Context, Decision, Envelope, State};
use serde::Deserialize;

#[derive(Deserialize)]
struct FileContext {
    scope: serde_json::Map<String, serde_json::Value>,
    producer_id: String,
    #[serde(default)]
    head_revision_ref: String,
    #[serde(default)]
    prior_inventory_digest: String,
    #[serde(default)]
    evidence_refs: std::collections::HashSet<String>,
    #[serde(default)]
    authorities: HashMap<String, serde_json::Value>,
    #[serde(default)]
    resources: HashMap<String, String>,
}

impl From<FileContext> for Context {
    fn from(f: FileContext) -> Self {
        Context {
            scope: f.scope,
            producer_id: f.producer_id,
            head_revision_ref: f.head_revision_ref,
            prior_inventory_digest: f.prior_inventory_digest,
            evidence_refs: f.evidence_refs,
            authorities: f.authorities,
            resources: f.resources,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: ingest-observation-verify <envelope.json> <context.json>");
        return ExitCode::from(2);
    }

    let envelope_path = &args[1];
    let context_path = &args[2];

    let envelope_raw = match std::fs::read(envelope_path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("error: cannot read envelope {}: {}", envelope_path, e);
            return ExitCode::from(2);
        }
    };
    let context_raw = match std::fs::read(context_path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("error: cannot read context {}: {}", context_path, e);
            return ExitCode::from(2);
        }
    };

    let envelope: Envelope = match serde_json::from_slice(&envelope_raw) {
        Ok(envelope) => envelope,
        Err(e) => {
            eprintln!("error: invalid envelope JSON: {}", e);
            return ExitCode::from(2);
        }
    };
    let file_context: FileContext = match serde_json::from_slice(&context_raw) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("error: invalid context JSON: {}", e);
            return ExitCode::from(2);
        }
    };
    let context: Context = file_context.into();

    let mut state = State::default();
    match verify(&envelope, &context, &mut state) {
        Ok(decision) => {
            print_decision(&decision);
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = err.code().to_string();
            let message = err.to_string();
            let output = serde_json::json!({
                "status": "rejected",
                "error_code": code,
                "error": message,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            ExitCode::FAILURE
        }
    }
}

fn print_decision(decision: &Decision) {
    let mut sorted_ids = decision.authorized_delete_ids.clone();
    sorted_ids.sort();
    let output = serde_json::json!({
        "status": "verified",
        "authorized_delete_ids": sorted_ids,
        "replay": decision.replay,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}