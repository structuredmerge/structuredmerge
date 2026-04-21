use std::{fs, path::PathBuf};

use go_merge::{
    GoBackend, GoDialect, go_backend_feature_profile, go_backends, go_feature_profile,
    go_plan_context, match_go_owners, merge_go, parse_go,
};
use serde_json::Value;

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

fn diagnostic_shape(diagnostics: &[ast_merge::Diagnostic]) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|diagnostic| {
                serde_json::json!({
                    "severity": match diagnostic.severity {
                        ast_merge::DiagnosticSeverity::Info => "info",
                        ast_merge::DiagnosticSeverity::Warning => "warning",
                        ast_merge::DiagnosticSeverity::Error => "error",
                    },
                    "category": match diagnostic.category {
                        ast_merge::DiagnosticCategory::ParseError => "parse_error",
                        ast_merge::DiagnosticCategory::DestinationParseError => "destination_parse_error",
                        ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                        ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                        ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                        ast_merge::DiagnosticCategory::AssumedDefault => "assumed_default",
                        ast_merge::DiagnosticCategory::ConfigurationError => "configuration_error",
                        ast_merge::DiagnosticCategory::ReplayRejected => "replay_rejected",
                    }
                })
            })
            .collect(),
    )
}

#[test]
fn conforms_to_go_fixtures() {
    let profile_fixture = read_fixture(&[
        "diagnostics",
        "slice-109-go-family-feature-profile",
        "go-feature-profile.json",
    ]);
    let profile = go_feature_profile();
    assert_eq!(profile.family, profile_fixture["feature_profile"]["family"].as_str().unwrap());

    let analysis_fixture = read_fixture(&["go", "slice-110-analysis", "module-owners.json"]);
    let analysis = parse_go(analysis_fixture["source"].as_str().unwrap(), GoDialect::Go);
    assert!(analysis.ok);
    let owners = analysis
        .analysis
        .as_ref()
        .unwrap()
        .owners
        .iter()
        .map(|owner| {
            serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    go_merge::GoOwnerKind::Import => "import",
                    go_merge::GoOwnerKind::Declaration => "declaration",
                },
                "match_key": owner.match_key,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(owners), analysis_fixture["expected"]["owners"]);

    let matching_fixture = read_fixture(&["go", "slice-111-matching", "path-equality.json"]);
    let template = parse_go(matching_fixture["template"].as_str().unwrap(), GoDialect::Go);
    let destination = parse_go(matching_fixture["destination"].as_str().unwrap(), GoDialect::Go);
    let matched = match_go_owners(
        template.analysis.as_ref().unwrap(),
        destination.analysis.as_ref().unwrap(),
    );
    assert_eq!(
        Value::Array(
            matched
                .matched
                .iter()
                .map(|entry| serde_json::json!([entry.template_path, entry.destination_path]))
                .collect()
        ),
        matching_fixture["expected"]["matched"]
    );

    let merge_fixture = read_fixture(&["go", "slice-112-merge", "module-merge.json"]);
    let merge_result = merge_go(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        GoDialect::Go,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template = read_fixture(&["go", "slice-112-merge", "invalid-template.json"]);
    let invalid_template_result = merge_go(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        GoDialect::Go,
    );
    assert!(!invalid_template_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_template_result.diagnostics),
        invalid_template["expected"]["diagnostics"]
    );

    let invalid_destination = read_fixture(&["go", "slice-112-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_go(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        GoDialect::Go,
    );
    assert!(!invalid_destination_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_destination_result.diagnostics),
        invalid_destination["expected"]["diagnostics"]
    );

    let backends_fixture =
        read_fixture(&["diagnostics", "slice-113-go-family-backends", "go-backends.json"]);
    assert_eq!(go_backends(), vec![GoBackend::TreeSitter]);
    assert_eq!(
        Value::Array(vec![Value::String("kreuzberg-language-pack".to_string())]),
        backends_fixture["backends"]
    );

    let backend_fixture = read_fixture(&[
        "diagnostics",
        "slice-122-source-family-backend-feature-profiles",
        "go-backend-feature-profiles.json",
    ]);
    let backend_profile = go_backend_feature_profile(GoBackend::TreeSitter);
    let backend_ref = backend_profile
        .backend_ref
        .as_ref()
        .map(|reference| {
            serde_json::json!({
                "id": reference.id,
                "family": reference.family,
            })
        })
        .unwrap_or(Value::Null);
    assert_eq!(
        serde_json::json!({
            "backend": backend_profile.backend,
            "supports_dialects": backend_profile.supports_dialects,
            "supported_policies": backend_profile.supported_policies,
            "backend_ref": backend_ref,
        }),
        backend_fixture["tree_sitter"]
    );

    let plan_fixture = read_fixture(&[
        "diagnostics",
        "slice-123-source-family-plan-contexts",
        "go-plan-contexts.json",
    ]);
    assert_eq!(
        serde_json::to_value(go_plan_context(GoBackend::TreeSitter))
            .expect("go plan context should serialize"),
        plan_fixture["tree_sitter"]
    );

    let source_manifest_fixture = read_fixture(&[
        "conformance",
        "slice-124-source-family-manifest",
        "source-family-manifest.json",
    ]);
    let source_manifest =
        serde_json::from_value::<ast_merge::ConformanceManifest>(source_manifest_fixture)
            .expect("source manifest should deserialize");
    assert_eq!(
        ast_merge::conformance_family_feature_profile_path(&source_manifest, "go"),
        Some(vec![
            "diagnostics".to_string(),
            "slice-109-go-family-feature-profile".to_string(),
            "go-feature-profile.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "go", "analysis"),
        Some(vec![
            "go".to_string(),
            "slice-110-analysis".to_string(),
            "module-owners.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "go", "matching"),
        Some(vec![
            "go".to_string(),
            "slice-111-matching".to_string(),
            "path-equality.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "go", "merge"),
        Some(vec![
            "go".to_string(),
            "slice-112-merge".to_string(),
            "module-merge.json".to_string(),
        ])
        .as_deref()
    );

    let canonical_manifest_fixture =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let canonical_manifest =
        serde_json::from_value::<ast_merge::ConformanceManifest>(canonical_manifest_fixture)
            .expect("canonical manifest should deserialize");
    assert_eq!(
        ast_merge::conformance_family_feature_profile_path(&canonical_manifest, "go"),
        Some(vec![
            "diagnostics".to_string(),
            "slice-109-go-family-feature-profile".to_string(),
            "go-feature-profile.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "go", "analysis"),
        Some(vec![
            "go".to_string(),
            "slice-110-analysis".to_string(),
            "module-owners.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "go", "matching"),
        Some(vec![
            "go".to_string(),
            "slice-111-matching".to_string(),
            "path-equality.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "go", "merge"),
        Some(vec![
            "go".to_string(),
            "slice-112-merge".to_string(),
            "module-merge.json".to_string(),
        ])
        .as_deref()
    );
}
