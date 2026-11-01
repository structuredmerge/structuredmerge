use ast_merge::{BoundaryEdge, RemovePlan, RemovedOwnerAttachments, StructuralBoundary};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

#[test]
fn shared_remove_rehome_fixture_matches_ruby() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/polyglot/ast-merge/structural_remove_rehome.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let boundary = |key: &str, edge: BoundaryEdge| {
            case["boundaries"][key].as_object().map(|entry| StructuralBoundary {
                edge,
                owner_id: entry["owner_id"].as_str().map(str::to_string),
            })
        };
        let leading = boundary("leading", BoundaryEdge::Leading);
        let trailing = boundary("trailing", BoundaryEdge::Trailing);
        let removed_attachments = case["removed_attachments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| RemovedOwnerAttachments {
                owner_id: entry["owner_id"].as_str().map(str::to_string),
                leading_region_id: entry["leading_region_id"].as_str().map(str::to_string),
                inline_region_id: entry["inline_region_id"].as_str().map(str::to_string),
                orphan_region_ids: entry["orphan_region_ids"]
                    .as_array()
                    .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
                    .unwrap_or_default(),
                trailing_region_id: entry["trailing_region_id"].as_str().map(str::to_string),
                leading_gap_id: entry["leading_gap_id"].as_str().map(str::to_string),
                trailing_gap_id: entry["trailing_gap_id"].as_str().map(str::to_string),
            })
            .collect();
        let mut plan = RemovePlan::new(
            case["source"].as_str().unwrap(),
            case["remove_start_line"].as_u64().unwrap() as usize,
            case["remove_end_line"].as_u64().unwrap() as usize,
            leading,
            trailing,
            removed_attachments,
        )
        .unwrap();
        if let Some(overrides) = case.get("owner_overrides") {
            let ids = |key: &str| {
                overrides[key].as_array().map(|values| {
                    values.iter().filter_map(Value::as_str).map(str::to_string).collect()
                })
            };
            plan = plan.with_owner_overrides(ids("removed_owner_ids"), ids("retained_owner_ids"));
        }
        let actual = json!({
            "merged": plan.merged_content(),
            "removed_owner_ids": plan.removed_owner_ids,
            "retained_owner_ids": plan.retained_owner_ids,
            "promoted_comment_region_ids": plan.promoted_comment_region_ids(),
            "promoted_layout_gap_ids": plan.promoted_layout_gap_ids(),
            "plans": plan.rehome_plans.iter().map(|entry| json!({
                "source_owner_id": entry.source_owner_id,
                "target_owner_id": entry.target_owner_id(),
                "edge": match entry.edge() { BoundaryEdge::Leading => "leading", BoundaryEdge::Trailing => "trailing" },
                "kind": match entry.kind { ast_merge::RehomeFragmentKind::Leading => "leading", ast_merge::RehomeFragmentKind::Ambiguous => "ambiguous", ast_merge::RehomeFragmentKind::Trailing => "trailing" },
                "comment_region_ids": entry.comment_region_ids,
                "layout_gap_ids": entry.layout_gap_ids,
                "comment_attachment": entry.comment_attachment(),
                "layout_attachment": entry.layout_attachment(),
            })).collect::<Vec<_>>()
        });
        assert_eq!(actual, case["expected"], "{}", case["name"].as_str().unwrap());
    }
}
