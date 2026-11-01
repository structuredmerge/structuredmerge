use ast_merge::{
    LayoutOwner, TrackedComment, augment_comments, augment_comments_with_owner_indents,
};
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::PathBuf};

#[test]
fn shared_comment_line_ownership_fixture_matches_regions_and_layout() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/polyglot/ast-merge/comment_line_ownership.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let source = fixture["source"].as_str().unwrap();
    let mut lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
    if source.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
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
    let comments = fixture["comments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|comment| TrackedComment {
            line: comment["line"].as_u64().unwrap() as usize,
            text: comment["text"].as_str().unwrap().to_string(),
            normalized_content: comment["normalized_content"].as_str().unwrap().to_string(),
            full_line: comment["full_line"].as_bool().unwrap(),
            indent: Some(comment["indent"].as_u64().unwrap() as usize),
        })
        .collect::<Vec<_>>();
    let actual =
        augment_comments(&lines, &owners, &comments, fixture["style"].as_str().unwrap()).unwrap();
    let regions =
        actual.regions.iter().map(|region| (region.id.as_str(), region)).collect::<HashMap<_, _>>();
    let gaps = actual.gaps.iter().map(|gap| (gap.id.as_str(), gap)).collect::<HashMap<_, _>>();
    let region_summary = |id: &Option<String>| -> Value {
        id.as_ref().and_then(|id| regions.get(id.as_str()).copied()).map_or(Value::Null, |region| {
            json!({
                "kind": region.kind,
                "normalized_content": region.normalized_content(),
                "floating": region.floating,
                "start_line": region.start_line(),
                "end_line": region.end_line(),
            })
        })
    };
    let gap_summary = |id: &Option<String>| -> Value {
        id.as_ref().and_then(|id| gaps.get(id.as_str()).copied()).map_or(Value::Null, |gap| {
            json!({
                "kind": gap.kind,
                "start_line": gap.start_line,
                "end_line": gap.end_line,
            })
        })
    };
    let actual_attachments = actual
        .attachments
        .iter()
        .map(|attachment| {
            json!({
                "owner_id": attachment.owner_id,
                "leading": region_summary(&attachment.leading_region_id),
                "inline": region_summary(&attachment.inline_region_id),
                "trailing": region_summary(&attachment.trailing_region_id),
                "leading_gap": gap_summary(&attachment.leading_gap_id),
                "trailing_gap": gap_summary(&attachment.trailing_gap_id),
            })
        })
        .collect::<Vec<_>>();
    let actual_document_regions = json!({
        "preamble": region_summary(&actual.preamble_region_id)["normalized_content"],
        "postlude": region_summary(&actual.postlude_region_id)["normalized_content"],
        "orphans": actual.orphan_region_ids.iter().filter_map(|id| regions.get(id.as_str()).copied()).map(|region| region.normalized_content()).collect::<Vec<_>>(),
    });

    assert_eq!(actual_attachments, *fixture["expected"]["attachments"].as_array().unwrap());
    assert_eq!(actual_document_regions, fixture["expected"]["document_regions"]);

    for case in fixture["attachment_cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let mut case_lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
        if source.ends_with('\n') && case_lines.last().is_some_and(String::is_empty) {
            case_lines.pop();
        }
        let case_owners = case["owners"]
            .as_array()
            .unwrap()
            .iter()
            .map(|owner| LayoutOwner {
                owner_id: owner["id"].as_str().unwrap().to_string(),
                start_line: owner["start_line"].as_u64().unwrap() as usize,
                end_line: owner["end_line"].as_u64().unwrap() as usize,
            })
            .collect::<Vec<_>>();
        let case_comments = case["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|comment| TrackedComment {
                line: comment["line"].as_u64().unwrap() as usize,
                text: comment["text"].as_str().unwrap().to_string(),
                normalized_content: comment["normalized_content"].as_str().unwrap().to_string(),
                full_line: comment["full_line"].as_bool().unwrap(),
                indent: Some(comment["indent"].as_u64().unwrap() as usize),
            })
            .collect::<Vec<_>>();
        let augmentation = augment_comments(
            &case_lines,
            &case_owners,
            &case_comments,
            case["style"].as_str().unwrap(),
        )
        .unwrap();
        let regions = augmentation
            .regions
            .iter()
            .map(|region| (region.id.as_str(), region))
            .collect::<HashMap<_, _>>();
        let region_content = |id: &Option<String>| {
            id.as_ref()
                .and_then(|id| regions.get(id.as_str()).copied())
                .map(|region| region.normalized_content())
        };
        for (owner_id, expected) in case["expected"]["trailing_by_owner"].as_object().unwrap() {
            let attachment = augmentation
                .attachments
                .iter()
                .find(|attachment| attachment.owner_id == *owner_id)
                .unwrap();
            assert_eq!(
                region_content(&attachment.trailing_region_id).as_deref(),
                expected.as_str()
            );
        }
        assert_eq!(
            region_content(&augmentation.postlude_region_id),
            case["expected"]["postlude"].as_str().map(str::to_string)
        );
    }
    for case in fixture["indent_cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let mut case_lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
        if source.ends_with('\n') && case_lines.last().is_some_and(String::is_empty) {
            case_lines.pop();
        }
        let case_owners =
            vec![LayoutOwner { owner_id: "alpha".to_string(), start_line: 1, end_line: 1 }];
        let entry = &case["comment"];
        let case_comments = vec![TrackedComment {
            line: entry["line"].as_u64().unwrap() as usize,
            text: entry["text"].as_str().unwrap().to_string(),
            normalized_content: entry["normalized_content"].as_str().unwrap().to_string(),
            full_line: entry["full_line"].as_bool().unwrap(),
            indent: Some(entry["indent"].as_u64().unwrap() as usize),
        }];
        let owner_indents = case["owner_indent"]
            .as_u64()
            .map(|indent| HashMap::from([(String::from("alpha"), indent as usize)]))
            .unwrap_or_default();
        let augmentation = augment_comments_with_owner_indents(
            &case_lines,
            &case_owners,
            &case_comments,
            fixture["style"].as_str().unwrap(),
            &owner_indents,
        )
        .unwrap();
        let regions = augmentation
            .regions
            .iter()
            .map(|region| (region.id.as_str(), region))
            .collect::<HashMap<_, _>>();
        let attachment = &augmentation.attachments[0];
        let trailing = attachment
            .trailing_region_id
            .as_ref()
            .and_then(|id| regions.get(id.as_str()).copied())
            .map(|region| region.normalized_content());
        let postlude = augmentation
            .postlude_region_id
            .as_ref()
            .and_then(|id| regions.get(id.as_str()).copied())
            .map(|region| region.normalized_content());
        assert_eq!(trailing.as_deref(), case["expected_trailing"].as_str());
        assert_eq!(postlude.as_deref(), case["expected_postlude"].as_str());
    }

    for case in fixture["document_cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let mut case_lines = source.split(char::from(10)).map(str::to_string).collect::<Vec<_>>();
        if source.ends_with(char::from(10)) && case_lines.last().is_some_and(String::is_empty) {
            case_lines.pop();
        }
        let case_owners = case["owners"]
            .as_array()
            .unwrap()
            .iter()
            .map(|owner| LayoutOwner {
                owner_id: owner["id"].as_str().unwrap().to_string(),
                start_line: owner["start_line"].as_u64().unwrap() as usize,
                end_line: owner["end_line"].as_u64().unwrap() as usize,
            })
            .collect::<Vec<_>>();
        let case_comments = case["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|comment| TrackedComment {
                line: comment["line"].as_u64().unwrap() as usize,
                text: comment["text"].as_str().unwrap().to_string(),
                normalized_content: comment["normalized_content"].as_str().unwrap().to_string(),
                full_line: comment["full_line"].as_bool().unwrap(),
                indent: Some(comment["indent"].as_u64().unwrap() as usize),
            })
            .collect::<Vec<_>>();
        let augmentation = augment_comments(
            &case_lines,
            &case_owners,
            &case_comments,
            fixture["style"].as_str().unwrap(),
        )
        .unwrap();
        let regions = augmentation
            .regions
            .iter()
            .map(|region| (region.id.as_str(), region))
            .collect::<HashMap<_, _>>();
        let content = |id: &Option<String>| {
            id.as_ref()
                .and_then(|id| regions.get(id.as_str()).copied())
                .map(|region| region.normalized_content())
        };
        let leading_by_owner = augmentation
            .attachments
            .iter()
            .map(|attachment| {
                (attachment.owner_id.as_str(), content(&attachment.leading_region_id))
            })
            .collect::<HashMap<_, _>>();
        let orphans = augmentation
            .orphan_region_ids
            .iter()
            .filter_map(|id| regions.get(id.as_str()).copied())
            .map(|region| region.normalized_content())
            .collect::<Vec<_>>();
        assert_eq!(
            content(&augmentation.preamble_region_id).as_deref(),
            case["expected"]["preamble"].as_str()
        );
        assert_eq!(
            orphans,
            case["expected"]["orphans"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item.as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        );
        for (owner_id, expected) in case["expected"]["leading_by_owner"].as_object().unwrap() {
            assert_eq!(
                leading_by_owner.get(owner_id.as_str()).and_then(Option::as_deref),
                expected.as_str()
            );
        }
    }
}
