use ast_merge::{
    LayoutOwner, SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap,
    SourceLineOwner, TrackedComment, augment_comments_with_owner_indents, augment_layout,
    source_line_dispositions, source_line_dispositions_from_augmentations,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    source: String,
    style: String,
    owners: Vec<FixtureOwner>,
    comments: Vec<TrackedComment>,
    expected: Vec<SourceLineDisposition>,
}

#[derive(Deserialize)]
struct FixtureOwner {
    id: String,
    start_line: usize,
    end_line: usize,
    indent: usize,
}

fn source_lines(source: &str) -> Vec<String> {
    let mut lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
    if source.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[test]
fn shared_source_line_disposition_fixture_matches_ruby_ownership_facts() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/source_line_disposition.json"
    ))
    .expect("shared source line disposition fixture is valid");
    for case in fixture.cases {
        let layout_owners = case
            .owners
            .iter()
            .map(|owner| LayoutOwner {
                owner_id: owner.id.clone(),
                start_line: owner.start_line,
                end_line: owner.end_line,
            })
            .collect::<Vec<_>>();
        let owner_indents = case
            .owners
            .iter()
            .map(|owner| (owner.id.clone(), owner.indent))
            .collect::<HashMap<_, _>>();
        let lines = source_lines(&case.source);
        let comments = augment_comments_with_owner_indents(
            &lines,
            &layout_owners,
            &case.comments,
            &case.style,
            &owner_indents,
        )
        .expect("Rust comment augmenter classifies comments");
        let layout = augment_layout(&lines, &layout_owners)
            .expect("Rust layout augmenter classifies blank runs");
        assert_eq!(
            source_line_dispositions_from_augmentations(
                &case.source,
                &layout_owners,
                &comments,
                &layout
            )
            .expect("line disposition is unambiguous"),
            case.expected,
            "{}",
            case.name
        );
    }
}

#[test]
fn conflicting_line_dispositions_fail_closed() {
    let owners = vec![
        SourceLineOwner { id: "alpha".to_string(), start_line: 1, end_line: 1 },
        SourceLineOwner { id: "beta".to_string(), start_line: 2, end_line: 2 },
    ];
    let mismatched_comment = SourceLineCommentRegion {
        id: "comment-region:leading:1-1".to_string(),
        selection_key: "comment:beta:leading".to_string(),
        owner_id: Some("beta".to_string()),
        kind: "leading".to_string(),
        start_line: 1,
        end_line: 1,
    };
    assert!(
        source_line_dispositions("alpha\nbeta\n", &owners, &[mismatched_comment], &[])
            .unwrap_err()
            .contains("conflicting owner and comment")
    );

    let blank_owner = SourceLineOwner { id: "alpha".to_string(), start_line: 1, end_line: 1 };
    let overlapping_gap = SourceLineLayoutGap {
        id: "gap:1-1".to_string(),
        selection_key: "gap:preamble:root:alpha".to_string(),
        kind: "preamble".to_string(),
        start_line: 1,
        end_line: 1,
        before_owner_id: None,
        after_owner_id: Some("alpha".to_string()),
        controller_side: "after".to_string(),
        controller_owner_id: Some("alpha".to_string()),
    };
    assert!(
        source_line_dispositions("\n", &[blank_owner], &[], &[overlapping_gap])
            .unwrap_err()
            .contains("conflicting layout-gap")
    );
}
