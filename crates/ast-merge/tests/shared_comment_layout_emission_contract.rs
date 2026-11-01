use ast_merge::LeadingSegmentEmissionCase;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<LeadingSegmentEmissionCase>,
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
