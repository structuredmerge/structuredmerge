use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceManifest, ProjectedChildReviewCase, ProjectedChildReviewGroup,
    ProjectedChildReviewGroupProgress, conformance_family_feature_profile_path,
    conformance_fixture_path, group_projected_child_review_cases,
    select_projected_child_review_groups_ready_for_apply,
    summarize_projected_child_review_group_progress,
};
use markdown_merge::{
    MarkdownBackend, MarkdownDialect, MarkdownOwnerKind, available_markdown_backends,
    markdown_backend_feature_profile, markdown_delegated_child_operations,
    markdown_discovered_surfaces, markdown_embedded_families, markdown_feature_profile,
    markdown_plan_context_with_backend, match_markdown_owners, parse_markdown_with_backend,
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

    assert_eq!(
        available_markdown_backends(),
        vec![MarkdownBackend::PulldownCmark, MarkdownBackend::KreuzbergLanguagePack]
    );

    let native = markdown_backend_feature_profile(MarkdownBackend::PulldownCmark);
    assert_eq!(native.backend, fixture["native"]["backend"]);
    let tree_sitter = markdown_backend_feature_profile(MarkdownBackend::KreuzbergLanguagePack);
    assert_eq!(tree_sitter.backend, fixture["tree_sitter"]["backend"]);
}

#[test]
fn conforms_to_slice_196_markdown_plan_contexts() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-196-markdown-family-plan-contexts",
        "rust-markdown-plan-contexts.json",
    ]);

    let native = markdown_plan_context_with_backend(MarkdownBackend::PulldownCmark);
    assert_eq!(native.family_profile.family, fixture["native"]["family_profile"]["family"]);
    assert_eq!(
        native.feature_profile.expect("feature profile should be present").backend,
        fixture["native"]["feature_profile"]["backend"]
    );

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
}

#[test]
fn conforms_to_slice_198_markdown_analysis() {
    let fixture =
        read_fixture(&["markdown", "slice-198-analysis", "headings-and-code-fences.json"]);

    for backend in [MarkdownBackend::PulldownCmark, MarkdownBackend::KreuzbergLanguagePack] {
        let result = parse_markdown_with_backend(
            fixture["source"].as_str().unwrap(),
            MarkdownDialect::Markdown,
            backend,
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
}

#[test]
fn conforms_to_slice_199_markdown_matching() {
    let fixture = read_fixture(&["markdown", "slice-199-matching", "path-equality.json"]);

    for backend in [MarkdownBackend::PulldownCmark, MarkdownBackend::KreuzbergLanguagePack] {
        let template = parse_markdown_with_backend(
            fixture["template"].as_str().unwrap(),
            MarkdownDialect::Markdown,
            backend,
        )
        .analysis
        .expect("template analysis should exist");
        let destination = parse_markdown_with_backend(
            fixture["destination"].as_str().unwrap(),
            MarkdownDialect::Markdown,
            backend,
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
}

#[test]
fn conforms_to_slice_208_markdown_embedded_families() {
    let fixture =
        read_fixture(&["markdown", "slice-208-embedded-families", "code-fence-families.json"]);

    for backend in [MarkdownBackend::PulldownCmark, MarkdownBackend::KreuzbergLanguagePack] {
        let analysis = parse_markdown_with_backend(
            fixture["source"].as_str().unwrap(),
            MarkdownDialect::Markdown,
            backend,
        )
        .analysis
        .expect("analysis should exist");

        assert_eq!(
            serde_json::to_value(markdown_embedded_families(&analysis)).unwrap(),
            fixture["expected"]
        );
    }
}

#[test]
fn conforms_to_slice_212_markdown_discovered_surfaces() {
    let fixture =
        read_fixture(&["markdown", "slice-212-discovered-surfaces", "fenced-code-surfaces.json"]);

    let analysis = parse_markdown_with_backend(
        fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        MarkdownBackend::PulldownCmark,
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
        MarkdownBackend::PulldownCmark,
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
