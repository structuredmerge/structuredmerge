use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceManifest, ConformanceOutcome,
    plan_named_conformance_suites, report_named_conformance_suite_envelope,
    report_planned_named_conformance_suites,
};
use markdown_merge::MarkdownDialect;
use pulldown_cmark_merge::{
    available_markdown_backends, markdown_backend_feature_profile, markdown_embedded_families,
    markdown_plan_context, match_markdown_owners, merge_markdown,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle,
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope,
    merge_markdown_with_reviewed_nested_outputs_from_review_state,
    merge_markdown_with_reviewed_nested_outputs, parse_markdown,
    provider_markdown_feature_profile,
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
fn conforms_to_provider_feature_profile_fixture() {
    let family_fixture = read_fixture(&[
        "diagnostics",
        "slice-194-markdown-family-feature-profile",
        "markdown-feature-profile.json",
    ]);
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-204-markdown-provider-feature-profiles",
        "rust-markdown-provider-feature-profiles.json",
    ]);

    assert_eq!(available_markdown_backends(), vec!["pulldown-cmark".to_string()]);
    assert!(
        registered_backends()
            .iter()
            .any(|backend| backend.id == "pulldown-cmark" && backend.family == "native")
    );
    let family_profile = markdown_merge::markdown_feature_profile();
    assert_eq!(
        serde_json::json!({
            "family": family_profile.family,
            "supported_dialects": family_profile
                .supported_dialects
                .iter()
                .map(|dialect| match dialect {
                    markdown_merge::MarkdownDialect::Markdown => "markdown".to_string(),
                })
                .collect::<Vec<_>>(),
            "supported_policies": [],
        }),
        family_fixture["feature_profile"]
    );
    assert_eq!(
        serde_json::to_value(markdown_backend_feature_profile()).unwrap(),
        fixture["providers"]["pulldown-cmark"]["feature_profile"]
    );
    assert_eq!(provider_markdown_feature_profile().family, "markdown");
}

#[test]
fn conforms_to_provider_plan_context_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-205-markdown-provider-plan-contexts",
        "rust-markdown-provider-plan-contexts.json",
    ]);

    assert_eq!(
        serde_json::to_value(markdown_plan_context()).unwrap(),
        fixture["providers"]["pulldown-cmark"]
    );
}

#[test]
fn conforms_to_shared_markdown_analysis_and_matching_fixtures() {
    let analysis_fixture =
        read_fixture(&["markdown", "slice-198-analysis", "headings-and-code-fences.json"]);
    let matching_fixture = read_fixture(&["markdown", "slice-199-matching", "path-equality.json"]);

    let analysis = parse_markdown(
        analysis_fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    );
    assert!(analysis.ok);
    assert_eq!(analysis.analysis.unwrap().root_kind, markdown_merge::MarkdownRootKind::Document);

    let template = parse_markdown(
        matching_fixture["template"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    )
    .analysis
    .unwrap();
    let destination = parse_markdown(
        matching_fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    )
    .analysis
    .unwrap();

    let result = match_markdown_owners(template, destination);
    assert_eq!(
        result
            .matched
            .iter()
            .map(|item| vec![item.template_path.clone(), item.destination_path.clone()])
            .collect::<Vec<_>>(),
        matching_fixture["expected"]["matched"]
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
        matching_fixture["expected"]["unmatched_template"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_destination,
        matching_fixture["expected"]["unmatched_destination"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );

    let merge_fixture = read_fixture(&["markdown", "slice-286-merge", "section-merge.json"]);
    let merge_result = merge_markdown(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    );
    assert!(merge_result.ok);
    assert_eq!(
        merge_result.output,
        merge_fixture["expected"]["output"].as_str().map(str::to_string)
    );
}

#[test]
fn conforms_to_slice_208_embedded_family_fixture() {
    let fixture =
        read_fixture(&["markdown", "slice-208-embedded-families", "code-fence-families.json"]);
    let analysis =
        parse_markdown(fixture["source"].as_str().unwrap(), MarkdownDialect::Markdown, None)
            .analysis
            .expect("analysis should exist");

    assert_eq!(
        serde_json::to_value(markdown_embedded_families(&analysis)).unwrap(),
        fixture["expected"]
    );
}

#[test]
fn conforms_to_slice_298_reviewed_nested_merge_fixture() {
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
        None,
    );
    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_326_markdown_provider_reviewed_nested_review_artifact_application_fixture() {
    let provider_fixture = read_fixture(&[
        "diagnostics",
        "slice-326-markdown-provider-reviewed-nested-review-artifact-application",
        "rust-markdown-provider-reviewed-nested-review-artifact-application.json",
    ]);
    let shared_fixture_path = provider_fixture["shared_fixture_path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let fixture = read_fixture(&shared_fixture_path);
    let expected = &provider_fixture["providers"]["pulldown-cmark"]["expected"];
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
        None,
    );
    assert!(replay_result.ok);
    assert_eq!(replay_result.output, expected["output"].as_str().map(str::to_string));

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state,
        None,
    );
    assert!(state_result.ok);
    assert_eq!(state_result.output, expected["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_327_markdown_provider_reviewed_nested_review_artifact_rejection_fixture() {
    let provider_fixture = read_fixture(&[
        "diagnostics",
        "slice-327-markdown-provider-reviewed-nested-review-artifact-rejection",
        "rust-markdown-provider-reviewed-nested-review-artifact-rejection.json",
    ]);
    let shared_fixture_path = provider_fixture["shared_fixture_path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let fixture = read_fixture(&shared_fixture_path);
    let expected_replay = &provider_fixture["providers"]["pulldown-cmark"]["expected_replay_bundle"];
    let expected_state = &provider_fixture["providers"]["pulldown-cmark"]["expected_review_state"];
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
        None,
    );
    assert!(!replay_result.ok);
    assert_eq!(
        replay_result.diagnostics[0].message,
        expected_replay["diagnostics"][0]["message"].as_str().unwrap()
    );

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state,
        None,
    );
    assert!(!state_result.ok);
    assert_eq!(
        state_result.diagnostics[0].message,
        expected_state["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn conforms_to_slice_313_reviewed_nested_review_artifact_envelope_application_fixture() {
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
        None,
    );
    assert!(replay_result.ok);
    assert_eq!(replay_result.output, fixture["expected"]["output"].as_str().map(str::to_string));

    let state_result = merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope(
        fixture["template"].as_str().unwrap(),
        fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        &review_state_envelope,
        None,
    );
    assert!(state_result.ok);
    assert_eq!(state_result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_315_reviewed_nested_review_artifact_envelope_rejection_fixture() {
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
        None,
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
        None,
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
fn conforms_to_provider_named_suite_plan_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-206-markdown-provider-named-suite-plans",
        "rust-markdown-provider-named-suite-plans.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts =
        serde_json::from_value(fixture["contexts"].clone()).expect("valid provider contexts");

    let projected = plan_named_conformance_suites(&manifest, &contexts)
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "suite": entry.suite,
                "plan": {
                    "family": entry.plan.family,
                    "entries": entry.plan.entries.into_iter().map(|plan_entry| {
                        serde_json::json!({
                            "ref": plan_entry.ref_,
                            "path": plan_entry.path,
                            "run": {
                                "ref": plan_entry.run.ref_,
                                "requirements": {},
                                "family_profile": plan_entry.run.family_profile,
                                "feature_profile": plan_entry.run.feature_profile
                            }
                        })
                    }).collect::<Vec<_>>(),
                    "missing_roles": entry.plan.missing_roles
                }
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(serde_json::to_value(projected).unwrap(), fixture["expected_entries"]);
}

#[test]
fn conforms_to_provider_manifest_report_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-207-markdown-provider-manifest-report",
        "rust-markdown-provider-manifest-report.json",
    ]);

    let manifest: ConformanceManifest =
        serde_json::from_value(fixture["manifest"].clone()).expect("valid manifest");
    let contexts = serde_json::from_value(fixture["options"]["contexts"].clone())
        .expect("valid provider contexts");
    let executions =
        fixture["executions"].as_object().expect("executions should be an object").clone();

    let entries = report_planned_named_conformance_suites(
        &plan_named_conformance_suites(&manifest, &contexts),
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            executions
                .get(&key)
                .cloned()
                .map(|value| {
                    serde_json::from_value::<ConformanceCaseExecution>(value)
                        .expect("valid execution")
                })
                .unwrap_or(ConformanceCaseExecution {
                    outcome: ConformanceOutcome::Failed,
                    messages: vec!["missing execution".to_string()],
                })
        },
    );

    assert_eq!(
        serde_json::to_value(report_named_conformance_suite_envelope(&entries)).unwrap(),
        fixture["expected_report"]
    );
}

#[test]
fn rejects_unsupported_provider_backend_overrides() {
    let expected = serde_json::json!([{
        "severity": "error",
        "category": "unsupported_feature",
        "message": "Unsupported Markdown backend kreuzberg-language-pack.",
        "path": null,
        "review": null
    }]);

    let parse_result = parse_markdown(
        "# Title\n",
        MarkdownDialect::Markdown,
        Some("kreuzberg-language-pack"),
    );
    assert!(!parse_result.ok);
    assert_eq!(serde_json::to_value(parse_result.diagnostics).unwrap(), expected);
}
