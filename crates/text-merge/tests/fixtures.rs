use std::{fs, path::PathBuf};

use serde_json::Value;
use text_merge::{TextMatchPhase, analyze_text, match_text_blocks, merge_text};

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
fn conforms_to_slice_03_analysis_fixture() {
    let fixture = read_fixture(&["text", "slice-03-analysis", "whitespace-and-blocks.json"]);
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
    let fixture = read_fixture(&["text", "slice-11-matching", "exact-content.json"]);
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
fn conforms_to_slice_13_refined_matching_fixture() {
    let fixture =
        read_fixture(&["text", "slice-13-refined-matching", "content-refined-merge.json"]);
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
