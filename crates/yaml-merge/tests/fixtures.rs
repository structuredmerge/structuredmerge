use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceManifest, conformance_family_feature_profile_path, conformance_fixture_path,
};
use serde_json::Value;
use yaml_merge::{
    YamlDialect, YamlOwnerKind, YamlRootKind, match_yaml_owners, merge_yaml, parse_yaml,
    yaml_feature_profile, yaml_plan_context,
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
fn conforms_to_slice_95_yaml_feature_profile_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-95-yaml-family-feature-profile",
        "yaml-feature-profile.json",
    ]);

    let profile = yaml_feature_profile();
    assert_eq!(profile.family, fixture["feature_profile"]["family"].as_str().unwrap());
    assert_eq!(profile.supported_dialects, vec![YamlDialect::Yaml]);
    assert_eq!(
        profile.supported_policies,
        vec![ast_merge::PolicyReference {
            surface: ast_merge::PolicySurface::Array,
            name: "destination_wins_array".to_string()
        }]
    );
}

#[test]
fn conforms_to_slice_142_yaml_plan_context_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-142-yaml-family-plan-contexts",
        "rust-yaml-plan-contexts.json",
    ]);

    let context = yaml_plan_context();
    assert_eq!(context.family_profile.family, fixture["native"]["family_profile"]["family"]);
    let feature = context.feature_profile.expect("feature profile should be present");
    assert_eq!(feature.backend, fixture["native"]["feature_profile"]["backend"]);
    assert_eq!(
        feature.supports_dialects,
        fixture["native"]["feature_profile"]["supports_dialects"]
    );
}

#[test]
fn conforms_to_slice_143_yaml_family_manifest_fixture() {
    let fixture = read_fixture(&[
        "conformance",
        "slice-143-yaml-family-manifest",
        "yaml-family-manifest.json",
    ]);
    let manifest: ConformanceManifest = serde_json::from_value(fixture).expect("valid manifest");

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "yaml"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-95-yaml-family-feature-profile".to_string(),
                "yaml-feature-profile.json".to_string()
            ][..]
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "yaml", "analysis"),
        Some(
            &[
                "yaml".to_string(),
                "slice-97-structure".to_string(),
                "mapping-and-sequence.json".to_string()
            ][..]
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "yaml", "merge"),
        Some(
            &["yaml".to_string(), "slice-99-merge".to_string(), "mapping-merge.json".to_string()][..]
        )
    );
}

#[test]
fn resolves_yaml_paths_through_the_canonical_manifest() {
    let fixture =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let manifest: ConformanceManifest = serde_json::from_value(fixture).expect("valid manifest");

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "yaml"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-95-yaml-family-feature-profile".to_string(),
                "yaml-feature-profile.json".to_string()
            ][..]
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "yaml", "matching"),
        Some(
            &[
                "yaml".to_string(),
                "slice-98-matching".to_string(),
                "path-equality.json".to_string()
            ][..]
        )
    );
}

#[test]
fn conforms_to_slice_96_yaml_parse_fixtures() {
    let valid = read_fixture(&["yaml", "slice-96-parse", "valid-document.json"]);
    let valid_result = parse_yaml(valid["source"].as_str().unwrap(), YamlDialect::Yaml);
    assert!(valid_result.ok);
    assert_eq!(
        valid_result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(YamlRootKind::Mapping)
    );
    assert!(valid_result.diagnostics.is_empty());

    let invalid = read_fixture(&["yaml", "slice-96-parse", "invalid-document.json"]);
    let invalid_result = parse_yaml(invalid["source"].as_str().unwrap(), YamlDialect::Yaml);
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
fn conforms_to_slice_97_yaml_structure_fixture() {
    let fixture = read_fixture(&["yaml", "slice-97-structure", "mapping-and-sequence.json"]);
    let result = parse_yaml(fixture["source"].as_str().unwrap(), YamlDialect::Yaml);

    assert!(result.ok);
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(YamlRootKind::Mapping)
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
                    YamlOwnerKind::Mapping => "mapping",
                    YamlOwnerKind::KeyValue => "key_value",
                    YamlOwnerKind::SequenceItem => "sequence_item",
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
fn conforms_to_slice_98_yaml_matching_fixture() {
    let fixture = read_fixture(&["yaml", "slice-98-matching", "path-equality.json"]);
    let template = parse_yaml(fixture["template"].as_str().unwrap(), YamlDialect::Yaml);
    let destination = parse_yaml(fixture["destination"].as_str().unwrap(), YamlDialect::Yaml);
    let result = match_yaml_owners(
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
fn conforms_to_slice_99_yaml_merge_fixtures() {
    let merge_fixture = read_fixture(&["yaml", "slice-99-merge", "mapping-merge.json"]);
    let merge_result = merge_yaml(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        YamlDialect::Yaml,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template = read_fixture(&["yaml", "slice-99-merge", "invalid-template.json"]);
    let invalid_template_result = merge_yaml(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        YamlDialect::Yaml,
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

    let invalid_destination = read_fixture(&["yaml", "slice-99-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_yaml(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        YamlDialect::Yaml,
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
