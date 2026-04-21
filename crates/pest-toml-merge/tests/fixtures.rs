use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceManifest, ConformanceOutcome,
    plan_named_conformance_suites, report_named_conformance_suite_envelope,
    report_planned_named_conformance_suites,
};
use pest_toml_merge::{
    available_toml_backends, match_toml_owners, merge_toml, parse_toml,
    provider_toml_feature_profile, toml_backend_feature_profile, toml_plan_context,
};
use serde_json::Value;
use toml_merge::TomlDialect;

fn fixture_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("..");
    path.push("fixtures");
    for part in parts {
        path.push(part);
    }
    path
}

fn read_fixture(parts: &[&str]) -> Value {
    let source = fs::read_to_string(fixture_path(parts)).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_provider_feature_profile_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-269-toml-provider-feature-profiles",
        "rust-toml-provider-feature-profiles.json",
    ]);

    assert_eq!(available_toml_backends(), vec!["pest".to_string()]);
    assert_eq!(
        serde_json::to_value(toml_backend_feature_profile()).unwrap(),
        fixture["providers"]["pest"]["feature_profile"]
    );
    assert_eq!(provider_toml_feature_profile().family, "toml");
}

#[test]
fn conforms_to_provider_plan_context_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-270-toml-provider-plan-contexts",
        "rust-toml-provider-plan-contexts.json",
    ]);

    assert_eq!(serde_json::to_value(toml_plan_context()).unwrap(), fixture["providers"]["pest"]);
}

#[test]
fn conforms_to_shared_toml_parse_matching_and_merge_fixtures() {
    let valid = read_fixture(&["toml", "slice-91-parse", "valid-document.json"]);
    let valid_result = parse_toml(valid["source"].as_str().unwrap(), TomlDialect::Toml, None);
    assert!(valid_result.ok);

    let matching_fixture = read_fixture(&["toml", "slice-93-matching", "path-equality.json"]);
    let template =
        parse_toml(matching_fixture["template"].as_str().unwrap(), TomlDialect::Toml, None)
            .analysis
            .unwrap();
    let destination =
        parse_toml(matching_fixture["destination"].as_str().unwrap(), TomlDialect::Toml, None)
            .analysis
            .unwrap();
    let result = match_toml_owners(&template, &destination);
    assert_eq!(
        result
            .matched
            .iter()
            .map(|item| vec![item.template_path.clone(), item.destination_path.clone()])
            .collect::<Vec<_>>(),
        matching_fixture["expected"]["matched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                item.as_array()
                    .unwrap()
                    .iter()
                    .map(|part| part.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    );

    let merge_fixture = read_fixture(&["toml", "slice-94-merge", "table-merge.json"]);
    let merge_result = merge_toml(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        TomlDialect::Toml,
        None,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );
}

#[test]
fn conforms_to_provider_named_suite_plan_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-271-toml-provider-named-suite-plans",
        "rust-toml-provider-named-suite-plans.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts =
        serde_json::from_value(fixture["contexts"].clone()).expect("valid provider contexts");

    let projected = plan_named_conformance_suites(&manifest, &contexts)
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "suite": entry.suite,
                "plan": {
                    "family": entry.plan.family,
                    "entries": entry.plan.entries.into_iter().map(|plan_entry| {
                        serde_json::json!({
                            "ref": plan_entry.ref_,
                            "path": plan_entry.path,
                            "run": {
                                "ref": plan_entry.run.ref_,
                                "requirements": {},
                                "family_profile": plan_entry.run.family_profile,
                                "feature_profile": plan_entry.run.feature_profile
                            }
                        })
                    }).collect::<Vec<_>>(),
                    "missing_roles": entry.plan.missing_roles
                }
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(serde_json::to_value(projected).unwrap(), fixture["expected_entries"]);
}

#[test]
fn conforms_to_provider_manifest_report_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-272-toml-provider-manifest-report",
        "rust-toml-provider-manifest-report.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts =
        serde_json::from_value(fixture["options"]["contexts"].clone()).expect("valid contexts");
    let executions =
        fixture["executions"].as_object().expect("executions should be an object").clone();

    let entries = report_planned_named_conformance_suites(
        &plan_named_conformance_suites(&manifest, &contexts),
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            executions
                .get(&key)
                .cloned()
                .map(|value| {
                    serde_json::from_value::<ConformanceCaseExecution>(value)
                        .expect("valid execution")
                })
                .unwrap_or(ConformanceCaseExecution {
                    outcome: ConformanceOutcome::Failed,
                    messages: vec!["missing execution".to_string()],
                })
        },
    );

    assert_eq!(
        serde_json::to_value(report_named_conformance_suite_envelope(&entries)).unwrap(),
        fixture["expected_report"]
    );
}
