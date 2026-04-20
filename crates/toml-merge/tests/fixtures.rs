use std::{fs, path::PathBuf};

use serde_json::Value;
use toml_merge::{
    TomlDialect, TomlOwnerKind, TomlRootKind, match_toml_owners, merge_toml, parse_toml,
    toml_feature_profile,
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
}

#[test]
fn conforms_to_slice_91_toml_parse_fixtures() {
    let valid = read_fixture(&["toml", "slice-91-parse", "valid-document.json"]);
    let valid_result = parse_toml(valid["source"].as_str().unwrap(), TomlDialect::Toml);
    assert!(valid_result.ok);
    assert_eq!(
        valid_result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(TomlRootKind::Table)
    );
    assert!(valid_result.diagnostics.is_empty());

    let invalid = read_fixture(&["toml", "slice-91-parse", "invalid-document.json"]);
    let invalid_result = parse_toml(invalid["source"].as_str().unwrap(), TomlDialect::Toml);
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
    let result = parse_toml(fixture["source"].as_str().unwrap(), TomlDialect::Toml);

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
    let template = parse_toml(fixture["template"].as_str().unwrap(), TomlDialect::Toml);
    let destination = parse_toml(fixture["destination"].as_str().unwrap(), TomlDialect::Toml);
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
    let merge_result = merge_toml(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        TomlDialect::Toml,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template = read_fixture(&["toml", "slice-94-merge", "invalid-template.json"]);
    let invalid_template_result = merge_toml(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        TomlDialect::Toml,
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

    let invalid_destination = read_fixture(&["toml", "slice-94-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_toml(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        TomlDialect::Toml,
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
