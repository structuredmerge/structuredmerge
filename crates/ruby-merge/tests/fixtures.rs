use std::{fs, path::PathBuf};

use ast_merge::{
    ProjectedChildReviewCase, ProjectedChildReviewGroup, ProjectedChildReviewGroupProgress,
    group_projected_child_review_cases, select_projected_child_review_groups_ready_for_apply,
    summarize_projected_child_review_group_progress,
};
use ruby_merge::{
    RubyDialect, RubyOwnerKind, match_ruby_owners, parse_ruby, ruby_delegated_child_operations,
    ruby_discovered_surfaces, ruby_feature_profile,
};
use serde_json::Value;

fn fixture_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("..");
    path.push("fixtures");
    for part in parts {
        path.push(part);
    }
    path
}

fn read_fixture(parts: &[&str]) -> Value {
    let source = fs::read_to_string(fixture_path(parts)).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_ruby_fixtures() {
    let profile_fixture = read_fixture(&[
        "diagnostics",
        "slice-214-ruby-family-feature-profile",
        "ruby-feature-profile.json",
    ]);
    let profile = ruby_feature_profile();
    assert_eq!(profile.family, profile_fixture["feature_profile"]["family"].as_str().unwrap());

    let analysis_fixture = read_fixture(&["ruby", "slice-218-analysis", "module-owners.json"]);
    let analysis = parse_ruby(analysis_fixture["source"].as_str().unwrap(), RubyDialect::Ruby);
    assert!(analysis.ok);
    let owners = analysis
        .analysis
        .as_ref()
        .unwrap()
        .owners
        .iter()
        .map(|owner| {
            serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    RubyOwnerKind::Require => "require",
                    RubyOwnerKind::Declaration => "declaration",
                },
                "match_key": owner.match_key,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(owners), analysis_fixture["expected"]["owners"]);

    let matching_fixture = read_fixture(&["ruby", "slice-219-matching", "path-equality.json"]);
    let template = parse_ruby(matching_fixture["template"].as_str().unwrap(), RubyDialect::Ruby);
    let destination =
        parse_ruby(matching_fixture["destination"].as_str().unwrap(), RubyDialect::Ruby);
    let matched = match_ruby_owners(
        template.analysis.as_ref().unwrap(),
        destination.analysis.as_ref().unwrap(),
    );
    assert_eq!(
        Value::Array(
            matched
                .matched
                .iter()
                .map(|entry| serde_json::json!([entry.template_path, entry.destination_path]))
                .collect(),
        ),
        matching_fixture["expected"]["matched"]
    );
    assert_eq!(
        Value::Array(
            matched.unmatched_template.iter().map(|path| Value::String(path.clone())).collect(),
        ),
        matching_fixture["expected"]["unmatched_template"]
    );
    assert_eq!(
        Value::Array(
            matched.unmatched_destination.iter().map(|path| Value::String(path.clone())).collect(),
        ),
        matching_fixture["expected"]["unmatched_destination"]
    );

    let surfaces_fixture =
        read_fixture(&["ruby", "slice-220-discovered-surfaces", "doc-comment-surfaces.json"]);
    let surface_analysis =
        parse_ruby(surfaces_fixture["source"].as_str().unwrap(), RubyDialect::Ruby);
    assert_eq!(
        serde_json::to_value(ruby_discovered_surfaces(surface_analysis.analysis.as_ref().unwrap()))
            .expect("surfaces should serialize"),
        surfaces_fixture["expected"]
    );

    let child_fixture = read_fixture(&[
        "ruby",
        "slice-221-delegated-child-operations",
        "yard-example-child-operations.json",
    ]);
    let child_analysis = parse_ruby(child_fixture["source"].as_str().unwrap(), RubyDialect::Ruby);
    assert_eq!(
        serde_json::to_value(ruby_delegated_child_operations(
            child_analysis.analysis.as_ref().unwrap(),
            child_fixture["parent_operation_id"].as_str().unwrap(),
        ))
        .expect("child operations should serialize"),
        child_fixture["expected"]
    );

    let grouped_fixture = read_fixture(&[
        "ruby",
        "slice-229-projected-child-review-groups",
        "yard-example-review-groups.json",
    ]);
    let grouped_cases =
        serde_json::from_value::<Vec<ProjectedChildReviewCase>>(grouped_fixture["cases"].clone())
            .expect("projected cases should deserialize");
    let expected_groups = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        grouped_fixture["expected_groups"].clone(),
    )
    .expect("projected groups should deserialize");
    assert_eq!(group_projected_child_review_cases(&grouped_cases), expected_groups);

    let progress_fixture = read_fixture(&[
        "ruby",
        "slice-232-projected-child-review-group-progress",
        "yard-example-review-progress.json",
    ]);
    let progress_groups = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        progress_fixture["groups"].clone(),
    )
    .expect("projected groups should deserialize");
    let resolved_case_ids =
        serde_json::from_value::<Vec<String>>(progress_fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected_progress = serde_json::from_value::<Vec<ProjectedChildReviewGroupProgress>>(
        progress_fixture["expected_progress"].clone(),
    )
    .expect("projected group progress should deserialize");
    assert_eq!(
        summarize_projected_child_review_group_progress(&progress_groups, &resolved_case_ids),
        expected_progress
    );

    let ready_fixture = read_fixture(&[
        "ruby",
        "slice-235-projected-child-review-groups-ready-for-apply",
        "yard-example-ready-groups.json",
    ]);
    let ready_groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(ready_fixture["groups"].clone())
            .expect("projected groups should deserialize");
    let ready_resolved_case_ids =
        serde_json::from_value::<Vec<String>>(ready_fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected_ready_groups = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        ready_fixture["expected_ready_groups"].clone(),
    )
    .expect("ready groups should deserialize");
    assert_eq!(
        select_projected_child_review_groups_ready_for_apply(
            &ready_groups,
            &ready_resolved_case_ids,
        ),
        expected_ready_groups
    );
}
