use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseRef, ConformanceCaseResult, ConformanceManifest, ConformanceOutcome,
    ConformanceSuiteSummary, DiagnosticCategory, DiagnosticSeverity, FamilyFeatureProfile,
    PolicySurface, conformance_family_feature_profile_path, conformance_fixture_path,
    summarize_conformance_results,
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

fn read_manifest() -> ConformanceManifest {
    let path = fixture_path(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let source = fs::read_to_string(path).expect("manifest should be readable");
    serde_json::from_str(&source).expect("manifest should be valid json")
}

fn path_buf_from_segments(segments: &[String]) -> PathBuf {
    let mut path = fixture_path(&[]);
    for segment in segments {
        path.push(segment);
    }

    path
}

fn diagnostics_fixture_path(role: &str) -> PathBuf {
    let manifest = read_manifest();
    let path = conformance_fixture_path(&manifest, "diagnostics", role)
        .expect("diagnostics fixture entry should be present");

    path_buf_from_segments(path)
}

fn read_fixture_from_path(path: PathBuf) -> Value {
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_slice_02_diagnostic_vocabulary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("diagnostic_vocabulary"));

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
    let fixture = read_fixture_from_path(diagnostics_fixture_path("policy_vocabulary"));

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

#[test]
fn conforms_to_slice_18_policy_reporting_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("policy_reporting"));

    let merge_policies = serde_json::json!([
        {
            "surface": "array",
            "name": "destination_wins_array"
        },
        {
            "surface": "fallback",
            "name": "trailing_comma_destination_fallback"
        }
    ]);

    assert_eq!(merge_policies, fixture["merge_policies"]);
}

#[test]
fn conforms_to_slice_22_shared_family_feature_profile_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("shared_family_feature_profile"));

    let feature_profile = FamilyFeatureProfile {
        family: "example".to_string(),
        supported_dialects: vec!["alpha".to_string(), "beta".to_string()],
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let rendered = serde_json::json!({
        "family": feature_profile.family,
        "supported_dialects": feature_profile.supported_dialects,
        "supported_policies": feature_profile.supported_policies.iter().map(|policy| {
            serde_json::json!({
                "surface": match policy.surface {
                    PolicySurface::Fallback => "fallback",
                    PolicySurface::Array => "array",
                },
                "name": policy.name,
            })
        }).collect::<Vec<_>>()
    });

    assert_eq!(rendered, fixture["feature_profile"]);
}

#[test]
fn conforms_to_slice_28_conformance_runner_shape_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("runner_shape"));

    let case_ref = ConformanceCaseRef {
        family: "json".to_string(),
        role: "tree_sitter_adapter".to_string(),
        case: "valid_strict_json".to_string(),
    };
    let result = ConformanceCaseResult {
        ref_: case_ref.clone(),
        outcome: ConformanceOutcome::Passed,
        messages: vec![],
    };

    assert_eq!(
        serde_json::json!({
            "family": case_ref.family,
            "role": case_ref.role,
            "case": case_ref.case,
        }),
        fixture["case_ref"]
    );
    assert_eq!(
        serde_json::json!({
            "ref": {
                "family": result.ref_.family,
                "role": result.ref_.role,
                "case": result.ref_.case,
            },
            "outcome": match result.outcome {
                ConformanceOutcome::Passed => "passed",
                ConformanceOutcome::Failed => "failed",
                ConformanceOutcome::Skipped => "skipped",
            },
            "messages": result.messages,
        }),
        fixture["result"]
    );
}

#[test]
fn conforms_to_slice_30_normalized_manifest_contract() {
    let manifest = read_manifest();

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "json"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-21-family-feature-profile".to_string(),
                "json-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "text", "analysis"),
        Some(
            &[
                "text".to_string(),
                "slice-03-analysis".to_string(),
                "whitespace-and-blocks.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "diagnostics", "runner_shape"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-28-conformance-runner".to_string(),
                "runner-shape.json".to_string(),
            ][..],
        )
    );
}

#[test]
fn conforms_to_slice_32_conformance_suite_summary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("runner_summary"));
    let results: Vec<ConformanceCaseResult> =
        serde_json::from_value(fixture["results"].clone()).expect("results should deserialize");
    let summary: ConformanceSuiteSummary =
        serde_json::from_value(fixture["summary"].clone()).expect("summary should deserialize");

    assert_eq!(summarize_conformance_results(&results), summary);
}
