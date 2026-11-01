use ast_merge::{CommentLine, CommentRegion, LayoutGap, preserve_removed_owner_lines};
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::PathBuf};

#[test]
fn shared_removed_owner_emission_fixture_matches_ruby() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/polyglot/ast-merge/structural_removed_owner_emission.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let strings = |key: &str| {
            case[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        let numbers = |key: &str| {
            case[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        };
        let region_data = &case["trailing_region"];
        let region_lines = region_data["lines"].as_array().unwrap();
        let region_numbers = region_data["line_numbers"].as_array().unwrap();
        let region = CommentRegion {
            id: "trailing".into(),
            kind: region_data["kind"].as_str().unwrap().into(),
            style: "line".into(),
            owner_id: case["removed_owner_id"].as_str().unwrap().into(),
            floating: false,
            nodes: region_lines
                .iter()
                .zip(region_numbers)
                .map(|(line, number)| {
                    let text = line.as_str().unwrap().to_owned();
                    CommentLine {
                        normalized_content: text.clone(),
                        text,
                        line_number: number.as_u64().unwrap() as usize,
                    }
                })
                .collect(),
        };
        let gap_data = &case["trailing_gap"];
        let gap = LayoutGap {
            id: "trailing-gap".into(),
            kind: gap_data["kind"].as_str().unwrap().into(),
            start_line: gap_data["start_line"].as_u64().unwrap() as usize,
            end_line: gap_data["end_line"].as_u64().unwrap() as usize,
            lines: gap_data["lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| line.as_str().unwrap().to_owned())
                .collect(),
            before_owner_id: gap_data["before_owner_id"].as_str().map(str::to_owned),
            after_owner_id: gap_data["after_owner_id"].as_str().map(str::to_owned),
            controller_side: gap_data["controller_side"].as_str().unwrap().into(),
            metadata: HashMap::new(),
        };
        let actual = preserve_removed_owner_lines(
            &strings("leading_segment_lines"),
            &numbers("leading_segment_line_numbers"),
            &strings("inline_lines"),
            &numbers("inline_line_numbers"),
            Some(&region),
            Some(&gap),
            case["removed_owner_id"].as_str().unwrap(),
        );
        assert_eq!(
            json!({"lines": actual.lines, "line_numbers": actual.line_numbers}),
            case["expected"],
            "{}",
            case["name"].as_str().unwrap()
        );
    }
}
