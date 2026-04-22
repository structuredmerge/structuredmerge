use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceManifest, ProjectedChildReviewCase, ProjectedChildReviewGroup,
    ProjectedChildReviewGroupProgress, conformance_family_feature_profile_path,
    conformance_fixture_path, delegated_child_apply_plan, group_projected_child_review_cases,
    projected_child_group_review_request, review_projected_child_groups,
    select_projected_child_review_groups_accepted_for_apply,
    select_projected_child_review_groups_ready_for_apply,
    summarize_projected_child_review_group_progress,
};
use markdown_merge::{
    MarkdownBackend, MarkdownDialect, MarkdownOwnerKind, apply_markdown_delegated_child_outputs,
    available_markdown_backends, markdown_backend_feature_profile,
    markdown_delegated_child_operations, markdown_discovered_surfaces, markdown_embedded_families,
    markdown_feature_profile, markdown_plan_context_with_backend, match_markdown_owners,
    merge_markdown, merge_markdown_with_nested_outputs,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle,
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope,
    merge_markdown_with_reviewed_nested_outputs_from_review_state,
    merge_markdown_with_reviewed_nested_outputs, parse_markdown_with_backend,
};
use serde_json::Value;
use tree_haver::registered_backends;

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
fn conforms_to_slice_194_markdown_feature_profile() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-194-markdown-family-feature-profile",
        "markdown-feature-profile.json",
    ]);

    let profile = markdown_feature_profile();
    assert_eq!(profile.family, fixture["feature_profile"]["family"].as_str().unwrap());
    assert_eq!(profile.supported_dialects, vec![MarkdownDialect::Markdown]);
}

#[test]
fn conforms_to_slice_195_markdown_backend_feature_profiles() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-195-markdown-family-backend-feature-profiles",
        "rust-markdown-backend-feature-profiles.json",
    ]);

    assert_eq!(available_markdown_backends(), vec![MarkdownBackend::KreuzbergLanguagePack]);
    assert!(
        registered_backends()
            .iter()
            .any(|backend| backend.id == "kreuzberg-language-pack"
                && backend.family == "tree-sitter")
    );

    let tree_sitter = markdown_backend_feature_profile(MarkdownBackend::KreuzbergLanguagePack);
    assert_eq!(tree_sitter.backend, fixture["tree_sitter"]["backend"]);
    assert_eq!(
        serde_json::json!({
            "backend": tree_sitter.backend,
            "supported_policies": [],
            "backend_ref": {
                "id": tree_sitter.backend_ref.id,
                "family": tree_sitter.backend_ref.family,
            }
        }),
        fixture["tree_sitter"]
    );
}

#[test]
fn conforms_to_slice_196_markdown_plan_contexts() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-196-markdown-family-plan-contexts",
        "rust-markdown-plan-contexts.json",
    ]);

    let tree_sitter = markdown_plan_context_with_backend(MarkdownBackend::KreuzbergLanguagePack);
    assert_eq!(
        tree_sitter.family_profile.family,
        fixture["tree_sitter"]["family_profile"]["family"]
    );
    assert_eq!(
        tree_sitter.feature_profile.expect("feature profile should be present").backend,
        fixture["tree_sitter"]["feature_profile"]["backend"]
    );
}

#[test]
fn conforms_to_slice_197_markdown_manifest() {
    let fixture = read_fixture(&[
        "conformance",
        "slice-197-markdown-family-manifest",
        "markdown-family-manifest.json",
    ]);
    let manifest: ConformanceManifest = serde_json::from_value(fixture).expect("valid manifest");

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "markdown"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-194-markdown-family-feature-profile".to_string(),
                "markdown-feature-profile.json".to_string(),
            ][..]
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "markdown", "analysis"),
        Some(
            &[
                "markdown".to_string(),
                "slice-198-analysis".to_string(),
                "headings-and-code-fences.json".to_string(),
            ][..]
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "markdown", "merge"),
        Some(
            &[
                "markdown".to_string(),
                "slice-286-merge".to_string(),
                "section-merge.json".to_string(),
            ][..]
        )
    );
}

#[test]
fn conforms_to_slice_198_markdown_analysis() {
    let fixture =
        read_fixture(&["markdown", "slice-198-analysis", "headings-and-code-fences.json"]);

    let result = parse_markdown_with_backend(
        fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(result.ok);
    let analysis = result.analysis.expect("analysis should be present");
    assert_eq!(analysis.root_kind, markdown_merge::MarkdownRootKind::Document);

    let owners = fixture["expected"]["owners"].as_array().unwrap();
    assert_eq!(analysis.owners.len(), owners.len());
    for (index, owner) in owners.iter().enumerate() {
        let expected = owner.as_object().unwrap();
        let actual = &analysis.owners[index];
        assert_eq!(actual.path, expected["path"]);
        assert_eq!(
            actual.owner_kind,
            match expected["owner_kind"].as_str().unwrap() {
                "heading" => MarkdownOwnerKind::Heading,
                "code_fence" => MarkdownOwnerKind::CodeFence,
                other => panic!("unexpected owner kind {other}"),
            }
        );
        assert_eq!(actual.match_key, expected["match_key"]);
    }
}

#[test]
fn conforms_to_slice_199_markdown_matching() {
    let fixture = read_fixture(&["markdown", "slice-199-matching", "path-equality.json"]);

    let template = parse_markdown_with_backend(
        fixture["template"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    )
    .analysis
    .expect("template analysis should exist");
    let destination = parse_markdown_with_backend(
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    )
    .analysis
    .expect("destination analysis should exist");

    let result = match_markdown_owners(template, destination);
    assert_eq!(
        result
            .matched
            .iter()
            .map(|item| vec![item.template_path.clone(), item.destination_path.clone()])
            .collect::<Vec<_>>(),
        fixture["expected"]["matched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                item.as_array()
                    .unwrap()
                    .iter()
                    .map(|part| part.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_template,
        fixture["expected"]["unmatched_template"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_destination,
        fixture["expected"]["unmatched_destination"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn conforms_to_slice_286_markdown_merge() {
    let fixture = read_fixture(&["markdown", "slice-286-merge", "section-merge.json"]);
    let result = merge_markdown(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_208_markdown_embedded_families() {
    let fixture =
        read_fixture(&["markdown", "slice-208-embedded-families", "code-fence-families.json"]);

    let analysis = parse_markdown_with_backend(
        fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    )
    .analysis
    .expect("analysis should exist");

    assert_eq!(
        serde_json::to_value(markdown_embedded_families(&analysis)).unwrap(),
        fixture["expected"]
    );
}

#[test]
fn conforms_to_slice_212_markdown_discovered_surfaces() {
    let fixture =
        read_fixture(&["markdown", "slice-212-discovered-surfaces", "fenced-code-surfaces.json"]);

    let analysis = parse_markdown_with_backend(
        fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    )
    .analysis
    .expect("analysis should exist");

    assert_eq!(
        serde_json::to_value(markdown_discovered_surfaces(&analysis)).unwrap(),
        fixture["expected"]
    );
}

#[test]
fn conforms_to_slice_213_markdown_delegated_child_operations() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-213-delegated-child-operations",
        "fenced-code-child-operations.json",
    ]);

    let analysis = parse_markdown_with_backend(
        fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::KreuzbergLanguagePack,
    )
    .analysis
    .expect("analysis should exist");

    assert_eq!(
        serde_json::to_value(markdown_delegated_child_operations(
            &analysis,
            fixture["parent_operation_id"].as_str().unwrap(),
        ))
        .unwrap(),
        fixture["expected"]
    );
}

#[test]
fn conforms_to_slice_228_markdown_projected_child_review_groups() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-228-projected-child-review-groups",
        "fenced-code-review-groups.json",
    ]);
    let cases = serde_json::from_value::<Vec<ProjectedChildReviewCase>>(fixture["cases"].clone())
        .expect("projected cases should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        fixture["expected_groups"].clone(),
    )
    .expect("projected groups should deserialize");

    assert_eq!(group_projected_child_review_cases(&cases), expected);
}

#[test]
fn conforms_to_slice_231_markdown_projected_child_review_group_progress() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-231-projected-child-review-group-progress",
        "fenced-code-review-progress.json",
    ]);
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("projected groups should deserialize");
    let resolved_case_ids =
        serde_json::from_value::<Vec<String>>(fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroupProgress>>(
        fixture["expected_progress"].clone(),
    )
    .expect("projected group progress should deserialize");

    assert_eq!(
        summarize_projected_child_review_group_progress(&groups, &resolved_case_ids),
        expected
    );
}

#[test]
fn conforms_to_slice_234_markdown_projected_child_review_groups_ready_for_apply() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-234-projected-child-review-groups-ready-for-apply",
        "fenced-code-ready-groups.json",
    ]);
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("projected groups should deserialize");
    let resolved_case_ids =
        serde_json::from_value::<Vec<String>>(fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        fixture["expected_ready_groups"].clone(),
    )
    .expect("ready groups should deserialize");

    assert_eq!(
        select_projected_child_review_groups_ready_for_apply(&groups, &resolved_case_ids),
        expected
    );
}

#[test]
fn conforms_to_slice_238_markdown_delegated_child_review_transport() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-238-delegated-child-review-transport",
        "fenced-code-review-transport.json",
    ]);
    let family = fixture["family"].as_str().expect("family should be a string");
    let group = serde_json::from_value::<ProjectedChildReviewGroup>(fixture["group"].clone())
        .expect("group should deserialize");
    let expected_request =
        serde_json::from_value::<ast_merge::ReviewRequest>(fixture["expected_request"].clone())
            .expect("expected request should deserialize");
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("groups should deserialize");
    let decisions =
        serde_json::from_value::<Vec<ast_merge::ReviewDecision>>(fixture["decisions"].clone())
            .expect("decisions should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        fixture["expected_accepted_groups"].clone(),
    )
    .expect("expected accepted groups should deserialize");

    assert_eq!(projected_child_group_review_request(&group, family), expected_request);
    assert_eq!(
        select_projected_child_review_groups_accepted_for_apply(&groups, family, &decisions),
        expected
    );
}

#[test]
fn conforms_to_slice_241_markdown_delegated_child_review_state() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-241-delegated-child-review-state",
        "fenced-code-review-state.json",
    ]);
    let family = fixture["family"].as_str().expect("family should be a string");
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("groups should deserialize");
    let decisions =
        serde_json::from_value::<Vec<ast_merge::ReviewDecision>>(fixture["decisions"].clone())
            .expect("decisions should deserialize");
    let expected = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        fixture["expected_state"].clone(),
    )
    .expect("expected state should deserialize");

    assert_eq!(review_projected_child_groups(&groups, family, &decisions), expected);
}

#[test]
fn conforms_to_slice_244_markdown_delegated_child_apply_plan() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-244-delegated-child-apply-plan",
        "fenced-code-apply-plan.json",
    ]);
    let family = fixture["family"].as_str().expect("family should be a string");
    let state = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");
    let expected = serde_json::from_value::<ast_merge::DelegatedChildApplyPlan>(
        fixture["expected_plan"].clone(),
    )
    .expect("expected plan should deserialize");

    assert_eq!(delegated_child_apply_plan(&state, family), expected);
}

#[test]
fn conforms_to_slice_288_markdown_delegated_child_apply_output() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-288-delegated-child-apply-output",
        "fenced-code-applied-output.json",
    ]);
    let operations = serde_json::from_value::<Vec<ast_merge::DelegatedChildOperation>>(
        fixture["delegated_operations"].clone(),
    )
    .expect("delegated operations should deserialize");
    let apply_plan =
        serde_json::from_value::<ast_merge::DelegatedChildApplyPlan>(fixture["apply_plan"].clone())
            .expect("apply plan should deserialize");
    let applied_children = serde_json::from_value::<Vec<markdown_merge::AppliedChildOutput>>(
        fixture["applied_children"].clone(),
    )
    .expect("applied children should deserialize");

    let result = apply_markdown_delegated_child_outputs(
        fixture["source"].as_str().unwrap(),
        &operations,
        &apply_plan,
        &applied_children,
    );
    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_290_markdown_nested_merge() {
    let fixture =
        read_fixture(&["markdown", "slice-290-nested-merge", "fenced-code-nested-merge.json"]);
    let nested_outputs = fixture["nested_outputs"]
        .as_array()
        .expect("nested outputs should be an array")
        .iter()
        .map(|entry| markdown_merge::NestedChildOutput {
            surface_address: entry["surface_address"].as_str().unwrap().to_string(),
            output: entry["output"].as_str().unwrap().to_string(),
        })
        .collect::<Vec<_>>();
    let result = merge_markdown_with_nested_outputs(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &nested_outputs,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_298_markdown_reviewed_nested_merge() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-298-reviewed-nested-merge",
        "fenced-code-reviewed-nested-merge.json",
    ]);
    let review_state = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");
    let applied_children = serde_json::from_value::<Vec<markdown_merge::AppliedChildOutput>>(
        fixture["applied_children"].clone(),
    )
    .expect("applied children should deserialize");
    let result = merge_markdown_with_reviewed_nested_outputs(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state,
        &applied_children,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_309_markdown_reviewed_nested_review_artifact_application() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-309-reviewed-nested-review-artifact-application",
        "fenced-code-reviewed-nested-review-artifact-application.json",
    ]);
    let replay_bundle = serde_json::from_value::<ast_merge::ReviewReplayBundle>(
        fixture["replay_bundle"].clone(),
    )
    .expect("replay bundle should deserialize");
    let review_state = serde_json::from_value::<ast_merge::ConformanceManifestReviewState>(
        fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");

    let replay_result = merge_markdown_with_reviewed_nested_outputs_from_replay_bundle(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &replay_bundle,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(replay_result.ok);
    assert_eq!(replay_result.output, fixture["expected"]["output"].as_str().map(str::to_string));

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(state_result.ok);
    assert_eq!(state_result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_311_markdown_reviewed_nested_review_artifact_rejection() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-311-reviewed-nested-review-artifact-rejection",
        "fenced-code-reviewed-nested-review-artifact-rejection.json",
    ]);
    let replay_bundle = serde_json::from_value::<ast_merge::ReviewReplayBundle>(
        fixture["replay_bundle"].clone(),
    )
    .expect("replay bundle should deserialize");
    let review_state = serde_json::from_value::<ast_merge::ConformanceManifestReviewState>(
        fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");
    let replay_result = merge_markdown_with_reviewed_nested_outputs_from_replay_bundle(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &replay_bundle,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(!replay_result.ok);
    assert_eq!(
        replay_result.diagnostics[0].message,
        fixture["expected"]["diagnostics"][0]["message"].as_str().unwrap()
    );
    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(!state_result.ok);
    assert_eq!(
        state_result.diagnostics[0].message,
        fixture["expected_review_state"]["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn conforms_to_slice_313_markdown_reviewed_nested_review_artifact_envelope_application() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-313-reviewed-nested-review-artifact-envelope-application",
        "fenced-code-reviewed-nested-review-artifact-envelope-application.json",
    ]);
    let replay_bundle_envelope =
        serde_json::from_value::<ast_merge::ReviewReplayBundleEnvelope>(
            fixture["replay_bundle_envelope"].clone(),
        )
        .expect("replay bundle envelope should deserialize");
    let review_state_envelope =
        serde_json::from_value::<ast_merge::ConformanceManifestReviewStateEnvelope>(
            fixture["review_state_envelope"].clone(),
        )
        .expect("review state envelope should deserialize");

    let replay_result = merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &replay_bundle_envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(replay_result.ok);
    assert_eq!(replay_result.output, fixture["expected"]["output"].as_str().map(str::to_string));

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state_envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(state_result.ok);
    assert_eq!(state_result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_315_markdown_reviewed_nested_review_artifact_envelope_rejection() {
    let fixture = read_fixture(&[
        "markdown",
        "slice-315-reviewed-nested-review-artifact-envelope-rejection",
        "fenced-code-reviewed-nested-review-artifact-envelope-rejection.json",
    ]);
    let replay_bundle_envelope =
        serde_json::from_value::<ast_merge::ReviewReplayBundleEnvelope>(
            fixture["replay_bundle_envelope"].clone(),
        )
        .expect("replay bundle envelope should deserialize");
    let review_state_envelope =
        serde_json::from_value::<ast_merge::ConformanceManifestReviewStateEnvelope>(
            fixture["review_state_envelope"].clone(),
        )
        .expect("review state envelope should deserialize");

    let replay_result = merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &replay_bundle_envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(!replay_result.ok);
    assert_eq!(
        replay_result.diagnostics[0].message,
        fixture["expected_replay_bundle"]["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
    );

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state_envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    );
    assert!(!state_result.ok);
    assert_eq!(
        state_result.diagnostics[0].message,
        fixture["expected_review_state"]["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
    );
}
