use std::{fs, path::PathBuf};

use ast_merge::{
    ProjectedChildReviewCase, ProjectedChildReviewGroup, ProjectedChildReviewGroupProgress,
    delegated_child_apply_plan, group_projected_child_review_cases,
    projected_child_group_review_request, review_projected_child_groups,
    select_projected_child_review_groups_accepted_for_apply,
    select_projected_child_review_groups_ready_for_apply,
    summarize_projected_child_review_group_progress,
};
use ruby_merge::{
    RubyDialect, RubyOwnerKind, apply_ruby_delegated_child_outputs, available_ruby_backends,
    match_ruby_owners, merge_ruby, parse_ruby, ruby_backend_feature_profile,
    ruby_delegated_child_operations, ruby_discovered_surfaces, ruby_feature_profile, ruby_plan_context,
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

    let backend_fixture = read_fixture(&[
        "diagnostics",
        "slice-215-ruby-family-backend-feature-profiles",
        "ruby-ruby-backend-feature-profiles.json",
    ]);
    assert_eq!(available_ruby_backends(), vec!["kreuzberg-language-pack".to_string()]);
    let backend_profile = ruby_backend_feature_profile();
    assert_eq!(backend_profile.backend, backend_fixture["tree_sitter"]["backend"]);

    let plan_fixture = read_fixture(&[
        "diagnostics",
        "slice-216-ruby-family-plan-contexts",
        "ruby-ruby-plan-contexts.json",
    ]);
    let plan_context = ruby_plan_context();
    assert_eq!(
        plan_context.family_profile.family,
        plan_fixture["tree_sitter"]["family_profile"]["family"]
    );
    assert_eq!(
        plan_context.feature_profile.as_ref().unwrap().backend,
        plan_fixture["tree_sitter"]["feature_profile"]["backend"]
    );

    let manifest_fixture = read_fixture(&[
        "conformance",
        "slice-217-ruby-family-manifest",
        "ruby-family-manifest.json",
    ]);
    let manifest = serde_json::from_value::<ast_merge::ConformanceManifest>(manifest_fixture)
        .expect("ruby manifest should deserialize");
    assert_eq!(
        ast_merge::conformance_family_feature_profile_path(&manifest, "ruby"),
        Some(vec![
            "diagnostics".to_string(),
            "slice-214-ruby-family-feature-profile".to_string(),
            "ruby-feature-profile.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&manifest, "ruby", "analysis"),
        Some(vec![
            "ruby".to_string(),
            "slice-218-analysis".to_string(),
            "module-owners.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&manifest, "ruby", "matching"),
        Some(vec![
            "ruby".to_string(),
            "slice-219-matching".to_string(),
            "path-equality.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&manifest, "ruby", "merge"),
        Some(vec![
            "ruby".to_string(),
            "slice-287-merge".to_string(),
            "module-merge.json".to_string(),
        ])
        .as_deref()
    );

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

    let merge_fixture = read_fixture(&["ruby", "slice-287-merge", "module-merge.json"]);
    let merge_result = merge_ruby(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        RubyDialect::Ruby,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );

    let invalid_template_fixture = read_fixture(&["ruby", "slice-287-merge", "invalid-template.json"]);
    let invalid_template_result = merge_ruby(
        invalid_template_fixture["template"].as_str().unwrap(),
        invalid_template_fixture["destination"].as_str().unwrap(),
        RubyDialect::Ruby,
    );
    assert!(!invalid_template_result.ok);
    assert_eq!(
        serde_json::json!([{
            "severity": invalid_template_result.diagnostics[0].severity,
            "category": invalid_template_result.diagnostics[0].category,
        }]),
        invalid_template_fixture["expected"]["diagnostics"]
    );

    let invalid_destination_fixture = read_fixture(&["ruby", "slice-287-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_ruby(
        invalid_destination_fixture["template"].as_str().unwrap(),
        invalid_destination_fixture["destination"].as_str().unwrap(),
        RubyDialect::Ruby,
    );
    assert!(!invalid_destination_result.ok);
    assert_eq!(
        serde_json::json!([{
            "severity": invalid_destination_result.diagnostics[0].severity,
            "category": invalid_destination_result.diagnostics[0].category,
        }]),
        invalid_destination_fixture["expected"]["diagnostics"]
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

    let transport_fixture = read_fixture(&[
        "ruby",
        "slice-239-delegated-child-review-transport",
        "yard-example-review-transport.json",
    ]);
    let family = transport_fixture["family"].as_str().expect("family should be a string");
    let transport_group =
        serde_json::from_value::<ProjectedChildReviewGroup>(transport_fixture["group"].clone())
            .expect("group should deserialize");
    let expected_request = serde_json::from_value::<ast_merge::ReviewRequest>(
        transport_fixture["expected_request"].clone(),
    )
    .expect("expected request should deserialize");
    let transport_groups = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        transport_fixture["groups"].clone(),
    )
    .expect("groups should deserialize");
    let transport_decisions = serde_json::from_value::<Vec<ast_merge::ReviewDecision>>(
        transport_fixture["decisions"].clone(),
    )
    .expect("decisions should deserialize");
    let expected_transport_groups = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        transport_fixture["expected_accepted_groups"].clone(),
    )
    .expect("expected accepted groups should deserialize");
    assert_eq!(projected_child_group_review_request(&transport_group, family), expected_request);
    assert_eq!(
        select_projected_child_review_groups_accepted_for_apply(
            &transport_groups,
            family,
            &transport_decisions,
        ),
        expected_transport_groups
    );

    let state_fixture = read_fixture(&[
        "ruby",
        "slice-242-delegated-child-review-state",
        "yard-example-review-state.json",
    ]);
    let state_family = state_fixture["family"].as_str().expect("family should be a string");
    let state_groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(state_fixture["groups"].clone())
            .expect("groups should deserialize");
    let state_decisions = serde_json::from_value::<Vec<ast_merge::ReviewDecision>>(
        state_fixture["decisions"].clone(),
    )
    .expect("decisions should deserialize");
    let expected_state = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        state_fixture["expected_state"].clone(),
    )
    .expect("expected state should deserialize");
    assert_eq!(
        review_projected_child_groups(&state_groups, state_family, &state_decisions),
        expected_state
    );

    let apply_plan_fixture = read_fixture(&[
        "ruby",
        "slice-245-delegated-child-apply-plan",
        "yard-example-apply-plan.json",
    ]);
    let apply_plan_family =
        apply_plan_fixture["family"].as_str().expect("family should be a string");
    let apply_plan_state = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        apply_plan_fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");
    let expected_apply_plan = serde_json::from_value::<ast_merge::DelegatedChildApplyPlan>(
        apply_plan_fixture["expected_plan"].clone(),
    )
    .expect("expected plan should deserialize");
    assert_eq!(
        delegated_child_apply_plan(&apply_plan_state, apply_plan_family),
        expected_apply_plan
    );

    let apply_output_fixture = read_fixture(&[
        "ruby",
        "slice-289-delegated-child-apply-output",
        "yard-example-applied-output.json",
    ]);
    let operations = serde_json::from_value::<Vec<ast_merge::DelegatedChildOperation>>(
        apply_output_fixture["delegated_operations"].clone(),
    )
    .expect("delegated operations should deserialize");
    let apply_plan = serde_json::from_value::<ast_merge::DelegatedChildApplyPlan>(
        apply_output_fixture["apply_plan"].clone(),
    )
    .expect("apply plan should deserialize");
    let applied_children = apply_output_fixture["applied_children"]
        .as_array()
        .expect("applied children should be an array")
        .iter()
        .map(|entry| ruby_merge::AppliedChildOutput {
            operation_id: entry["operation_id"].as_str().unwrap().to_string(),
            output: entry["output"].as_str().unwrap().to_string(),
        })
        .collect::<Vec<_>>();
    let apply_output_result = apply_ruby_delegated_child_outputs(
        apply_output_fixture["source"].as_str().unwrap(),
        &operations,
        &apply_plan,
        &applied_children,
    );
    assert!(apply_output_result.ok);
    assert_eq!(
        apply_output_result.output,
        apply_output_fixture["expected"]["output"].as_str().map(str::to_string)
    );
}
