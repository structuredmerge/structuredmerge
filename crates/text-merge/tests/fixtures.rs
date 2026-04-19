use std::{fs, path::PathBuf};

use serde_json::Value;
use text_merge::{
    TextMatchPhase, analyze_text, is_similar, match_text_blocks, merge_text, similarity_score,
    text_feature_profile,
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
    let path = fixture_path(parts);
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

fn family_feature_profile_fixture_path(family: &str) -> PathBuf {
    let manifest =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let entries = manifest["family_feature_profiles"]
        .as_array()
        .expect("family_feature_profiles should be an array");
    let entry = entries
        .iter()
        .find(|candidate| candidate["family"].as_str() == Some(family))
        .expect("family feature profile entry should be present");

    let mut path = fixture_path(&[]);
    for segment in entry["path"].as_array().expect("path should be an array") {
        path.push(segment.as_str().expect("path segment should be a string"));
    }

    path
}

fn read_fixture_from_path(path: PathBuf) -> Value {
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

fn text_fixture_path(role: &str) -> PathBuf {
    let manifest =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let entries = manifest["text"].as_array().expect("text should be an array");
    let entry = entries
        .iter()
        .find(|candidate| candidate["role"].as_str() == Some(role))
        .expect("text fixture entry should be present");

    let mut path = fixture_path(&[]);
    for segment in entry["path"].as_array().expect("path should be an array") {
        path.push(segment.as_str().expect("path segment should be a string"));
    }

    path
}

#[test]
fn conforms_to_slice_03_analysis_fixture() {
    let fixture = read_fixture_from_path(text_fixture_path("analysis"));
    let source = fixture["source"].as_str().expect("fixture source should be present");
    let analysis = analyze_text(source);

    assert_eq!(
        analysis.normalized_source,
        fixture["expected"]["normalized_source"]
            .as_str()
            .expect("expected normalized source should be present")
    );

    let blocks = analysis
        .blocks
        .iter()
        .map(|block| {
            serde_json::json!({
                "index": block.index,
                "normalized": block.normalized,
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(Value::Array(blocks), fixture["expected"]["blocks"]);
}

#[test]
fn conforms_to_slice_11_exact_matching_fixture() {
    let fixture = read_fixture_from_path(text_fixture_path("matching_exact"));
    let template = fixture["template"].as_str().expect("template should be present");
    let destination = fixture["destination"].as_str().expect("destination should be present");
    let result = match_text_blocks(template, destination);

    let matched = result
        .matched
        .iter()
        .map(|entry| serde_json::json!([entry.template_index, entry.destination_index]))
        .collect::<Vec<_>>();

    assert_eq!(Value::Array(matched), fixture["expected"]["matched"]);
    assert_eq!(
        result.unmatched_template,
        fixture["expected"]["unmatched_template"]
            .as_array()
            .expect("unmatched_template should be an array")
            .iter()
            .map(|value| value.as_u64().expect("indices should be numeric") as usize)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_destination,
        fixture["expected"]["unmatched_destination"]
            .as_array()
            .expect("unmatched_destination should be an array")
            .iter()
            .map(|value| value.as_u64().expect("indices should be numeric") as usize)
            .collect::<Vec<_>>()
    );
}

#[test]
fn conforms_to_slice_05_similarity_fixture() {
    let fixture = read_fixture_from_path(text_fixture_path("similarity"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let left = case["left"].as_str().expect("left should be present");
        let right = case["right"].as_str().expect("right should be present");
        let threshold = case["threshold"].as_f64().expect("threshold should be numeric");
        let expected_score =
            case["expected_score"].as_f64().expect("expected_score should be numeric");
        let expected_match =
            case["expected_match"].as_bool().expect("expected_match should be boolean");

        assert_eq!(similarity_score(left, right), expected_score);

        let similarity = is_similar(left, right, threshold);
        assert_eq!(similarity.score, expected_score);
        assert_eq!(similarity.threshold, threshold);
        assert_eq!(similarity.matched, expected_match);
    }
}

#[test]
fn conforms_to_slice_13_refined_matching_fixture() {
    let fixture = read_fixture_from_path(text_fixture_path("merge_refined"));
    let template = fixture["template"].as_str().expect("template should be present");
    let destination = fixture["destination"].as_str().expect("destination should be present");
    let result = match_text_blocks(template, destination);

    let matched = result
        .matched
        .iter()
        .map(|entry| {
            serde_json::json!({
                "templateIndex": entry.template_index,
                "destinationIndex": entry.destination_index,
                "phase": match entry.phase {
                    TextMatchPhase::Exact => "exact",
                    TextMatchPhase::Refined => "refined",
                }
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(Value::Array(matched), fixture["expected"]["matched"]);
    assert!(result.unmatched_template.is_empty());
    assert!(result.unmatched_destination.is_empty());

    let merged = merge_text(template, destination);
    assert_eq!(merged.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_23_text_family_feature_profile_fixture_via_the_conformance_manifest() {
    let fixture = read_fixture_from_path(family_feature_profile_fixture_path("text"));
    let profile = text_feature_profile();

    let rendered = serde_json::json!({
        "family": profile.family,
        "supported_dialects": profile.supported_dialects,
        "supported_policies": profile.supported_policies.iter().map(|policy| {
            serde_json::json!({
                "surface": match policy.surface {
                    ast_merge::PolicySurface::Fallback => "fallback",
                    ast_merge::PolicySurface::Array => "array",
                },
                "name": policy.name,
            })
        }).collect::<Vec<_>>()
    });

    assert_eq!(rendered, fixture["feature_profile"]);
}
