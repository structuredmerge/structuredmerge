use std::{fs, path::PathBuf};

use binary_merge::{binary_feature_profile, preservation_report, unsafe_diagnostic};
use serde_json::Value;
use tree_haver::ByteRange;

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
fn assembles_binary_preservation_report() {
    let fixture =
        read_fixture(&["diagnostics", "slice-723-binary-core-contract", "binary-core.json"]);
    let first_range = &fixture["merge_report"]["preserved_ranges"][0];
    let preserved_range = ByteRange {
        start_byte: first_range["start_byte"].as_u64().unwrap() as usize,
        end_byte: first_range["end_byte"].as_u64().unwrap() as usize,
    };

    let report = preservation_report(
        fixture["merge_report"]["format"].as_str().unwrap(),
        fixture["merge_report"]["schema"].as_str().unwrap(),
        vec!["/chunks/0".to_string(), "/chunks/1".to_string()],
        vec![preserved_range],
    );
    let diagnostic = unsafe_diagnostic(
        "/chunks/2",
        ByteRange { start_byte: 78, end_byte: 96 },
        "critical image data mutation is not enabled",
    );

    assert_eq!(binary_feature_profile().family, "binary");
    assert_eq!(report.preserved_ranges[0].len(), 25);
    assert!(report.rewritten_nodes.is_empty());
    assert_eq!(diagnostic.category, "unsafe_binary_mutation");
}
