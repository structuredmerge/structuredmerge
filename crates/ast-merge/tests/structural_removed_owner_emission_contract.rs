use ast_merge::{
    BoundaryEdge, CommentLine, CommentRegion, LayoutGap, RehomeFragmentKind, RehomePlan,
    SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap, SourceLineOwner,
    SourceRevision, StructuralBoundary, preserve_removed_owner_lines, render_source_plan,
    source_line_dispositions, source_line_selections_for_preserved_lines,
    source_line_selections_for_rehome_plan, source_render_plan_from_line_selections,
};
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

        let source_lines = strings("source");
        let source = format!("{}\n", source_lines.join("\n"));
        let removed_line = case["removed_owner_line_number"].as_u64().unwrap() as usize;
        let retained_line = case["retained_owner_line_number"].as_u64().unwrap() as usize;
        let owners = vec![
            SourceLineOwner {
                id: case["removed_owner_id"].as_str().unwrap().into(),
                start_line: removed_line,
                end_line: removed_line,
            },
            SourceLineOwner {
                id: "later".into(),
                start_line: retained_line,
                end_line: retained_line,
            },
        ];
        let leading_comment_lines = numbers("leading_segment_line_numbers")
            .into_iter()
            .filter(|line| source_lines[*line - 1].starts_with('#'))
            .collect::<Vec<_>>();
        let inline_lines = numbers("inline_line_numbers");
        let trailing_numbers = region_data["line_numbers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        let comment_regions = vec![
            SourceLineCommentRegion {
                id: "leading".into(),
                selection_key: "comment:removed:leading".into(),
                owner_id: Some("removed".into()),
                kind: "leading".into(),
                start_line: *leading_comment_lines.first().unwrap(),
                end_line: *leading_comment_lines.last().unwrap(),
            },
            SourceLineCommentRegion {
                id: "inline".into(),
                selection_key: "comment:removed:inline".into(),
                owner_id: Some("removed".into()),
                kind: "inline".into(),
                start_line: *inline_lines.first().unwrap(),
                end_line: *inline_lines.last().unwrap(),
            },
            SourceLineCommentRegion {
                id: "trailing".into(),
                selection_key: "comment:removed:trailing".into(),
                owner_id: Some("removed".into()),
                kind: "trailing".into(),
                start_line: *trailing_numbers.first().unwrap(),
                end_line: *trailing_numbers.last().unwrap(),
            },
        ];
        let leading_gap_lines = numbers("leading_segment_line_numbers")
            .into_iter()
            .filter(|line| source_lines[*line - 1].trim().is_empty())
            .collect::<Vec<_>>();
        let mut layout_gaps = vec![SourceLineLayoutGap {
            id: "leading-gap".into(),
            selection_key: "gap:preamble:root:removed".into(),
            kind: "preamble".into(),
            start_line: *leading_gap_lines.first().unwrap(),
            end_line: *leading_gap_lines.last().unwrap(),
            before_owner_id: None,
            after_owner_id: Some("removed".into()),
            controller_side: "after".into(),
            controller_owner_id: Some("removed".into()),
        }];
        let trailing_gap = &case["trailing_gap"];
        let before_owner_id = trailing_gap["before_owner_id"].as_str().map(str::to_owned);
        let after_owner_id = trailing_gap["after_owner_id"].as_str().map(str::to_owned);
        let controller_side = trailing_gap["controller_side"].as_str().unwrap();
        let controller_owner_id = match controller_side {
            "before" => before_owner_id.clone(),
            "after" => after_owner_id.clone(),
            _ => None,
        };
        let before = before_owner_id.as_deref().unwrap_or("root");
        let after = after_owner_id.as_deref().unwrap_or("root");
        let gap_kind = trailing_gap["kind"].as_str().unwrap();
        let trailing_gap_line = trailing_gap["start_line"].as_u64().unwrap() as usize;
        layout_gaps.push(SourceLineLayoutGap {
            id: "trailing-gap".into(),
            selection_key: format!("gap:{gap_kind}:{before}:{after}"),
            kind: gap_kind.into(),
            start_line: trailing_gap_line,
            end_line: trailing_gap["end_line"].as_u64().unwrap() as usize,
            before_owner_id,
            after_owner_id,
            controller_side: controller_side.into(),
            controller_owner_id,
        });
        let dispositions =
            source_line_dispositions(&source, &owners, &comment_regions, &layout_gaps)
                .expect("removed-owner fixture assigns each preserved line a semantic region");
        let selections = source_line_selections_for_preserved_lines(
            SourceRevision::Ours,
            &dispositions,
            &actual.line_numbers,
        )
        .expect("Ruby-compatible preservation lines select complete source regions");
        let plan = source_render_plan_from_line_selections(
            HashMap::from([(SourceRevision::Ours, source)]),
            HashMap::from([(SourceRevision::Ours, dispositions.clone())]),
            &selections,
        )
        .expect("removed-owner selections form a source render plan");
        let rendered = render_source_plan(&plan).expect("preserved lines render exactly");
        let expected = case["expected"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(rendered.content, expected, "{}", case["name"].as_str().unwrap());

        let trailing_gap_id = if actual.line_numbers.contains(&trailing_gap_line) {
            vec!["trailing-gap".to_string()]
        } else {
            vec![]
        };
        let rehome = RehomePlan {
            source_owner_id: Some("removed".into()),
            target_boundary: StructuralBoundary {
                edge: BoundaryEdge::Trailing,
                owner_id: Some("later".into()),
            },
            kind: RehomeFragmentKind::Trailing,
            comment_region_ids: vec!["trailing".into()],
            layout_gap_ids: trailing_gap_id,
            metadata: HashMap::new(),
        };
        let rehomed =
            source_line_selections_for_rehome_plan(SourceRevision::Ours, &dispositions, &rehome)
                .expect("rehome boundaries select their complete source regions");
        assert_eq!(rehomed.target_owner_id.as_deref(), Some("later"));
        assert_eq!(rehomed.edge, BoundaryEdge::Trailing);
        let trailing_source = source_render_plan_from_line_selections(
            HashMap::from([(SourceRevision::Ours, source_lines.join("\n") + "\n")]),
            HashMap::from([(SourceRevision::Ours, dispositions.clone())]),
            &rehomed.selections,
        )
        .expect("rehome selections form a source plan");
        let trailing_render = render_source_plan(&trailing_source).unwrap();
        let mut trailing_expected = region_data["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        if actual.line_numbers.contains(&trailing_gap_line) {
            trailing_expected.extend(
                trailing_gap["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|line| line.as_str().unwrap().to_owned()),
            );
        }
        assert_eq!(
            trailing_render.content,
            trailing_expected.join("\n") + "\n",
            "{}",
            case["name"].as_str().unwrap()
        );
    }
}

#[test]
fn rehome_rejects_inline_comments_that_share_a_structural_line() {
    let dispositions = vec![SourceLineDisposition {
        line_number: 1,
        line: "x=1 # inline\n".into(),
        kind: "structural_owner".into(),
        selection_key: "owner:removed".into(),
        owner_id: Some("removed".into()),
        comment_region_ids: vec!["inline".into()],
        comment_kind: Some("inline".into()),
        gap_id: None,
        controller_owner_id: None,
        root_region_kind: None,
    }];
    let rehome = RehomePlan {
        source_owner_id: Some("removed".into()),
        target_boundary: StructuralBoundary {
            edge: BoundaryEdge::Trailing,
            owner_id: Some("retained".into()),
        },
        kind: RehomeFragmentKind::Ambiguous,
        comment_region_ids: vec!["inline".into()],
        layout_gap_ids: vec![],
        metadata: HashMap::new(),
    };
    let error =
        source_line_selections_for_rehome_plan(SourceRevision::Ours, &dispositions, &rehome)
            .unwrap_err();
    assert!(error.to_string().contains("shares a structural owner line"));
}
