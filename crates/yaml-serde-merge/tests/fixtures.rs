use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceManifest, ConformanceOutcome,
    plan_named_conformance_suites, report_named_conformance_suite_envelope,
    report_planned_named_conformance_suites,
};
use serde_json::Value;
use tree_haver::registered_backends;
use yaml_merge::YamlDialect;
use yaml_serde_merge::{
    available_yaml_backends, match_yaml_owners, merge_yaml, parse_yaml,
    provider_yaml_feature_profile, yaml_backend_feature_profile, yaml_plan_context,
};

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
        "slice-277-yaml-provider-feature-profiles",
        "rust-yaml-provider-feature-profiles.json",
    ]);

    assert_eq!(available_yaml_backends(), vec!["yaml_serde".to_string()]);
    assert!(registered_backends()
        .iter()
        .any(|backend| backend.id == "yaml_serde" && backend.family == "native"));
    let profile = yaml_backend_feature_profile();
    let supported_dialects = profile
        .supported_dialects
        .iter()
        .map(|dialect| match dialect {
            yaml_merge::YamlDialect::Yaml => "yaml".to_string(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::json!({
            "family": profile.family,
            "supported_dialects": supported_dialects,
            "supported_policies": profile.supported_policies,
            "backend": profile.backend,
            "backend_ref": profile.backend_ref.map(|backend| serde_json::json!({
                "id": backend.id,
                "family": backend.family,
            })),
        }),
        fixture["providers"]["yaml_serde"]["feature_profile"]
    );
    assert_eq!(provider_yaml_feature_profile().family, "yaml");
}

#[test]
fn conforms_to_provider_plan_context_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-278-yaml-provider-plan-contexts",
        "rust-yaml-provider-plan-contexts.json",
    ]);

    assert_eq!(
        serde_json::to_value(yaml_plan_context()).unwrap(),
        fixture["providers"]["yaml_serde"]
    );
}

#[test]
fn conforms_to_shared_yaml_parse_matching_and_merge_fixtures() {
    let valid = read_fixture(&["yaml", "slice-96-parse", "valid-document.json"]);
    let valid_result = parse_yaml(valid["source"].as_str().unwrap(), YamlDialect::Yaml, None);
    assert!(valid_result.ok);

    let matching_fixture = read_fixture(&["yaml", "slice-98-matching", "path-equality.json"]);
    let template =
        parse_yaml(matching_fixture["template"].as_str().unwrap(), YamlDialect::Yaml, None)
            .analysis
            .unwrap();
    let destination =
        parse_yaml(matching_fixture["destination"].as_str().unwrap(), YamlDialect::Yaml, None)
            .analysis
            .unwrap();
    let result = match_yaml_owners(&template, &destination);
    assert_eq!(
        result.matched.len(),
        matching_fixture["expected"]["matched"].as_array().unwrap().len()
    );

    let merge_fixture = read_fixture(&["yaml", "slice-99-merge", "mapping-merge.json"]);
    let merge_result = merge_yaml(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        YamlDialect::Yaml,
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
        "slice-279-yaml-provider-named-suite-plans",
        "rust-yaml-provider-named-suite-plans.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts =
        serde_json::from_value(fixture["contexts"]["yaml_serde"].clone()).expect("valid contexts");

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

    assert_eq!(serde_json::to_value(projected).unwrap(), fixture["expected_entries"]["yaml_serde"]);
}

#[test]
fn conforms_to_provider_manifest_report_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-280-yaml-provider-manifest-report",
        "rust-yaml-provider-manifest-report.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts = serde_json::from_value(fixture["options"]["yaml_serde"]["contexts"].clone())
        .expect("valid contexts");
    let executions = fixture["executions"]["yaml_serde"]
        .as_object()
        .expect("executions should be an object")
        .clone();

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
        fixture["expected_reports"]["yaml_serde"]
    );
}
