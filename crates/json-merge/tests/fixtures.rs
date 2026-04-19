use std::{fs, path::PathBuf};

use json_merge::{JsonDialect, merge_json, parse_json};
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
fn conforms_to_jsonc_comments_accepted_fixture() {
    let fixture = read_fixture(&["jsonc", "slice-04-parse", "comments-accepted.json"]);
    let source = fixture["source"].as_str().expect("source should be present");
    let result = parse_json(source, JsonDialect::Jsonc);

    assert_eq!(result.ok, fixture["expected"]["ok"].as_bool().unwrap_or(false));
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.allows_comments).unwrap_or(false),
        fixture["expected"]["allows_comments"].as_bool().unwrap_or(false)
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn conforms_to_slice_09_object_merge_fixture() {
    let fixture = read_fixture(&["json", "slice-09-merge", "object-merge.json"]);
    let template = fixture["template"].as_str().expect("template should be present");
    let destination = fixture["destination"].as_str().expect("destination should be present");
    let result = merge_json(template, destination, JsonDialect::Json);

    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}
