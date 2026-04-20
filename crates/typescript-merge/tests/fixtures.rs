use std::{fs, path::PathBuf};

use serde_json::Value;
use typescript_merge::{
    TypeScriptDialect, match_typescript_owners, merge_typescript, parse_typescript,
    typescript_feature_profile,
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
fn conforms_to_typescript_fixtures() {
    let profile_fixture = read_fixture(&[
        "diagnostics",
        "slice-101-typescript-family-feature-profile",
        "typescript-feature-profile.json",
    ]);
    let profile = typescript_feature_profile();
    assert_eq!(profile.family, profile_fixture["feature_profile"]["family"].as_str().unwrap());

    let analysis_fixture =
        read_fixture(&["typescript", "slice-102-analysis", "module-owners.json"]);
    let analysis = parse_typescript(
        analysis_fixture["source"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
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
                    typescript_merge::TypeScriptOwnerKind::Import => "import",
                    typescript_merge::TypeScriptOwnerKind::Declaration => "declaration",
                },
                "match_key": owner.match_key,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(owners), analysis_fixture["expected"]["owners"]);

    let matching_fixture =
        read_fixture(&["typescript", "slice-103-matching", "path-equality.json"]);
    let template = parse_typescript(
        matching_fixture["template"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    let destination = parse_typescript(
        matching_fixture["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    let matched = match_typescript_owners(
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

    let merge_fixture = read_fixture(&["typescript", "slice-104-merge", "module-merge.json"]);
    let merge_result = merge_typescript(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template =
        read_fixture(&["typescript", "slice-104-merge", "invalid-template.json"]);
    let invalid_template_result = merge_typescript(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    assert!(!invalid_template_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_template_result.diagnostics),
        invalid_template["expected"]["diagnostics"]
    );

    let invalid_destination =
        read_fixture(&["typescript", "slice-104-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_typescript(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    assert!(!invalid_destination_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_destination_result.diagnostics),
        invalid_destination["expected"]["diagnostics"]
    );
}
