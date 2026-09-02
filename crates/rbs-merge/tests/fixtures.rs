use std::{fs, path::PathBuf};

use rbs_merge::{RbsDialect, match_rbs_owners, merge_rbs, parse_rbs};
use serde_json::Value;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/rbs/slice-1036-shared-kernel/declarations-comments.json");
    serde_json::from_str(&fs::read_to_string(path).expect("fixture should be readable"))
        .expect("fixture should be valid JSON")
}

#[test]
fn exercises_rbs_through_the_shared_kernel() {
    let fixture = fixture();
    let analysis = parse_rbs(fixture["analysis"]["source"].as_str().unwrap(), RbsDialect::Rbs);
    assert!(analysis.ok, "{:?}", analysis.diagnostics);
    let analysis = analysis.analysis.expect("analysis should exist");
    assert_eq!(
        analysis.owners.iter().map(|owner| owner.match_key.as_str()).collect::<Vec<_>>(),
        fixture["analysis"]["expected"]["owner_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>()
    );
    let mut comments = analysis
        .comment_regions
        .iter()
        .map(|region| region.normalized_content())
        .collect::<Vec<_>>();
    comments.sort();
    let mut expected_comments = fixture["analysis"]["expected"]["comment_contents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    expected_comments.sort();
    assert_eq!(comments, expected_comments);
    assert_eq!(
        analysis.layout_gaps.len() as u64,
        fixture["analysis"]["expected"]["layout_gap_count"].as_u64().unwrap()
    );

    let merge = merge_rbs(
        fixture["merge"]["template"].as_str().unwrap(),
        fixture["merge"]["destination"].as_str().unwrap(),
        RbsDialect::Rbs,
    );
    assert!(merge.ok, "{:?}", merge.diagnostics);
    assert_eq!(merge.output.as_deref(), fixture["merge"]["expected"].as_str());

    let destination = parse_rbs(fixture["merge"]["destination"].as_str().unwrap(), RbsDialect::Rbs)
        .analysis
        .unwrap();
    let matched = match_rbs_owners(&analysis, &destination);
    assert!(!matched.matched.is_empty());
}
