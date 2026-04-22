use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceCaseRef, ConformanceCaseRequirements,
    ConformanceCaseResult, ConformanceCaseRun, ConformanceFamilyPlanContext,
    ConformanceFeatureProfileView, ConformanceManifest, ConformanceManifestPlanningOptions,
    ConformanceManifestReport, ConformanceManifestReviewOptions, ConformanceManifestReviewState,
    ConformanceManifestReviewStateEnvelope, ConformanceOutcome, ConformanceSelectionStatus,
    ConformanceSuiteDefinition, ConformanceSuitePlan, ConformanceSuiteReport,
    ConformanceSuiteSelector, ConformanceSuiteSubject, ConformanceSuiteSummary,
    DelegatedChildOperation, DiagnosticCategory, DiagnosticSeverity, DiscoveredSurface,
    FamilyFeatureProfile, NamedConformanceSuitePlan, NamedConformanceSuiteReport,
    NamedConformanceSuiteReportEnvelope, NamedConformanceSuiteResults, PolicySurface,
    ProjectedChildReviewCase, ProjectedChildReviewGroup, ProjectedChildReviewGroupProgress,
    REVIEW_TRANSPORT_VERSION, ReviewHostHints, ReviewReplayBundle, ReviewReplayBundleEnvelope,
    ReviewReplayContext, ReviewRequest, ReviewedNestedExecution, ReviewedNestedExecutionEnvelope,
    conformance_family_feature_profile_path, conformance_fixture_path,
    conformance_manifest_replay_context, conformance_manifest_review_request_ids,
    conformance_manifest_review_state_envelope, conformance_review_host_hints,
    conformance_suite_definition, conformance_suite_selectors, default_conformance_family_context,
    delegated_child_apply_plan, group_projected_child_review_cases,
    import_conformance_manifest_review_state_envelope, import_review_replay_bundle_envelope,
    import_reviewed_nested_execution_envelope, plan_conformance_suite,
    plan_named_conformance_suite, plan_named_conformance_suite_entry,
    plan_named_conformance_suites, plan_named_conformance_suites_with_diagnostics,
    projected_child_group_review_request, report_conformance_manifest, report_conformance_suite,
    report_named_conformance_suite, report_named_conformance_suite_entry,
    report_named_conformance_suite_envelope, report_named_conformance_suite_manifest,
    report_planned_conformance_suite, report_planned_named_conformance_suites,
    resolve_conformance_family_context, resolve_delegated_child_outputs,
    review_conformance_family_context, review_conformance_manifest, review_projected_child_groups,
    review_replay_bundle_envelope, review_replay_bundle_inputs, review_replay_context_compatible,
    review_request_id_for_family_context, review_request_id_for_projected_child_group,
    reviewed_nested_execution, reviewed_nested_execution_envelope, run_conformance_case,
    run_conformance_suite, run_named_conformance_suite, run_named_conformance_suite_entry,
    run_planned_conformance_suite, run_planned_named_conformance_suites, select_conformance_case,
    select_projected_child_review_groups_accepted_for_apply,
    select_projected_child_review_groups_ready_for_apply, summarize_conformance_results,
    summarize_named_conformance_suite_reports, summarize_projected_child_review_group_progress,
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

fn read_manifest() -> ConformanceManifest {
    let path = fixture_path(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let source = fs::read_to_string(path).expect("manifest should be readable");
    serde_json::from_str(&source).expect("manifest should be valid json")
}

fn path_buf_from_segments(segments: &[String]) -> PathBuf {
    let mut path = fixture_path(&[]);
    for segment in segments {
        path.push(segment);
    }

    path
}

fn diagnostics_fixture_path(role: &str) -> PathBuf {
    let manifest = read_manifest();
    let path = conformance_fixture_path(&manifest, "diagnostics", role)
        .expect("diagnostics fixture entry should be present");

    path_buf_from_segments(path)
}

fn read_fixture_from_path(path: PathBuf) -> Value {
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

fn fixture_suite_selector(fixture: &Value) -> ConformanceSuiteSelector {
    serde_json::from_value::<ConformanceSuiteSelector>(fixture["suite_selector"].clone())
        .expect("suite selector should deserialize")
}

fn fixture_suite_selectors(fixture: &Value) -> Vec<ConformanceSuiteSelector> {
    serde_json::from_value::<Vec<ConformanceSuiteSelector>>(fixture["suite_selectors"].clone())
        .expect("suite selectors should deserialize")
}

fn diagnostic_category_name(category: DiagnosticCategory) -> &'static str {
    match category {
        DiagnosticCategory::ParseError => "parse_error",
        DiagnosticCategory::DestinationParseError => "destination_parse_error",
        DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
        DiagnosticCategory::FallbackApplied => "fallback_applied",
        DiagnosticCategory::Ambiguity => "ambiguity",
        DiagnosticCategory::AssumedDefault => "assumed_default",
        DiagnosticCategory::ConfigurationError => "configuration_error",
        DiagnosticCategory::ReplayRejected => "replay_rejected",
    }
}

#[test]
fn conforms_to_slice_02_diagnostic_vocabulary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("diagnostic_vocabulary"));

    let severities = vec![
        match DiagnosticSeverity::Info {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
        match DiagnosticSeverity::Warning {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
        match DiagnosticSeverity::Error {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        },
    ];

    let categories = vec![
        diagnostic_category_name(DiagnosticCategory::ParseError),
        diagnostic_category_name(DiagnosticCategory::DestinationParseError),
        diagnostic_category_name(DiagnosticCategory::UnsupportedFeature),
        diagnostic_category_name(DiagnosticCategory::FallbackApplied),
        diagnostic_category_name(DiagnosticCategory::Ambiguity),
        diagnostic_category_name(DiagnosticCategory::AssumedDefault),
        diagnostic_category_name(DiagnosticCategory::ConfigurationError),
        diagnostic_category_name(DiagnosticCategory::ReplayRejected),
    ];

    assert_eq!(
        Value::Array(
            severities.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["severities"]
    );
    assert_eq!(
        Value::Array(
            categories.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["categories"]
    );
}

#[test]
fn conforms_to_slice_17_policy_vocabulary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("policy_vocabulary"));

    let surfaces = vec![
        match PolicySurface::Fallback {
            PolicySurface::Fallback => "fallback",
            PolicySurface::Array => "array",
        },
        match PolicySurface::Array {
            PolicySurface::Fallback => "fallback",
            PolicySurface::Array => "array",
        },
    ];

    let policies = serde_json::json!([
        {
            "surface": "fallback",
            "name": "trailing_comma_destination_fallback"
        },
        {
            "surface": "array",
            "name": "destination_wins_array"
        }
    ]);

    assert_eq!(
        Value::Array(
            surfaces.into_iter().map(|value| serde_json::json!(value)).collect::<Vec<_>>(),
        ),
        fixture["surfaces"]
    );
    assert_eq!(policies, fixture["policies"]);
}

#[test]
fn conforms_to_slice_18_policy_reporting_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("policy_reporting"));

    let merge_policies = serde_json::json!([
        {
            "surface": "array",
            "name": "destination_wins_array"
        },
        {
            "surface": "fallback",
            "name": "trailing_comma_destination_fallback"
        }
    ]);

    assert_eq!(merge_policies, fixture["merge_policies"]);
}

#[test]
fn conforms_to_slice_22_shared_family_feature_profile_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("shared_family_feature_profile"));

    let feature_profile = FamilyFeatureProfile {
        family: "example".to_string(),
        supported_dialects: vec!["alpha".to_string(), "beta".to_string()],
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let rendered = serde_json::json!({
        "family": feature_profile.family,
        "supported_dialects": feature_profile.supported_dialects,
        "supported_policies": feature_profile.supported_policies.iter().map(|policy| {
            serde_json::json!({
                "surface": match policy.surface {
                    PolicySurface::Fallback => "fallback",
                    PolicySurface::Array => "array",
                },
                "name": policy.name,
            })
        }).collect::<Vec<_>>()
    });

    assert_eq!(rendered, fixture["feature_profile"]);
}

#[test]
fn conforms_to_slice_28_conformance_runner_shape_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("runner_shape"));

    let case_ref = ConformanceCaseRef {
        family: "json".to_string(),
        role: "tree_sitter_adapter".to_string(),
        case: "valid_strict_json".to_string(),
    };
    let result = ConformanceCaseResult {
        ref_: case_ref.clone(),
        outcome: ConformanceOutcome::Passed,
        messages: vec![],
    };

    assert_eq!(
        serde_json::json!({
            "family": case_ref.family,
            "role": case_ref.role,
            "case": case_ref.case,
        }),
        fixture["case_ref"]
    );
    assert_eq!(
        serde_json::json!({
            "ref": {
                "family": result.ref_.family,
                "role": result.ref_.role,
                "case": result.ref_.case,
            },
            "outcome": match result.outcome {
                ConformanceOutcome::Passed => "passed",
                ConformanceOutcome::Failed => "failed",
                ConformanceOutcome::Skipped => "skipped",
            },
            "messages": result.messages,
        }),
        fixture["result"]
    );
}

#[test]
fn conforms_to_slice_30_normalized_manifest_contract() {
    let manifest = read_manifest();

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "json"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-21-family-feature-profile".to_string(),
                "json-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "text", "analysis"),
        Some(
            &[
                "text".to_string(),
                "slice-03-analysis".to_string(),
                "whitespace-and-blocks.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "diagnostics", "runner_shape"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-28-conformance-runner".to_string(),
                "runner-shape.json".to_string(),
            ][..],
        )
    );
}

#[test]
fn conforms_to_slice_32_conformance_suite_summary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("runner_summary"));
    let results: Vec<ConformanceCaseResult> =
        serde_json::from_value(fixture["results"].clone()).expect("results should deserialize");
    let summary: ConformanceSuiteSummary =
        serde_json::from_value(fixture["summary"].clone()).expect("summary should deserialize");

    assert_eq!(summarize_conformance_results(&results), summary);
}

#[test]
fn conforms_to_slice_33_capability_aware_selection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("capability_selection"));
    let cases = fixture["cases"].as_array().expect("cases should be present");

    for case in cases {
        let ref_ = serde_json::from_value::<ConformanceCaseRef>(case["ref"].clone())
            .expect("ref should deserialize");
        let requirements =
            serde_json::from_value::<ConformanceCaseRequirements>(case["requirements"].clone())
                .expect("requirements should deserialize");
        let family_profile =
            serde_json::from_value::<FamilyFeatureProfile>(case["family_profile"].clone())
                .expect("family_profile should deserialize");
        let feature_profile =
            serde_json::from_value::<serde_json::Value>(case["feature_profile"].clone())
                .expect("feature_profile should deserialize");
        let backend = feature_profile["backend"].as_str().expect("backend should be present");
        let supports_dialects = feature_profile["supports_dialects"]
            .as_bool()
            .expect("supports_dialects should be present");
        let supported_policies = serde_json::from_value::<Vec<ast_merge::PolicyReference>>(
            case["feature_profile"]["supported_policies"].clone(),
        )
        .expect("supported_policies should deserialize");

        let selection = select_conformance_case(
            ref_.clone(),
            &requirements,
            &family_profile,
            Some(&ConformanceFeatureProfileView {
                backend: backend.to_string(),
                supports_dialects,
                supported_policies,
            }),
        );

        let expected_status =
            match case["expected"]["status"].as_str().expect("status should be present") {
                "selected" => ConformanceSelectionStatus::Selected,
                "skipped" => ConformanceSelectionStatus::Skipped,
                other => panic!("unexpected status: {other}"),
            };
        let expected_messages = case["expected"]["messages"]
            .as_array()
            .expect("messages should be present")
            .iter()
            .map(|message| message.as_str().expect("message should be a string").to_string())
            .collect::<Vec<_>>();

        assert_eq!(selection.ref_, ref_);
        assert_eq!(selection.status, expected_status);
        assert_eq!(selection.messages, expected_messages);
    }
}

#[test]
fn conforms_to_slice_119_backend_aware_selection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("backend_selection"));
    let cases = fixture["cases"].as_array().expect("cases should be present");

    for case in cases {
        let ref_ = serde_json::from_value::<ConformanceCaseRef>(case["ref"].clone())
            .expect("ref should deserialize");
        let requirements =
            serde_json::from_value::<ConformanceCaseRequirements>(case["requirements"].clone())
                .expect("requirements should deserialize");
        let family_profile =
            serde_json::from_value::<FamilyFeatureProfile>(case["family_profile"].clone())
                .expect("family_profile should deserialize");
        let feature_profile =
            serde_json::from_value::<serde_json::Value>(case["feature_profile"].clone())
                .expect("feature_profile should deserialize");
        let backend = feature_profile["backend"].as_str().expect("backend should be present");
        let supports_dialects = feature_profile["supports_dialects"]
            .as_bool()
            .expect("supports_dialects should be present");
        let supported_policies = serde_json::from_value::<Vec<ast_merge::PolicyReference>>(
            case["feature_profile"]["supported_policies"].clone(),
        )
        .expect("supported_policies should deserialize");

        let selection = select_conformance_case(
            ref_.clone(),
            &requirements,
            &family_profile,
            Some(&ConformanceFeatureProfileView {
                backend: backend.to_string(),
                supports_dialects,
                supported_policies,
            }),
        );

        let expected_status =
            match case["expected"]["status"].as_str().expect("status should be present") {
                "selected" => ConformanceSelectionStatus::Selected,
                "skipped" => ConformanceSelectionStatus::Skipped,
                other => panic!("unexpected status: {other}"),
            };
        let expected_messages = case["expected"]["messages"]
            .as_array()
            .expect("messages should be present")
            .iter()
            .map(|message| message.as_str().expect("message should be a string").to_string())
            .collect::<Vec<_>>();

        assert_eq!(selection.ref_, ref_);
        assert_eq!(selection.status, expected_status);
        assert_eq!(selection.messages, expected_messages);
    }
}

#[test]
fn conforms_to_slice_34_conformance_case_runner_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("case_runner"));
    let cases = fixture["cases"].as_array().expect("cases should be present");

    for case in cases {
        let run = serde_json::from_value::<ConformanceCaseRun>(case["run"].clone())
            .expect("run should deserialize");
        let execution =
            serde_json::from_value::<ConformanceCaseExecution>(case["execution"].clone())
                .expect("execution should deserialize");
        let expected = serde_json::from_value::<ConformanceCaseResult>(case["expected"].clone())
            .expect("expected should deserialize");

        let result = run_conformance_case(&run, |_| execution.clone());
        assert_eq!(result, expected);
    }
}

#[test]
fn conforms_to_slice_35_conformance_suite_runner_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("suite_runner"));
    let runs = serde_json::from_value::<Vec<ConformanceCaseRun>>(fixture["cases"].clone())
        .expect("cases should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let expected =
        serde_json::from_value::<Vec<ConformanceCaseResult>>(fixture["expected_results"].clone())
            .expect("expected_results should deserialize");

    let results = run_conformance_suite(&runs, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(results, expected);
}

#[test]
fn conforms_to_slice_36_conformance_suite_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("suite_report"));
    let results = serde_json::from_value::<Vec<ConformanceCaseResult>>(fixture["results"].clone())
        .expect("results should deserialize");
    let report = serde_json::from_value::<ConformanceSuiteReport>(fixture["report"].clone())
        .expect("report should deserialize");

    assert_eq!(report_conformance_suite(&results), report);
}

#[test]
fn conforms_to_slice_39_conformance_suite_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("suite_plan"));
    let manifest = read_manifest();
    let roles = fixture["roles"]
        .as_array()
        .expect("roles should be present")
        .iter()
        .map(|value| value.as_str().expect("role should be a string").to_string())
        .collect::<Vec<_>>();
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let feature_profile =
        serde_json::from_value::<ConformanceFeatureProfileView>(fixture["feature_profile"].clone())
            .ok();
    let expected = serde_json::from_value::<ConformanceSuitePlan>(fixture["expected"].clone())
        .expect("expected suite plan should deserialize");

    let plan = plan_conformance_suite(
        &manifest,
        fixture["family"].as_str().expect("family should be a string"),
        &roles,
        &family_profile,
        feature_profile.as_ref(),
    );

    assert_eq!(plan, expected);
}

#[test]
fn conforms_to_slice_120_manifest_backend_requirements_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("manifest_backend_requirements"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let roles = fixture["roles"]
        .as_array()
        .expect("roles should be an array")
        .iter()
        .map(|role| role.as_str().expect("role should be a string").to_string())
        .collect::<Vec<_>>();
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let feature_profile =
        serde_json::from_value::<ConformanceFeatureProfileView>(fixture["feature_profile"].clone())
            .expect("feature profile should deserialize");
    let expected = serde_json::from_value::<ConformanceSuitePlan>(fixture["expected"].clone())
        .expect("expected suite plan should deserialize");

    let plan = plan_conformance_suite(
        &manifest,
        fixture["family"].as_str().expect("family should be a string"),
        &roles,
        &family_profile,
        Some(&feature_profile),
    );

    assert_eq!(plan, expected);
}

#[test]
fn conforms_to_slice_121_manifest_backend_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("manifest_backend_report"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let roles = fixture["roles"]
        .as_array()
        .expect("roles should be an array")
        .iter()
        .map(|role| role.as_str().expect("role should be a string").to_string())
        .collect::<Vec<_>>();
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let feature_profile =
        serde_json::from_value::<ConformanceFeatureProfileView>(fixture["feature_profile"].clone())
            .expect("feature profile should deserialize");
    let expected =
        serde_json::from_value::<ConformanceSuiteReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");

    let plan = plan_conformance_suite(
        &manifest,
        fixture["family"].as_str().expect("family should be a string"),
        &roles,
        &family_profile,
        Some(&feature_profile),
    );

    let report = report_planned_conformance_suite(&plan, |_| ConformanceCaseExecution {
        outcome: ConformanceOutcome::Failed,
        messages: vec!["unexpected execution".to_string()],
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_40_planned_conformance_suite_runner_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("planned_suite_runner"));
    let plan = serde_json::from_value::<ConformanceSuitePlan>(fixture["plan"].clone())
        .expect("plan should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let expected =
        serde_json::from_value::<Vec<ConformanceCaseResult>>(fixture["expected_results"].clone())
            .expect("expected results should deserialize");

    let results = run_planned_conformance_suite(&plan, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(results, expected);
}

#[test]
fn conforms_to_slice_41_planned_conformance_suite_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("planned_suite_report"));
    let plan = serde_json::from_value::<ConformanceSuitePlan>(fixture["plan"].clone())
        .expect("plan should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let expected =
        serde_json::from_value::<ConformanceSuiteReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");

    let report = report_planned_conformance_suite(&plan, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_42_manifest_case_requirements_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("manifest_requirements"));
    let manifest = read_manifest();
    let roles = fixture["roles"]
        .as_array()
        .expect("roles should be present")
        .iter()
        .map(|value| value.as_str().expect("role should be a string").to_string())
        .collect::<Vec<_>>();
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceCaseRequirements>,
    >(fixture["expected_requirements"].clone())
    .expect("expected requirements should deserialize");

    let plan = plan_conformance_suite(
        &manifest,
        fixture["family"].as_str().expect("family should be a string"),
        &roles,
        &family_profile,
        None,
    );
    let actual = plan
        .entries
        .iter()
        .map(|entry| (entry.ref_.role.clone(), entry.run.requirements.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_slice_43_conformance_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("suite_definitions"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let expected = serde_json::from_value::<ast_merge::ConformanceSuiteDefinition>(
        fixture["expected"].clone(),
    )
    .expect("expected definition should deserialize");
    let family_profile = FamilyFeatureProfile {
        family: "json".to_string(),
        supported_dialects: vec!["json".to_string(), "jsonc".to_string()],
        supported_policies: vec![
            ast_merge::PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            },
            ast_merge::PolicyReference {
                surface: PolicySurface::Fallback,
                name: "trailing_comma_destination_fallback".to_string(),
            },
        ],
    };

    assert_eq!(conformance_suite_definition(&manifest, &selector), Some(&expected));
    assert_eq!(
        plan_named_conformance_suite(&manifest, &selector, &family_profile, None),
        Some(plan_conformance_suite(
            &manifest,
            &expected.subject.grammar,
            &expected.roles,
            &family_profile,
            None,
        )),
    );
}

#[test]
fn conforms_to_slice_125_source_family_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-125-source-family-suite-definitions",
        "source-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let expected_selectors = fixture_suite_selectors(&fixture);
    let expected_definitions = serde_json::from_value::<Vec<ConformanceSuiteDefinition>>(
        fixture["suite_definitions"].clone(),
    )
    .expect("suite definitions should deserialize");

    assert_eq!(conformance_suite_selectors(&manifest), expected_selectors);

    for (selector, expected) in expected_selectors.iter().zip(expected_definitions.iter()) {
        assert_eq!(conformance_suite_definition(&manifest, selector), Some(expected));
    }
}

#[test]
fn conforms_to_slice_44_named_conformance_suite_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_report"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected =
        serde_json::from_value::<ConformanceSuiteReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let feature_profile = ConformanceFeatureProfileView {
        backend: "kreuzberg-language-pack".to_string(),
        supports_dialects: false,
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let report = report_named_conformance_suite(
        &manifest,
        &selector,
        &family_profile,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        },
        Some(&feature_profile),
    );

    assert_eq!(report, Some(expected));
}

#[test]
fn conforms_to_slice_45_named_conformance_suite_runner_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_runner"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected =
        serde_json::from_value::<Vec<ConformanceCaseResult>>(fixture["expected_results"].clone())
            .expect("expected results should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let feature_profile = ConformanceFeatureProfileView {
        backend: "kreuzberg-language-pack".to_string(),
        supports_dialects: false,
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let results = run_named_conformance_suite(
        &manifest,
        &selector,
        &family_profile,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        },
        Some(&feature_profile),
    );

    assert_eq!(results, Some(expected));
}

#[test]
fn conforms_to_slice_46_conformance_suite_names_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("suite_names"));
    let manifest = read_manifest();
    let expected = fixture_suite_selectors(&fixture);

    assert_eq!(conformance_suite_selectors(&manifest), expected);
}

#[test]
fn conforms_to_slice_47_named_conformance_suite_entry_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_entry"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected =
        serde_json::from_value::<NamedConformanceSuiteReport>(fixture["expected_entry"].clone())
            .expect("expected entry should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let feature_profile = ConformanceFeatureProfileView {
        backend: "kreuzberg-language-pack".to_string(),
        supports_dialects: false,
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let entry = report_named_conformance_suite_entry(
        &manifest,
        &selector,
        &family_profile,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        },
        Some(&feature_profile),
    );

    assert_eq!(entry, Some(expected));
}

#[test]
fn conforms_to_slice_48_named_conformance_suite_plan_entry_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_plan_entry"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let context =
        serde_json::from_value::<ConformanceFamilyPlanContext>(fixture["context"].clone())
            .expect("context should deserialize");
    let expected =
        serde_json::from_value::<NamedConformanceSuitePlan>(fixture["expected_entry"].clone())
            .expect("expected entry should deserialize");

    assert_eq!(plan_named_conformance_suite_entry(&manifest, &selector, &context), Some(expected),);
}

#[test]
fn conforms_to_slice_49_conformance_family_plan_context_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("family_plan_context"));
    let context =
        serde_json::from_value::<ConformanceFamilyPlanContext>(fixture["context"].clone())
            .expect("context should deserialize");

    assert_eq!(
        context,
        ConformanceFamilyPlanContext {
            family_profile: FamilyFeatureProfile {
                family: "json".to_string(),
                supported_dialects: vec!["json".to_string(), "jsonc".to_string()],
                supported_policies: vec![
                    ast_merge::PolicyReference {
                        surface: PolicySurface::Array,
                        name: "destination_wins_array".to_string(),
                    },
                    ast_merge::PolicyReference {
                        surface: PolicySurface::Fallback,
                        name: "trailing_comma_destination_fallback".to_string(),
                    },
                ],
            },
            feature_profile: Some(ConformanceFeatureProfileView {
                backend: "kreuzberg-language-pack".to_string(),
                supports_dialects: false,
                supported_policies: vec![ast_merge::PolicyReference {
                    surface: PolicySurface::Array,
                    name: "destination_wins_array".to_string(),
                }],
            }),
        },
    );
}

#[test]
fn conforms_to_slice_50_named_conformance_suite_plans_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_plans"));
    let manifest = read_manifest();
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_126_source_family_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-126-source-family-named-suite-plans",
        "source-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_127_source_family_native_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-127-source-family-native-suite-plans",
        "source-native-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_138_toml_family_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-138-toml-family-suite-definitions",
        "toml-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![ConformanceSuiteSelector {
            kind: "portable".to_string(),
            subject: ConformanceSuiteSubject { grammar: "toml".to_string(), variant: None },
        }]
    );
    let expected_definition = ConformanceSuiteDefinition {
        kind: "portable".to_string(),
        subject: ConformanceSuiteSubject { grammar: "toml".to_string(), variant: None },
        roles: vec!["analysis".to_string(), "matching".to_string(), "merge".to_string()],
    };
    assert_eq!(
        conformance_suite_definition(
            &manifest,
            &ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "toml".to_string(), variant: None },
            },
        ),
        Some(&expected_definition)
    );
}

#[test]
fn conforms_to_slice_139_toml_family_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-139-toml-family-named-suite-plans",
        "rust-toml-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_200_markdown_family_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-200-markdown-family-suite-definitions",
        "markdown-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![ConformanceSuiteSelector {
            kind: "portable".to_string(),
            subject: ConformanceSuiteSubject { grammar: "markdown".to_string(), variant: None },
        }]
    );
    let expected_definition = ConformanceSuiteDefinition {
        kind: "portable".to_string(),
        subject: ConformanceSuiteSubject { grammar: "markdown".to_string(), variant: None },
        roles: vec!["analysis".to_string(), "matching".to_string(), "merge".to_string()],
    };
    assert_eq!(
        conformance_suite_definition(
            &manifest,
            &ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "markdown".to_string(), variant: None },
            },
        ),
        Some(&expected_definition)
    );
}

#[test]
fn conforms_to_slice_201_markdown_family_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-201-markdown-family-named-suite-plans",
        "rust-markdown-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_202_markdown_family_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-202-markdown-family-manifest-report",
        "rust-markdown-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_246_markdown_nested_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-246-markdown-nested-suite-definitions",
        "markdown-nested-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![ConformanceSuiteSelector {
            kind: "portable".to_string(),
            subject: ConformanceSuiteSubject {
                grammar: "markdown".to_string(),
                variant: Some("nested".to_string()),
            },
        }]
    );
    assert_eq!(
        conformance_suite_definition(
            &manifest,
            &ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject {
                    grammar: "markdown".to_string(),
                    variant: Some("nested".to_string()),
                },
            },
        )
        .expect("definition should exist")
        .roles,
        vec![
            "analysis".to_string(),
            "matching".to_string(),
            "embedded_families".to_string(),
            "discovered_surfaces".to_string(),
            "delegated_child_operations".to_string(),
            "delegated_child_review_transport".to_string(),
            "delegated_child_review_state".to_string(),
            "delegated_child_apply_plan".to_string(),
        ]
    );
}

#[test]
fn conforms_to_slice_247_markdown_nested_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-247-markdown-nested-named-suite-plans",
        "markdown-nested-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_248_markdown_nested_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-248-markdown-nested-manifest-report",
        "markdown-nested-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        executions
            .get(&key)
            .map(|value| {
                serde_json::from_value::<ConformanceCaseExecution>(value.clone())
                    .expect("execution should deserialize")
            })
            .unwrap_or_else(|| ConformanceCaseExecution {
                outcome: ConformanceOutcome::Failed,
                messages: vec!["missing execution".to_string()],
            })
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_249_ruby_nested_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-249-ruby-nested-suite-definitions",
        "ruby-nested-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![ConformanceSuiteSelector {
            kind: "portable".to_string(),
            subject: ConformanceSuiteSubject {
                grammar: "ruby".to_string(),
                variant: Some("nested".to_string()),
            },
        }]
    );
    assert_eq!(
        conformance_suite_definition(
            &manifest,
            &ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject {
                    grammar: "ruby".to_string(),
                    variant: Some("nested".to_string()),
                },
            },
        )
        .expect("definition should exist")
        .roles,
        vec![
            "analysis".to_string(),
            "matching".to_string(),
            "discovered_surfaces".to_string(),
            "delegated_child_operations".to_string(),
            "delegated_child_review_transport".to_string(),
            "delegated_child_review_state".to_string(),
            "delegated_child_apply_plan".to_string(),
        ]
    );
}

#[test]
fn conforms_to_slice_250_ruby_nested_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-250-ruby-nested-named-suite-plans",
        "ruby-nested-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_251_ruby_nested_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-251-ruby-nested-manifest-report",
        "ruby-nested-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        executions
            .get(&key)
            .map(|value| {
                serde_json::from_value::<ConformanceCaseExecution>(value.clone())
                    .expect("execution should deserialize")
            })
            .unwrap_or_else(|| ConformanceCaseExecution {
                outcome: ConformanceOutcome::Failed,
                messages: vec!["missing execution".to_string()],
            })
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_51_named_conformance_suite_results_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_results"));
    let manifest = read_manifest();
    let selector = fixture_suite_selector(&fixture);
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected =
        serde_json::from_value::<NamedConformanceSuiteResults>(fixture["expected_entry"].clone())
            .expect("expected entry should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let feature_profile = ConformanceFeatureProfileView {
        backend: "kreuzberg-language-pack".to_string(),
        supports_dialects: false,
        supported_policies: vec![ast_merge::PolicyReference {
            surface: PolicySurface::Array,
            name: "destination_wins_array".to_string(),
        }],
    };

    let entry = run_named_conformance_suite_entry(
        &manifest,
        &selector,
        &family_profile,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        },
        Some(&feature_profile),
    );

    assert_eq!(entry, Some(expected));
}

#[test]
fn conforms_to_slice_52_planned_named_conformance_suite_runner_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_runner_entries"));
    let manifest = read_manifest();
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuiteResults>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let plans = plan_named_conformance_suites(&manifest, &contexts);

    let entries = run_planned_named_conformance_suites(&plans, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(entries, expected);
}

#[test]
fn conforms_to_slice_53_planned_named_conformance_suite_reports_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_report_entries"));
    let manifest = read_manifest();
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuiteReport>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let plans = plan_named_conformance_suites(&manifest, &contexts);

    let entries = report_planned_named_conformance_suites(&plans, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(entries, expected);
}

#[test]
fn conforms_to_slice_54_named_conformance_suite_summary_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_summary"));
    let entries =
        serde_json::from_value::<Vec<NamedConformanceSuiteReport>>(fixture["entries"].clone())
            .expect("entries should deserialize");
    let expected =
        serde_json::from_value::<ConformanceSuiteSummary>(fixture["expected_summary"].clone())
            .expect("expected summary should deserialize");

    assert_eq!(summarize_named_conformance_suite_reports(&entries), expected);
}

#[test]
fn conforms_to_slice_55_named_conformance_suite_report_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_report_envelope"));
    let entries =
        serde_json::from_value::<Vec<NamedConformanceSuiteReport>>(fixture["entries"].clone())
            .expect("entries should deserialize");
    let expected = serde_json::from_value::<NamedConformanceSuiteReportEnvelope>(
        fixture["expected_report"].clone(),
    )
    .expect("expected report should deserialize");

    assert_eq!(report_named_conformance_suite_envelope(&entries), expected);
}

#[test]
fn conforms_to_slice_56_named_conformance_suite_report_manifest_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_report_manifest"));
    let manifest = read_manifest();
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<NamedConformanceSuiteReportEnvelope>(
        fixture["expected_report"].clone(),
    )
    .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_named_conformance_suite_manifest(&manifest, &contexts, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_57_default_family_context_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("default_family_context"));
    let family = fixture["family"].as_str().expect("family should be a string");
    let family_profile =
        serde_json::from_value::<FamilyFeatureProfile>(fixture["family_profile"].clone())
            .expect("family profile should deserialize");
    let expected_context =
        serde_json::from_value::<ConformanceFamilyPlanContext>(fixture["expected_context"].clone())
            .expect("expected context should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");

    assert_eq!(default_conformance_family_context(&family_profile), expected_context);
    let options = ConformanceManifestPlanningOptions {
        contexts: std::collections::HashMap::new(),
        family_profiles: std::collections::HashMap::from([(family.to_string(), family_profile)]),
        require_explicit_contexts: false,
    };
    let (context, diagnostics) = resolve_conformance_family_context(family, &options);
    assert_eq!(context, Some(expected_context));
    assert_eq!(diagnostics, vec![expected_diagnostic]);
}

#[test]
fn conforms_to_slice_58_explicit_family_context_mode_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("explicit_family_context_mode"));
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");

    let (_, diagnostics) = resolve_conformance_family_context("text", &options);
    assert_eq!(diagnostics, vec![expected_diagnostic]);
}

#[test]
fn conforms_to_slice_59_missing_suite_roles_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("missing_suite_roles"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");

    let planned = plan_named_conformance_suites_with_diagnostics(&manifest, &options);
    assert!(planned.entries.is_empty());
    assert!(planned.diagnostics.contains(&expected_diagnostic));
}

#[test]
fn conforms_to_slice_60_conformance_manifest_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("conformance_manifest_report"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_128_source_family_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-128-source-family-manifest-report",
        "source-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_140_toml_family_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-140-toml-family-manifest-report",
        "rust-toml-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_144_yaml_family_suite_definitions_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-144-yaml-family-suite-definitions",
        "yaml-suite-definitions.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![ConformanceSuiteSelector {
            kind: "portable".to_string(),
            subject: ConformanceSuiteSubject { grammar: "yaml".to_string(), variant: None },
        }]
    );
    let expected_definition = ConformanceSuiteDefinition {
        kind: "portable".to_string(),
        subject: ConformanceSuiteSubject { grammar: "yaml".to_string(), variant: None },
        roles: vec!["analysis".to_string(), "matching".to_string(), "merge".to_string()],
    };
    assert_eq!(
        conformance_suite_definition(
            &manifest,
            &ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "yaml".to_string(), variant: None },
            },
        ),
        Some(&expected_definition)
    );
}

#[test]
fn conforms_to_slice_145_yaml_family_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-145-yaml-family-named-suite-plans",
        "rust-yaml-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_146_yaml_family_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-146-yaml-family-manifest-report",
        "rust-yaml-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_173_yaml_family_backend_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-173-yaml-family-backend-named-suite-plans",
        "rust-yaml-backend-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_174_yaml_family_backend_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-174-yaml-family-backend-manifest-report",
        "rust-yaml-backend-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_185_yaml_family_polyglot_backend_named_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-185-yaml-family-polyglot-backend-named-suite-plans",
        "rust-yaml-polyglot-named-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_186_yaml_family_polyglot_backend_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-186-yaml-family-polyglot-backend-manifest-report",
        "rust-yaml-polyglot-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_148_config_family_aggregate_manifest_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-148-config-family-aggregate-manifest",
        "config-family-aggregate.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");

    assert_eq!(
        conformance_suite_selectors(&manifest),
        vec![
            ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "json".to_string(), variant: None },
            },
            ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "text".to_string(), variant: None },
            },
            ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "toml".to_string(), variant: None },
            },
            ConformanceSuiteSelector {
                kind: "portable".to_string(),
                subject: ConformanceSuiteSubject { grammar: "yaml".to_string(), variant: None },
            },
        ]
    );
}

#[test]
fn conforms_to_slice_149_config_family_aggregate_suite_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-149-config-family-aggregate-suite-plans",
        "config-family-aggregate-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_150_config_family_aggregate_manifest_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-150-config-family-aggregate-manifest-report",
        "config-family-aggregate-manifest-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_aggregate_config_family_review_state_fixtures() {
    for fixture_name in [
        "slice-151-config-family-aggregate-review-state/config-family-aggregate-review-state.json",
        "slice-152-config-family-aggregate-reviewed-default/config-family-aggregate-reviewed-default.json",
        "slice-153-config-family-aggregate-replay-application/config-family-aggregate-replay-application.json",
    ] {
        let fixture = read_fixture_from_path(fixture_path(&["diagnostics"]).join(fixture_name));
        let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
            .expect("manifest should deserialize");
        let options =
            serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
                .expect("options should deserialize");
        let expected = serde_json::from_value::<ConformanceManifestReviewState>(
            fixture["expected_state"].clone(),
        )
        .expect("expected state should deserialize");
        let executions = fixture["executions"].as_object().expect("executions should be an object");

        let state = review_conformance_manifest(&manifest, &options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(state, expected);
    }
}

#[test]
fn conforms_to_canonical_stable_suite_planning_and_review_fixtures() {
    let plans_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-155-canonical-stable-suite-plans",
        "canonical-stable-suite-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(plans_fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(plans_fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected_plans = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        plans_fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");
    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected_plans);

    let report_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-156-canonical-stable-suite-report",
        "canonical-stable-suite-report.json",
    ]));
    let report_options = serde_json::from_value::<ConformanceManifestPlanningOptions>(
        report_fixture["options"].clone(),
    )
    .expect("options should deserialize");
    let expected_report = serde_json::from_value::<ConformanceManifestReport>(
        report_fixture["expected_report"].clone(),
    )
    .expect("expected report should deserialize");
    let report_executions =
        report_fixture["executions"].as_object().expect("executions should be an object");
    let report = report_conformance_manifest(&manifest, &report_options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            report_executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });
    assert_eq!(report, expected_report);

    let review_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-157-canonical-stable-suite-review-state",
        "canonical-stable-suite-review-state.json",
    ]));
    let review_options = serde_json::from_value::<ConformanceManifestReviewOptions>(
        review_fixture["options"].clone(),
    )
    .expect("options should deserialize");
    let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
        review_fixture["expected_state"].clone(),
    )
    .expect("expected state should deserialize");
    let review_executions =
        review_fixture["executions"].as_object().expect("executions should be an object");
    let review_state = review_conformance_manifest(&manifest, &review_options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            review_executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });
    assert_eq!(review_state, expected_state);
}

#[test]
fn conforms_to_canonical_stable_suite_backend_fixtures() {
    let plans_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-175-canonical-stable-suite-backend-plans",
        "rust-canonical-stable-suite-backend-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(plans_fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(plans_fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected_plans = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        plans_fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected_plans);

    let report_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-176-canonical-stable-suite-backend-report",
        "rust-canonical-stable-suite-backend-report.json",
    ]));
    let report_options = serde_json::from_value::<ConformanceManifestPlanningOptions>(
        report_fixture["options"].clone(),
    )
    .expect("options should deserialize");
    let expected_report = serde_json::from_value::<ConformanceManifestReport>(
        report_fixture["expected_report"].clone(),
    )
    .expect("expected report should deserialize");
    let report_executions =
        report_fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &report_options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            report_executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected_report);

    let review_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-177-canonical-stable-suite-backend-review-state",
        "rust-canonical-stable-suite-backend-review-state.json",
    ]));
    let review_options = serde_json::from_value::<ConformanceManifestReviewOptions>(
        review_fixture["options"].clone(),
    )
    .expect("options should deserialize");
    let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
        review_fixture["expected_state"].clone(),
    )
    .expect("expected state should deserialize");
    let review_executions =
        review_fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &review_options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            review_executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected_state);
}

#[test]
fn conforms_to_canonical_widened_suite_backend_fixtures() {
    for (plans_slice, plans_file, report_slice, report_file, review_fixtures) in [
        (
            "slice-178-canonical-widened-suite-backend-plans",
            "rust-canonical-widened-suite-backend-plans.json",
            "slice-179-canonical-widened-suite-backend-report",
            "rust-canonical-widened-suite-backend-report.json",
            [
                "slice-180-canonical-widened-suite-backend-review-state/rust-canonical-widened-suite-backend-review-state.json",
                "slice-181-canonical-widened-suite-backend-reviewed-default/rust-canonical-widened-suite-backend-reviewed-default.json",
                "slice-182-canonical-widened-suite-backend-replay-application/rust-canonical-widened-suite-backend-replay-application.json",
            ],
        ),
        (
            "slice-187-canonical-widened-suite-polyglot-backend-plans",
            "rust-canonical-widened-suite-polyglot-backend-plans.json",
            "slice-188-canonical-widened-suite-polyglot-backend-report",
            "rust-canonical-widened-suite-polyglot-backend-report.json",
            [
                "slice-189-canonical-widened-suite-polyglot-backend-review-state/rust-canonical-widened-suite-polyglot-backend-review-state.json",
                "slice-190-canonical-widened-suite-polyglot-backend-reviewed-default/rust-canonical-widened-suite-polyglot-backend-reviewed-default.json",
                "slice-191-canonical-widened-suite-polyglot-backend-replay-application/rust-canonical-widened-suite-polyglot-backend-replay-application.json",
            ],
        ),
    ] {
        let plans_fixture =
            read_fixture_from_path(fixture_path(&["diagnostics", plans_slice, plans_file]));
        let manifest =
            serde_json::from_value::<ConformanceManifest>(plans_fixture["manifest"].clone())
                .expect("manifest should deserialize");
        let contexts = serde_json::from_value::<
            std::collections::HashMap<String, ConformanceFamilyPlanContext>,
        >(plans_fixture["contexts"].clone())
        .expect("contexts should deserialize");
        let expected_plans = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
            plans_fixture["expected_entries"].clone(),
        )
        .expect("expected entries should deserialize");

        assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected_plans);

        let report_fixture =
            read_fixture_from_path(fixture_path(&["diagnostics", report_slice, report_file]));
        let report_options = serde_json::from_value::<ConformanceManifestPlanningOptions>(
            report_fixture["options"].clone(),
        )
        .expect("options should deserialize");
        let expected_report = serde_json::from_value::<ConformanceManifestReport>(
            report_fixture["expected_report"].clone(),
        )
        .expect("expected report should deserialize");
        let report_executions =
            report_fixture["executions"].as_object().expect("executions should be an object");

        let report = report_conformance_manifest(&manifest, &report_options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                report_executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(report, expected_report);

        for fixture_name in review_fixtures {
            let fixture = read_fixture_from_path(fixture_path(&["diagnostics"]).join(fixture_name));
            let options = serde_json::from_value::<ConformanceManifestReviewOptions>(
                fixture["options"].clone(),
            )
            .expect("options should deserialize");
            let expected = serde_json::from_value::<ConformanceManifestReviewState>(
                fixture["expected_state"].clone(),
            )
            .expect("expected state should deserialize");
            let executions =
                fixture["executions"].as_object().expect("executions should be an object");

            let state = review_conformance_manifest(&manifest, &options, |run| {
                let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
                serde_json::from_value::<ConformanceCaseExecution>(
                    executions.get(&key).cloned().unwrap_or_else(
                        || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                    ),
                )
                .expect("execution should deserialize")
            });

            assert_eq!(state, expected);
        }
    }
}

#[test]
fn conforms_to_slice_129_source_family_backend_restricted_plans_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-129-source-family-backend-restricted-plans",
        "source-backend-restricted-plans.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");

    assert_eq!(plan_named_conformance_suites(&manifest, &contexts), expected);
}

#[test]
fn conforms_to_slice_130_source_family_backend_restricted_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-130-source-family-backend-restricted-report",
        "source-backend-restricted-report.json",
    ]));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestPlanningOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(report, expected);
}

#[test]
fn conforms_to_slice_131_canonical_manifest_source_family_paths() {
    let manifest = read_manifest();

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "typescript"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-101-typescript-family-feature-profile".to_string(),
                "typescript-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "rust"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-105-rust-family-feature-profile".to_string(),
                "rust-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "go"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-109-go-family-feature-profile".to_string(),
                "go-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "typescript", "analysis"),
        Some(
            &[
                "typescript".to_string(),
                "slice-102-analysis".to_string(),
                "module-owners.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "matching"),
        Some(
            &[
                "rust".to_string(),
                "slice-107-matching".to_string(),
                "path-equality.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "go", "merge"),
        Some(
            &["go".to_string(), "slice-112-merge".to_string(), "module-merge.json".to_string(),][..],
        )
    );
}

#[test]
fn conforms_to_source_family_review_state_fixtures() {
    for path in [
        &["diagnostics", "slice-158-source-family-review-state", "source-family-review-state.json"]
            [..],
        &[
            "diagnostics",
            "slice-159-source-family-reviewed-default",
            "source-family-reviewed-default.json",
        ][..],
        &[
            "diagnostics",
            "slice-160-source-family-replay-application",
            "source-family-replay-application.json",
        ][..],
    ] {
        let fixture = read_fixture_from_path(fixture_path(path));
        let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
            .expect("manifest should deserialize");
        let options =
            serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
                .expect("options should deserialize");
        let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
            fixture["expected_state"].clone(),
        )
        .expect("expected state should deserialize");
        let executions = fixture["executions"].as_object().expect("executions should be an object");

        let state = review_conformance_manifest(&manifest, &options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(state, expected_state);
    }
}

#[test]
fn conforms_to_canonical_widened_suite_fixtures() {
    let plans_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-162-canonical-widened-suite-plans",
        "canonical-widened-suite-plans.json",
    ]));
    let plans_manifest =
        serde_json::from_value::<ConformanceManifest>(plans_fixture["manifest"].clone())
            .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(plans_fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected_entries = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        plans_fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");
    assert_eq!(plan_named_conformance_suites(&plans_manifest, &contexts), expected_entries);

    let report_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-163-canonical-widened-suite-report",
        "canonical-widened-suite-report.json",
    ]));
    let report_manifest =
        serde_json::from_value::<ConformanceManifest>(report_fixture["manifest"].clone())
            .expect("manifest should deserialize");
    let report_options = serde_json::from_value::<ConformanceManifestPlanningOptions>(
        report_fixture["options"].clone(),
    )
    .expect("options should deserialize");
    let expected_report = serde_json::from_value::<ConformanceManifestReport>(
        report_fixture["expected_report"].clone(),
    )
    .expect("expected report should deserialize");
    let report_executions =
        report_fixture["executions"].as_object().expect("executions should be an object");

    let report = report_conformance_manifest(&report_manifest, &report_options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            report_executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });
    assert_eq!(report, expected_report);

    for path in [
        &[
            "diagnostics",
            "slice-164-canonical-widened-suite-review-state",
            "canonical-widened-suite-review-state.json",
        ][..],
        &[
            "diagnostics",
            "slice-165-canonical-widened-suite-reviewed-default",
            "canonical-widened-suite-reviewed-default.json",
        ][..],
        &[
            "diagnostics",
            "slice-166-canonical-widened-suite-replay-application",
            "canonical-widened-suite-replay-application.json",
        ][..],
    ] {
        let fixture = read_fixture_from_path(fixture_path(path));
        let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
            .expect("manifest should deserialize");
        let options =
            serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
                .expect("options should deserialize");
        let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
            fixture["expected_state"].clone(),
        )
        .expect("expected state should deserialize");
        let executions = fixture["executions"].as_object().expect("executions should be an object");

        let state = review_conformance_manifest(&manifest, &options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(state, expected_state);
    }
}

#[test]
fn conforms_to_backend_sensitive_aggregate_fixtures() {
    let plans_fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-167-backend-sensitive-aggregate-suite-plans",
        "backend-sensitive-aggregate-suite-plans.json",
    ]));
    let plans_manifest =
        serde_json::from_value::<ConformanceManifest>(plans_fixture["manifest"].clone())
            .expect("manifest should deserialize");
    let contexts = serde_json::from_value::<
        std::collections::HashMap<String, ConformanceFamilyPlanContext>,
    >(plans_fixture["contexts"].clone())
    .expect("contexts should deserialize");
    let expected_entries = serde_json::from_value::<Vec<NamedConformanceSuitePlan>>(
        plans_fixture["expected_entries"].clone(),
    )
    .expect("expected entries should deserialize");
    assert_eq!(plan_named_conformance_suites(&plans_manifest, &contexts), expected_entries);

    for path in [
        &[
            "diagnostics",
            "slice-168-backend-sensitive-aggregate-tree-sitter-report",
            "backend-sensitive-aggregate-tree-sitter-report.json",
        ][..],
        &[
            "diagnostics",
            "slice-169-backend-sensitive-aggregate-native-report",
            "backend-sensitive-aggregate-native-report.json",
        ][..],
    ] {
        let fixture = read_fixture_from_path(fixture_path(path));
        let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
            .expect("manifest should deserialize");
        let options = serde_json::from_value::<ConformanceManifestPlanningOptions>(
            fixture["options"].clone(),
        )
        .expect("options should deserialize");
        let expected_report =
            serde_json::from_value::<ConformanceManifestReport>(fixture["expected_report"].clone())
                .expect("expected report should deserialize");
        let executions = fixture["executions"].as_object().expect("executions should be an object");

        let report = report_conformance_manifest(&manifest, &options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(report, expected_report);
    }

    for path in [
        &[
            "diagnostics",
            "slice-192-backend-sensitive-aggregate-tree-sitter-review-state",
            "backend-sensitive-aggregate-tree-sitter-review-state.json",
        ][..],
        &[
            "diagnostics",
            "slice-193-backend-sensitive-aggregate-native-review-state",
            "backend-sensitive-aggregate-native-review-state.json",
        ][..],
    ] {
        let fixture = read_fixture_from_path(fixture_path(path));
        let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
            .expect("manifest should deserialize");
        let options =
            serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
                .expect("options should deserialize");
        let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
            fixture["expected_state"].clone(),
        )
        .expect("expected state should deserialize");
        let executions = fixture["executions"].as_object().expect("executions should be an object");

        let state = review_conformance_manifest(&manifest, &options, |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                ),
            )
            .expect("execution should deserialize")
        });

        assert_eq!(state, expected_state);
    }
}

#[test]
fn conforms_to_slice_61_review_host_hints_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_host_hints"));
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected = serde_json::from_value::<ReviewHostHints>(fixture["expected_hints"].clone())
        .expect("expected hints should deserialize");

    assert_eq!(conformance_review_host_hints(&options), expected);
}

#[test]
fn conforms_to_slice_62_family_context_review_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("family_context_review_request"));
    let family = fixture["family"].as_str().expect("family should be a string");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");
    let expected_request =
        serde_json::from_value::<ReviewRequest>(fixture["expected_request"].clone())
            .expect("expected request should deserialize");

    let (_context, diagnostics, requests, _applied_decisions) =
        review_conformance_family_context(family, &options);

    assert_eq!(review_request_id_for_family_context(family), expected_request.id);
    assert_eq!(diagnostics, vec![expected_diagnostic]);
    assert_eq!(requests, vec![expected_request]);
}

#[test]
fn conforms_to_slice_77_family_context_review_proposal_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("family_context_review_proposal"));
    let family = fixture["family"].as_str().expect("family should be a string");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_request =
        serde_json::from_value::<ReviewRequest>(fixture["expected_request"].clone())
            .expect("expected request should deserialize");

    let (_context, _diagnostics, requests, _applied_decisions) =
        review_conformance_family_context(family, &options);

    assert_eq!(requests, vec![expected_request]);
}

#[test]
fn conforms_to_slice_78_family_context_explicit_review_decision_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("family_context_explicit_review_decision"));
    let family = fixture["family"].as_str().expect("family should be a string");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_context =
        serde_json::from_value::<ConformanceFamilyPlanContext>(fixture["expected_context"].clone())
            .expect("expected context should deserialize");
    let expected_applied_decisions = serde_json::from_value::<Vec<ast_merge::ReviewDecision>>(
        fixture["expected_applied_decisions"].clone(),
    )
    .expect("expected applied decisions should deserialize");

    let (context, diagnostics, requests, applied_decisions) =
        review_conformance_family_context(family, &options);

    assert_eq!(context, Some(expected_context));
    assert_eq!(diagnostics, Vec::<ast_merge::Diagnostic>::new());
    assert_eq!(requests, Vec::<ReviewRequest>::new());
    assert_eq!(applied_decisions, expected_applied_decisions);
}

#[test]
fn conforms_to_slice_80_explicit_review_decision_payload_validation_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "explicit_review_decision_missing_context",
    ));
    let family = fixture["family"].as_str().expect("family should be a string");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");
    let expected_request =
        serde_json::from_value::<ReviewRequest>(fixture["expected_request"].clone())
            .expect("expected request should deserialize");

    let (context, diagnostics, requests, applied_decisions) =
        review_conformance_family_context(family, &options);

    assert_eq!(context, None);
    assert_eq!(diagnostics, vec![expected_diagnostic]);
    assert_eq!(requests, vec![expected_request]);
    assert_eq!(applied_decisions, Vec::<ast_merge::ReviewDecision>::new());
}

#[test]
fn conforms_to_slice_81_explicit_review_decision_family_validation_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "explicit_review_decision_family_mismatch",
    ));
    let family = fixture["family"].as_str().expect("family should be a string");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected_diagnostic =
        serde_json::from_value::<ast_merge::Diagnostic>(fixture["expected_diagnostic"].clone())
            .expect("expected diagnostic should deserialize");
    let expected_request =
        serde_json::from_value::<ReviewRequest>(fixture["expected_request"].clone())
            .expect("expected request should deserialize");

    let (context, diagnostics, requests, applied_decisions) =
        review_conformance_family_context(family, &options);

    assert_eq!(context, None);
    assert_eq!(diagnostics, vec![expected_diagnostic]);
    assert_eq!(requests, vec![expected_request]);
    assert_eq!(applied_decisions, Vec::<ast_merge::ReviewDecision>::new());
}

#[test]
fn conforms_to_slice_63_conformance_manifest_review_state_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("conformance_manifest_review_state"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let expected_replay_context = serde_json::from_value::<ReviewReplayContext>(
        fixture["expected_state"]["replay_context"].clone(),
    )
    .expect("expected replay context should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
    assert_eq!(conformance_manifest_replay_context(&manifest, &options), expected_replay_context);
}

#[test]
fn conforms_to_slice_64_reviewed_default_context_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("reviewed_default_context"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_65_review_replay_compatibility_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_replay_compatibility"));
    let current = serde_json::from_value::<ReviewReplayContext>(fixture["current_context"].clone())
        .expect("current context should deserialize");
    let compatible =
        serde_json::from_value::<ReviewReplayContext>(fixture["compatible_context"].clone())
            .expect("compatible context should deserialize");
    let incompatible =
        serde_json::from_value::<ReviewReplayContext>(fixture["incompatible_context"].clone())
            .expect("incompatible context should deserialize");

    assert!(review_replay_context_compatible(&current, Some(&compatible)));
    assert!(!review_replay_context_compatible(&current, Some(&incompatible)));
    assert!(!review_replay_context_compatible(&current, None));
}

#[test]
fn conforms_to_slice_66_review_replay_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_replay_rejection"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_67_review_request_ids_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_request_ids"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected = serde_json::from_value::<Vec<String>>(fixture["expected_request_ids"].clone())
        .expect("expected request ids should deserialize");

    assert_eq!(conformance_manifest_review_request_ids(&manifest, &options), expected);
}

#[test]
fn conforms_to_slice_68_stale_review_decision_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("stale_review_decision"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_69_review_replay_bundle_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_replay_bundle"));
    let bundle =
        serde_json::from_value::<ast_merge::ReviewReplayBundle>(fixture["replay_bundle"].clone())
            .expect("replay bundle should deserialize");
    let options = ConformanceManifestReviewOptions {
        contexts: std::collections::HashMap::new(),
        family_profiles: std::collections::HashMap::new(),
        require_explicit_contexts: false,
        review_decisions: Vec::new(),
        review_replay_context: None,
        review_replay_bundle: Some(bundle.clone()),
        interactive: false,
    };

    assert_eq!(
        review_replay_bundle_inputs(&options),
        (Some(bundle.replay_context), bundle.decisions, bundle.reviewed_nested_executions)
    );
}

#[test]
fn conforms_to_slice_305_review_replay_bundle_reviewed_nested_executions_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_reviewed_nested_executions",
    ));
    let bundle =
        serde_json::from_value::<ast_merge::ReviewReplayBundle>(fixture["replay_bundle"].clone())
            .expect("replay bundle should deserialize");
    let options = ConformanceManifestReviewOptions {
        contexts: std::collections::HashMap::new(),
        family_profiles: std::collections::HashMap::new(),
        require_explicit_contexts: false,
        review_decisions: Vec::new(),
        review_replay_context: None,
        review_replay_bundle: Some(bundle.clone()),
        interactive: false,
    };

    assert_eq!(
        review_replay_bundle_inputs(&options),
        (Some(bundle.replay_context), bundle.decisions, bundle.reviewed_nested_executions)
    );
}

#[test]
fn conforms_to_slice_70_review_replay_bundle_application_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("review_replay_bundle_application"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_79_explicit_review_replay_bundle_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "explicit_review_replay_bundle_application",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_209_surface_ownership_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("surface_ownership"));
    let surface = serde_json::from_value::<DiscoveredSurface>(fixture["surface"].clone())
        .expect("surface should deserialize");
    let round_tripped = serde_json::from_value::<DiscoveredSurface>(
        serde_json::to_value(&surface).expect("surface should serialize"),
    )
    .expect("surface should deserialize after roundtrip");

    assert_eq!(round_tripped, surface);
}

#[test]
fn conforms_to_slice_210_delegated_child_operation_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("delegated_child_operation"));
    let operation = serde_json::from_value::<DelegatedChildOperation>(fixture["operation"].clone())
        .expect("operation should deserialize");
    let round_tripped = serde_json::from_value::<DelegatedChildOperation>(
        serde_json::to_value(&operation).expect("operation should serialize"),
    )
    .expect("operation should deserialize after roundtrip");

    assert_eq!(round_tripped, operation);
}

#[test]
fn conforms_to_slice_211_projected_child_review_cases_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("projected_child_review_cases"));
    let cases = serde_json::from_value::<Vec<ProjectedChildReviewCase>>(fixture["cases"].clone())
        .expect("projected child review cases should deserialize");
    let round_tripped = serde_json::from_value::<Vec<ProjectedChildReviewCase>>(
        serde_json::to_value(&cases).expect("cases should serialize"),
    )
    .expect("cases should deserialize after roundtrip");

    assert_eq!(round_tripped, cases);
}

#[test]
fn conforms_to_slice_227_projected_child_review_groups_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("projected_child_review_groups"));
    let cases = serde_json::from_value::<Vec<ProjectedChildReviewCase>>(fixture["cases"].clone())
        .expect("projected child review cases should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        fixture["expected_groups"].clone(),
    )
    .expect("projected child review groups should deserialize");

    assert_eq!(group_projected_child_review_cases(&cases), expected);
}

#[test]
fn conforms_to_slice_230_projected_child_review_group_progress_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("projected_child_review_group_progress"));
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("projected child review groups should deserialize");
    let resolved_case_ids =
        serde_json::from_value::<Vec<String>>(fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroupProgress>>(
        fixture["expected_progress"].clone(),
    )
    .expect("projected child review group progress should deserialize");

    assert_eq!(
        summarize_projected_child_review_group_progress(&groups, &resolved_case_ids),
        expected
    );
}

#[test]
fn conforms_to_slice_233_projected_child_review_groups_ready_for_apply_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "projected_child_review_groups_ready_for_apply",
    ));
    let groups =
        serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(fixture["groups"].clone())
            .expect("projected child review groups should deserialize");
    let resolved_case_ids =
        serde_json::from_value::<Vec<String>>(fixture["resolved_case_ids"].clone())
            .expect("resolved case ids should deserialize");
    let expected = serde_json::from_value::<Vec<ProjectedChildReviewGroup>>(
        fixture["expected_ready_groups"].clone(),
    )
    .expect("ready projected child review groups should deserialize");

    assert_eq!(
        select_projected_child_review_groups_ready_for_apply(&groups, &resolved_case_ids),
        expected
    );
}

#[test]
fn conforms_to_slice_236_delegated_child_group_review_request_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("delegated_child_group_review_request"));
    let family = fixture["family"].as_str().expect("family should be a string");
    let group = serde_json::from_value::<ProjectedChildReviewGroup>(fixture["group"].clone())
        .expect("group should deserialize");
    let expected = serde_json::from_value::<ReviewRequest>(fixture["expected_request"].clone())
        .expect("expected request should deserialize");

    assert_eq!(review_request_id_for_projected_child_group(&group), expected.id);
    assert_eq!(projected_child_group_review_request(&group, family), expected);
}

#[test]
fn conforms_to_slice_237_delegated_child_groups_accepted_for_apply_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "delegated_child_groups_accepted_for_apply",
    ));
    let family = fixture["family"].as_str().expect("family should be a string");
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

    assert_eq!(
        select_projected_child_review_groups_accepted_for_apply(&groups, family, &decisions),
        expected
    );
}

#[test]
fn conforms_to_slice_240_delegated_child_group_review_state_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("delegated_child_group_review_state"));
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
fn conforms_to_slice_243_delegated_child_apply_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("delegated_child_apply_plan"));
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
fn conforms_to_slice_292_delegated_child_nested_output_resolution_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "delegated_child_nested_output_resolution",
    ));
    let operations = serde_json::from_value::<Vec<ast_merge::DelegatedChildOperation>>(
        fixture["operations"].clone(),
    )
    .expect("operations should deserialize");
    let nested_outputs = serde_json::from_value::<Vec<ast_merge::DelegatedChildSurfaceOutput>>(
        fixture["nested_outputs"].clone(),
    )
    .expect("nested outputs should deserialize");
    let options = ast_merge::DelegatedChildOutputResolutionOptions {
        default_family: fixture["default_family"]
            .as_str()
            .expect("default family should be a string")
            .to_string(),
        request_id_prefix: fixture["request_id_prefix"]
            .as_str()
            .expect("request id prefix should be a string")
            .to_string(),
    };
    let expected = serde_json::from_value::<ast_merge::DelegatedChildOutputResolution>(
        fixture["expected"].clone(),
    )
    .expect("expected resolution should deserialize");

    assert_eq!(resolve_delegated_child_outputs(&operations, &nested_outputs, &options), expected);
}

#[test]
fn conforms_to_slice_293_delegated_child_nested_output_rejection_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("delegated_child_nested_output_rejection"));
    let operations = serde_json::from_value::<Vec<ast_merge::DelegatedChildOperation>>(
        fixture["operations"].clone(),
    )
    .expect("operations should deserialize");
    let nested_outputs = serde_json::from_value::<Vec<ast_merge::DelegatedChildSurfaceOutput>>(
        fixture["nested_outputs"].clone(),
    )
    .expect("nested outputs should deserialize");
    let options = ast_merge::DelegatedChildOutputResolutionOptions {
        default_family: fixture["default_family"]
            .as_str()
            .expect("default family should be a string")
            .to_string(),
        request_id_prefix: fixture["request_id_prefix"]
            .as_str()
            .expect("request id prefix should be a string")
            .to_string(),
    };
    let expected = serde_json::from_value::<ast_merge::DelegatedChildOutputResolution>(
        fixture["expected"].clone(),
    )
    .expect("expected rejection should deserialize");

    assert_eq!(resolve_delegated_child_outputs(&operations, &nested_outputs, &options), expected);
}

#[test]
fn conforms_to_slice_71_review_state_json_roundtrip_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_state_json_roundtrip"));
    let state = serde_json::from_value::<ConformanceManifestReviewState>(fixture["state"].clone())
        .expect("state should deserialize");

    let round_tripped: ConformanceManifestReviewState =
        serde_json::from_str(&serde_json::to_string(&state).expect("state should serialize"))
            .expect("state should deserialize after roundtrip");

    assert_eq!(round_tripped, state);
}

#[test]
fn conforms_to_slice_72_review_replay_bundle_json_roundtrip_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("review_replay_bundle_json_roundtrip"));
    let bundle =
        serde_json::from_value::<ast_merge::ReviewReplayBundle>(fixture["replay_bundle"].clone())
            .expect("replay bundle should deserialize");

    let round_tripped: ast_merge::ReviewReplayBundle =
        serde_json::from_str(&serde_json::to_string(&bundle).expect("bundle should serialize"))
            .expect("bundle should deserialize after roundtrip");

    assert_eq!(round_tripped, bundle);
}

#[test]
fn conforms_to_slice_73_review_state_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_state_envelope"));
    let state = serde_json::from_value::<ConformanceManifestReviewState>(fixture["state"].clone())
        .expect("state should deserialize");
    let expected = serde_json::from_value::<ConformanceManifestReviewStateEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("expected envelope should deserialize");

    assert_eq!(conformance_manifest_review_state_envelope(&state), expected);
    assert_eq!(import_conformance_manifest_review_state_envelope(&expected), Ok(state));
    assert_eq!(expected.version, REVIEW_TRANSPORT_VERSION);
}

#[test]
fn conforms_to_slice_74_review_replay_bundle_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("review_replay_bundle_envelope"));
    let bundle = serde_json::from_value::<ReviewReplayBundle>(fixture["replay_bundle"].clone())
        .expect("bundle should deserialize");
    let expected =
        serde_json::from_value::<ReviewReplayBundleEnvelope>(fixture["expected_envelope"].clone())
            .expect("expected envelope should deserialize");

    assert_eq!(review_replay_bundle_envelope(&bundle), expected);
    assert_eq!(import_review_replay_bundle_envelope(&expected), Ok(bundle));
    assert_eq!(expected.version, REVIEW_TRANSPORT_VERSION);
}

#[test]
fn conforms_to_slice_75_review_state_transport_rejection_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("review_state_envelope_rejection"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<ConformanceManifestReviewStateEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected: ast_merge::ReviewTransportImportError =
            serde_json::from_value(case["expected_error"].clone())
                .expect("expected error should deserialize");

        assert_eq!(import_conformance_manifest_review_state_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_76_review_replay_bundle_transport_rejection_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("review_replay_bundle_envelope_rejection"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<ReviewReplayBundleEnvelope>(case["envelope"].clone())
                .expect("envelope should deserialize");
        let expected: ast_merge::ReviewTransportImportError =
            serde_json::from_value(case["expected_error"].clone())
                .expect("expected error should deserialize");

        assert_eq!(import_review_replay_bundle_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_300_reviewed_nested_execution_json_roundtrip_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "reviewed_nested_execution_json_roundtrip",
    ));
    let execution = serde_json::from_value::<ReviewedNestedExecution>(fixture["execution"].clone())
        .expect("reviewed nested execution should deserialize");

    let round_tripped: ReviewedNestedExecution = serde_json::from_str(
        &serde_json::to_string(&execution).expect("execution should serialize"),
    )
    .expect("execution should deserialize after roundtrip");

    assert_eq!(round_tripped, execution);
}

#[test]
fn conforms_to_slice_301_reviewed_nested_execution_transport_envelope_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("reviewed_nested_execution_envelope"));
    let execution = serde_json::from_value::<ReviewedNestedExecution>(fixture["execution"].clone())
        .expect("reviewed nested execution should deserialize");
    let expected = serde_json::from_value::<ReviewedNestedExecutionEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("expected reviewed nested execution envelope should deserialize");

    assert_eq!(reviewed_nested_execution_envelope(&execution), expected);
    assert_eq!(import_reviewed_nested_execution_envelope(&expected), Ok(execution));
    assert_eq!(expected.version, REVIEW_TRANSPORT_VERSION);
}

#[test]
fn conforms_to_slice_302_reviewed_nested_execution_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "reviewed_nested_execution_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<ReviewedNestedExecutionEnvelope>(case["envelope"].clone())
                .expect("envelope should deserialize");
        let expected: ast_merge::ReviewTransportImportError =
            serde_json::from_value(case["expected_error"].clone())
                .expect("expected error should deserialize");

        assert_eq!(import_reviewed_nested_execution_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_303_reviewed_nested_execution_payload_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("reviewed_nested_execution_payload"));
    let review_state = serde_json::from_value::<ast_merge::DelegatedChildGroupReviewState>(
        fixture["review_state"].clone(),
    )
    .expect("review state should deserialize");
    let applied_children = serde_json::from_value::<Vec<ast_merge::AppliedDelegatedChildOutput>>(
        fixture["applied_children"].clone(),
    )
    .expect("applied children should deserialize");
    let expected =
        serde_json::from_value::<ReviewedNestedExecution>(fixture["expected_execution"].clone())
            .expect("expected reviewed nested execution should deserialize");

    assert_eq!(
        reviewed_nested_execution(
            fixture["family"].as_str().expect("family should be a string"),
            &review_state,
            &applied_children,
        ),
        expected
    );
}

#[test]
fn conforms_to_slice_306_review_state_reviewed_nested_executions_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("review_state_reviewed_nested_executions"));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest(&manifest, &options, |run| {
        let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
        serde_json::from_value::<ConformanceCaseExecution>(
            executions.get(&key).cloned().unwrap_or_else(
                || serde_json::json!({ "outcome": "failed", "messages": ["missing execution"] }),
            ),
        )
        .expect("execution should deserialize")
    });

    assert_eq!(state, expected);
}
