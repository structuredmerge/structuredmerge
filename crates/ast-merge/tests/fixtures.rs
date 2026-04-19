use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceCaseExecution, ConformanceCaseRef, ConformanceCaseRequirements,
    ConformanceCaseResult, ConformanceCaseRun, ConformanceFamilyPlanContext,
    ConformanceFeatureProfileView, ConformanceManifest, ConformanceOutcome,
    ConformanceSelectionStatus, ConformanceSuitePlan, ConformanceSuiteReport,
    ConformanceSuiteSummary, DiagnosticCategory, DiagnosticSeverity, FamilyFeatureProfile,
    NamedConformanceSuitePlan, NamedConformanceSuiteReport, NamedConformanceSuiteReportEnvelope,
    NamedConformanceSuiteResults, PolicySurface, conformance_family_feature_profile_path,
    conformance_fixture_path, conformance_suite_definition, conformance_suite_names,
    plan_conformance_suite, plan_named_conformance_suite, plan_named_conformance_suite_entry,
    plan_named_conformance_suites, report_conformance_suite, report_named_conformance_suite,
    report_named_conformance_suite_entry, report_named_conformance_suite_envelope,
    report_named_conformance_suite_manifest, report_planned_conformance_suite,
    report_planned_named_conformance_suites, run_conformance_case, run_conformance_suite,
    run_named_conformance_suite, run_named_conformance_suite_entry, run_planned_conformance_suite,
    run_planned_named_conformance_suites, select_conformance_case, summarize_conformance_results,
    summarize_named_conformance_suite_reports,
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
        match DiagnosticCategory::ParseError {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::DestinationParseError {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::UnsupportedFeature {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::FallbackApplied {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
        match DiagnosticCategory::Ambiguity {
            DiagnosticCategory::ParseError => "parse_error",
            DiagnosticCategory::DestinationParseError => "destination_parse_error",
            DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
            DiagnosticCategory::FallbackApplied => "fallback_applied",
            DiagnosticCategory::Ambiguity => "ambiguity",
        },
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
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
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

    assert_eq!(conformance_suite_definition(&manifest, suite_name), Some(&expected));
    assert_eq!(
        plan_named_conformance_suite(&manifest, suite_name, &family_profile, None),
        Some(plan_conformance_suite(
            &manifest,
            &expected.family,
            &expected.roles,
            &family_profile,
            None,
        )),
    );
}

#[test]
fn conforms_to_slice_44_named_conformance_suite_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_report"));
    let manifest = read_manifest();
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
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
        suite_name,
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
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
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
        suite_name,
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
    let expected = fixture["suite_names"]
        .as_array()
        .expect("suite_names should be an array")
        .iter()
        .map(|value| value.as_str().expect("suite name should be a string").to_string())
        .collect::<Vec<_>>();

    assert_eq!(conformance_suite_names(&manifest), expected);
}

#[test]
fn conforms_to_slice_47_named_conformance_suite_entry_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_entry"));
    let manifest = read_manifest();
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
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
        suite_name,
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
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
    let context =
        serde_json::from_value::<ConformanceFamilyPlanContext>(fixture["context"].clone())
            .expect("context should deserialize");
    let expected =
        serde_json::from_value::<NamedConformanceSuitePlan>(fixture["expected_entry"].clone())
            .expect("expected entry should deserialize");

    assert_eq!(plan_named_conformance_suite_entry(&manifest, suite_name, &context), Some(expected),);
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
fn conforms_to_slice_51_named_conformance_suite_results_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("named_suite_results"));
    let manifest = read_manifest();
    let suite_name = fixture["suite_name"].as_str().expect("suite name should be a string");
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
        suite_name,
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
