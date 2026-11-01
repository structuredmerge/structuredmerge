use ast_merge::{
    CommentAttachment, CommentAugmentation, CommentLine, CommentRegion, LayoutAugmentation,
    LayoutOwner, SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap,
    SourceLineOwner, TrackedComment, augment_comments_with_owner_indents,
    augment_layout_with_comment_augmentation, source_line_dispositions,
    source_line_dispositions_from_augmentations,
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
fn shared_source_line_disposition_fixture_matches_common_contract() {
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
        let layout = augment_layout_with_comment_augmentation(&lines, &layout_owners, &comments)
            .expect("Rust layout augmenter classifies owner and comment boundary gaps");
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

#[test]
fn nested_attached_comment_lines_belong_to_the_containing_structural_owner() {
    let source = "extern \"C\" {\n    // nested comment\n}\nfn value() {}\n";
    let owners = vec![
        LayoutOwner { owner_id: "foreign".to_string(), start_line: 1, end_line: 3 },
        LayoutOwner { owner_id: "value".to_string(), start_line: 4, end_line: 4 },
    ];
    let comments = CommentAugmentation {
        regions: vec![CommentRegion {
            id: "nested-comment".to_string(),
            kind: "leading".to_string(),
            style: "rust".to_string(),
            owner_id: "value".to_string(),
            floating: false,
            nodes: vec![CommentLine {
                text: "    // nested comment".to_string(),
                line_number: 2,
                normalized_content: "nested comment".to_string(),
            }],
        }],
        gaps: vec![],
        attachments: vec![CommentAttachment {
            owner_id: "value".to_string(),
            leading_region_id: Some("nested-comment".to_string()),
            inline_region_id: None,
            trailing_region_id: None,
            orphan_region_ids: vec![],
            leading_gap_id: None,
            trailing_gap_id: None,
            metadata: HashMap::new(),
        }],
        preamble_region_id: None,
        postlude_region_id: None,
        orphan_region_ids: vec![],
    };
    let dispositions = source_line_dispositions_from_augmentations(
        source,
        &owners,
        &comments,
        &LayoutAugmentation { gaps: vec![], attachments: vec![] },
    )
    .unwrap();
    let nested = &dispositions[1];
    assert_eq!(nested.kind, "structural_owner");
    assert_eq!(nested.owner_id.as_deref(), Some("foreign"));
    assert_eq!(nested.comment_region_ids, vec!["nested-comment"]);
}
