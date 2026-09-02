use std::{collections::BTreeMap, fs, path::PathBuf};

use ast_merge::{
    PortableBenchmarkCanonicalSummary, PortableBenchmarkConsumer,
    PortableBenchmarkFalseAutoMergeSummary, PortableBenchmarkSafetyGateSummary,
    PortableBenchmarkSummaryCounts, PortableBenchmarkValidationCategory,
};
use serde_json::Value;

const FIXTURE_DIGEST: &str = "cc65ce8cb9312e1487fbde257bb80ae53169da6c5e23299957af653af9a616a8";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/diagnostics/slice-1022-portable-benchmark-contract/contract.json")
}

fn fixture_source() -> Vec<u8> {
    fs::read(fixture_path()).expect("read exact shared Slice 1022 fixture")
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn canonical_summary_matches_the_exact_shared_contract() {
    let consumer =
        PortableBenchmarkConsumer::load_file(fixture_path()).expect("valid shared contract");
    let expected = PortableBenchmarkCanonicalSummary {
        schema: "structuredmerge.benchmark/v1".into(),
        counts: PortableBenchmarkSummaryCounts {
            adapters: 2,
            cases: 6,
            results: 7,
            selected_cases: 5,
            score_eligible_results: 5,
            quality_denominator_excluded_results: 2,
            eligible_false_auto_merges: 1,
        },
        case_ids_by_operation: BTreeMap::from([
            ("diff".into(), string_vec(&["case.diff.json.object-update.v1"])),
            ("merge2".into(), string_vec(&["case.merge2.json.current-owned-fields.v1"])),
            (
                "merge3".into(),
                string_vec(&[
                    "case.merge3.json.independent-fields.v1",
                    "case.merge3.json.region-conflict.v1",
                ]),
            ),
            (
                "history_replay".into(),
                string_vec(&["case.history.ruby.release-events-dotenv-provider.v1"]),
            ),
            ("metamorphic".into(), string_vec(&["case.metamorphic.json.reorder-format.v1"])),
        ]),
        case_ids_by_partition: BTreeMap::from([
            ("sentinel".into(), string_vec(&["case.diff.json.object-update.v1"])),
            (
                "gold".into(),
                string_vec(&[
                    "case.merge2.json.current-owned-fields.v1",
                    "case.merge3.json.independent-fields.v1",
                    "case.merge3.json.region-conflict.v1",
                ]),
            ),
            ("metamorphic".into(), string_vec(&["case.metamorphic.json.reorder-format.v1"])),
            (
                "history".into(),
                string_vec(&["case.history.ruby.release-events-dotenv-provider.v1"]),
            ),
            ("holdout".into(), Vec::new()),
        ]),
        score_eligible_result_ids: string_vec(&[
            "result.base.clean.false-conflict",
            "result.base.conflict.true",
            "result.candidate.clean.correct",
            "result.candidate.conflict.false-auto-merge",
            "result.candidate.diff.error",
        ]),
        quality_denominator_excluded_result_ids: string_vec(&[
            "result.candidate.history.excluded-ambiguous",
            "result.candidate.metamorphic.unsupported",
        ]),
        false_auto_merges: vec![PortableBenchmarkFalseAutoMergeSummary {
            id: "result.candidate.conflict.false-auto-merge".into(),
            severity: "critical".into(),
        }],
        safety_gate: PortableBenchmarkSafetyGateSummary {
            status: "fail".into(),
            eligible_false_auto_merge_count: 1,
            non_compensable: true,
        },
        selection_reason_categories: string_vec(&[
            "budget_exceed",
            "changed_paths",
            "direct_cases",
            "inferred_capabilities",
            "neighbor_samples",
            "sentinels",
        ]),
        contract_digest: FIXTURE_DIGEST.into(),
    };

    assert_eq!(consumer.canonical_summary().unwrap(), expected);
    assert_eq!(consumer.contract_digest, FIXTURE_DIGEST);
    assert_eq!(
        consumer.raw["cases"][0]["family"],
        Value::String("data".into()),
        "raw document retains fields outside the typed interpretation"
    );
}

fn assert_mutation_error(
    mutate: impl FnOnce(&mut Value),
    category: PortableBenchmarkValidationCategory,
    message: &str,
) {
    let mut contract: Value = serde_json::from_slice(&fixture_source()).unwrap();
    mutate(&mut contract);
    let source = serde_json::to_vec(&contract).unwrap();
    let error = PortableBenchmarkConsumer::parse(&source).unwrap_err();
    assert_eq!(error.category, category);
    assert!(
        error.message.contains(message),
        "message {:?} does not contain {message:?}",
        error.message
    );
}

#[test]
fn rejects_bad_inline_digest() {
    assert_mutation_error(
        |contract| {
            contract["cases"][0]["inputs"]["before"]["sha256"] = Value::String("0".repeat(64));
        },
        PortableBenchmarkValidationCategory::InvalidDigest,
        "SHA-256 does not match exact bytes",
    );
}

#[test]
fn rejects_duplicate_case_id() {
    assert_mutation_error(
        |contract| {
            contract["cases"][1]["id"] = contract["cases"][0]["id"].clone();
        },
        PortableBenchmarkValidationCategory::InvalidReference,
        "duplicate case ID",
    );
}

#[test]
fn rejects_dangling_result_case_reference() {
    assert_mutation_error(
        |contract| {
            contract["case_results"][0]["case_id"] = Value::String("case.missing".into());
        },
        PortableBenchmarkValidationCategory::InvalidReference,
        "case_id is dangling",
    );
}

#[test]
fn rejects_eligible_excluded_result() {
    assert_mutation_error(
        |contract| {
            let result = contract["case_results"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|result| result["outcome"] == "excluded_ambiguous")
                .unwrap();
            result["score_eligible"] = Value::Bool(true);
        },
        PortableBenchmarkValidationCategory::InvalidEligibility,
        "score eligibility conflicts",
    );
}

#[test]
fn rejects_compensable_false_auto_merge() {
    assert_mutation_error(
        |contract| {
            let result = contract["case_results"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|result| result["outcome"] == "false_auto_merge")
                .unwrap();
            result["dimensions"]["safety"]["compensable"] = Value::Bool(true);
        },
        PortableBenchmarkValidationCategory::InvalidSafety,
        "cannot be compensable",
    );
}

#[test]
fn rejects_llm_oracle_in_micro_hard_gate() {
    assert_mutation_error(
        |contract| {
            contract["run_manifest"]["profile"] = Value::String("micro".into());
            contract["cases"][0]["oracle"]["class"] = Value::String("llm".into());
        },
        PortableBenchmarkValidationCategory::ProhibitedLlmGate,
        "LLM oracle cannot participate in micro hard gates",
    );
}
