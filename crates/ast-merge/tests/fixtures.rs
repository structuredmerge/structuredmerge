use std::{fs, path::PathBuf};

use ast_merge::{DiagnosticCategory, DiagnosticSeverity, PolicySurface};
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
    let path = fixture_path(parts);
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_slice_02_diagnostic_vocabulary_fixture() {
    let fixture = read_fixture(&["diagnostics", "slice-02-core", "diagnostic-categories.json"]);

    let severities = vec![
        match DiagnosticSeverity::Info {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
        match DiagnosticSeverity::Warning {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
        match DiagnosticSeverity::Error {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
    ];

    let categories = vec![
        match DiagnosticCategory::ParseError {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::DestinationParseError {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::UnsupportedFeature {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::FallbackApplied {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::Ambiguity {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
    ];

    assert_eq!(
        Value::Array(
            severities.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["severities"]
    );
    assert_eq!(
        Value::Array(
            categories.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["categories"]
    );
}

#[test]
fn conforms_to_slice_17_policy_vocabulary_fixture() {
    let fixture =
        read_fixture(&["diagnostics", "slice-17-policy-vocabulary", "policy-references.json"]);

    let surfaces = vec![
        match PolicySurface::Fallback {
            PolicySurface::Fallback => "fallback",
            PolicySurface::Array => "array",
        },
        match PolicySurface::Array {
            PolicySurface::Fallback => "fallback",
            PolicySurface::Array => "array",
        },
    ];

    let policies = serde_json::json!([
        {
            "surface": "fallback",
            "name": "trailing_comma_destination_fallback"
        },
        {
            "surface": "array",
            "name": "destination_wins_array"
        }
    ]);

    assert_eq!(
        Value::Array(
            surfaces.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["surfaces"]
    );
    assert_eq!(policies, fixture["policies"]);
}
