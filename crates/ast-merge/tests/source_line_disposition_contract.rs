use ast_merge::{
    SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap, SourceLineOwner,
    source_line_dispositions,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    source: String,
    owners: Vec<SourceLineOwner>,
    comment_regions: Vec<SourceLineCommentRegion>,
    layout_gaps: Vec<SourceLineLayoutGap>,
    expected: Vec<SourceLineDisposition>,
}

#[test]
fn shared_source_line_disposition_fixture_matches_ruby_ownership_facts() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/source_line_disposition.json"
    ))
    .expect("shared source line disposition fixture is valid");
    for case in fixture.cases {
        assert_eq!(
            source_line_dispositions(
                &case.source,
                &case.owners,
                &case.comment_regions,
                &case.layout_gaps
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
        id: "comment:beta:leading:1-1".to_string(),
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
