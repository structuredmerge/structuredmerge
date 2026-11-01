use ast_merge::{LayoutOwner, augment_layout};
use serde_json::Value;
use std::{collections::HashSet, fs, path::PathBuf};

#[test]
fn shared_line_layout_fixture_matches_gap_ownership_and_fallback() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/polyglot/ast-merge/layout_line_ownership.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let lines = fixture["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let owners = fixture["owners"]
        .as_array()
        .unwrap()
        .iter()
        .map(|owner| LayoutOwner {
            owner_id: owner["id"].as_str().unwrap().to_string(),
            start_line: owner["start_line"].as_u64().unwrap() as usize,
            end_line: owner["end_line"].as_u64().unwrap() as usize,
        })
        .collect::<Vec<_>>();
    let actual = augment_layout(&lines, &owners).unwrap();
    let removed = fixture["removed_owner_ids_for_fallback"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect::<HashSet<_>>();

    let actual_gaps = actual
        .gaps
        .iter()
        .map(|gap| {
            serde_json::json!({
                "id": gap.id,
                "kind": gap.kind,
                "start_line": gap.start_line,
                "end_line": gap.end_line,
                "lines": gap.lines,
                "before_owner_id": gap.before_owner_id,
                "after_owner_id": gap.after_owner_id,
                "controller_side": gap.controller_side,
                "controller_after_removal_owner_id": gap.effective_controller_owner_id(&removed),
            })
        })
        .collect::<Vec<_>>();
    let actual_attachments = actual
        .attachments
        .iter()
        .map(|attachment| {
            serde_json::json!({
                "owner_id": attachment.owner_id,
                "leading_gap_id": attachment.leading_gap_id,
                "trailing_gap_id": attachment.trailing_gap_id,
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(actual_gaps, *fixture["gaps"].as_array().unwrap());
    assert_eq!(actual_attachments, *fixture["attachments"].as_array().unwrap());
}
