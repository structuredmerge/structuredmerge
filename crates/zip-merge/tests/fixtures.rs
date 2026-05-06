use std::{collections::HashMap, fs, path::PathBuf};

use serde_json::Value;
use zip_merge::{new_stored_zip, parse_zip_inventory, plan_zip_merge, render_with_raw_preservation};

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
    serde_json::from_str(&fs::read_to_string(fixture_path(parts)).expect("fixture should be readable"))
        .expect("fixture should be valid json")
}

#[test]
fn parses_plans_and_raw_preserves_stored_zip_members() {
    let current_source = new_stored_zip(&[
        ("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n"),
        ("docs/readme.md", "# Old\n"),
    ]);
    let ancestor = parse_zip_inventory(&current_source).expect("current ZIP should parse");
    let incoming_source = new_stored_zip(&[
        ("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n"),
        ("docs/readme.md", "# New\n"),
    ]);
    let incoming = parse_zip_inventory(&incoming_source).expect("incoming ZIP should parse");
    let plan = plan_zip_merge(&ancestor, &ancestor, &incoming);
    let mut member_bytes = HashMap::new();
    member_bytes.insert("docs/readme.md".to_string(), b"# New\n".to_vec());

    let (_output, inventory, report) =
        render_with_raw_preservation(&current_source, &plan, &member_bytes).expect("ZIP should render");

    assert_eq!(inventory.archive.entry_count, 2);
    assert_eq!(plan.merge_report.nested_dispatches[0].family, "markdown");
    assert_eq!(report.preserved_ranges.len(), 1);
}

#[test]
fn conforms_to_slice_736_raw_preservation_fixture_categories() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-736-zip-raw-preservation-edge-cases",
        "zip-raw-preservation-edge-cases.json",
    ]);
    assert_eq!(fixture["success"]["expected_nested_family"].as_str().unwrap(), "markdown");
    let categories = fixture["rejections"].as_array().unwrap();
    assert!(categories.iter().any(|item| item["category"] == "unsupported_compression"));
    assert!(categories.iter().any(|item| item["category"] == "archive_comment"));
    assert!(categories.iter().any(|item| item["category"] == "encrypted_member"));
}
