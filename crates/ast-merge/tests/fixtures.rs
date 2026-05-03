use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use ast_merge::{
    ConformanceCaseExecution, ConformanceCaseRef, ConformanceCaseRequirements,
    ConformanceCaseResult, ConformanceCaseRun, ConformanceFamilyPlanContext,
    ConformanceFeatureProfileView, ConformanceManifest, ConformanceManifestPlanningOptions,
    ConformanceManifestReport, ConformanceManifestReviewOptions, ConformanceManifestReviewState,
    ConformanceManifestReviewStateEnvelope, ConformanceManifestReviewedNestedApplication,
    ConformanceOutcome, ConformanceSelectionStatus, ConformanceSuiteDefinition,
    ConformanceSuitePlan, ConformanceSuiteReport, ConformanceSuiteSelector,
    ConformanceSuiteSubject, ConformanceSuiteSummary, DelegatedChildOperation, DiagnosticCategory,
    DiagnosticSeverity, DiscoveredSurface, FamilyFeatureProfile, NamedConformanceSuitePlan,
    NamedConformanceSuiteReport, NamedConformanceSuiteReportEnvelope, NamedConformanceSuiteResults,
    PolicySurface, ProjectedChildReviewCase, ProjectedChildReviewGroup,
    ProjectedChildReviewGroupProgress, REVIEW_TRANSPORT_VERSION, ReviewHostHints,
    ReviewReplayBundle, ReviewReplayBundleEnvelope, ReviewReplayContext, ReviewRequest,
    ReviewedNestedExecution, ReviewedNestedExecutionEnvelope, StructuredEditApplication,
    StructuredEditApplicationEnvelope, StructuredEditBatchReport,
    StructuredEditBatchReportEnvelope, StructuredEditBatchRequest,
    StructuredEditDestinationProfile, StructuredEditExecutionReport,
    StructuredEditExecutionReportEnvelope, StructuredEditMatchProfile,
    StructuredEditOperationProfile, StructuredEditProviderBatchExecutionDispatch,
    StructuredEditProviderBatchExecutionDispatchEnvelope,
    StructuredEditProviderBatchExecutionHandoff,
    StructuredEditProviderBatchExecutionHandoffEnvelope,
    StructuredEditProviderBatchExecutionInvocation,
    StructuredEditProviderBatchExecutionInvocationEnvelope,
    StructuredEditProviderBatchExecutionOutcome,
    StructuredEditProviderBatchExecutionOutcomeEnvelope, StructuredEditProviderBatchExecutionPlan,
    StructuredEditProviderBatchExecutionPlanEnvelope,
    StructuredEditProviderBatchExecutionProvenance,
    StructuredEditProviderBatchExecutionProvenanceEnvelope,
    StructuredEditProviderBatchExecutionReceipt,
    StructuredEditProviderBatchExecutionReceiptEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayApplication,
    StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayRequest,
    StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplaySession,
    StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecision,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcome,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequest,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequestEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResult,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResultEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySession,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySessionEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowResult,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowResultEnvelope,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequest,
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequestEnvelope,
    StructuredEditProviderBatchExecutionReplayBundle,
    StructuredEditProviderBatchExecutionReplayBundleEnvelope,
    StructuredEditProviderBatchExecutionReport, StructuredEditProviderBatchExecutionReportEnvelope,
    StructuredEditProviderBatchExecutionRequest,
    StructuredEditProviderBatchExecutionRequestEnvelope,
    StructuredEditProviderBatchExecutionRunResult,
    StructuredEditProviderBatchExecutionRunResultEnvelope,
    StructuredEditProviderExecutionApplication, StructuredEditProviderExecutionApplicationEnvelope,
    StructuredEditProviderExecutionDispatch, StructuredEditProviderExecutionDispatchEnvelope,
    StructuredEditProviderExecutionHandoff, StructuredEditProviderExecutionHandoffEnvelope,
    StructuredEditProviderExecutionInvocation, StructuredEditProviderExecutionInvocationEnvelope,
    StructuredEditProviderExecutionOutcome, StructuredEditProviderExecutionOutcomeEnvelope,
    StructuredEditProviderExecutionPlan, StructuredEditProviderExecutionPlanEnvelope,
    StructuredEditProviderExecutionProvenance, StructuredEditProviderExecutionProvenanceEnvelope,
    StructuredEditProviderExecutionReceipt, StructuredEditProviderExecutionReceiptEnvelope,
    StructuredEditProviderExecutionReceiptReplayApplication,
    StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
    StructuredEditProviderExecutionReceiptReplayRequest,
    StructuredEditProviderExecutionReceiptReplayRequestEnvelope,
    StructuredEditProviderExecutionReceiptReplaySession,
    StructuredEditProviderExecutionReceiptReplaySessionEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflow,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecision,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcome,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlement,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlementEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequest,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequestEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyResult,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplyResultEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplySession,
    StructuredEditProviderExecutionReceiptReplayWorkflowApplySessionEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowResult,
    StructuredEditProviderExecutionReceiptReplayWorkflowResultEnvelope,
    StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequest,
    StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequestEnvelope,
    StructuredEditProviderExecutionReplayBundle,
    StructuredEditProviderExecutionReplayBundleEnvelope, StructuredEditProviderExecutionRequest,
    StructuredEditProviderExecutionRequestEnvelope, StructuredEditProviderExecutionRunResult,
    StructuredEditProviderExecutionRunResultEnvelope, StructuredEditProviderExecutorProfile,
    StructuredEditProviderExecutorProfileEnvelope, StructuredEditProviderExecutorRegistry,
    StructuredEditProviderExecutorRegistryEnvelope, StructuredEditProviderExecutorResolution,
    StructuredEditProviderExecutorResolutionEnvelope,
    StructuredEditProviderExecutorSelectionPolicy,
    StructuredEditProviderExecutorSelectionPolicyEnvelope, StructuredEditRequest,
    StructuredEditResult, StructuredEditSelectionProfile, StructuredEditStructureProfile,
    StructuredEditTransportImportError, TemplateApplyResult, TemplateConvergenceResult,
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplatePlanEntry,
    TemplatePlanStateEntry, TemplatePlanTokenStateEntry, TemplatePreparedEntry,
    TemplatePreviewResult, TemplateStrategy, TemplateStrategyOverride, TemplateTokenConfig,
    TemplateTreeRunReport, TemplateTreeRunResult, apply_template_execution,
    classify_template_target_path, conformance_family_feature_profile_path,
    conformance_fixture_path, conformance_manifest_replay_context,
    conformance_manifest_review_request_ids, conformance_manifest_review_state_envelope,
    conformance_review_host_hints, conformance_suite_definition, conformance_suite_selectors,
    default_conformance_family_context, delegated_child_apply_plan, enrich_template_plan_entries,
    enrich_template_plan_entries_with_token_state, evaluate_template_tree_convergence,
    execute_review_replay_bundle_envelope_reviewed_nested_executions,
    execute_review_replay_bundle_reviewed_nested_executions,
    execute_review_state_envelope_reviewed_nested_executions,
    execute_review_state_reviewed_nested_executions, group_projected_child_review_cases,
    import_conformance_manifest_review_state_envelope, import_review_replay_bundle_envelope,
    import_reviewed_nested_execution_envelope, import_structured_edit_application_envelope,
    import_structured_edit_batch_report_envelope, import_structured_edit_execution_report_envelope,
    import_structured_edit_provider_batch_execution_dispatch_envelope,
    import_structured_edit_provider_batch_execution_handoff_envelope,
    import_structured_edit_provider_batch_execution_invocation_envelope,
    import_structured_edit_provider_batch_execution_outcome_envelope,
    import_structured_edit_provider_batch_execution_plan_envelope,
    import_structured_edit_provider_batch_execution_provenance_envelope,
    import_structured_edit_provider_batch_execution_receipt_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_application_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_request_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_session_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope,
    import_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope,
    import_structured_edit_provider_batch_execution_replay_bundle_envelope,
    import_structured_edit_provider_batch_execution_report_envelope,
    import_structured_edit_provider_batch_execution_request_envelope,
    import_structured_edit_provider_batch_execution_run_result_envelope,
    import_structured_edit_provider_execution_application_envelope,
    import_structured_edit_provider_execution_dispatch_envelope,
    import_structured_edit_provider_execution_handoff_envelope,
    import_structured_edit_provider_execution_invocation_envelope,
    import_structured_edit_provider_execution_outcome_envelope,
    import_structured_edit_provider_execution_plan_envelope,
    import_structured_edit_provider_execution_provenance_envelope,
    import_structured_edit_provider_execution_receipt_envelope,
    import_structured_edit_provider_execution_receipt_replay_application_envelope,
    import_structured_edit_provider_execution_receipt_replay_request_envelope,
    import_structured_edit_provider_execution_receipt_replay_session_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_result_envelope,
    import_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope,
    import_structured_edit_provider_execution_replay_bundle_envelope,
    import_structured_edit_provider_execution_request_envelope,
    import_structured_edit_provider_execution_run_result_envelope,
    import_structured_edit_provider_executor_profile_envelope,
    import_structured_edit_provider_executor_registry_envelope,
    import_structured_edit_provider_executor_resolution_envelope,
    import_structured_edit_provider_executor_selection_policy_envelope,
    normalize_template_source_path, plan_conformance_suite, plan_named_conformance_suite,
    plan_named_conformance_suite_entry, plan_named_conformance_suites,
    plan_named_conformance_suites_with_diagnostics, plan_template_entries, plan_template_execution,
    plan_template_tree_execution, prepare_template_entries, preview_template_execution,
    projected_child_group_review_request, report_conformance_manifest, report_conformance_suite,
    report_named_conformance_suite, report_named_conformance_suite_entry,
    report_named_conformance_suite_envelope, report_named_conformance_suite_manifest,
    report_planned_conformance_suite, report_planned_named_conformance_suites,
    report_template_directory_apply, report_template_directory_plan,
    report_template_directory_runner, report_template_tree_run, resolve_conformance_family_context,
    resolve_delegated_child_outputs, resolve_template_destination_path,
    review_and_execute_conformance_manifest_with_replay_bundle_envelope,
    review_conformance_family_context, review_conformance_manifest,
    review_conformance_manifest_with_replay_bundle_envelope, review_projected_child_groups,
    review_replay_bundle_envelope, review_replay_bundle_inputs, review_replay_context_compatible,
    review_request_id_for_family_context, review_request_id_for_projected_child_group,
    reviewed_nested_execution, reviewed_nested_execution_envelope, run_conformance_case,
    run_conformance_suite, run_named_conformance_suite, run_named_conformance_suite_entry,
    run_planned_conformance_suite, run_planned_named_conformance_suites,
    run_template_tree_execution, select_conformance_case,
    select_projected_child_review_groups_accepted_for_apply,
    select_projected_child_review_groups_ready_for_apply, select_template_strategy,
    structured_edit_application_envelope, structured_edit_batch_report_envelope,
    structured_edit_execution_report_envelope,
    structured_edit_provider_batch_execution_dispatch_envelope,
    structured_edit_provider_batch_execution_handoff_envelope,
    structured_edit_provider_batch_execution_invocation_envelope,
    structured_edit_provider_batch_execution_outcome_envelope,
    structured_edit_provider_batch_execution_plan_envelope,
    structured_edit_provider_batch_execution_provenance_envelope,
    structured_edit_provider_batch_execution_receipt_envelope,
    structured_edit_provider_batch_execution_receipt_replay_application_envelope,
    structured_edit_provider_batch_execution_receipt_replay_request_envelope,
    structured_edit_provider_batch_execution_receipt_replay_session_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope,
    structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope,
    structured_edit_provider_batch_execution_replay_bundle_envelope,
    structured_edit_provider_batch_execution_report_envelope,
    structured_edit_provider_batch_execution_request_envelope,
    structured_edit_provider_batch_execution_run_result_envelope,
    structured_edit_provider_execution_application_envelope,
    structured_edit_provider_execution_dispatch_envelope,
    structured_edit_provider_execution_handoff_envelope,
    structured_edit_provider_execution_invocation_envelope,
    structured_edit_provider_execution_outcome_envelope,
    structured_edit_provider_execution_plan_envelope,
    structured_edit_provider_execution_provenance_envelope,
    structured_edit_provider_execution_receipt_envelope,
    structured_edit_provider_execution_receipt_replay_application_envelope,
    structured_edit_provider_execution_receipt_replay_request_envelope,
    structured_edit_provider_execution_receipt_replay_session_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_result_envelope,
    structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope,
    structured_edit_provider_execution_replay_bundle_envelope,
    structured_edit_provider_execution_request_envelope,
    structured_edit_provider_execution_run_result_envelope,
    structured_edit_provider_executor_profile_envelope,
    structured_edit_provider_executor_registry_envelope,
    structured_edit_provider_executor_resolution_envelope,
    structured_edit_provider_executor_selection_policy_envelope, summarize_conformance_results,
    summarize_named_conformance_suite_reports, summarize_projected_child_review_group_progress,
    template_token_keys,
};
use markdown_merge::{MarkdownDialect, merge_markdown};
use ruby_merge::{RubyDialect, merge_ruby};
use serde_json::Value;
use toml_merge::{TomlDialect, merge_toml};

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

fn prune_empty_metadata(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for nested in map.values_mut() {
                prune_empty_metadata(nested);
            }
            if matches!(map.get("metadata"), Some(Value::Object(metadata)) if metadata.is_empty()) {
                map.remove("metadata");
            }
        }
        Value::Array(items) => {
            for item in items {
                prune_empty_metadata(item);
            }
        }
        _ => {}
    }
}

fn read_relative_file_tree(root: &Path) -> HashMap<String, String> {
    let mut files = HashMap::new();

    fn walk(root: &Path, current: &Path, files: &mut HashMap<String, String>) {
        for entry in fs::read_dir(current).expect("tree directory should be readable") {
            let entry = entry.expect("tree entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, files);
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .expect("path should be under root")
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&path).expect("tree file should be readable");
            files.insert(relative_path, content);
        }
    }

    walk(root, root, &mut files);
    files
}

fn multi_family_merge_callback(
    entry: &TemplateExecutionPlanEntry,
) -> ast_merge::MergeResult<String> {
    match entry.classification.family.as_str() {
        "markdown" => merge_markdown(
            entry.prepared_template_content.as_ref().expect("prepared content should exist"),
            entry.destination_content.as_ref().expect("destination content should exist"),
            MarkdownDialect::Markdown,
        ),
        "toml" => merge_toml(
            entry.prepared_template_content.as_ref().expect("prepared content should exist"),
            entry.destination_content.as_ref().expect("destination content should exist"),
            TomlDialect::Toml,
            None,
        ),
        "ruby" => merge_ruby(
            entry.prepared_template_content.as_ref().expect("prepared content should exist"),
            entry.destination_content.as_ref().expect("destination content should exist"),
            RubyDialect::Ruby,
        ),
        family => ast_merge::MergeResult {
            ok: false,
            diagnostics: vec![ast_merge::Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::ConfigurationError,
                message: format!("missing family merge adapter for {family}"),
                path: None,
                review: None,
            }],
            output: None,
            policies: vec![],
        },
    }
}

fn repo_temp_dir() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("tmp");
    fs::create_dir_all(&base).expect("tmp root should be creatable");
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let path = base.join(format!("ast-merge-{suffix}"));
    fs::create_dir_all(&path).expect("temp dir should be creatable");
    path
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
        DiagnosticCategory::KindMismatch => "kind_mismatch",
        DiagnosticCategory::UnsupportedVersion => "unsupported_version",
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
fn conforms_to_template_source_path_mapping_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_source_path_mapping"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for entry in cases {
        let template_source_path = entry["template_source_path"]
            .as_str()
            .expect("template_source_path should be a string");
        let expected_destination_path = entry["expected_destination_path"]
            .as_str()
            .expect("expected_destination_path should be a string");

        assert_eq!(normalize_template_source_path(template_source_path), expected_destination_path);
    }
}

#[test]
fn conforms_to_template_target_classification_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("template_target_classification"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for entry in cases {
        let destination_path =
            entry["destination_path"].as_str().expect("destination_path should be a string");
        let actual = serde_json::to_value(classify_template_target_path(destination_path))
            .expect("classification should serialize");

        assert_eq!(actual, entry["expected"]);
    }
}

#[test]
fn conforms_to_template_destination_mapping_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_destination_mapping"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for entry in cases {
        let logical_destination_path = entry["logical_destination_path"]
            .as_str()
            .expect("logical_destination_path should be a string");
        let context =
            serde_json::from_value::<TemplateDestinationContext>(entry["context"].clone())
                .expect("context should deserialize");
        let actual = resolve_template_destination_path(logical_destination_path, &context);
        let expected = if entry["expected_destination_path"].is_null() {
            None
        } else {
            Some(
                entry["expected_destination_path"]
                    .as_str()
                    .expect("expected_destination_path should be a string")
                    .to_string(),
            )
        };

        assert_eq!(actual, expected);
    }
}

#[test]
fn conforms_to_template_strategy_selection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_strategy_selection"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for entry in cases {
        let destination_path =
            entry["destination_path"].as_str().expect("destination_path should be a string");
        let default_strategy =
            serde_json::from_value::<TemplateStrategy>(entry["default_strategy"].clone())
                .expect("default_strategy should deserialize");
        let overrides =
            serde_json::from_value::<Vec<TemplateStrategyOverride>>(entry["overrides"].clone())
                .expect("overrides should deserialize");
        let expected_strategy =
            serde_json::from_value::<TemplateStrategy>(entry["expected_strategy"].clone())
                .expect("expected_strategy should deserialize");

        assert_eq!(
            select_template_strategy(destination_path, default_strategy, &overrides),
            expected_strategy
        );
    }
}

#[test]
fn conforms_to_template_entry_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_entry_plan"));
    let template_source_paths =
        serde_json::from_value::<Vec<String>>(fixture["template_source_paths"].clone())
            .expect("template_source_paths should deserialize");
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let expected =
        serde_json::from_value::<Vec<TemplatePlanEntry>>(fixture["expected_entries"].clone())
            .expect("expected_entries should deserialize");

    assert_eq!(
        plan_template_entries(&template_source_paths, &context, default_strategy, &overrides),
        expected
    );
}

#[test]
fn conforms_to_template_entry_plan_state_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_entry_plan_state"));
    let planned_entries =
        serde_json::from_value::<Vec<TemplatePlanEntry>>(fixture["planned_entries"].clone())
            .expect("planned_entries should deserialize");
    let existing_destination_paths =
        serde_json::from_value::<Vec<String>>(fixture["existing_destination_paths"].clone())
            .expect("existing_destination_paths should deserialize");
    let expected =
        serde_json::from_value::<Vec<TemplatePlanStateEntry>>(fixture["expected_entries"].clone())
            .expect("expected_entries should deserialize");

    assert_eq!(
        enrich_template_plan_entries(&planned_entries, &existing_destination_paths),
        expected
    );
}

#[test]
fn conforms_to_template_token_keys_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_token_keys"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for entry in cases {
        let content = entry["content"].as_str().expect("content should be a string");
        let config = entry
            .get("config")
            .map(|raw| {
                serde_json::from_value::<TemplateTokenConfig>(raw.clone())
                    .expect("config should deserialize")
            })
            .unwrap_or_else(ast_merge::default_template_token_config);
        let expected = serde_json::from_value::<Vec<String>>(entry["expected_token_keys"].clone())
            .expect("expected_token_keys should deserialize");

        assert_eq!(template_token_keys(content, &config), expected);
    }
}

#[test]
fn conforms_to_template_entry_token_state_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_entry_token_state"));
    let planned_entries =
        serde_json::from_value::<Vec<TemplatePlanStateEntry>>(fixture["planned_entries"].clone())
            .expect("planned_entries should deserialize");
    let template_contents = serde_json::from_value::<std::collections::HashMap<String, String>>(
        fixture["template_contents"].clone(),
    )
    .expect("template_contents should deserialize");
    let replacements = serde_json::from_value::<std::collections::HashMap<String, String>>(
        fixture["replacements"].clone(),
    )
    .expect("replacements should deserialize");
    let expected = serde_json::from_value::<Vec<TemplatePlanTokenStateEntry>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected_entries should deserialize");

    assert_eq!(
        enrich_template_plan_entries_with_token_state(
            &planned_entries,
            &template_contents,
            &replacements,
            &ast_merge::default_template_token_config(),
        ),
        expected
    );
}

#[test]
fn conforms_to_template_entry_prepared_content_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("template_entry_prepared_content"));
    let planned_entries = serde_json::from_value::<Vec<TemplatePlanTokenStateEntry>>(
        fixture["planned_entries"].clone(),
    )
    .expect("planned_entries should deserialize");
    let template_contents = serde_json::from_value::<std::collections::HashMap<String, String>>(
        fixture["template_contents"].clone(),
    )
    .expect("template_contents should deserialize");
    let replacements = serde_json::from_value::<std::collections::HashMap<String, String>>(
        fixture["replacements"].clone(),
    )
    .expect("replacements should deserialize");
    let expected =
        serde_json::from_value::<Vec<TemplatePreparedEntry>>(fixture["expected_entries"].clone())
            .expect("expected_entries should deserialize");

    assert_eq!(
        prepare_template_entries(
            &planned_entries,
            &template_contents,
            &replacements,
            &ast_merge::default_template_token_config(),
        ),
        expected
    );
}

#[test]
fn conforms_to_template_execution_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("template_execution_plan"));
    let prepared_entries =
        serde_json::from_value::<Vec<TemplatePreparedEntry>>(fixture["prepared_entries"].clone())
            .expect("prepared_entries should deserialize");
    let destination_contents = serde_json::from_value::<std::collections::HashMap<String, String>>(
        fixture["destination_contents"].clone(),
    )
    .expect("destination_contents should deserialize");
    let expected = serde_json::from_value::<Vec<TemplateExecutionPlanEntry>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected_entries should deserialize");

    assert_eq!(plan_template_execution(&prepared_entries, &destination_contents), expected);
}

#[test]
fn conforms_to_mini_template_tree_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let expected = serde_json::from_value::<Vec<TemplateExecutionPlanEntry>>(
        fixture["expected_entries"].clone(),
    )
    .expect("expected_entries should deserialize");

    assert_eq!(
        plan_template_tree_execution(
            &template_source_paths,
            &template_contents,
            &existing_destination_paths,
            &destination_contents,
            &context,
            default_strategy,
            &overrides,
            &replacements,
            &ast_merge::default_template_token_config(),
        ),
        expected
    );
}

#[test]
fn conforms_to_mini_template_tree_preview_fixture() {
    let plan_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let preview_fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_preview"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let context =
        serde_json::from_value::<TemplateDestinationContext>(plan_fixture["context"].clone())
            .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(plan_fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(plan_fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(plan_fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let execution_plan = plan_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &existing_destination_paths,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        &ast_merge::default_template_token_config(),
    );
    let expected = serde_json::from_value::<TemplatePreviewResult>(
        preview_fixture["expected_preview"].clone(),
    )
    .expect("expected_preview should deserialize");

    assert_eq!(preview_template_execution(&execution_plan), expected);
}

#[test]
fn conforms_to_mini_template_tree_apply_fixture() {
    let plan_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let apply_fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_apply"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let context =
        serde_json::from_value::<TemplateDestinationContext>(plan_fixture["context"].clone())
            .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(plan_fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(plan_fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(plan_fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let merge_results = serde_json::from_value::<HashMap<String, ast_merge::MergeResult<String>>>(
        apply_fixture["merge_results"].clone(),
    )
    .expect("merge_results should deserialize");

    let execution_plan = plan_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &existing_destination_paths,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        &ast_merge::default_template_token_config(),
    );
    let actual = apply_template_execution(&execution_plan, |entry| {
        let destination_path = entry
            .destination_path
            .as_ref()
            .expect("apply merge entry should have destination path");
        merge_results.get(destination_path).cloned().expect("merge result should be present")
    });
    let expected =
        serde_json::from_value::<TemplateApplyResult>(apply_fixture["expected_result"].clone())
            .expect("expected_result should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_convergence_fixture() {
    let plan_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let apply_fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_apply"));
    let convergence_fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_convergence"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let context =
        serde_json::from_value::<TemplateDestinationContext>(plan_fixture["context"].clone())
            .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(plan_fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(plan_fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(plan_fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let merge_results = serde_json::from_value::<HashMap<String, ast_merge::MergeResult<String>>>(
        apply_fixture["merge_results"].clone(),
    )
    .expect("merge_results should deserialize");
    let convergence_replacements = serde_json::from_value::<HashMap<String, String>>(
        convergence_fixture["replacements"].clone(),
    )
    .expect("convergence replacements should deserialize");

    let execution_plan = plan_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &existing_destination_paths,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        &ast_merge::default_template_token_config(),
    );
    let apply_result = apply_template_execution(&execution_plan, |entry| {
        let destination_path = entry
            .destination_path
            .as_ref()
            .expect("apply merge entry should have destination path");
        merge_results.get(destination_path).cloned().expect("merge result should be present")
    });
    let actual = evaluate_template_tree_convergence(
        &template_source_paths,
        &template_contents,
        &apply_result.result_files,
        &context,
        default_strategy,
        &overrides,
        &convergence_replacements,
        &ast_merge::default_template_token_config(),
    );
    let expected = serde_json::from_value::<TemplateConvergenceResult>(
        convergence_fixture["expected"].clone(),
    )
    .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_run_fixture() {
    let plan_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let run_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_run"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let context =
        serde_json::from_value::<TemplateDestinationContext>(plan_fixture["context"].clone())
            .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(plan_fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(plan_fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(plan_fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let merge_results = serde_json::from_value::<HashMap<String, ast_merge::MergeResult<String>>>(
        run_fixture["merge_results"].clone(),
    )
    .expect("merge_results should deserialize");

    let actual = run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        |entry| {
            let destination_path = entry
                .destination_path
                .as_ref()
                .expect("run merge entry should have destination path");
            merge_results.get(destination_path).cloned().expect("merge result should be present")
        },
        &ast_merge::default_template_token_config(),
    );
    let expected = serde_json::from_value::<TemplateTreeRunResult>(run_fixture["expected"].clone())
        .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_run_report_fixture() {
    let plan_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_plan"));
    let run_fixture = read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_run"));
    let report_fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_run_report"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_plan")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let context =
        serde_json::from_value::<TemplateDestinationContext>(plan_fixture["context"].clone())
            .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(plan_fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(plan_fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(plan_fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let merge_results = serde_json::from_value::<HashMap<String, ast_merge::MergeResult<String>>>(
        run_fixture["merge_results"].clone(),
    )
    .expect("merge_results should deserialize");

    let run_result = run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        |entry| {
            let destination_path = entry
                .destination_path
                .as_ref()
                .expect("run merge entry should have destination path");
            merge_results.get(destination_path).cloned().expect("merge result should be present")
        },
        &ast_merge::default_template_token_config(),
    );
    let actual = report_template_tree_run(&run_result);
    let expected =
        serde_json::from_value::<TemplateTreeRunReport>(report_fixture["expected"].clone())
            .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_family_merge_callback_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_family_merge_callback",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_family_merge_callback")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");

    let actual = run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        |entry| match entry.classification.family.as_str() {
            "markdown" => merge_markdown(
                entry.prepared_template_content.as_ref().expect("prepared content should exist"),
                entry.destination_content.as_ref().expect("destination content should exist"),
                MarkdownDialect::Markdown,
            ),
            family => ast_merge::MergeResult {
                ok: false,
                diagnostics: vec![ast_merge::Diagnostic {
                    severity: ast_merge::DiagnosticSeverity::Error,
                    category: ast_merge::DiagnosticCategory::ConfigurationError,
                    message: format!("missing family merge adapter for {family}"),
                    path: None,
                    review: None,
                }],
                output: None,
                policies: vec![],
            },
        },
        &ast_merge::default_template_token_config(),
    );
    let expected = serde_json::from_value::<TemplateTreeRunResult>(fixture["expected"].clone())
        .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_multi_family_merge_callback_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_multi_family_merge_callback",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_multi_family_merge_callback")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");

    let actual = run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    );
    let expected = serde_json::from_value::<TemplateTreeRunResult>(fixture["expected"].clone())
        .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_multi_family_run_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_multi_family_merge_callback",
    ));
    let report_fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_multi_family_run_report",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_multi_family_merge_callback")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let template_contents = read_relative_file_tree(&fixture_dir.join("template"));
    let destination_contents = read_relative_file_tree(&fixture_dir.join("destination"));
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");

    let run_result = run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    );
    let actual = report_template_tree_run(&run_result);
    let expected =
        serde_json::from_value::<TemplateTreeRunReport>(report_fixture["expected"].clone())
            .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_directory_run_report_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("mini_template_tree_directory_run_report"));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_directory_run_report")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");

    let run_result = ast_merge::run_template_tree_execution_from_directories(
        &fixture_dir.join("template"),
        &fixture_dir.join("destination"),
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("directory-backed run should succeed");
    let actual = report_template_tree_run(&run_result);
    let expected = serde_json::from_value::<TemplateTreeRunReport>(fixture["expected"].clone())
        .expect("expected should deserialize");

    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_directory_apply_convergence_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_directory_apply_convergence",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_directory_apply_convergence")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let temp_root = repo_temp_dir();
    let destination_root = temp_root.join("destination");
    let initial_destination = read_relative_file_tree(&fixture_dir.join("destination"));
    ast_merge::write_relative_file_tree(&destination_root, &initial_destination)
        .expect("destination tree should be writable");

    let first_run = ast_merge::apply_template_tree_execution_to_directory(
        &fixture_dir.join("template"),
        &destination_root,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("first directory apply should succeed");
    let first_actual = report_template_tree_run(&first_run);
    let first_expected =
        serde_json::from_value::<TemplateTreeRunReport>(fixture["expected_first_report"].clone())
            .expect("expected_first_report should deserialize");
    assert_eq!(first_actual, first_expected);

    let actual_files = ast_merge::read_relative_file_tree(&destination_root)
        .expect("applied destination tree should be readable");
    let expected_files = serde_json::from_value::<HashMap<String, String>>(
        fixture["expected_destination_files"].clone(),
    )
    .expect("expected_destination_files should deserialize");
    assert_eq!(actual_files, expected_files);

    let second_run = ast_merge::apply_template_tree_execution_to_directory(
        &fixture_dir.join("template"),
        &destination_root,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("second directory apply should succeed");
    let second_actual = report_template_tree_run(&second_run);
    let second_expected =
        serde_json::from_value::<TemplateTreeRunReport>(fixture["expected_second_report"].clone())
            .expect("expected_second_report should deserialize");
    assert_eq!(second_actual, second_expected);

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
}

#[test]
fn conforms_to_mini_template_tree_directory_apply_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_directory_apply_report",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_directory_apply_report")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");
    let temp_root = repo_temp_dir();
    let destination_root = temp_root.join("destination");
    let initial_destination = read_relative_file_tree(&fixture_dir.join("destination"));
    ast_merge::write_relative_file_tree(&destination_root, &initial_destination)
        .expect("destination tree should be writable");

    let first_run = ast_merge::apply_template_tree_execution_to_directory(
        &fixture_dir.join("template"),
        &destination_root,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("first directory apply should succeed");
    let first_actual = report_template_directory_apply(&first_run);
    let first_expected = serde_json::from_value(fixture["expected_first_report"].clone())
        .expect("expected_first_report should deserialize");
    assert_eq!(first_actual, first_expected);

    let second_run = ast_merge::apply_template_tree_execution_to_directory(
        &fixture_dir.join("template"),
        &destination_root,
        &context,
        default_strategy,
        &overrides,
        &replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("second directory apply should succeed");
    let second_actual = report_template_directory_apply(&second_run);
    let second_expected = serde_json::from_value(fixture["expected_second_report"].clone())
        .expect("expected_second_report should deserialize");
    assert_eq!(second_actual, second_expected);

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
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
fn conforms_to_slice_419_structured_edit_structure_profile_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_structure_profile"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let profile =
            serde_json::from_value::<StructuredEditStructureProfile>(case["profile"].clone())
                .expect("profile should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditStructureProfile>(
            serde_json::to_value(&profile).expect("profile should serialize"),
        )
        .expect("profile should deserialize after roundtrip");

        assert_eq!(round_tripped, profile);
    }
}

#[test]
fn conforms_to_slice_420_structured_edit_selection_profile_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_selection_profile"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let profile =
            serde_json::from_value::<StructuredEditSelectionProfile>(case["profile"].clone())
                .expect("profile should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditSelectionProfile>(
            serde_json::to_value(&profile).expect("profile should serialize"),
        )
        .expect("profile should deserialize after roundtrip");

        assert_eq!(round_tripped, profile);
    }
}

#[test]
fn conforms_to_slice_421_structured_edit_match_profile_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_match_profile"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let profile = serde_json::from_value::<StructuredEditMatchProfile>(case["profile"].clone())
            .expect("profile should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditMatchProfile>(
            serde_json::to_value(&profile).expect("profile should serialize"),
        )
        .expect("profile should deserialize after roundtrip");

        assert_eq!(round_tripped, profile);
    }
}

#[test]
fn conforms_to_slice_422_structured_edit_operation_profile_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_operation_profile"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let profile =
            serde_json::from_value::<StructuredEditOperationProfile>(case["profile"].clone())
                .expect("profile should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditOperationProfile>(
            serde_json::to_value(&profile).expect("profile should serialize"),
        )
        .expect("profile should deserialize after roundtrip");

        assert_eq!(round_tripped, profile);
    }
}

#[test]
fn conforms_to_slice_423_structured_edit_destination_profile_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_destination_profile"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let profile =
            serde_json::from_value::<StructuredEditDestinationProfile>(case["profile"].clone())
                .expect("profile should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditDestinationProfile>(
            serde_json::to_value(&profile).expect("profile should serialize"),
        )
        .expect("profile should deserialize after roundtrip");

        assert_eq!(round_tripped, profile);
    }
}

#[test]
fn conforms_to_slice_426_structured_edit_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_request"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let request = serde_json::from_value::<StructuredEditRequest>(case["request"].clone())
            .expect("request should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditRequest>(
            serde_json::to_value(&request).expect("request should serialize"),
        )
        .expect("request should deserialize after roundtrip");

        assert_eq!(round_tripped, request);
    }
}

#[test]
fn conforms_to_slice_427_structured_edit_result_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_result"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let result = serde_json::from_value::<StructuredEditResult>(case["result"].clone())
            .expect("result should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditResult>(
            serde_json::to_value(&result).expect("result should serialize"),
        )
        .expect("result should deserialize after roundtrip");

        assert_eq!(round_tripped, result);
    }
}

#[test]
fn conforms_to_slice_432_structured_edit_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_application"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let application =
            serde_json::from_value::<StructuredEditApplication>(case["application"].clone())
                .expect("application should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditApplication>(
            serde_json::to_value(&application).expect("application should serialize"),
        )
        .expect("application should deserialize after roundtrip");

        assert_eq!(round_tripped, application);
    }
}

#[test]
fn conforms_to_slice_435_structured_edit_transport_envelope_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_application_envelope"));
    let application = serde_json::from_value::<StructuredEditApplication>(
        fixture["structured_edit_application"].clone(),
    )
    .expect("application should deserialize");
    let expected = serde_json::from_value::<StructuredEditApplicationEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_application_envelope(&application), expected);
    assert_eq!(import_structured_edit_application_envelope(&expected), Ok(application));
}

#[test]
fn conforms_to_slice_436_structured_edit_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_application_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditApplicationEnvelope>(case["envelope"].clone())
                .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(import_structured_edit_application_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_437_structured_edit_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_application_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditApplicationEnvelope>(
        fixture["structured_edit_application_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditApplication>(
        fixture["expected_application"].clone(),
    )
    .expect("expected application should deserialize");

    assert_eq!(import_structured_edit_application_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope =
            serde_json::from_value::<StructuredEditApplicationEnvelope>(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_application_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_438_structured_edit_execution_report_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_execution_report"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let report =
            serde_json::from_value::<StructuredEditExecutionReport>(case["report"].clone())
                .expect("report should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditExecutionReport>(
            serde_json::to_value(&report).expect("report should serialize"),
        )
        .expect("report should deserialize after roundtrip");

        assert_eq!(round_tripped, report);
    }
}

#[test]
fn conforms_to_slice_453_structured_edit_provider_execution_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_request",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_request = serde_json::from_value::<StructuredEditProviderExecutionRequest>(
            case["execution_request"].clone(),
        )
        .expect("provider execution request should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderExecutionRequest>(
            serde_json::to_value(&execution_request)
                .expect("provider execution request should serialize"),
        )
        .expect("provider execution request should deserialize after roundtrip");

        assert_eq!(round_tripped, execution_request);
    }
}

#[test]
fn conforms_to_slice_454_structured_edit_provider_execution_request_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_request_envelope",
    ));
    let execution_request = serde_json::from_value::<StructuredEditProviderExecutionRequest>(
        fixture["structured_edit_provider_execution_request"].clone(),
    )
    .expect("provider execution request should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionRequestEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_request_envelope(&execution_request), expected);
    assert_eq!(
        import_structured_edit_provider_execution_request_envelope(&expected),
        Ok(execution_request)
    );
}

#[test]
fn conforms_to_slice_455_structured_edit_provider_execution_request_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_request_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionRequestEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_request_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_456_structured_edit_provider_execution_request_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_request_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionRequestEnvelope>(
        fixture["structured_edit_provider_execution_request_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionRequest>(
        fixture["expected_execution_request"].clone(),
    )
    .expect("expected execution request should deserialize");

    assert_eq!(import_structured_edit_provider_execution_request_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_request_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_461_structured_edit_provider_execution_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_application",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let application = serde_json::from_value::<StructuredEditProviderExecutionApplication>(
            case["application"].clone(),
        )
        .expect("provider execution application should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderExecutionApplication>(
            serde_json::to_value(&application)
                .expect("provider execution application should serialize"),
        )
        .expect("provider execution application should deserialize after roundtrip");

        assert_eq!(round_tripped, application);
    }
}

#[test]
fn conforms_to_slice_469_structured_edit_provider_execution_dispatch_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_dispatch",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let dispatch = serde_json::from_value::<StructuredEditProviderExecutionDispatch>(
            case["dispatch"].clone(),
        )
        .expect("provider execution dispatch should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderExecutionDispatch>(
            serde_json::to_value(&dispatch).expect("provider execution dispatch should serialize"),
        )
        .expect("provider execution dispatch should deserialize after roundtrip");

        assert_eq!(round_tripped, dispatch);
    }
}

#[test]
fn conforms_to_slice_470_structured_edit_provider_execution_dispatch_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_dispatch_envelope",
    ));
    let dispatch = serde_json::from_value::<StructuredEditProviderExecutionDispatch>(
        fixture["structured_edit_provider_execution_dispatch"].clone(),
    )
    .expect("provider execution dispatch should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionDispatchEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("dispatch envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_dispatch_envelope(&dispatch), expected);
    assert_eq!(
        import_structured_edit_provider_execution_dispatch_envelope(&expected),
        Ok(dispatch)
    );
}

#[test]
fn conforms_to_slice_471_structured_edit_provider_execution_dispatch_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_dispatch_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionDispatchEnvelope>(
            case["envelope"].clone(),
        )
        .expect("dispatch envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_dispatch_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_472_structured_edit_provider_execution_dispatch_envelope_application_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_dispatch_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionDispatchEnvelope>(
        fixture["structured_edit_provider_execution_dispatch_envelope"].clone(),
    )
    .expect("dispatch envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionDispatch>(
        fixture["expected_dispatch"].clone(),
    )
    .expect("expected dispatch should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_dispatch_envelope(&envelope),
        Ok(expected)
    );

    for case in fixture["cases"].as_array().expect("cases should be an array") {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionDispatchEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_dispatch_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_477_structured_edit_provider_execution_outcome_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_outcome",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let outcome = serde_json::from_value::<StructuredEditProviderExecutionOutcome>(
            case["outcome"].clone(),
        )
        .expect("provider execution outcome should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderExecutionOutcome>(
            serde_json::to_value(&outcome).expect("provider execution outcome should serialize"),
        )
        .expect("provider execution outcome should deserialize after roundtrip");

        assert_eq!(round_tripped, outcome);
    }
}

#[test]
fn conforms_to_slice_478_structured_edit_provider_execution_outcome_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_outcome_envelope",
    ));
    let outcome = serde_json::from_value::<StructuredEditProviderExecutionOutcome>(
        fixture["structured_edit_provider_execution_outcome"].clone(),
    )
    .expect("provider execution outcome should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionOutcomeEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("outcome envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_outcome_envelope(&outcome), expected);
    assert_eq!(import_structured_edit_provider_execution_outcome_envelope(&expected), Ok(outcome));
}

#[test]
fn conforms_to_slice_479_structured_edit_provider_execution_outcome_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_outcome_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionOutcomeEnvelope>(
            case["envelope"].clone(),
        )
        .expect("outcome envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_outcome_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_480_structured_edit_provider_execution_outcome_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_outcome_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionOutcomeEnvelope>(
        fixture["structured_edit_provider_execution_outcome_envelope"].clone(),
    )
    .expect("outcome envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionOutcome>(
        fixture["expected_outcome"].clone(),
    )
    .expect("expected outcome should deserialize");

    assert_eq!(import_structured_edit_provider_execution_outcome_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionOutcomeEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_outcome_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_481_structured_edit_provider_batch_execution_outcome_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_outcome",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcome>(
            case["batch_outcome"].clone(),
        )
        .expect("provider batch execution outcome should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcome>(
            serde_json::to_value(&batch)
                .expect("provider batch execution outcome should serialize"),
        )
        .expect("provider batch execution outcome should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_482_structured_edit_provider_batch_execution_outcome_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_outcome_envelope",
    ));
    let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcome>(
        fixture["structured_edit_provider_batch_execution_outcome"].clone(),
    )
    .expect("provider batch execution outcome should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcomeEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_batch_execution_outcome_envelope(&batch), expected);
    assert_eq!(
        import_structured_edit_provider_batch_execution_outcome_envelope(&expected),
        Ok(batch)
    );
}

#[test]
fn conforms_to_slice_483_structured_edit_provider_batch_execution_outcome_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_outcome_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionOutcomeEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_outcome_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_484_structured_edit_provider_batch_execution_outcome_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_outcome_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcomeEnvelope>(
        fixture["structured_edit_provider_batch_execution_outcome_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionOutcome>(
        fixture["expected_batch_outcome"].clone(),
    )
    .expect("expected batch outcome should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_outcome_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionOutcomeEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_outcome_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_485_structured_edit_provider_execution_provenance_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_provenance",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let provenance = serde_json::from_value::<StructuredEditProviderExecutionProvenance>(
            case["provenance"].clone(),
        )
        .expect("provider execution provenance should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderExecutionProvenance>(
            serde_json::to_value(&provenance)
                .expect("provider execution provenance should serialize"),
        )
        .expect("provider execution provenance should deserialize after roundtrip");

        assert_eq!(round_tripped, provenance);
    }
}

#[test]
fn conforms_to_slice_486_structured_edit_provider_execution_provenance_transport_envelope_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_provenance_envelope",
    ));
    let provenance = serde_json::from_value::<StructuredEditProviderExecutionProvenance>(
        fixture["structured_edit_provider_execution_provenance"].clone(),
    )
    .expect("provider execution provenance should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionProvenanceEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_provenance_envelope(&provenance), expected);
    assert_eq!(
        import_structured_edit_provider_execution_provenance_envelope(&expected),
        Ok(provenance)
    );
}

#[test]
fn conforms_to_slice_487_structured_edit_provider_execution_provenance_transport_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_provenance_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionProvenanceEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_provenance_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_488_structured_edit_provider_execution_provenance_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_provenance_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionProvenanceEnvelope>(
        fixture["structured_edit_provider_execution_provenance_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionProvenance>(
        fixture["expected_provenance"].clone(),
    )
    .expect("expected provenance should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_provenance_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionProvenanceEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_provenance_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_489_structured_edit_provider_batch_execution_provenance_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_provenance",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionProvenance>(
            case["batch_provenance"].clone(),
        )
        .expect("provider batch execution provenance should deserialize");
        let round_tripped =
            serde_json::from_value::<StructuredEditProviderBatchExecutionProvenance>(
                serde_json::to_value(&batch)
                    .expect("provider batch execution provenance should serialize"),
            )
            .expect("provider batch execution provenance should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_490_structured_edit_provider_batch_execution_provenance_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_provenance_envelope",
    ));
    let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionProvenance>(
        fixture["structured_edit_provider_batch_execution_provenance"].clone(),
    )
    .expect("provider batch execution provenance should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderBatchExecutionProvenanceEnvelope>(
            fixture["expected_envelope"].clone(),
        )
        .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_batch_execution_provenance_envelope(&batch), expected);
    assert_eq!(
        import_structured_edit_provider_batch_execution_provenance_envelope(&expected),
        Ok(batch)
    );
}

#[test]
fn conforms_to_slice_491_structured_edit_provider_batch_execution_provenance_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_provenance_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionProvenanceEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_provenance_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_492_structured_edit_provider_batch_execution_provenance_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_provenance_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderBatchExecutionProvenanceEnvelope>(
            fixture["structured_edit_provider_batch_execution_provenance_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionProvenance>(
        fixture["expected_batch_provenance"].clone(),
    )
    .expect("expected batch provenance should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_provenance_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionProvenanceEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_provenance_envelope(&rejected_envelope,),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_493_structured_edit_provider_execution_replay_bundle_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_replay_bundle",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let replay_bundle = serde_json::from_value::<StructuredEditProviderExecutionReplayBundle>(
            case["replay_bundle"].clone(),
        )
        .expect("replay bundle should deserialize");
        let roundtrip = serde_json::to_value(&replay_bundle).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutionReplayBundle>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, replay_bundle);
    }
}

#[test]
fn conforms_to_slice_494_structured_edit_provider_execution_replay_bundle_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_replay_bundle_envelope",
    ));
    let replay_bundle = serde_json::from_value::<StructuredEditProviderExecutionReplayBundle>(
        fixture["structured_edit_provider_execution_replay_bundle"].clone(),
    )
    .expect("replay bundle should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReplayBundleEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_replay_bundle_envelope(&replay_bundle), expected);
    assert_eq!(
        import_structured_edit_provider_execution_replay_bundle_envelope(&expected),
        Ok(replay_bundle)
    );
}

#[test]
fn conforms_to_slice_495_structured_edit_provider_execution_replay_bundle_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_replay_bundle_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderExecutionReplayBundleEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_replay_bundle_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_496_structured_edit_provider_execution_replay_bundle_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_replay_bundle_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionReplayBundleEnvelope>(
        fixture["structured_edit_provider_execution_replay_bundle_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReplayBundle>(
        fixture["expected_replay_bundle"].clone(),
    )
    .expect("replay bundle should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_replay_bundle_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReplayBundleEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_replay_bundle_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_497_structured_edit_provider_batch_execution_replay_bundle_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_replay_bundle",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_replay_bundle = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReplayBundle,
        >(case["batch_replay_bundle"].clone())
        .expect("batch replay bundle should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_replay_bundle).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderBatchExecutionReplayBundle>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_replay_bundle);
    }
}

#[test]
fn conforms_to_slice_498_structured_edit_provider_batch_execution_replay_bundle_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_replay_bundle_envelope",
    ));
    let batch_replay_bundle =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReplayBundle>(
            fixture["structured_edit_provider_batch_execution_replay_bundle"].clone(),
        )
        .expect("batch replay bundle should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReplayBundleEnvelope>(
            fixture["expected_envelope"].clone(),
        )
        .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_replay_bundle_envelope(&batch_replay_bundle),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_replay_bundle_envelope(&expected),
        Ok(batch_replay_bundle)
    );
}

#[test]
fn conforms_to_slice_499_structured_edit_provider_batch_execution_replay_bundle_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_replay_bundle_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReplayBundleEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_replay_bundle_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_500_structured_edit_provider_batch_execution_replay_bundle_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_replay_bundle_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReplayBundleEnvelope>(
            fixture["structured_edit_provider_batch_execution_replay_bundle_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionReplayBundle>(
        fixture["expected_batch_replay_bundle"].clone(),
    )
    .expect("batch replay bundle should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_replay_bundle_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReplayBundleEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_replay_bundle_envelope(
                &rejected_envelope,
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_501_structured_edit_provider_executor_profile_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_profile",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let executor_profile = serde_json::from_value::<StructuredEditProviderExecutorProfile>(
            case["executor_profile"].clone(),
        )
        .expect("executor profile should deserialize");
        let roundtrip =
            serde_json::to_value(&executor_profile).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutorProfile>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, executor_profile);
    }
}

#[test]
fn conforms_to_slice_502_structured_edit_provider_executor_profile_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_profile_envelope",
    ));
    let executor_profile = serde_json::from_value::<StructuredEditProviderExecutorProfile>(
        fixture["structured_edit_provider_executor_profile"].clone(),
    )
    .expect("executor profile should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorProfileEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_executor_profile_envelope(&executor_profile), expected);
    assert_eq!(
        import_structured_edit_provider_executor_profile_envelope(&expected),
        Ok(executor_profile)
    );
}

#[test]
fn conforms_to_slice_503_structured_edit_provider_executor_profile_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_profile_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutorProfileEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_profile_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_504_structured_edit_provider_executor_profile_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_profile_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutorProfileEnvelope>(
        fixture["structured_edit_provider_executor_profile_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorProfile>(
        fixture["expected_executor_profile"].clone(),
    )
    .expect("executor profile should deserialize");

    assert_eq!(import_structured_edit_provider_executor_profile_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutorProfileEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_profile_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_505_structured_edit_provider_executor_registry_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_registry",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let executor_registry = serde_json::from_value::<StructuredEditProviderExecutorRegistry>(
            case["executor_registry"].clone(),
        )
        .expect("executor registry should deserialize");
        let roundtrip =
            serde_json::to_value(&executor_registry).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutorRegistry>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, executor_registry);
    }
}

#[test]
fn conforms_to_slice_506_structured_edit_provider_executor_registry_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_registry_envelope",
    ));
    let executor_registry = serde_json::from_value::<StructuredEditProviderExecutorRegistry>(
        fixture["structured_edit_provider_executor_registry"].clone(),
    )
    .expect("executor registry should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorRegistryEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_executor_registry_envelope(&executor_registry), expected);
    assert_eq!(
        import_structured_edit_provider_executor_registry_envelope(&expected),
        Ok(executor_registry)
    );
}

#[test]
fn conforms_to_slice_507_structured_edit_provider_executor_registry_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_registry_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutorRegistryEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_registry_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_508_structured_edit_provider_executor_registry_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_registry_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutorRegistryEnvelope>(
        fixture["structured_edit_provider_executor_registry_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorRegistry>(
        fixture["expected_executor_registry"].clone(),
    )
    .expect("executor registry should deserialize");

    assert_eq!(import_structured_edit_provider_executor_registry_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutorRegistryEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_registry_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_509_structured_edit_provider_executor_selection_policy_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_selection_policy",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let selection_policy = serde_json::from_value::<
            StructuredEditProviderExecutorSelectionPolicy,
        >(case["selection_policy"].clone())
        .expect("selection policy should deserialize");
        let roundtrip =
            serde_json::to_value(&selection_policy).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutorSelectionPolicy>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, selection_policy);
    }
}

#[test]
fn conforms_to_slice_510_structured_edit_provider_executor_selection_policy_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_selection_policy_envelope",
    ));
    let selection_policy = serde_json::from_value::<StructuredEditProviderExecutorSelectionPolicy>(
        fixture["structured_edit_provider_executor_selection_policy"].clone(),
    )
    .expect("selection policy should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorSelectionPolicyEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_executor_selection_policy_envelope(&selection_policy),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_executor_selection_policy_envelope(&expected),
        Ok(selection_policy)
    );
}

#[test]
fn conforms_to_slice_511_structured_edit_provider_executor_selection_policy_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_selection_policy_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderExecutorSelectionPolicyEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_selection_policy_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_512_structured_edit_provider_executor_selection_policy_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_selection_policy_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutorSelectionPolicyEnvelope>(
        fixture["structured_edit_provider_executor_selection_policy_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorSelectionPolicy>(
        fixture["expected_selection_policy"].clone(),
    )
    .expect("selection policy should deserialize");

    assert_eq!(
        import_structured_edit_provider_executor_selection_policy_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutorSelectionPolicyEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_selection_policy_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_513_structured_edit_provider_executor_resolution_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_resolution",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let executor_resolution =
            serde_json::from_value::<StructuredEditProviderExecutorResolution>(
                case["executor_resolution"].clone(),
            )
            .expect("executor resolution should deserialize");
        let roundtrip =
            serde_json::to_value(&executor_resolution).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutorResolution>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, executor_resolution);
    }
}

#[test]
fn conforms_to_slice_514_structured_edit_provider_executor_resolution_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_resolution_envelope",
    ));
    let executor_resolution = serde_json::from_value::<StructuredEditProviderExecutorResolution>(
        fixture["structured_edit_provider_executor_resolution"].clone(),
    )
    .expect("executor resolution should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorResolutionEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_executor_resolution_envelope(&executor_resolution),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_executor_resolution_envelope(&expected),
        Ok(executor_resolution)
    );
}

#[test]
fn conforms_to_slice_515_structured_edit_provider_executor_resolution_transport_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_resolution_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutorResolutionEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_resolution_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_516_structured_edit_provider_executor_resolution_envelope_application_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_executor_resolution_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutorResolutionEnvelope>(
        fixture["structured_edit_provider_executor_resolution_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutorResolution>(
        fixture["expected_executor_resolution"].clone(),
    )
    .expect("executor resolution should deserialize");

    assert_eq!(
        import_structured_edit_provider_executor_resolution_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutorResolutionEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_executor_resolution_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_517_structured_edit_provider_execution_plan_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_provider_execution_plan"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_plan = serde_json::from_value::<StructuredEditProviderExecutionPlan>(
            case["execution_plan"].clone(),
        )
        .expect("execution plan should deserialize");
        let roundtrip = serde_json::to_value(&execution_plan).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutionPlan>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, execution_plan);
    }
}

#[test]
fn conforms_to_slice_525_structured_edit_provider_execution_handoff_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_handoff",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_handoff = serde_json::from_value::<StructuredEditProviderExecutionHandoff>(
            case["execution_handoff"].clone(),
        )
        .expect("execution handoff should deserialize");
        let roundtrip =
            serde_json::to_value(&execution_handoff).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutionHandoff>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, execution_handoff);
    }
}

#[test]
fn conforms_to_slice_526_structured_edit_provider_execution_handoff_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_handoff_envelope",
    ));
    let execution_handoff = serde_json::from_value::<StructuredEditProviderExecutionHandoff>(
        fixture["structured_edit_provider_execution_handoff"].clone(),
    )
    .expect("execution handoff should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionHandoffEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_handoff_envelope(&execution_handoff), expected);
    assert_eq!(
        import_structured_edit_provider_execution_handoff_envelope(&expected),
        Ok(execution_handoff)
    );
}

#[test]
fn conforms_to_slice_527_structured_edit_provider_execution_handoff_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_handoff_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionHandoffEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_handoff_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_528_structured_edit_provider_execution_handoff_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_handoff_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionHandoffEnvelope>(
        fixture["structured_edit_provider_execution_handoff_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionHandoff>(
        fixture["expected_execution_handoff"].clone(),
    )
    .expect("execution handoff should deserialize");

    assert_eq!(import_structured_edit_provider_execution_handoff_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionHandoffEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_handoff_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_533_structured_edit_provider_execution_invocation_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_invocation",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_invocation = serde_json::from_value::<
            StructuredEditProviderExecutionInvocation,
        >(case["execution_invocation"].clone())
        .expect("execution invocation should deserialize");
        let roundtrip =
            serde_json::to_value(&execution_invocation).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutionInvocation>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, execution_invocation);
    }
}

#[test]
fn conforms_to_slice_534_structured_edit_provider_execution_invocation_transport_envelope_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_invocation_envelope",
    ));
    let execution_invocation = serde_json::from_value::<StructuredEditProviderExecutionInvocation>(
        fixture["structured_edit_provider_execution_invocation"].clone(),
    )
    .expect("execution invocation should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionInvocationEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_execution_invocation_envelope(&execution_invocation),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_execution_invocation_envelope(&expected),
        Ok(execution_invocation)
    );
}

#[test]
fn conforms_to_slice_535_structured_edit_provider_execution_invocation_transport_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_invocation_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionInvocationEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_invocation_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_536_structured_edit_provider_execution_invocation_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_invocation_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionInvocationEnvelope>(
        fixture["structured_edit_provider_execution_invocation_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionInvocation>(
        fixture["expected_execution_invocation"].clone(),
    )
    .expect("execution invocation should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_invocation_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionInvocationEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_invocation_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_537_structured_edit_provider_batch_execution_invocation_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_invocation",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_execution_invocation = serde_json::from_value::<
            StructuredEditProviderBatchExecutionInvocation,
        >(case["batch_execution_invocation"].clone())
        .expect("batch execution invocation should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_execution_invocation).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderBatchExecutionInvocation>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_execution_invocation);
    }
}

#[test]
fn conforms_to_slice_538_structured_edit_provider_batch_execution_invocation_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_invocation_envelope",
    ));
    let batch_execution_invocation =
        serde_json::from_value::<StructuredEditProviderBatchExecutionInvocation>(
            fixture["structured_edit_provider_batch_execution_invocation"].clone(),
        )
        .expect("batch execution invocation should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderBatchExecutionInvocationEnvelope>(
            fixture["expected_envelope"].clone(),
        )
        .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_invocation_envelope(&batch_execution_invocation),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_invocation_envelope(&expected),
        Ok(batch_execution_invocation)
    );
}

#[test]
fn conforms_to_slice_539_structured_edit_provider_batch_execution_invocation_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_invocation_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionInvocationEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_invocation_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_540_structured_edit_provider_batch_execution_invocation_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_invocation_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderBatchExecutionInvocationEnvelope>(
            fixture["structured_edit_provider_batch_execution_invocation_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionInvocation>(
        fixture["expected_batch_execution_invocation"].clone(),
    )
    .expect("batch execution invocation should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_invocation_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionInvocationEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_invocation_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_541_structured_edit_provider_execution_run_result_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_run_result",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_run_result =
            serde_json::from_value::<StructuredEditProviderExecutionRunResult>(
                case["execution_run_result"].clone(),
            )
            .expect("execution run result should deserialize");
        let roundtrip =
            serde_json::to_value(&execution_run_result).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutionRunResult>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, execution_run_result);
    }
}

#[test]
fn conforms_to_slice_542_structured_edit_provider_execution_run_result_transport_envelope_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_run_result_envelope",
    ));
    let execution_run_result = serde_json::from_value::<StructuredEditProviderExecutionRunResult>(
        fixture["structured_edit_provider_execution_run_result"].clone(),
    )
    .expect("execution run result should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionRunResultEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_execution_run_result_envelope(&execution_run_result),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_execution_run_result_envelope(&expected),
        Ok(execution_run_result)
    );
}

#[test]
fn conforms_to_slice_543_structured_edit_provider_execution_run_result_transport_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_run_result_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionRunResultEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_run_result_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_544_structured_edit_provider_execution_run_result_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_run_result_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionRunResultEnvelope>(
        fixture["structured_edit_provider_execution_run_result_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionRunResult>(
        fixture["expected_execution_run_result"].clone(),
    )
    .expect("execution run result should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_run_result_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionRunResultEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_run_result_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_545_structured_edit_provider_batch_execution_run_result_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_run_result",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_execution_run_result = serde_json::from_value::<
            StructuredEditProviderBatchExecutionRunResult,
        >(case["batch_execution_run_result"].clone())
        .expect("batch execution run result should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_execution_run_result).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderBatchExecutionRunResult>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_execution_run_result);
    }
}

#[test]
fn conforms_to_slice_546_structured_edit_provider_batch_execution_run_result_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_run_result_envelope",
    ));
    let batch_execution_run_result =
        serde_json::from_value::<StructuredEditProviderBatchExecutionRunResult>(
            fixture["structured_edit_provider_batch_execution_run_result"].clone(),
        )
        .expect("batch execution run result should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionRunResultEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_run_result_envelope(&batch_execution_run_result),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_run_result_envelope(&expected),
        Ok(batch_execution_run_result)
    );
}

#[test]
fn conforms_to_slice_547_structured_edit_provider_batch_execution_run_result_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_run_result_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionRunResultEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_run_result_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_548_structured_edit_provider_batch_execution_run_result_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_run_result_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionRunResultEnvelope>(
        fixture["structured_edit_provider_batch_execution_run_result_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionRunResult>(
        fixture["expected_batch_execution_run_result"].clone(),
    )
    .expect("expected batch execution run result should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_run_result_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionRunResultEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_run_result_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_549_structured_edit_provider_execution_receipt_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let execution_receipt = serde_json::from_value::<StructuredEditProviderExecutionReceipt>(
            case["execution_receipt"].clone(),
        )
        .expect("execution receipt should deserialize");
        let roundtrip =
            serde_json::to_value(&execution_receipt).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderExecutionReceipt>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, execution_receipt);
    }
}

#[test]
fn conforms_to_slice_550_structured_edit_provider_execution_receipt_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_envelope",
    ));
    let execution_receipt = serde_json::from_value::<StructuredEditProviderExecutionReceipt>(
        fixture["structured_edit_provider_execution_receipt"].clone(),
    )
    .expect("execution receipt should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReceiptEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_receipt_envelope(&execution_receipt), expected);
    assert_eq!(
        import_structured_edit_provider_execution_receipt_envelope(&expected),
        Ok(execution_receipt)
    );
}

#[test]
fn conforms_to_slice_551_structured_edit_provider_execution_receipt_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionReceiptEnvelope>(
            case["envelope"].clone(),
        )
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_552_structured_edit_provider_execution_receipt_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionReceiptEnvelope>(
        fixture["structured_edit_provider_execution_receipt_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReceipt>(
        fixture["expected_execution_receipt"].clone(),
    )
    .expect("expected execution receipt should deserialize");

    assert_eq!(import_structured_edit_provider_execution_receipt_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_553_structured_edit_provider_batch_execution_receipt_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_execution_receipt = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceipt,
        >(case["batch_execution_receipt"].clone())
        .expect("batch execution receipt should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_execution_receipt).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderBatchExecutionReceipt>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_execution_receipt);
    }
}

#[test]
fn conforms_to_slice_554_structured_edit_provider_batch_execution_receipt_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_envelope",
    ));
    let batch_execution_receipt =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceipt>(
            fixture["structured_edit_provider_batch_execution_receipt"].clone(),
        )
        .expect("batch execution receipt should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_receipt_envelope(&batch_execution_receipt),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_envelope(&expected),
        Ok(batch_execution_receipt)
    );
}

#[test]
fn conforms_to_slice_555_structured_edit_provider_batch_execution_receipt_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptEnvelope>(
                case["envelope"].clone(),
            )
            .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_556_structured_edit_provider_batch_execution_receipt_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptEnvelope>(
        fixture["structured_edit_provider_batch_execution_receipt_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionReceipt>(
        fixture["expected_batch_execution_receipt"].clone(),
    )
    .expect("expected batch execution receipt should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_557_structured_edit_provider_execution_receipt_replay_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_request",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let receipt_replay_request = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayRequest,
        >(case["receipt_replay_request"].clone())
        .expect("receipt replay request should deserialize");
        let roundtrip =
            serde_json::to_value(&receipt_replay_request).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayRequest>(
                roundtrip,
            )
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, receipt_replay_request);
    }
}

#[test]
fn conforms_to_slice_558_structured_edit_provider_execution_receipt_replay_request_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_request_envelope",
    ));
    let receipt_replay_request =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayRequest>(
            fixture["structured_edit_provider_execution_receipt_replay_request"].clone(),
        )
        .expect("receipt replay request should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderExecutionReceiptReplayRequestEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_execution_receipt_replay_request_envelope(&receipt_replay_request),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_request_envelope(&expected),
        Ok(receipt_replay_request)
    );
}

#[test]
fn conforms_to_slice_559_structured_edit_provider_execution_receipt_replay_request_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_request_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_request_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_560_structured_edit_provider_execution_receipt_replay_request_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_request_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayRequestEnvelope>(
            fixture["structured_edit_provider_execution_receipt_replay_request_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayRequest>(
        fixture["expected_receipt_replay_request"].clone(),
    )
    .expect("expected receipt replay request should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_request_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_request_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_561_structured_edit_provider_batch_execution_receipt_replay_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_request",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_receipt_replay_request = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayRequest,
        >(case["batch_receipt_replay_request"].clone())
        .expect("batch receipt replay request should deserialize");
        let roundtrip = serde_json::to_value(&batch_receipt_replay_request)
            .expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayRequest,
        >(roundtrip)
        .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_receipt_replay_request);
    }
}

#[test]
fn conforms_to_slice_562_structured_edit_provider_batch_execution_receipt_replay_request_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_request_envelope",
    ));
    let batch_receipt_replay_request =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplayRequest>(
            fixture["structured_edit_provider_batch_execution_receipt_replay_request"].clone(),
        )
        .expect("batch receipt replay request should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_receipt_replay_request_envelope(
            &batch_receipt_replay_request
        ),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_request_envelope(&expected),
        Ok(batch_receipt_replay_request)
    );
}

#[test]
fn conforms_to_slice_563_structured_edit_provider_batch_execution_receipt_replay_request_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_request_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_request_envelope(
                &envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_564_structured_edit_provider_batch_execution_receipt_replay_request_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_request_envelope_application",
    ));
    let envelope = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
    >(
        fixture["structured_edit_provider_batch_execution_receipt_replay_request_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplayRequest>(
            fixture["expected_batch_receipt_replay_request"].clone(),
        )
        .expect("expected batch receipt replay request should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_request_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_request_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_565_structured_edit_provider_execution_receipt_replay_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_application",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let receipt_replay_application = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayApplication,
        >(case["receipt_replay_application"].clone())
        .expect("receipt replay application should deserialize");
        let roundtrip =
            serde_json::to_value(&receipt_replay_application).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayApplication,
        >(roundtrip)
        .expect("roundtrip should deserialize");

        assert_eq!(decoded, receipt_replay_application);
    }
}

#[test]
fn conforms_to_slice_566_structured_edit_provider_execution_receipt_replay_application_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_application_envelope",
    ));
    let receipt_replay_application =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayApplication>(
            fixture["structured_edit_provider_execution_receipt_replay_application"].clone(),
        )
        .expect("receipt replay application should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_execution_receipt_replay_application_envelope(
            &receipt_replay_application
        ),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_application_envelope(&expected),
        Ok(receipt_replay_application)
    );
}

#[test]
fn conforms_to_slice_567_structured_edit_provider_execution_receipt_replay_application_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_application_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_application_envelope(
                &envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_568_structured_edit_provider_execution_receipt_replay_application_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_application_envelope_application",
    ));
    let envelope = serde_json::from_value::<
        StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
    >(
        fixture["structured_edit_provider_execution_receipt_replay_application_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayApplication>(
            fixture["expected_receipt_replay_application"].clone(),
        )
        .expect("expected receipt replay application should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_application_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_application_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_569_structured_edit_provider_batch_execution_receipt_replay_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_application",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_receipt_replay_application =
            serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplayApplication>(
                case["batch_receipt_replay_application"].clone(),
            )
            .expect("batch receipt replay application should deserialize");
        let roundtrip = serde_json::to_value(&batch_receipt_replay_application)
            .expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayApplication,
        >(roundtrip)
        .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_receipt_replay_application);
    }
}

#[test]
fn conforms_to_slice_570_structured_edit_provider_batch_execution_receipt_replay_application_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_application_envelope",
    ));
    let batch_receipt_replay_application =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplayApplication>(
            fixture["structured_edit_provider_batch_execution_receipt_replay_application"].clone(),
        )
        .expect("batch receipt replay application should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_receipt_replay_application_envelope(
            &batch_receipt_replay_application
        ),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_application_envelope(
            &expected
        ),
        Ok(batch_receipt_replay_application)
    );
}

#[test]
fn conforms_to_slice_571_structured_edit_provider_batch_execution_receipt_replay_application_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_application_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_application_envelope(
                &envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_572_structured_edit_provider_batch_execution_receipt_replay_application_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_application_envelope_application",
    ));
    let envelope = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
    >(
        fixture["structured_edit_provider_batch_execution_receipt_replay_application_envelope"]
            .clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayApplication,
    >(fixture["expected_batch_receipt_replay_application"].clone())
    .expect("expected batch receipt replay application should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_application_envelope(
            &envelope
        ),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_application_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_573_structured_edit_provider_execution_receipt_replay_session_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_session",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let receipt_replay_session = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplaySession,
        >(case["receipt_replay_session"].clone())
        .expect("receipt replay session should deserialize");
        let roundtrip =
            serde_json::to_value(&receipt_replay_session).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutionReceiptReplaySession>(
                roundtrip,
            )
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, receipt_replay_session);
    }
}

#[test]
fn conforms_to_slice_574_structured_edit_provider_execution_receipt_replay_session_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_session_envelope",
    ));
    let receipt_replay_session =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplaySession>(
            fixture["structured_edit_provider_execution_receipt_replay_session"].clone(),
        )
        .expect("receipt replay session should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderExecutionReceiptReplaySessionEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_execution_receipt_replay_session_envelope(&receipt_replay_session),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_session_envelope(&expected),
        Ok(receipt_replay_session)
    );
}

#[test]
fn conforms_to_slice_575_structured_edit_provider_execution_receipt_replay_session_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_session_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplaySessionEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_session_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_576_structured_edit_provider_execution_receipt_replay_session_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_session_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplaySessionEnvelope>(
            fixture["structured_edit_provider_execution_receipt_replay_session_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionReceiptReplaySession>(
        fixture["expected_receipt_replay_session"].clone(),
    )
    .expect("expected receipt replay session should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_receipt_replay_session_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplaySessionEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_session_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_577_structured_edit_provider_batch_execution_receipt_replay_session_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_session",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_receipt_replay_session = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplaySession,
        >(case["batch_receipt_replay_session"].clone())
        .expect("batch receipt replay session should deserialize");
        let roundtrip = serde_json::to_value(&batch_receipt_replay_session)
            .expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplaySession,
        >(roundtrip)
        .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_receipt_replay_session);
    }
}

#[test]
fn conforms_to_slice_578_structured_edit_provider_batch_execution_receipt_replay_session_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_session_envelope",
    ));
    let batch_receipt_replay_session =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplaySession>(
            fixture["structured_edit_provider_batch_execution_receipt_replay_session"].clone(),
        )
        .expect("batch receipt replay session should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_receipt_replay_session_envelope(
            &batch_receipt_replay_session
        ),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_session_envelope(&expected),
        Ok(batch_receipt_replay_session)
    );
}

#[test]
fn conforms_to_slice_579_structured_edit_provider_batch_execution_receipt_replay_session_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_session_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_session_envelope(
                &envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_580_structured_edit_provider_batch_execution_receipt_replay_session_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_session_envelope_application",
    ));
    let envelope = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
    >(
        fixture["structured_edit_provider_batch_execution_receipt_replay_session_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplaySession>(
            fixture["expected_batch_receipt_replay_session"].clone(),
        )
        .expect("expected batch receipt replay session should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_receipt_replay_session_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_session_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_581_structured_edit_provider_execution_receipt_replay_workflow_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_workflow",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let receipt_replay_workflow = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayWorkflow,
        >(case["receipt_replay_workflow"].clone())
        .expect("receipt replay workflow should deserialize");
        let roundtrip =
            serde_json::to_value(&receipt_replay_workflow).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayWorkflow>(
                roundtrip,
            )
            .expect("roundtrip should deserialize");

        assert_eq!(
            serde_json::to_value(decoded).expect("decoded workflow should serialize"),
            serde_json::to_value(receipt_replay_workflow)
                .expect("expected workflow should serialize")
        );
    }
}

#[test]
fn conforms_to_slice_582_structured_edit_provider_execution_receipt_replay_workflow_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_workflow_envelope",
    ));
    let receipt_replay_workflow =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayWorkflow>(
            fixture["structured_edit_provider_execution_receipt_replay_workflow"].clone(),
        )
        .expect("receipt replay workflow should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        serde_json::to_value(structured_edit_provider_execution_receipt_replay_workflow_envelope(
            &receipt_replay_workflow
        ))
        .expect("workflow envelope should serialize"),
        serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
    );
    assert_eq!(
        serde_json::to_value(
            import_structured_edit_provider_execution_receipt_replay_workflow_envelope(&expected)
                .expect("workflow envelope should import")
        )
        .expect("imported workflow should serialize"),
        serde_json::to_value(receipt_replay_workflow).expect("expected workflow should serialize")
    );
}

#[test]
fn conforms_to_slice_583_structured_edit_provider_execution_receipt_replay_workflow_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_workflow_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_workflow_envelope(&envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_584_structured_edit_provider_execution_receipt_replay_workflow_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_receipt_replay_workflow_envelope_application",
    ));
    let envelope =
        serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope>(
            fixture["structured_edit_provider_execution_receipt_replay_workflow_envelope"].clone(),
        )
        .expect("envelope should deserialize");
    let mut actual = serde_json::to_value(
        import_structured_edit_provider_execution_receipt_replay_workflow_envelope(&envelope)
            .expect("workflow envelope application should import"),
    )
    .expect("applied workflow should serialize");
    let mut expected = fixture["expected_receipt_replay_workflow"].clone();
    prune_empty_metadata(&mut actual);
    prune_empty_metadata(&mut expected);
    assert!(actual == expected, "workflow envelope application payload should match fixture");

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_receipt_replay_workflow_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_585_structured_edit_provider_batch_execution_receipt_replay_workflow_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_workflow",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_receipt_replay_workflow = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
        >(case["batch_receipt_replay_workflow"].clone())
        .expect("batch receipt replay workflow should deserialize");
        let roundtrip = serde_json::to_value(&batch_receipt_replay_workflow)
            .expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
        >(roundtrip)
        .expect("roundtrip should deserialize");

        assert_eq!(
            serde_json::to_value(decoded).expect("decoded batch workflow should serialize"),
            serde_json::to_value(batch_receipt_replay_workflow)
                .expect("expected batch workflow should serialize")
        );
    }
}

#[test]
fn conforms_to_slice_586_structured_edit_provider_batch_execution_receipt_replay_workflow_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_workflow_envelope",
    ));
    let batch_receipt_replay_workflow =
        serde_json::from_value::<StructuredEditProviderBatchExecutionReceiptReplayWorkflow>(
            fixture["structured_edit_provider_batch_execution_receipt_replay_workflow"].clone(),
        )
        .expect("batch receipt replay workflow should deserialize");
    let expected = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
    >(fixture["expected_envelope"].clone())
    .expect("envelope should deserialize");

    assert_eq!(
        serde_json::to_value(
            structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
                &batch_receipt_replay_workflow
            )
        )
        .expect("batch workflow envelope should serialize"),
        serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
    );
    assert_eq!(
        serde_json::to_value(
            import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
                &expected
            )
            .expect("batch workflow envelope should import")
        )
        .expect("imported batch workflow should serialize"),
        serde_json::to_value(batch_receipt_replay_workflow)
            .expect("expected batch workflow should serialize")
    );
}

#[test]
fn conforms_to_slice_587_structured_edit_provider_batch_execution_receipt_replay_workflow_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_workflow_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
                &envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_588_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_receipt_replay_workflow_envelope_application",
    ));
    let envelope = serde_json::from_value::<
        StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
    >(
        fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_envelope"]
            .clone(),
    )
    .expect("envelope should deserialize");
    let mut actual = serde_json::to_value(
        import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(&envelope)
            .expect("batch workflow envelope application should import"),
    )
    .expect("applied batch workflow should serialize");
    let mut expected = fixture["expected_batch_receipt_replay_workflow"].clone();
    prune_empty_metadata(&mut actual);
    prune_empty_metadata(&mut expected);
    assert!(actual == expected, "batch workflow envelope application payload should match fixture");

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
                &rejected_envelope
            ),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_589_structured_edit_provider_execution_receipt_replay_workflow_result_fixture()
{
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_result",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowResult,
                    >(case["receipt_replay_workflow_result"].clone())
                    .expect("receipt replay workflow result should deserialize"),
                )
                .expect("receipt replay workflow result should serialize");
                let mut expected = case["receipt_replay_workflow_result"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "workflow result payload should match fixture");
            }
        })
        .expect("workflow result validation thread should spawn")
        .join()
        .expect("workflow result validation thread should complete");
}

#[test]
fn conforms_to_slice_590_structured_edit_provider_execution_receipt_replay_workflow_result_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_result_envelope",
            ));
            let receipt_replay_workflow_result =
                serde_json::from_value::<StructuredEditProviderExecutionReceiptReplayWorkflowResult>(
                    fixture["structured_edit_provider_execution_receipt_replay_workflow_result"]
                        .clone(),
                )
                .expect("receipt replay workflow result should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowResultEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_execution_receipt_replay_workflow_result_envelope(
                        &receipt_replay_workflow_result
                    )
                )
                .expect("workflow result envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_execution_receipt_replay_workflow_result_envelope(
                        &expected
                    )
                    .expect("workflow result envelope should import")
                )
                .expect("imported workflow result should serialize"),
                serde_json::to_value(receipt_replay_workflow_result)
                    .expect("expected workflow result should serialize")
            );
        })
        .expect("workflow result envelope thread should spawn")
        .join()
        .expect("workflow result envelope thread should complete");
}

#[test]
fn conforms_to_slice_591_structured_edit_provider_execution_receipt_replay_workflow_result_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_result_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_result_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("workflow result rejection thread should spawn")
        .join()
        .expect("workflow result rejection thread should complete");
}

#[test]
fn conforms_to_slice_592_structured_edit_provider_execution_receipt_replay_workflow_result_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_result_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowResultEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_result_envelope"]
                    .clone(),
            )
            .expect("envelope should deserialize");
            let expected_payload = fixture["expected_receipt_replay_workflow_result"].clone();
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_result_envelope(
                    &envelope,
                )
                .expect("workflow result envelope application should import"),
            )
            .expect("applied workflow result should serialize");
            let mut expected = expected_payload.clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "workflow result envelope application payload should match fixture"
            );
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_result_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("workflow result application thread should spawn")
        .join()
        .expect("workflow result application thread should complete");
}

#[test]
fn conforms_to_slice_593_structured_edit_provider_batch_execution_receipt_replay_workflow_result_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_result",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowResult,
                    >(case["batch_receipt_replay_workflow_result"].clone())
                    .expect("batch receipt replay workflow result should deserialize"),
                )
                .expect("batch receipt replay workflow result should serialize");
                let mut expected = case["batch_receipt_replay_workflow_result"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch workflow result payload should match fixture");
            }
        })
        .expect("batch workflow result thread should spawn")
        .join()
        .expect("batch workflow result thread should complete");
}

#[test]
fn conforms_to_slice_597_structured_edit_provider_execution_receipt_replay_workflow_review_request_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_review_request",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequest,
                    >(case["receipt_replay_workflow_review_request"].clone())
                    .expect("receipt replay workflow review request should deserialize"),
                )
                .expect("receipt replay workflow review request should serialize");
                let mut expected = case["receipt_replay_workflow_review_request"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "workflow review request payload should match fixture");
            }
        })
        .expect("workflow review request thread should spawn")
        .join()
        .expect("workflow review request thread should complete");
}

#[test]
fn conforms_to_slice_605_structured_edit_provider_execution_receipt_replay_workflow_apply_request_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_request",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequest,
                    >(case["receipt_replay_workflow_apply_request"].clone())
                    .expect("receipt replay workflow apply request should deserialize"),
                )
                .expect("receipt replay workflow apply request should serialize");
                let mut expected = case["receipt_replay_workflow_apply_request"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "workflow apply request payload should match fixture");
            }
        })
        .expect("workflow apply request thread should spawn")
        .join()
        .expect("workflow apply request thread should complete");
}

#[test]
fn conforms_to_slice_613_structured_edit_provider_execution_receipt_replay_workflow_apply_session_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_session",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplySession,
                    >(case["receipt_replay_workflow_apply_session"].clone())
                    .expect("receipt replay workflow apply session should deserialize"),
                )
                .expect("receipt replay workflow apply session should serialize");
                let mut expected = case["receipt_replay_workflow_apply_session"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "workflow apply session payload should match fixture");
            }
        })
        .expect("workflow apply session thread should spawn")
        .join()
        .expect("workflow apply session thread should complete");
}

#[test]
fn conforms_to_slice_621_structured_edit_provider_execution_receipt_replay_workflow_apply_result_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_result",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyResult,
                    >(case["receipt_replay_workflow_apply_result"].clone())
                    .expect("receipt replay workflow apply result should deserialize"),
                )
                .expect("receipt replay workflow apply result should serialize");
                let mut expected = case["receipt_replay_workflow_apply_result"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "workflow apply result payload should match fixture");
            }
        })
        .expect("workflow apply result thread should spawn")
        .join()
        .expect("workflow apply result thread should complete");
}

#[test]
fn conforms_to_slice_622_structured_edit_provider_execution_receipt_replay_workflow_apply_result_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope",
            ));
            let apply_result = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyResult,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_result"].clone())
            .expect("apply result should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyResultEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope(
                    &apply_result
                ),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope(&expected)
                    .expect("apply result envelope should import"),
                apply_result
            );
        })
        .expect("workflow apply result transport envelope thread should spawn")
        .join()
        .expect("workflow apply result transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_623_structured_edit_provider_execution_receipt_replay_workflow_apply_result_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope(&envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("workflow apply result transport rejection thread should spawn")
        .join()
        .expect("workflow apply result transport rejection thread should complete");
}

#[test]
fn conforms_to_slice_624_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyResultEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope"]
                    .clone(),
            )
            .expect("supported envelope should deserialize");
            let mut expected = fixture["expected_receipt_replay_workflow_apply_result"].clone();
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope(&envelope)
                    .expect("supported envelope should import"),
            )
            .expect("imported apply result should serialize");
            prune_empty_metadata(&mut expected);
            prune_empty_metadata(&mut actual);
            assert_eq!(actual, expected);

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_result_envelope(&envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("workflow apply result envelope application thread should spawn")
        .join()
        .expect("workflow apply result envelope application thread should complete");
}

#[test]
fn conforms_to_slice_614_structured_edit_provider_execution_receipt_replay_workflow_apply_session_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope",
            ));
            let apply_session = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplySession,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_session"].clone())
            .expect("apply session should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplySessionEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope(
                        &apply_session
                    )
                )
                .expect("apply session envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope(
                        &expected
                    )
                    .expect("apply session envelope should import")
                )
                .expect("imported apply session should serialize"),
                serde_json::to_value(apply_session).expect("expected apply session should serialize")
            );
        })
        .expect("apply session envelope thread should spawn")
        .join()
        .expect("apply session envelope thread should complete");
}

#[test]
fn conforms_to_slice_615_structured_edit_provider_execution_receipt_replay_workflow_apply_session_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplySessionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply session rejection thread should spawn")
        .join()
        .expect("apply session rejection thread should complete");
}

#[test]
fn conforms_to_slice_616_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplySessionEnvelope,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope(
                    &envelope,
                )
                .expect("apply session envelope application should import"),
            )
            .expect("applied apply session should serialize");
            let mut expected = fixture["expected_receipt_replay_workflow_apply_session"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "apply session envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplySessionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_session_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply session application thread should spawn")
        .join()
        .expect("apply session application thread should complete");
}

#[test]
fn conforms_to_slice_606_structured_edit_provider_execution_receipt_replay_workflow_apply_request_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope",
            ));
            let apply_request = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequest,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_request"]
                    .clone(),
            )
            .expect("apply request should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequestEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope(
                        &apply_request
                    )
                )
                .expect("apply request envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope(
                        &expected
                    )
                    .expect("apply request envelope should import")
                )
                .expect("imported apply request should serialize"),
                serde_json::to_value(apply_request).expect("expected apply request should serialize")
            );
        })
        .expect("apply request envelope thread should spawn")
        .join()
        .expect("apply request envelope thread should complete");
}

#[test]
fn conforms_to_slice_607_structured_edit_provider_execution_receipt_replay_workflow_apply_request_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply request rejection thread should spawn")
        .join()
        .expect("apply request rejection thread should complete");
}

#[test]
fn conforms_to_slice_608_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequestEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope"]
                    .clone(),
            )
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope(
                    &envelope,
                )
                .expect("apply request envelope application should import"),
            )
            .expect("applied apply request should serialize");
            let mut expected = fixture["expected_receipt_replay_workflow_apply_request"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "apply request envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_request_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply request application thread should spawn")
        .join()
        .expect("apply request application thread should complete");
}

#[test]
fn conforms_to_slice_609_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequest,
                    >(
                        case["batch_receipt_replay_workflow_apply_request"].clone()
                    )
                    .expect("batch apply request should deserialize"),
                )
                .expect("batch apply request should serialize");
                let mut expected = case["batch_receipt_replay_workflow_apply_request"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch apply request payload should match fixture");
            }
        })
        .expect("batch apply request thread should spawn")
        .join()
        .expect("batch apply request thread should complete");
}

#[test]
fn conforms_to_slice_610_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope",
            ));
            let apply_request = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequest,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request"].clone())
            .expect("batch apply request should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequestEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope(
                        &apply_request
                    )
                )
                .expect("batch apply request envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope(
                        &expected
                    )
                    .expect("batch apply request envelope should import")
                )
                .expect("imported batch apply request should serialize"),
                serde_json::to_value(apply_request).expect("expected batch apply request should serialize")
            );
        })
        .expect("batch apply request envelope thread should spawn")
        .join()
        .expect("batch apply request envelope thread should complete");
}

#[test]
fn conforms_to_slice_611_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply request rejection thread should spawn")
        .join()
        .expect("batch apply request rejection thread should complete");
}

#[test]
fn conforms_to_slice_612_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequestEnvelope,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope(
                    &envelope,
                )
                .expect("batch apply request envelope application should import"),
            )
            .expect("applied batch apply request should serialize");
            let mut expected = fixture["expected_batch_receipt_replay_workflow_apply_request"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch apply request envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_request_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply request application thread should spawn")
        .join()
        .expect("batch apply request application thread should complete");
}

#[test]
fn conforms_to_slice_617_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySession,
                    >(
                        case["batch_receipt_replay_workflow_apply_session"].clone()
                    )
                    .expect("batch apply session should deserialize"),
                )
                .expect("batch apply session should serialize");
                let mut expected = case["batch_receipt_replay_workflow_apply_session"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch apply session payload should match fixture");
            }
        })
        .expect("batch apply session thread should spawn")
        .join()
        .expect("batch apply session thread should complete");
}

#[test]
fn conforms_to_slice_618_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope",
            ));
            let apply_session = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySession,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session"].clone())
            .expect("batch apply session should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySessionEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope(
                        &apply_session
                    )
                )
                .expect("batch apply session envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope(
                        &expected
                    )
                    .expect("batch apply session envelope should import")
                )
                .expect("imported batch apply session should serialize"),
                serde_json::to_value(apply_session).expect("expected batch apply session should serialize")
            );
        })
        .expect("batch apply session envelope thread should spawn")
        .join()
        .expect("batch apply session envelope thread should complete");
}

#[test]
fn conforms_to_slice_619_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySessionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply session rejection thread should spawn")
        .join()
        .expect("batch apply session rejection thread should complete");
}

#[test]
fn conforms_to_slice_620_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySessionEnvelope,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope(
                    &envelope,
                )
                .expect("batch apply session envelope application should import"),
            )
            .expect("applied batch apply session should serialize");
            let mut expected = fixture["expected_batch_receipt_replay_workflow_apply_session"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch apply session envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplySessionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_session_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply session application thread should spawn")
        .join()
        .expect("batch apply session application thread should complete");
}

#[test]
fn conforms_to_slice_625_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResult,
                    >(
                        case["batch_receipt_replay_workflow_apply_result"].clone()
                    )
                    .expect("batch apply result should deserialize"),
                )
                .expect("batch apply result should serialize");
                let mut expected = case["batch_receipt_replay_workflow_apply_result"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch apply result payload should match fixture");
            }
        })
        .expect("batch apply result thread should spawn")
        .join()
        .expect("batch apply result thread should complete");
}

#[test]
fn conforms_to_slice_626_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope",
            ));
            let apply_result = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResult,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result"].clone())
            .expect("batch apply result should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResultEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope(
                    &apply_result
                ),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope(
                    &expected
                )
                .expect("batch apply result envelope should import"),
                apply_result
            );
        })
        .expect("batch apply result envelope thread should spawn")
        .join()
        .expect("batch apply result envelope thread should complete");
}

#[test]
fn conforms_to_slice_627_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply result rejection thread should spawn")
        .join()
        .expect("batch apply result rejection thread should complete");
}

#[test]
fn conforms_to_slice_628_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResultEnvelope,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope(
                    &envelope,
                )
                .expect("batch apply result envelope application should import"),
            )
            .expect("applied batch apply result should serialize");
            let mut expected = fixture["expected_batch_receipt_replay_workflow_apply_result"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch apply result envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_result_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply result application thread should spawn")
        .join()
        .expect("batch apply result application thread should complete");
}

#[test]
fn conforms_to_slice_629_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecision,
                    >(case["receipt_replay_workflow_apply_decision"].clone())
                    .expect("apply decision should deserialize"),
                )
                .expect("apply decision should serialize");
                let mut expected = case["receipt_replay_workflow_apply_decision"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "apply decision payload should match fixture");
            }
        })
        .expect("apply decision thread should spawn")
        .join()
        .expect("apply decision thread should complete");
}

#[test]
fn conforms_to_slice_637_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcome,
                    >(
                        case["receipt_replay_workflow_apply_decision_outcome"].clone()
                    )
                    .expect("apply decision outcome should deserialize"),
                )
                .expect("apply decision outcome should serialize");
                let mut expected = case["receipt_replay_workflow_apply_decision_outcome"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "apply decision outcome payload should match fixture");
            }
        })
        .expect("apply decision outcome thread should spawn")
        .join()
        .expect("apply decision outcome thread should complete");
}

#[test]
fn conforms_to_slice_645_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlement,
                    >(
                        case["receipt_replay_workflow_apply_decision_settlement"].clone()
                    )
                    .expect("apply decision settlement should deserialize"),
                )
                .expect("apply decision settlement should serialize");
                let mut expected =
                    case["receipt_replay_workflow_apply_decision_settlement"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(
                    actual == expected,
                    "apply decision settlement payload should match fixture"
                );
            }
        })
        .expect("apply decision settlement thread should spawn")
        .join()
        .expect("apply decision settlement thread should complete");
}

#[test]
fn conforms_to_slice_653_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
                    >(
                        case["receipt_replay_workflow_apply_decision_confirmation"].clone()
                    )
                    .expect("apply decision confirmation should deserialize"),
                )
                .expect("apply decision confirmation should serialize");
                let mut expected =
                    case["receipt_replay_workflow_apply_decision_confirmation"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(
                    actual == expected,
                    "apply decision confirmation payload should match fixture"
                );
            }
        })
        .expect("apply decision confirmation thread should spawn")
        .join()
        .expect("apply decision confirmation thread should complete");
}

#[test]
fn conforms_to_slice_654_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope",
            ));
            let confirmation = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation"].clone(),
            )
            .expect("apply decision confirmation should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&confirmation),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&expected)
                    .expect("apply decision confirmation envelope should import"),
                confirmation
            );
        })
        .expect("apply decision confirmation transport envelope thread should spawn")
        .join()
        .expect("apply decision confirmation transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_655_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
                >(case["envelope"].clone())
                .expect("envelope should deserialize");
                let expected = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&envelope)
                    .expect_err("confirmation envelope import should fail");
                assert_eq!(actual, expected);
            }
        })
        .expect("apply decision confirmation rejection thread should spawn")
        .join()
        .expect("apply decision confirmation rejection thread should complete");
}

#[test]
fn conforms_to_slice_656_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope"].clone(),
            )
            .expect("envelope should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
            >(
                fixture["expected_receipt_replay_workflow_apply_decision_confirmation"].clone(),
            )
            .expect("expected confirmation should deserialize");

            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&envelope)
                    .expect("apply decision confirmation envelope should import"),
                expected
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&rejected)
                    .expect_err("confirmation envelope application rejection should fail");
                assert_eq!(actual, expected);
            }
        })
        .expect("apply decision confirmation application thread should spawn")
        .join()
        .expect("apply decision confirmation application thread should complete");
}

#[test]
fn conforms_to_slice_657_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
                    >(
                        case["batch_receipt_replay_workflow_apply_decision_confirmation"].clone()
                    )
                    .expect("batch apply decision confirmation should deserialize"),
                )
                .expect("batch apply decision confirmation should serialize");
                let mut expected =
                    case["batch_receipt_replay_workflow_apply_decision_confirmation"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(
                    actual == expected,
                    "batch apply decision confirmation payload should match fixture"
                );
            }
        })
        .expect("batch apply decision confirmation thread should spawn")
        .join()
        .expect("batch apply decision confirmation thread should complete");
}

#[test]
fn conforms_to_slice_658_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope",
            ));
            let confirmation = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
            >(
                fixture["batch_receipt_replay_workflow_apply_decision_confirmation"].clone(),
            )
            .expect("batch apply decision confirmation should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&confirmation),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&expected)
                    .expect("batch apply decision confirmation envelope should import"),
                confirmation
            );
        })
        .expect("batch apply decision confirmation transport envelope thread should spawn")
        .join()
        .expect("batch apply decision confirmation transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_659_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
                >(case["envelope"].clone())
                .expect("envelope should deserialize");
                let expected = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&envelope)
                    .expect_err("batch confirmation envelope import should fail");
                assert_eq!(actual, expected);
            }
        })
        .expect("batch apply decision confirmation rejection thread should spawn")
        .join()
        .expect("batch apply decision confirmation rejection thread should complete");
}

#[test]
fn conforms_to_slice_660_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
            >(
                fixture["batch_receipt_replay_workflow_apply_decision_confirmation_envelope"].clone(),
            )
            .expect("envelope should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmation,
            >(
                fixture["expected_batch_receipt_replay_workflow_apply_decision_confirmation"].clone(),
            )
            .expect("expected batch confirmation should deserialize");

            assert_eq!(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&envelope)
                    .expect("batch apply decision confirmation envelope should import"),
                expected
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionConfirmationEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_confirmation_envelope(&rejected)
                    .expect_err("batch confirmation envelope application rejection should fail");
                assert_eq!(actual, expected);
            }
        })
        .expect("batch apply decision confirmation application thread should spawn")
        .join()
        .expect("batch apply decision confirmation application thread should complete");
}

#[test]
fn conforms_to_slice_646_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope",
            ));
            let settlement = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlement,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement"].clone())
            .expect("apply decision settlement should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlementEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope(&settlement),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope(&expected)
                    .expect("apply decision settlement envelope should import"),
                settlement
            );
        })
        .expect("apply decision settlement transport envelope thread should spawn")
        .join()
        .expect("apply decision settlement transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_647_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlementEnvelope,
                >(case["envelope"].clone())
                .expect("rejection envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope(&envelope)
                    .expect_err("rejection case should fail");
                assert_eq!(actual, expected_error);
            }
        })
        .expect("apply decision settlement rejection thread should spawn")
        .join()
        .expect("apply decision settlement rejection thread should complete");
}

#[test]
fn conforms_to_slice_648_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlementEnvelope,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope"].clone())
            .expect("application envelope should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlement,
            >(fixture["expected_receipt_replay_workflow_apply_decision_settlement"].clone())
            .expect("expected settlement should deserialize");

            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope(&envelope)
                    .expect("application envelope should import"),
                expected
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionSettlementEnvelope,
                >(case["envelope"].clone())
                .expect("rejection envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                let actual = import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_settlement_envelope(&rejected)
                    .expect_err("application rejection case should fail");
                assert_eq!(actual, expected_error);
            }
        })
        .expect("apply decision settlement application thread should spawn")
        .join()
        .expect("apply decision settlement application thread should complete");
}

#[test]
fn conforms_to_slice_638_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope",
            ));
            let outcome = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcome,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome"].clone())
            .expect("apply decision outcome should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&outcome),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&expected)
                    .expect("apply decision outcome envelope should import"),
                outcome
            );
        })
        .expect("apply decision outcome transport envelope thread should spawn")
        .join()
        .expect("apply decision outcome transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_639_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("apply decision outcome transport rejection thread should spawn")
        .join()
        .expect("apply decision outcome transport rejection thread should complete");
}

#[test]
fn conforms_to_slice_640_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope"]
                    .clone(),
            )
            .expect("supported envelope should deserialize");
            let mut expected =
                fixture["expected_receipt_replay_workflow_apply_decision_outcome"].clone();
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&envelope)
                    .expect("supported envelope should import"),
            )
            .expect("imported apply decision outcome should serialize");
            prune_empty_metadata(&mut expected);
            prune_empty_metadata(&mut actual);
            assert_eq!(actual, expected);

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("apply decision outcome envelope application thread should spawn")
        .join()
        .expect("apply decision outcome envelope application thread should complete");
}

#[test]
fn conforms_to_slice_630_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope",
            ));
            let apply_decision = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecision,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision"]
                    .clone(),
            )
            .expect("apply decision should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope(
                        &apply_decision
                    )
                )
                .expect("apply decision envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope(
                        &expected
                    )
                    .expect("apply decision envelope should import")
                )
                .expect("imported apply decision should serialize"),
                serde_json::to_value(apply_decision).expect("expected apply decision should serialize")
            );
        })
        .expect("apply decision envelope thread should spawn")
        .join()
        .expect("apply decision envelope thread should complete");
}

#[test]
fn conforms_to_slice_631_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply decision rejection thread should spawn")
        .join()
        .expect("apply decision rejection thread should complete");
}

#[test]
fn conforms_to_slice_632_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
            >(fixture["structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope(
                    &envelope,
                )
                .expect("apply decision envelope application should import"),
            )
            .expect("applied apply decision should serialize");
            let mut expected =
                fixture["expected_receipt_replay_workflow_apply_decision"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "apply decision envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_apply_decision_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("apply decision application thread should spawn")
        .join()
        .expect("apply decision application thread should complete");
}

#[test]
fn conforms_to_slice_633_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecision,
                    >(
                        case["batch_receipt_replay_workflow_apply_decision"].clone()
                    )
                    .expect("batch apply decision should deserialize"),
                )
                .expect("batch apply decision should serialize");
                let mut expected = case["batch_receipt_replay_workflow_apply_decision"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch apply decision payload should match fixture");
            }
        })
        .expect("batch apply decision thread should spawn")
        .join()
        .expect("batch apply decision thread should complete");
}

#[test]
fn conforms_to_slice_634_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope",
            ));
            let batch_apply_decision = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecision,
            >(
                fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision"]
                    .clone(),
            )
            .expect("batch apply decision should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope(
                        &batch_apply_decision
                    )
                )
                .expect("batch apply decision envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope(
                        &expected
                    )
                    .expect("batch apply decision envelope should import")
                )
                .expect("imported batch apply decision should serialize"),
                serde_json::to_value(batch_apply_decision).expect("expected batch apply decision should serialize")
            );
        })
        .expect("batch apply decision envelope thread should spawn")
        .join()
        .expect("batch apply decision envelope thread should complete");
}

#[test]
fn conforms_to_slice_635_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply decision rejection thread should spawn")
        .join()
        .expect("batch apply decision rejection thread should complete");
}

#[test]
fn conforms_to_slice_636_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope"].clone())
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope(
                    &envelope,
                )
                .expect("batch apply decision envelope application should import"),
            )
            .expect("applied batch apply decision should serialize");
            let mut expected =
                fixture["expected_batch_receipt_replay_workflow_apply_decision"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch apply decision envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch apply decision application thread should spawn")
        .join()
        .expect("batch apply decision application thread should complete");
}

#[test]
fn conforms_to_slice_641_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcome,
                    >(case["batch_receipt_replay_workflow_apply_decision_outcome"].clone())
                    .expect("batch apply decision outcome should deserialize"),
                )
                .expect("batch apply decision outcome should serialize");
                let mut expected =
                    case["batch_receipt_replay_workflow_apply_decision_outcome"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(
                    actual == expected,
                    "batch apply decision outcome payload should match fixture"
                );
            }
        })
        .expect("batch apply decision outcome thread should spawn")
        .join()
        .expect("batch apply decision outcome thread should complete");
}

#[test]
fn conforms_to_slice_642_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope",
            ));
            let batch_outcome = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcome,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome"].clone())
            .expect("batch apply decision outcome should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&batch_outcome),
                expected
            );
            assert_eq!(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&expected)
                    .expect("batch apply decision outcome envelope should import"),
                batch_outcome
            );
        })
        .expect("batch apply decision outcome transport envelope thread should spawn")
        .join()
        .expect("batch apply decision outcome transport envelope thread should complete");
}

#[test]
fn conforms_to_slice_643_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("batch apply decision outcome rejection thread should spawn")
        .join()
        .expect("batch apply decision outcome rejection thread should complete");
}

#[test]
fn conforms_to_slice_644_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
            >(fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope"].clone())
            .expect("supported envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&envelope)
                    .expect("batch apply decision outcome envelope application should import"),
            )
            .expect("applied batch apply decision outcome should serialize");
            let mut expected =
                fixture["expected_batch_receipt_replay_workflow_apply_decision_outcome"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch apply decision outcome envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowApplyDecisionOutcomeEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_apply_decision_outcome_envelope(&rejected_envelope)
                        .expect_err("rejected envelope should fail"),
                    expected_error
                );
            }
        })
        .expect("batch apply decision outcome application thread should spawn")
        .join()
        .expect("batch apply decision outcome application thread should complete");
}

#[test]
fn conforms_to_slice_598_structured_edit_provider_execution_receipt_replay_workflow_review_request_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope",
            ));
            let review_request = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequest,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_review_request"]
                    .clone(),
            )
            .expect("review request should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequestEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope(
                        &review_request
                    )
                )
                .expect("review request envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope(
                        &expected
                    )
                    .expect("review request envelope should import")
                )
                .expect("imported review request should serialize"),
                serde_json::to_value(review_request).expect("expected review request should serialize")
            );
        })
        .expect("review request envelope thread should spawn")
        .join()
        .expect("review request envelope thread should complete");
}

#[test]
fn conforms_to_slice_599_structured_edit_provider_execution_receipt_replay_workflow_review_request_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("review request rejection thread should spawn")
        .join()
        .expect("review request rejection thread should complete");
}

#[test]
fn conforms_to_slice_600_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequestEnvelope,
            >(
                fixture["structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope"]
                    .clone(),
            )
            .expect("envelope should deserialize");
            let expected_payload = fixture["expected_receipt_replay_workflow_review_request"].clone();
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope(
                    &envelope,
                )
                .expect("review request envelope application should import"),
            )
            .expect("applied review request should serialize");
            let mut expected = expected_payload.clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "review request envelope application payload should match fixture"
            );
            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderExecutionReceiptReplayWorkflowReviewRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_execution_receipt_replay_workflow_review_request_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("review request application thread should spawn")
        .join()
        .expect("review request application thread should complete");
}

#[test]
fn conforms_to_slice_601_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_review_request",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let mut actual = serde_json::to_value(
                    serde_json::from_value::<
                        StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequest,
                    >(
                        case["batch_receipt_replay_workflow_review_request"].clone()
                    )
                    .expect("batch review request should deserialize"),
                )
                .expect("batch review request should serialize");
                let mut expected = case["batch_receipt_replay_workflow_review_request"].clone();
                prune_empty_metadata(&mut actual);
                prune_empty_metadata(&mut expected);
                assert!(actual == expected, "batch review request payload should match fixture");
            }
        })
        .expect("batch review request thread should spawn")
        .join()
        .expect("batch review request thread should complete");
}

#[test]
fn conforms_to_slice_602_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope",
            ));
            let batch_review_request = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequest,
            >(
                fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_review_request"]
                    .clone(),
            )
            .expect("batch review request should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequestEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope(
                        &batch_review_request
                    )
                )
                .expect("batch review request envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope(
                        &expected
                    )
                    .expect("batch review request envelope should import")
                )
                .expect("imported batch review request should serialize"),
                serde_json::to_value(batch_review_request)
                    .expect("expected batch review request should serialize")
            );
        })
        .expect("batch review request envelope thread should spawn")
        .join()
        .expect("batch review request envelope thread should complete");
}

#[test]
fn conforms_to_slice_603_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch review request rejection thread should spawn")
        .join()
        .expect("batch review request rejection thread should complete");
}

#[test]
fn conforms_to_slice_604_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequestEnvelope,
            >(
                fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope"]
                    .clone(),
            )
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope(
                    &envelope,
                )
                .expect("batch review request envelope application should import"),
            )
            .expect("applied batch review request should serialize");
            let mut expected = fixture["expected_batch_receipt_replay_workflow_review_request"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch review request envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowReviewRequestEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_review_request_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch review request application thread should spawn")
        .join()
        .expect("batch review request application thread should complete");
}

#[test]
fn conforms_to_slice_594_structured_edit_provider_batch_execution_receipt_replay_workflow_result_transport_envelope_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope",
            ));
            let batch_receipt_replay_workflow_result = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowResult,
            >(
                fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_result"]
                    .clone(),
            )
            .expect("batch receipt replay workflow result should deserialize");
            let expected = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowResultEnvelope,
            >(fixture["expected_envelope"].clone())
            .expect("envelope should deserialize");

            assert_eq!(
                serde_json::to_value(
                    structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope(
                        &batch_receipt_replay_workflow_result
                    )
                )
                .expect("batch workflow result envelope should serialize"),
                serde_json::to_value(expected.clone()).expect("expected envelope should serialize")
            );
            assert_eq!(
                serde_json::to_value(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope(
                        &expected
                    )
                    .expect("batch workflow result envelope should import")
                )
                .expect("imported batch workflow result should serialize"),
                serde_json::to_value(batch_receipt_replay_workflow_result)
                    .expect("expected batch workflow result should serialize")
            );
        })
        .expect("batch workflow result envelope thread should spawn")
        .join()
        .expect("batch workflow result envelope thread should complete");
}

#[test]
fn conforms_to_slice_595_structured_edit_provider_batch_execution_receipt_replay_workflow_result_transport_rejection_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope_rejection",
            ));
            let cases = fixture["cases"].as_array().expect("cases should be an array");

            for case in cases {
                let envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope(
                        &envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch workflow result rejection thread should spawn")
        .join()
        .expect("batch workflow result rejection thread should complete");
}

#[test]
fn conforms_to_slice_596_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope_application_fixture()
 {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let fixture = read_fixture_from_path(diagnostics_fixture_path(
                "structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope_application",
            ));
            let envelope = serde_json::from_value::<
                StructuredEditProviderBatchExecutionReceiptReplayWorkflowResultEnvelope,
            >(
                fixture["structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope"]
                    .clone(),
            )
            .expect("envelope should deserialize");
            let mut actual = serde_json::to_value(
                import_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope(
                    &envelope,
                )
                .expect("batch workflow result envelope application should import"),
            )
            .expect("applied batch workflow result should serialize");
            let mut expected = fixture["expected_batch_receipt_replay_workflow_result"].clone();
            prune_empty_metadata(&mut actual);
            prune_empty_metadata(&mut expected);
            assert!(
                actual == expected,
                "batch workflow result envelope application payload should match fixture"
            );

            let cases = fixture["cases"].as_array().expect("cases should be an array");
            for case in cases {
                let rejected_envelope = serde_json::from_value::<
                    StructuredEditProviderBatchExecutionReceiptReplayWorkflowResultEnvelope,
                >(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
                let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
                    case["expected_error"].clone(),
                )
                .expect("expected error should deserialize");

                assert_eq!(
                    import_structured_edit_provider_batch_execution_receipt_replay_workflow_result_envelope(
                        &rejected_envelope
                    ),
                    Err(expected_error)
                );
            }
        })
        .expect("batch workflow result application thread should spawn")
        .join()
        .expect("batch workflow result application thread should complete");
}

#[test]
fn conforms_to_slice_529_structured_edit_provider_batch_execution_handoff_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_handoff",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_execution_handoff = serde_json::from_value::<
            StructuredEditProviderBatchExecutionHandoff,
        >(case["batch_execution_handoff"].clone())
        .expect("batch execution handoff should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_execution_handoff).expect("roundtrip should serialize");
        let decoded =
            serde_json::from_value::<StructuredEditProviderBatchExecutionHandoff>(roundtrip)
                .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_execution_handoff);
    }
}

#[test]
fn conforms_to_slice_530_structured_edit_provider_batch_execution_handoff_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_handoff_envelope",
    ));
    let batch_execution_handoff =
        serde_json::from_value::<StructuredEditProviderBatchExecutionHandoff>(
            fixture["structured_edit_provider_batch_execution_handoff"].clone(),
        )
        .expect("batch execution handoff should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionHandoffEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_handoff_envelope(&batch_execution_handoff),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_handoff_envelope(&expected),
        Ok(batch_execution_handoff)
    );
}

#[test]
fn conforms_to_slice_531_structured_edit_provider_batch_execution_handoff_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_handoff_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionHandoffEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_handoff_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_532_structured_edit_provider_batch_execution_handoff_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_handoff_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionHandoffEnvelope>(
        fixture["structured_edit_provider_batch_execution_handoff_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionHandoff>(
        fixture["expected_batch_execution_handoff"].clone(),
    )
    .expect("batch execution handoff should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_handoff_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionHandoffEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_handoff_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_518_structured_edit_provider_execution_plan_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_plan_envelope",
    ));
    let execution_plan = serde_json::from_value::<StructuredEditProviderExecutionPlan>(
        fixture["structured_edit_provider_execution_plan"].clone(),
    )
    .expect("execution plan should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionPlanEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_plan_envelope(&execution_plan), expected);
    assert_eq!(
        import_structured_edit_provider_execution_plan_envelope(&expected),
        Ok(execution_plan)
    );
}

#[test]
fn conforms_to_slice_519_structured_edit_provider_execution_plan_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_plan_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderExecutionPlanEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_plan_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_520_structured_edit_provider_execution_plan_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_plan_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionPlanEnvelope>(
        fixture["structured_edit_provider_execution_plan_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionPlan>(
        fixture["expected_execution_plan"].clone(),
    )
    .expect("execution plan should deserialize");

    assert_eq!(import_structured_edit_provider_execution_plan_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope =
            serde_json::from_value::<StructuredEditProviderExecutionPlanEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_plan_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_521_structured_edit_provider_batch_execution_plan_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_plan",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch_execution_plan =
            serde_json::from_value::<StructuredEditProviderBatchExecutionPlan>(
                case["batch_execution_plan"].clone(),
            )
            .expect("batch execution plan should deserialize");
        let roundtrip =
            serde_json::to_value(&batch_execution_plan).expect("roundtrip should serialize");
        let decoded = serde_json::from_value::<StructuredEditProviderBatchExecutionPlan>(roundtrip)
            .expect("roundtrip should deserialize");

        assert_eq!(decoded, batch_execution_plan);
    }
}

#[test]
fn conforms_to_slice_522_structured_edit_provider_batch_execution_plan_transport_envelope_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_plan_envelope",
    ));
    let batch_execution_plan = serde_json::from_value::<StructuredEditProviderBatchExecutionPlan>(
        fixture["structured_edit_provider_batch_execution_plan"].clone(),
    )
    .expect("batch execution plan should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionPlanEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(
        structured_edit_provider_batch_execution_plan_envelope(&batch_execution_plan),
        expected
    );
    assert_eq!(
        import_structured_edit_provider_batch_execution_plan_envelope(&expected),
        Ok(batch_execution_plan)
    );
}

#[test]
fn conforms_to_slice_523_structured_edit_provider_batch_execution_plan_transport_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_plan_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionPlanEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_plan_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_524_structured_edit_provider_batch_execution_plan_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_plan_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionPlanEnvelope>(
        fixture["structured_edit_provider_batch_execution_plan_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionPlan>(
        fixture["expected_batch_execution_plan"].clone(),
    )
    .expect("batch execution plan should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_plan_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionPlanEnvelope,
        >(case["envelope"].clone())
        .expect("envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_plan_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_462_structured_edit_provider_execution_application_transport_envelope_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_application_envelope",
    ));
    let application = serde_json::from_value::<StructuredEditProviderExecutionApplication>(
        fixture["structured_edit_provider_execution_application"].clone(),
    )
    .expect("provider execution application should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionApplicationEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_execution_application_envelope(&application), expected);
    assert_eq!(
        import_structured_edit_provider_execution_application_envelope(&expected),
        Ok(application)
    );
}

#[test]
fn conforms_to_slice_463_structured_edit_provider_execution_application_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_application_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderExecutionApplicationEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_application_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_464_structured_edit_provider_execution_application_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_execution_application_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderExecutionApplicationEnvelope>(
        fixture["structured_edit_provider_execution_application_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderExecutionApplication>(
        fixture["expected_application"].clone(),
    )
    .expect("expected application should deserialize");

    assert_eq!(
        import_structured_edit_provider_execution_application_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderExecutionApplicationEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_execution_application_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_439_structured_edit_execution_report_transport_envelope_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_execution_report_envelope",
    ));
    let report = serde_json::from_value::<StructuredEditExecutionReport>(
        fixture["structured_edit_execution_report"].clone(),
    )
    .expect("report should deserialize");
    let expected = serde_json::from_value::<StructuredEditExecutionReportEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_execution_report_envelope(&report), expected);
    assert_eq!(import_structured_edit_execution_report_envelope(&expected), Ok(report));
}

#[test]
fn conforms_to_slice_440_structured_edit_execution_report_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_execution_report_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<StructuredEditExecutionReportEnvelope>(
            case["envelope"].clone(),
        )
        .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(import_structured_edit_execution_report_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_441_structured_edit_execution_report_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_execution_report_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditExecutionReportEnvelope>(
        fixture["structured_edit_execution_report_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected =
        serde_json::from_value::<StructuredEditExecutionReport>(fixture["expected_report"].clone())
            .expect("expected report should deserialize");

    assert_eq!(import_structured_edit_execution_report_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<StructuredEditExecutionReportEnvelope>(
            case["envelope"].clone(),
        )
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_execution_report_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_442_structured_edit_batch_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_batch_request"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch =
            serde_json::from_value::<StructuredEditBatchRequest>(case["batch_request"].clone())
                .expect("batch request should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditBatchRequest>(
            serde_json::to_value(&batch).expect("batch request should serialize"),
        )
        .expect("batch request should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_457_structured_edit_provider_batch_execution_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_request",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionRequest>(
            case["batch_execution_request"].clone(),
        )
        .expect("provider batch execution request should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderBatchExecutionRequest>(
            serde_json::to_value(&batch)
                .expect("provider batch execution request should serialize"),
        )
        .expect("provider batch execution request should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_458_structured_edit_provider_batch_execution_request_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_request_envelope",
    ));
    let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionRequest>(
        fixture["structured_edit_provider_batch_execution_request"].clone(),
    )
    .expect("provider batch execution request should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionRequestEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_batch_execution_request_envelope(&batch), expected);
    assert_eq!(
        import_structured_edit_provider_batch_execution_request_envelope(&expected),
        Ok(batch)
    );
}

#[test]
fn conforms_to_slice_459_structured_edit_provider_batch_execution_request_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_request_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionRequestEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_request_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_460_structured_edit_provider_batch_execution_request_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_request_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionRequestEnvelope>(
        fixture["structured_edit_provider_batch_execution_request_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionRequest>(
        fixture["expected_batch_execution_request"].clone(),
    )
    .expect("expected batch execution request should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_request_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionRequestEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_request_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_473_structured_edit_provider_batch_execution_dispatch_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_dispatch",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatch>(
            case["batch_dispatch"].clone(),
        )
        .expect("provider batch execution dispatch should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatch>(
            serde_json::to_value(&batch)
                .expect("provider batch execution dispatch should serialize"),
        )
        .expect("provider batch execution dispatch should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_474_structured_edit_provider_batch_execution_dispatch_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_dispatch_envelope",
    ));
    let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatch>(
        fixture["structured_edit_provider_batch_execution_dispatch"].clone(),
    )
    .expect("provider batch execution dispatch should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatchEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_batch_execution_dispatch_envelope(&batch), expected);
    assert_eq!(
        import_structured_edit_provider_batch_execution_dispatch_envelope(&expected),
        Ok(batch)
    );
}

#[test]
fn conforms_to_slice_475_structured_edit_provider_batch_execution_dispatch_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_dispatch_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionDispatchEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_dispatch_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_476_structured_edit_provider_batch_execution_dispatch_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_dispatch_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatchEnvelope>(
        fixture["structured_edit_provider_batch_execution_dispatch_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionDispatch>(
        fixture["expected_batch_dispatch"].clone(),
    )
    .expect("expected batch dispatch should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_dispatch_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionDispatchEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_dispatch_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_465_structured_edit_provider_batch_execution_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_report",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionReport>(
            case["batch_report"].clone(),
        )
        .expect("provider batch execution report should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditProviderBatchExecutionReport>(
            serde_json::to_value(&batch).expect("provider batch execution report should serialize"),
        )
        .expect("provider batch execution report should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_466_structured_edit_provider_batch_execution_report_transport_envelope_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_report_envelope",
    ));
    let batch = serde_json::from_value::<StructuredEditProviderBatchExecutionReport>(
        fixture["structured_edit_provider_batch_execution_report"].clone(),
    )
    .expect("provider batch execution report should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionReportEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_provider_batch_execution_report_envelope(&batch), expected);
    assert_eq!(
        import_structured_edit_provider_batch_execution_report_envelope(&expected),
        Ok(batch)
    );
}

#[test]
fn conforms_to_slice_467_structured_edit_provider_batch_execution_report_transport_rejection_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_report_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditProviderBatchExecutionReportEnvelope>(
                case["envelope"].clone(),
            )
            .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_report_envelope(&envelope),
            Err(expected)
        );
    }
}

#[test]
fn conforms_to_slice_468_structured_edit_provider_batch_execution_report_envelope_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_provider_batch_execution_report_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditProviderBatchExecutionReportEnvelope>(
        fixture["structured_edit_provider_batch_execution_report_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditProviderBatchExecutionReport>(
        fixture["expected_batch_report"].clone(),
    )
    .expect("expected batch report should deserialize");

    assert_eq!(
        import_structured_edit_provider_batch_execution_report_envelope(&envelope),
        Ok(expected)
    );

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope = serde_json::from_value::<
            StructuredEditProviderBatchExecutionReportEnvelope,
        >(case["envelope"].clone())
        .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_provider_batch_execution_report_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
}

#[test]
fn conforms_to_slice_443_structured_edit_batch_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("structured_edit_batch_report"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let batch =
            serde_json::from_value::<StructuredEditBatchReport>(case["batch_report"].clone())
                .expect("batch report should deserialize");
        let round_tripped = serde_json::from_value::<StructuredEditBatchReport>(
            serde_json::to_value(&batch).expect("batch report should serialize"),
        )
        .expect("batch report should deserialize after roundtrip");

        assert_eq!(round_tripped, batch);
    }
}

#[test]
fn conforms_to_slice_444_structured_edit_batch_report_transport_envelope_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("structured_edit_batch_report_envelope"));
    let batch_report = serde_json::from_value::<StructuredEditBatchReport>(
        fixture["structured_edit_batch_report"].clone(),
    )
    .expect("batch report should deserialize");
    let expected = serde_json::from_value::<StructuredEditBatchReportEnvelope>(
        fixture["expected_envelope"].clone(),
    )
    .expect("envelope should deserialize");

    assert_eq!(structured_edit_batch_report_envelope(&batch_report), expected);
    assert_eq!(import_structured_edit_batch_report_envelope(&expected), Ok(batch_report));
}

#[test]
fn conforms_to_slice_445_structured_edit_batch_report_transport_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_batch_report_envelope_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope =
            serde_json::from_value::<StructuredEditBatchReportEnvelope>(case["envelope"].clone())
                .expect("envelope should deserialize");
        let expected = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(import_structured_edit_batch_report_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_slice_446_structured_edit_batch_report_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "structured_edit_batch_report_envelope_application",
    ));
    let envelope = serde_json::from_value::<StructuredEditBatchReportEnvelope>(
        fixture["structured_edit_batch_report_envelope"].clone(),
    )
    .expect("envelope should deserialize");
    let expected = serde_json::from_value::<StructuredEditBatchReport>(
        fixture["expected_batch_report"].clone(),
    )
    .expect("expected batch report should deserialize");

    assert_eq!(import_structured_edit_batch_report_envelope(&envelope), Ok(expected));

    let cases = fixture["cases"].as_array().expect("cases should be an array");
    for case in cases {
        let rejected_envelope =
            serde_json::from_value::<StructuredEditBatchReportEnvelope>(case["envelope"].clone())
                .expect("rejected envelope should deserialize");
        let expected_error = serde_json::from_value::<StructuredEditTransportImportError>(
            case["expected_error"].clone(),
        )
        .expect("expected error should deserialize");

        assert_eq!(
            import_structured_edit_batch_report_envelope(&rejected_envelope),
            Err(expected_error)
        );
    }
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
fn conforms_to_slice_317_review_replay_bundle_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_application",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
        fixture["review_replay_bundle_envelope"].clone(),
    )
    .expect("replay bundle envelope should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest_with_replay_bundle_envelope(
        &manifest,
        &options,
        &envelope,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({ "outcome": "failed", "messages": ["missing execution"] }),
                ),
            )
            .expect("execution should deserialize")
        },
    );

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_318_explicit_review_replay_bundle_envelope_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "explicit_review_replay_bundle_envelope_application",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
        fixture["review_replay_bundle_envelope"].clone(),
    )
    .expect("replay bundle envelope should deserialize");
    let expected =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected state should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let state = review_conformance_manifest_with_replay_bundle_envelope(
        &manifest,
        &options,
        &envelope,
        |run| {
            let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
            serde_json::from_value::<ConformanceCaseExecution>(
                executions.get(&key).cloned().unwrap_or_else(
                    || serde_json::json!({ "outcome": "failed", "messages": ["missing execution"] }),
                ),
            )
            .expect("execution should deserialize")
        },
    );

    assert_eq!(state, expected);
}

#[test]
fn conforms_to_slice_319_review_replay_bundle_envelope_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_review_rejection",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("options should deserialize");
    let cases = fixture["cases"].as_array().expect("cases should be an array");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    for case in cases {
        let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
            case["review_replay_bundle_envelope"].clone(),
        )
        .expect("replay bundle envelope should deserialize");
        let expected = serde_json::from_value::<ConformanceManifestReviewState>(
            case["expected_state"].clone(),
        )
        .expect("expected state should deserialize");

        let state = review_conformance_manifest_with_replay_bundle_envelope(
            &manifest,
            &options,
            &envelope,
            |run| {
                let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
                serde_json::from_value::<ConformanceCaseExecution>(
                    executions.get(&key).cloned().unwrap_or_else(
                        || serde_json::json!({ "outcome": "failed", "messages": ["missing execution"] }),
                    ),
                )
                .expect("execution should deserialize")
            },
        );

        assert_eq!(state, expected);
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

#[test]
fn conforms_to_slice_307_review_replay_bundle_reviewed_nested_execution_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_reviewed_nested_execution_application",
    ));
    let bundle = serde_json::from_value::<ReviewReplayBundle>(fixture["replay_bundle"].clone())
        .expect("replay bundle should deserialize");
    let expected =
        fixture["expected_results"].as_array().expect("expected_results should be an array");

    let runs =
        execute_review_replay_bundle_reviewed_nested_executions(&bundle, |execution, index| {
            reviewed_nested_execution_callbacks_from_fixture(
                execution.clone(),
                expected[index]["result"]["output"].as_str().map(str::to_string),
            )
        });

    assert_reviewed_nested_execution_runs(&runs, expected);
}

#[test]
fn conforms_to_slice_308_review_state_reviewed_nested_execution_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_state_reviewed_nested_execution_application",
    ));
    let state =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["review_state"].clone())
            .expect("review state should deserialize");
    let expected =
        fixture["expected_results"].as_array().expect("expected_results should be an array");

    let runs = execute_review_state_reviewed_nested_executions(&state, |execution, index| {
        reviewed_nested_execution_callbacks_from_fixture(
            execution.clone(),
            expected[index]["result"]["output"].as_str().map(str::to_string),
        )
    });

    assert_reviewed_nested_execution_runs(&runs, expected);
}

#[test]
fn conforms_to_slice_320_review_replay_bundle_envelope_reviewed_nested_execution_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_reviewed_nested_execution_application",
    ));
    let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
        fixture["replay_bundle_envelope"].clone(),
    )
    .expect("replay bundle envelope should deserialize");
    let expected_application = fixture["expected_application"]
        .as_object()
        .expect("expected_application should be an object");
    let expected =
        expected_application["results"].as_array().expect("expected results should be an array");

    let application = execute_review_replay_bundle_envelope_reviewed_nested_executions(
        &envelope,
        |execution, index| {
            reviewed_nested_execution_callbacks_from_fixture(
                execution.clone(),
                expected[index]["result"]["output"].as_str().map(str::to_string),
            )
        },
    );

    assert!(application.diagnostics.is_empty());
    assert_reviewed_nested_execution_runs(&application.results, expected);
}

#[test]
fn conforms_to_slice_321_review_state_envelope_reviewed_nested_execution_application_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_state_envelope_reviewed_nested_execution_application",
    ));
    let envelope = serde_json::from_value::<ConformanceManifestReviewStateEnvelope>(
        fixture["review_state_envelope"].clone(),
    )
    .expect("review state envelope should deserialize");
    let expected_application = fixture["expected_application"]
        .as_object()
        .expect("expected_application should be an object");
    let expected =
        expected_application["results"].as_array().expect("expected results should be an array");

    let application =
        execute_review_state_envelope_reviewed_nested_executions(&envelope, |execution, index| {
            reviewed_nested_execution_callbacks_from_fixture(
                execution.clone(),
                expected[index]["result"]["output"].as_str().map(str::to_string),
            )
        });

    assert!(application.diagnostics.is_empty());
    assert_reviewed_nested_execution_runs(&application.results, expected);
}

#[test]
fn conforms_to_slice_322_review_replay_bundle_envelope_reviewed_nested_execution_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_reviewed_nested_execution_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
            case["replay_bundle_envelope"].clone(),
        )
        .expect("replay bundle envelope should deserialize");
        let expected_application = case["expected_application"]
            .as_object()
            .expect("expected_application should be an object");
        let expected_diagnostics = serde_json::from_value::<Vec<ast_merge::Diagnostic>>(
            expected_application["diagnostics"].clone(),
        )
        .expect("expected diagnostics should deserialize");

        let application =
            execute_review_replay_bundle_envelope_reviewed_nested_executions::<String, _, _, _, _>(
                &envelope,
                |execution, _| {
                    reviewed_nested_execution_callbacks_from_fixture(
                        execution.clone(),
                        Some("unused".to_string()),
                    )
                },
            );

        assert_eq!(application.diagnostics, expected_diagnostics);
        assert!(application.results.is_empty());
    }
}

#[test]
fn conforms_to_slice_323_review_state_envelope_reviewed_nested_execution_rejection_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_state_envelope_reviewed_nested_execution_rejection",
    ));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<ConformanceManifestReviewStateEnvelope>(
            case["review_state_envelope"].clone(),
        )
        .expect("review state envelope should deserialize");
        let expected_application = case["expected_application"]
            .as_object()
            .expect("expected_application should be an object");
        let expected_diagnostics = serde_json::from_value::<Vec<ast_merge::Diagnostic>>(
            expected_application["diagnostics"].clone(),
        )
        .expect("expected diagnostics should deserialize");

        let application =
            execute_review_state_envelope_reviewed_nested_executions::<String, _, _, _, _>(
                &envelope,
                |execution, _| {
                    reviewed_nested_execution_callbacks_from_fixture(
                        execution.clone(),
                        Some("unused".to_string()),
                    )
                },
            );

        assert_eq!(application.diagnostics, expected_diagnostics);
        assert!(application.results.is_empty());
    }
}

#[test]
fn conforms_to_slice_324_review_replay_bundle_envelope_reviewed_nested_manifest_application_fixture()
 {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_reviewed_nested_manifest_application",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("review options should deserialize");
    let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
        fixture["review_replay_bundle_envelope"].clone(),
    )
    .expect("replay bundle envelope should deserialize");
    let expected_state =
        serde_json::from_value::<ConformanceManifestReviewState>(fixture["expected_state"].clone())
            .expect("expected review state should deserialize");
    let expected_application = fixture["expected_application"]
        .as_object()
        .expect("expected_application should be an object");
    let expected =
        expected_application["results"].as_array().expect("expected results should be an array");
    let executions = fixture["executions"].as_object().expect("executions should be an object");

    let application: ConformanceManifestReviewedNestedApplication<String> =
        review_and_execute_conformance_manifest_with_replay_bundle_envelope(
            &manifest,
            &options,
            &envelope,
            |run| {
                let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
                serde_json::from_value::<ConformanceCaseExecution>(
                    executions.get(&key).cloned().unwrap_or_else(
                        || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                    ),
                )
                .expect("execution should deserialize")
            },
            |execution, index| {
                reviewed_nested_execution_callbacks_from_fixture(
                    execution.clone(),
                    expected[index]["result"]["output"].as_str().map(str::to_string),
                )
            },
        );

    assert_eq!(application.state, expected_state);
    assert_reviewed_nested_execution_runs(&application.results, expected);
}

#[test]
fn conforms_to_slice_325_review_replay_bundle_envelope_reviewed_nested_manifest_rejection_fixture()
{
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "review_replay_bundle_envelope_reviewed_nested_manifest_rejection",
    ));
    let manifest = serde_json::from_value::<ConformanceManifest>(fixture["manifest"].clone())
        .expect("manifest should deserialize");
    let options =
        serde_json::from_value::<ConformanceManifestReviewOptions>(fixture["options"].clone())
            .expect("review options should deserialize");
    let executions = fixture["executions"].as_object().expect("executions should be an object");
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let envelope = serde_json::from_value::<ReviewReplayBundleEnvelope>(
            case["review_replay_bundle_envelope"].clone(),
        )
        .expect("replay bundle envelope should deserialize");
        let expected_state = serde_json::from_value::<ConformanceManifestReviewState>(
            case["expected_state"].clone(),
        )
        .expect("expected review state should deserialize");
        let application: ConformanceManifestReviewedNestedApplication<String> =
            review_and_execute_conformance_manifest_with_replay_bundle_envelope(
                &manifest,
                &options,
                &envelope,
                |run| {
                    let key = format!("{}:{}:{}", run.ref_.family, run.ref_.role, run.ref_.case);
                    serde_json::from_value::<ConformanceCaseExecution>(
                        executions
                            .get(&key)
                            .cloned()
                            .unwrap_or_else(
                                || serde_json::json!({"outcome":"failed","messages":["missing execution"]}),
                            ),
                    )
                    .expect("execution should deserialize")
                },
                |execution, _| {
                    reviewed_nested_execution_callbacks_from_fixture(
                        execution.clone(),
                        Some("unused".to_string()),
                    )
                },
            );

        assert_eq!(
            application,
            ConformanceManifestReviewedNestedApplication {
                state: expected_state,
                results: Vec::new(),
            }
        );
    }
}

fn assert_reviewed_nested_execution_runs(
    runs: &[ast_merge::ReviewedNestedExecutionResult<String>],
    expected: &[Value],
) {
    assert_eq!(runs.len(), expected.len());

    for (run, expected_run) in runs.iter().zip(expected) {
        assert_eq!(
            run.execution.family,
            expected_run["execution_family"].as_str().expect("execution_family should be a string")
        );
        assert_eq!(
            run.result.ok,
            expected_run["result"]["ok"].as_bool().expect("result ok should be a bool")
        );
        assert_eq!(run.result.output.as_deref(), expected_run["result"]["output"].as_str());
        assert_eq!(
            run.result.diagnostics.len(),
            expected_run["result"]["diagnostics"]
                .as_array()
                .expect("diagnostics should be an array")
                .len()
        );
        assert_eq!(
            run.result.policies.len(),
            expected_run["result"]["policies"]
                .as_array()
                .expect("policies should be an array")
                .len()
        );
    }
}

#[test]
fn conforms_to_mini_template_tree_directory_plan_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_directory_plan_report",
    ));
    let fixture_dir = diagnostics_fixture_path("mini_template_tree_directory_plan_report")
        .parent()
        .expect("fixture should have parent")
        .to_path_buf();
    let context = serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
        .expect("context should deserialize");
    let default_strategy =
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize");
    let replacements =
        serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize");

    let execution_plan = ast_merge::plan_template_tree_execution_from_directories(
        &fixture_dir.join("template"),
        &fixture_dir.join("destination"),
        &context,
        default_strategy,
        &overrides,
        &replacements,
        &ast_merge::default_template_token_config(),
    )
    .expect("directory-backed plan should succeed");
    let actual = report_template_directory_plan(&execution_plan);
    let expected =
        serde_json::from_value(fixture["expected"].clone()).expect("expected should deserialize");
    assert_eq!(actual, expected);
}

#[test]
fn conforms_to_mini_template_tree_directory_runner_report_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path(
        "mini_template_tree_directory_runner_report",
    ));

    let dry_run = fixture["dry_run"].clone();
    let dry_run_dir = diagnostics_fixture_path("mini_template_tree_directory_runner_report")
        .parent()
        .expect("fixture should have parent")
        .join("dry-run");
    let dry_run_context =
        serde_json::from_value::<TemplateDestinationContext>(dry_run["context"].clone())
            .expect("context should deserialize");
    let dry_run_default_strategy =
        serde_json::from_value::<TemplateStrategy>(dry_run["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let dry_run_overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(dry_run["overrides"].clone())
            .expect("overrides should deserialize");
    let dry_run_replacements =
        serde_json::from_value::<HashMap<String, String>>(dry_run["replacements"].clone())
            .expect("replacements should deserialize");
    let dry_run_plan = ast_merge::plan_template_tree_execution_from_directories(
        &dry_run_dir.join("template"),
        &dry_run_dir.join("destination"),
        &dry_run_context,
        dry_run_default_strategy,
        &dry_run_overrides,
        &dry_run_replacements,
        &ast_merge::default_template_token_config(),
    )
    .expect("dry-run plan should succeed");
    let dry_run_actual = report_template_directory_runner(&dry_run_plan, None);
    let dry_run_expected =
        serde_json::from_value(dry_run["expected"].clone()).expect("expected should deserialize");
    assert_eq!(dry_run_actual, dry_run_expected);

    let apply_run = fixture["apply_run"].clone();
    let apply_run_dir = diagnostics_fixture_path("mini_template_tree_directory_runner_report")
        .parent()
        .expect("fixture should have parent")
        .join("apply-run");
    let apply_context =
        serde_json::from_value::<TemplateDestinationContext>(apply_run["context"].clone())
            .expect("context should deserialize");
    let apply_default_strategy =
        serde_json::from_value::<TemplateStrategy>(apply_run["default_strategy"].clone())
            .expect("default_strategy should deserialize");
    let apply_overrides =
        serde_json::from_value::<Vec<TemplateStrategyOverride>>(apply_run["overrides"].clone())
            .expect("overrides should deserialize");
    let apply_replacements =
        serde_json::from_value::<HashMap<String, String>>(apply_run["replacements"].clone())
            .expect("replacements should deserialize");
    let temp_root = repo_temp_dir();
    let destination_root = temp_root.join("destination");
    let initial_destination = read_relative_file_tree(&apply_run_dir.join("destination"));
    ast_merge::write_relative_file_tree(&destination_root, &initial_destination)
        .expect("destination tree should be writable");
    let apply_plan = ast_merge::plan_template_tree_execution_from_directories(
        &apply_run_dir.join("template"),
        &destination_root,
        &apply_context,
        apply_default_strategy,
        &apply_overrides,
        &apply_replacements,
        &ast_merge::default_template_token_config(),
    )
    .expect("apply-run plan should succeed");
    let apply_result = ast_merge::apply_template_tree_execution_to_directory(
        &apply_run_dir.join("template"),
        &destination_root,
        &apply_context,
        apply_default_strategy,
        &apply_overrides,
        &apply_replacements,
        multi_family_merge_callback,
        &ast_merge::default_template_token_config(),
    )
    .expect("apply-run execution should succeed");
    let apply_actual = report_template_directory_runner(&apply_plan, Some(&apply_result));
    let apply_expected =
        serde_json::from_value(apply_run["expected"].clone()).expect("expected should deserialize");
    assert_eq!(apply_actual, apply_expected);

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
}

fn reviewed_nested_execution_callbacks_from_fixture(
    execution: ReviewedNestedExecution,
    expected_output: Option<String>,
) -> ast_merge::NestedMergeExecutionCallbacks<
    String,
    impl Fn() -> ast_merge::MergeResult<String>,
    impl Fn(&String) -> ast_merge::NestedMergeDiscoveryResult,
    impl Fn(
        &String,
        &[ast_merge::DelegatedChildOperation],
        &ast_merge::DelegatedChildApplyPlan,
        &[ast_merge::AppliedDelegatedChildOutput],
    ) -> ast_merge::MergeResult<String>,
> {
    let family_for_merge = execution.family.clone();
    let family_for_discovery = execution.family.clone();
    let expected_applied_children = execution.applied_children.clone();
    let accepted_groups = execution.review_state.accepted_groups.clone();
    ast_merge::NestedMergeExecutionCallbacks {
        merge_parent: move || ast_merge::MergeResult {
            ok: true,
            diagnostics: vec![],
            output: Some(format!("{family_for_merge}-merged-parent")),
            policies: vec![],
        },
        discover_operations: move |_| ast_merge::NestedMergeDiscoveryResult {
            ok: true,
            diagnostics: vec![],
            operations: Some(
                accepted_groups
                    .iter()
                    .map(|group| match family_for_discovery.as_str() {
                        "markdown" => ast_merge::DelegatedChildOperation {
                            operation_id: group.child_operation_id.clone(),
                            parent_operation_id: group.parent_operation_id.clone(),
                            requested_strategy: "delegate_child_surface".to_string(),
                            language_chain: vec!["markdown".to_string(), "typescript".to_string()],
                            surface: ast_merge::DiscoveredSurface {
                                surface_kind: "fenced_code_block".to_string(),
                                declared_language: None,
                                effective_language: "typescript".to_string(),
                                address: group.delegated_runtime_surface_path.clone(),
                                parent_address: None,
                                span: None,
                                owner: ast_merge::SurfaceOwnerRef {
                                    kind: ast_merge::SurfaceOwnerKind::OwnedRegion,
                                    address: "/code_fence/0".to_string(),
                                },
                                reconstruction_strategy: "portable_write".to_string(),
                                metadata: std::collections::HashMap::from([(
                                    "family".to_string(),
                                    serde_json::json!("typescript"),
                                )]),
                            },
                        },
                        _ => ast_merge::DelegatedChildOperation {
                            operation_id: group.child_operation_id.clone(),
                            parent_operation_id: group.parent_operation_id.clone(),
                            requested_strategy: "delegate_child_surface".to_string(),
                            language_chain: vec!["ruby".to_string(), "ruby".to_string()],
                            surface: ast_merge::DiscoveredSurface {
                                surface_kind: "yard_example".to_string(),
                                declared_language: None,
                                effective_language: "ruby".to_string(),
                                address: group.delegated_runtime_surface_path.clone(),
                                parent_address: None,
                                span: None,
                                owner: ast_merge::SurfaceOwnerRef {
                                    kind: ast_merge::SurfaceOwnerKind::OwnedRegion,
                                    address: "/yard_example/1".to_string(),
                                },
                                reconstruction_strategy: "portable_write".to_string(),
                                metadata: std::collections::HashMap::from([(
                                    "family".to_string(),
                                    serde_json::json!("ruby"),
                                )]),
                            },
                        },
                    })
                    .collect(),
            ),
        },
        apply_resolved_outputs: move |_, _, _, applied_children| {
            assert_eq!(applied_children, expected_applied_children);
            ast_merge::MergeResult {
                ok: true,
                diagnostics: vec![],
                output: expected_output.clone(),
                policies: vec![],
            }
        },
    }
}
