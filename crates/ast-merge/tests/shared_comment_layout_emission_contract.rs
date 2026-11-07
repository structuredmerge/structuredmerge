use ast_merge::{
    LeadingSegmentEmissionInput, OwnerLineRange, RootBoundaryKind, leading_segment_line_numbers,
    root_boundary_line_numbers,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<LeadingSegmentCase>,
    root_boundary_cases: Vec<RootBoundaryCase>,
}

#[derive(Deserialize)]
struct LeadingSegmentCase {
    name: String,
    source_lines: Vec<String>,
    output_lines: Vec<String>,
    source_region_start_line: usize,
    output_owner_start_line: usize,
    target_leading_region_start_line: Option<usize>,
    previous_owner_trailing_region_matches: bool,
    expected_line_numbers: Vec<usize>,
}

#[derive(Deserialize)]
struct RootBoundaryCase {
    name: String,
    kind: String,
    lines: Vec<String>,
    owners: Vec<OwnerLineRange>,
    preamble_region: Option<OwnerLineRange>,
    postlude_region: Option<OwnerLineRange>,
    fallback_to_owner_bounds: bool,
    expected_line_numbers: Vec<usize>,
}

#[test]
fn shared_comment_layout_emission_fixture_matches_ruby() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/comment_layout_emission.json"
    ))
    .expect("shared comment layout emission fixture is valid");
    for case in fixture.cases {
        let input = LeadingSegmentEmissionInput {
            source_lines: &case.source_lines,
            output_lines: &case.output_lines,
            source_region_start_line: case.source_region_start_line,
            output_owner_start_line: case.output_owner_start_line,
            target_leading_region_start_line: case.target_leading_region_start_line,
            previous_owner_trailing_region_matches: case.previous_owner_trailing_region_matches,
        };
        assert_eq!(
            leading_segment_line_numbers(&input).expect("leading segment decision succeeds"),
            case.expected_line_numbers,
            "{}",
            case.name
        );
    }
}

#[test]
fn shared_root_boundary_emission_fixture_matches_ruby() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/comment_layout_emission.json"
    ))
    .expect("shared comment layout emission fixture is valid");
    for case in fixture.root_boundary_cases {
        let kind = match case.kind.as_str() {
            "preamble" => RootBoundaryKind::Preamble,
            "postlude" => RootBoundaryKind::Postlude,
            _ => panic!("unknown fixture boundary kind {}", case.kind),
        };
        let region = if kind == RootBoundaryKind::Preamble {
            case.preamble_region
        } else {
            case.postlude_region
        };
        assert_eq!(
            root_boundary_line_numbers(
                kind,
                case.lines.len(),
                &case.owners,
                region,
                case.fallback_to_owner_bounds,
            )
            .expect("root boundary decision succeeds"),
            case.expected_line_numbers,
            "{}",
            case.name
        );
    }
}
