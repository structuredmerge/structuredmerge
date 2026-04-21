use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceManifest, ConformanceOutcome,
    conformance_family_feature_profile_path, conformance_fixture_path,
    plan_named_conformance_suites, report_conformance_manifest,
};
use serde_json::Value;
use toml_merge::{
    TomlBackend, TomlDialect, TomlOwnerKind, TomlRootKind, available_toml_backends,
    match_toml_owners, merge_toml, parse_toml, toml_backend_feature_profile, toml_feature_profile,
    toml_plan_context,
};
use tree_haver::registered_backends;

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

fn diagnostic_severity_name(severity: ast_merge::DiagnosticSeverity) -> &'static str {
    match severity {
        ast_merge::DiagnosticSeverity::Info => "info",
        ast_merge::DiagnosticSeverity::Warning => "warning",
        ast_merge::DiagnosticSeverity::Error => "error",
    }
}

fn diagnostic_category_name(category: ast_merge::DiagnosticCategory) -> &'static str {
    match category {
        ast_merge::DiagnosticCategory::ParseError => "parse_error",
        ast_merge::DiagnosticCategory::DestinationParseError => "destination_parse_error",
        ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
        ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
        ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
        ast_merge::DiagnosticCategory::AssumedDefault => "assumed_default",
        ast_merge::DiagnosticCategory::ConfigurationError => "configuration_error",
        ast_merge::DiagnosticCategory::ReplayRejected => "replay_rejected",
    }
}

#[test]
fn conforms_to_slice_90_toml_feature_profile_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-90-toml-family-feature-profile",
        "toml-feature-profile.json",
    ]);

    let profile = toml_feature_profile();
    assert_eq!(profile.family, fixture["feature_profile"]["family"].as_str().unwrap());
    assert_eq!(profile.supported_dialects, vec![TomlDialect::Toml]);
    assert_eq!(
        profile.supported_policies,
        vec![ast_merge::PolicyReference {
            surface: ast_merge::PolicySurface::Array,
            name: "destination_wins_array".to_string()
        }]
    );
    assert_eq!(available_toml_backends(), vec![TomlBackend::TreeSitter]);
    assert!(
        registered_backends()
            .iter()
            .any(|backend| backend.id == "kreuzberg-language-pack"
                && backend.family == "tree-sitter")
    );
    assert_eq!(
        toml_backend_feature_profile(Some(TomlBackend::TreeSitter)).backend,
        "kreuzberg-language-pack"
    );
}

#[test]
fn conforms_to_slice_135_toml_backend_feature_profiles() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-135-toml-family-backend-feature-profiles",
        "rust-toml-backend-feature-profiles.json",
    ]);

    let tree_sitter = toml_backend_feature_profile(Some(TomlBackend::TreeSitter));
    assert_eq!(tree_sitter.backend, fixture["tree_sitter"]["backend"].as_str().unwrap());
    assert_eq!(tree_sitter.backend_ref.id, "kreuzberg-language-pack");
    assert_eq!(
        serde_json::json!({
            "id": tree_sitter.backend_ref.id,
            "family": tree_sitter.backend_ref.family,
        }),
        fixture["tree_sitter"]["backend_ref"]
    );
}

#[test]
fn conforms_to_slice_136_toml_plan_contexts() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-136-toml-family-plan-contexts",
        "rust-toml-plan-contexts.json",
    ]);

    let tree_sitter = toml_plan_context(Some(TomlBackend::TreeSitter));
    assert_eq!(
        tree_sitter.family_profile.family,
        fixture["tree_sitter"]["family_profile"]["family"]
    );
    let feature =
        tree_sitter.feature_profile.expect("tree-sitter feature profile should be present");
    assert_eq!(feature.backend, fixture["tree_sitter"]["feature_profile"]["backend"]);
    assert_eq!(
        feature.supports_dialects,
        fixture["tree_sitter"]["feature_profile"]["supports_dialects"]
    );
}

#[test]
fn conforms_to_slice_91_toml_parse_fixtures() {
    let valid = read_fixture(&["toml", "slice-91-parse", "valid-document.json"]);
    let invalid = read_fixture(&["toml", "slice-91-parse", "invalid-document.json"]);

    let valid_result = parse_toml(
        valid["source"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(valid_result.ok);
    assert_eq!(
        valid_result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(TomlRootKind::Table)
    );
    assert!(valid_result.diagnostics.is_empty());

    let invalid_result = parse_toml(
        invalid["source"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(!invalid_result.ok);
    let diagnostics = invalid_result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            serde_json::json!({
                "severity": diagnostic_severity_name(diagnostic.severity),
                "category": diagnostic_category_name(diagnostic.category),
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(diagnostics), invalid["expected"]["diagnostics"]);
}

#[test]
fn conforms_to_slice_92_toml_structure_fixture() {
    let fixture = read_fixture(&["toml", "slice-92-structure", "table-and-array.json"]);

    let result = parse_toml(
        fixture["source"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(result.ok);
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(TomlRootKind::Table)
    );
    let owners = result
        .analysis
        .as_ref()
        .unwrap()
        .owners
        .iter()
        .map(|owner| {
            let mut value = serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    TomlOwnerKind::Table => "table",
                    TomlOwnerKind::KeyValue => "key_value",
                    TomlOwnerKind::ArrayItem => "array_item",
                }
            });
            if let Some(match_key) = &owner.match_key {
                value["match_key"] = serde_json::json!(match_key);
            }
            value
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(owners), fixture["expected"]["owners"]);
}

#[test]
fn conforms_to_slice_93_toml_matching_fixture() {
    let fixture = read_fixture(&["toml", "slice-93-matching", "path-equality.json"]);
    let template = parse_toml(
        fixture["template"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    let destination = parse_toml(
        fixture["destination"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    let result = match_toml_owners(
        template.analysis.as_ref().unwrap(),
        destination.analysis.as_ref().unwrap(),
    );

    let matched = result
        .matched
        .iter()
        .map(|entry| serde_json::json!([entry.template_path, entry.destination_path]))
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(matched), fixture["expected"]["matched"]);
    assert_eq!(
        result.unmatched_template,
        fixture["expected"]["unmatched_template"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_destination,
        fixture["expected"]["unmatched_destination"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn conforms_to_slice_94_toml_merge_fixtures() {
    let merge_fixture = read_fixture(&["toml", "slice-94-merge", "table-merge.json"]);
    let invalid_template = read_fixture(&["toml", "slice-94-merge", "invalid-template.json"]);
    let invalid_destination = read_fixture(&["toml", "slice-94-merge", "invalid-destination.json"]);

    let merge_result = merge_toml(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template_result = merge_toml(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(!invalid_template_result.ok);
    let invalid_template_diagnostics = invalid_template_result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            serde_json::json!({
                "severity": diagnostic_severity_name(diagnostic.severity),
                "category": diagnostic_category_name(diagnostic.category),
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        Value::Array(invalid_template_diagnostics),
        invalid_template["expected"]["diagnostics"]
    );

    let invalid_destination_result = merge_toml(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        TomlDialect::Toml,
        Some(TomlBackend::TreeSitter),
    );
    assert!(!invalid_destination_result.ok);
    let invalid_destination_diagnostics = invalid_destination_result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            serde_json::json!({
                "severity": diagnostic_severity_name(diagnostic.severity),
                "category": diagnostic_category_name(diagnostic.category),
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        Value::Array(invalid_destination_diagnostics),
        invalid_destination["expected"]["diagnostics"]
    );
}

#[test]
fn conforms_to_slice_137_toml_family_manifest_fixture() {
    let manifest_value = read_fixture(&[
        "conformance",
        "slice-137-toml-family-manifest",
        "toml-family-manifest.json",
    ]);
    let manifest: ConformanceManifest =
        serde_json::from_value(manifest_value.clone()).expect("manifest should decode");
    let expected_profile_path = vec![
        "diagnostics".to_string(),
        "slice-90-toml-family-feature-profile".to_string(),
        "toml-feature-profile.json".to_string(),
    ];
    let expected_analysis_path = vec![
        "toml".to_string(),
        "slice-92-structure".to_string(),
        "table-and-array.json".to_string(),
    ];
    let expected_matching_path =
        vec!["toml".to_string(), "slice-93-matching".to_string(), "path-equality.json".to_string()];
    let expected_merge_path =
        vec!["toml".to_string(), "slice-94-merge".to_string(), "table-merge.json".to_string()];

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "toml"),
        Some(expected_profile_path.as_slice())
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "toml", "analysis"),
        Some(expected_analysis_path.as_slice())
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "toml", "matching"),
        Some(expected_matching_path.as_slice())
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "toml", "merge"),
        Some(expected_merge_path.as_slice())
    );
}

#[test]
fn resolves_toml_paths_through_the_canonical_manifest() {
    let manifest_value =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let manifest: ConformanceManifest =
        serde_json::from_value(manifest_value.clone()).expect("manifest should decode");
    let expected_profile_path = vec![
        "diagnostics".to_string(),
        "slice-90-toml-family-feature-profile".to_string(),
        "toml-feature-profile.json".to_string(),
    ];
    let expected_analysis_path = vec![
        "toml".to_string(),
        "slice-92-structure".to_string(),
        "table-and-array.json".to_string(),
    ];

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "toml"),
        Some(expected_profile_path.as_slice())
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "toml", "analysis"),
        Some(expected_analysis_path.as_slice())
    );
}

#[test]
fn conforms_to_slice_139_toml_family_named_suite_plan_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-139-toml-family-named-suite-plans",
        "rust-toml-named-suite-plans.json",
    ]);
    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts = serde_json::from_value(fixture["contexts"].clone()).expect("valid contexts");

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
fn conforms_to_slice_140_toml_family_manifest_report_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-140-toml-family-manifest-report",
        "rust-toml-manifest-report.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let options =
        serde_json::from_value(fixture["options"].clone()).expect("valid planning options");
    let executions =
        fixture["executions"].as_object().expect("executions should be an object").clone();

    assert_eq!(
        serde_json::to_value(report_conformance_manifest(&manifest, &options, |run| {
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
        }))
        .unwrap(),
        fixture["expected_report"]
    );
}
