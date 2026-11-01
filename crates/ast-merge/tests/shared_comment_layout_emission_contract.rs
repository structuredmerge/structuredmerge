use ast_merge::{LeadingSegmentEmissionCase, RootBoundaryEmissionCase};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<LeadingSegmentEmissionCase>,
    root_boundary_cases: Vec<RootBoundaryEmissionCase>,
}

#[test]
fn shared_comment_layout_emission_fixture_matches_ruby() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/comment_layout_emission.json"
    ))
    .expect("shared comment layout emission fixture is valid");
    for case in fixture.cases {
        assert_eq!(
            case.selected_line_numbers().expect("leading segment decision succeeds"),
            case.expected_line_numbers,
            "{}",
            case.name
        );
    }
}

#[test]
fn shared_root_boundary_emission_fixture_matches_ruby() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/comment_layout_emission.json"
    ))
    .expect("shared comment layout emission fixture is valid");
    for case in fixture.root_boundary_cases {
        assert_eq!(
            case.selected_line_numbers().expect("root boundary decision succeeds"),
            case.expected_line_numbers,
            "{}",
            case.name
        );
    }
}
