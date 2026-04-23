use ast_merge::{
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplateStrategy,
    TemplateStrategyOverride, default_template_token_config, read_relative_file_tree,
    write_relative_file_tree,
};
use ast_template::{
    FamilyMergeAdapter, FamilyMergeAdapterRegistry,
    apply_template_directory_session_diagnostics_with_default_registry_to_directory,
    apply_template_directory_session_envelope_with_default_registry_to_directory,
    apply_template_directory_session_outcome_with_default_registry_to_directory,
    apply_template_directory_session_to_directory,
    apply_template_directory_session_with_default_registry_to_directory,
    apply_template_directory_session_with_registry_to_directory,
    plan_template_directory_session_diagnostics_from_directories,
    plan_template_directory_session_envelope_from_directories,
    plan_template_directory_session_from_directories,
    plan_template_directory_session_outcome_from_directories,
    reapply_template_directory_session_to_directory, report_adapter_capabilities_from_directories,
    report_default_adapter_capabilities_from_directories, report_template_directory_session_status,
    run_template_directory_session_with_default_registry_to_directory,
};
use markdown_merge::{MarkdownDialect, merge_markdown};
use ruby_merge::{RubyDialect, merge_ruby};
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf};
use toml_merge::{TomlDialect, merge_toml};

#[test]
fn conforms_to_template_directory_session_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-353-template-directory-session-report/template-directory-session-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let dry_run = fixture["dry_run"].clone();
    let dry_run_report = plan_template_directory_session_from_directories(
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(dry_run["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(dry_run["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(dry_run["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(dry_run["replacements"].clone())
            .expect("replacements should deserialize"),
        &default_template_token_config(),
    )
    .expect("dry-run session should succeed");
    assert_eq!(
        serde_json::to_value(dry_run_report).expect("report should serialize"),
        dry_run["expected"]
    );

    let temp_root = repo_root().join("rust/crates/ast-template/tmp/session");
    let _ = fs::remove_dir_all(&temp_root);
    let apply_source_root = fixture_root.join("apply-run/destination");
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&apply_source_root).expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let apply_run = fixture["apply_run"].clone();
    let apply_report = apply_template_directory_session_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(apply_run["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(apply_run["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(apply_run["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(apply_run["replacements"].clone())
            .expect("replacements should deserialize"),
        multi_family_merge_callback,
        &default_template_token_config(),
    )
    .expect("apply session should succeed");
    assert_eq!(
        serde_json::to_value(apply_report).expect("report should serialize"),
        apply_run["expected"]
    );

    let reapply_run = fixture["reapply_run"].clone();
    let reapply_report = reapply_template_directory_session_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(reapply_run["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(reapply_run["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(reapply_run["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(reapply_run["replacements"].clone())
            .expect("replacements should deserialize"),
        multi_family_merge_callback,
        &default_template_token_config(),
    )
    .expect("reapply session should succeed");
    assert_eq!(
        serde_json::to_value(reapply_report).expect("report should serialize"),
        reapply_run["expected"]
    );

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
}

#[test]
fn conforms_to_template_directory_adapter_registry_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-354-template-directory-adapter-registry-report/template-directory-adapter-registry-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let full_registry = HashMap::from([
        ("markdown".to_string(), markdown_adapter as FamilyMergeAdapter),
        ("ruby".to_string(), ruby_adapter as FamilyMergeAdapter),
        ("toml".to_string(), toml_adapter as FamilyMergeAdapter),
    ]);
    let partial_registry = HashMap::from([
        ("markdown".to_string(), markdown_adapter as FamilyMergeAdapter),
        ("toml".to_string(), toml_adapter as FamilyMergeAdapter),
    ]);

    assert_registry_fixture_case(&fixture["full_registry"], fixture_root, &full_registry);
    assert_registry_fixture_case(&fixture["partial_registry"], fixture_root, &partial_registry);
}

#[test]
fn conforms_to_template_directory_default_adapter_discovery_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-355-template-directory-default-adapter-discovery-report/template-directory-default-adapter-discovery-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    assert_default_discovery_fixture_case(&fixture["default_discovery"], fixture_root);
    assert_default_discovery_fixture_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_adapter_capability_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-356-template-directory-adapter-capability-report/template-directory-adapter-capability-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let full_registry = HashMap::from([
        ("markdown".to_string(), markdown_adapter as FamilyMergeAdapter),
        ("ruby".to_string(), ruby_adapter as FamilyMergeAdapter),
        ("toml".to_string(), toml_adapter as FamilyMergeAdapter),
    ]);
    let partial_registry = HashMap::from([
        ("markdown".to_string(), markdown_adapter as FamilyMergeAdapter),
        ("toml".to_string(), toml_adapter as FamilyMergeAdapter),
    ]);

    assert_capability_fixture_case(&fixture["full_registry"], fixture_root, &full_registry);
    assert_capability_fixture_case(&fixture["partial_registry"], fixture_root, &partial_registry);
    assert_default_capability_fixture_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_envelope_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-357-template-directory-session-envelope-report/template-directory-session-envelope-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let dry_run_actual = plan_template_directory_session_envelope_from_directories(
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(
            fixture["dry_run"]["context"].clone(),
        )
        .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["dry_run"]["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(
            fixture["dry_run"]["overrides"].clone(),
        )
        .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(
            fixture["dry_run"]["replacements"].clone(),
        )
        .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("dry-run envelope should succeed");
    assert_eq!(
        serde_json::to_value(dry_run_actual).expect("report should serialize"),
        fixture["dry_run"]["expected"]
    );

    assert_session_envelope_apply_case(&fixture["apply_run"], fixture_root);
    assert_session_envelope_apply_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_status_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-358-template-directory-session-status-report/template-directory-session-status-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let dry_run_actual = plan_template_directory_session_envelope_from_directories(
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(
            fixture["dry_run"]["context"].clone(),
        )
        .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["dry_run"]["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(
            fixture["dry_run"]["overrides"].clone(),
        )
        .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(
            fixture["dry_run"]["replacements"].clone(),
        )
        .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("dry-run status should succeed");
    assert_eq!(
        serde_json::to_value(report_template_directory_session_status(&dry_run_actual))
            .expect("status should serialize"),
        fixture["dry_run"]["expected"]
    );

    assert_session_status_apply_case(&fixture["apply_run"], fixture_root);
    assert_session_status_apply_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_diagnostics_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-359-template-directory-session-diagnostics-report/template-directory-session-diagnostics-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let dry_run_actual = plan_template_directory_session_diagnostics_from_directories(
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(
            fixture["dry_run"]["context"].clone(),
        )
        .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["dry_run"]["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(
            fixture["dry_run"]["overrides"].clone(),
        )
        .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(
            fixture["dry_run"]["replacements"].clone(),
        )
        .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("dry-run diagnostics should succeed");
    assert_eq!(
        serde_json::to_value(dry_run_actual).expect("report should serialize"),
        fixture["dry_run"]["expected"]
    );

    assert_session_diagnostics_apply_case(&fixture["apply_run"], fixture_root);
    assert_session_diagnostics_apply_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_outcome_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-360-template-directory-session-outcome-report/template-directory-session-outcome-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let dry_run_actual = plan_template_directory_session_outcome_from_directories(
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(
            fixture["dry_run"]["context"].clone(),
        )
        .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["dry_run"]["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(
            fixture["dry_run"]["overrides"].clone(),
        )
        .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(
            fixture["dry_run"]["replacements"].clone(),
        )
        .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("dry-run outcome should succeed");
    assert_eq!(
        serde_json::to_value(dry_run_actual).expect("report should serialize"),
        fixture["dry_run"]["expected"]
    );

    assert_session_outcome_apply_case(&fixture["apply_run"], fixture_root);
    assert_session_outcome_apply_case(&fixture["filtered_discovery"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_runner_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-361-template-directory-session-runner-report/template-directory-session-runner-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let plan_actual = run_template_directory_session_with_default_registry_to_directory(
        ast_template::DirectorySessionMode::Plan,
        &fixture_root.join("dry-run/template"),
        &fixture_root.join("dry-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(
            fixture["plan_run"]["context"].clone(),
        )
        .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["plan_run"]["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(
            fixture["plan_run"]["overrides"].clone(),
        )
        .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(
            fixture["plan_run"]["replacements"].clone(),
        )
        .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("plan runner should succeed");
    assert_eq!(
        serde_json::to_value(plan_actual).expect("report should serialize"),
        fixture["plan_run"]["expected"]
    );

    assert_session_runner_apply_case(
        &fixture["apply_run"],
        fixture_root,
        ast_template::DirectorySessionMode::Apply,
    );
    assert_session_runner_apply_case(
        &fixture["reapply_run"],
        fixture_root,
        ast_template::DirectorySessionMode::Reapply,
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root should resolve")
}

fn assert_registry_fixture_case(
    fixture: &Value,
    fixture_root: &std::path::Path,
    registry: &FamilyMergeAdapterRegistry,
) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/registry");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let actual = apply_template_directory_session_with_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        registry,
        &default_template_token_config(),
    )
    .expect("registry session should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
}

fn assert_default_discovery_fixture_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/discovery");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = apply_template_directory_session_with_default_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("discovery session should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);

    fs::remove_dir_all(temp_root).expect("temp dir should be removable");
}

fn assert_capability_fixture_case(
    fixture: &Value,
    fixture_root: &std::path::Path,
    registry: &FamilyMergeAdapterRegistry,
) {
    let actual = report_adapter_capabilities_from_directories(
        &fixture_root.join("apply-run/template"),
        &fixture_root.join("apply-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        registry,
        &default_template_token_config(),
    )
    .expect("capability report should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_default_capability_fixture_case(fixture: &Value, fixture_root: &std::path::Path) {
    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = report_default_adapter_capabilities_from_directories(
        &fixture_root.join("apply-run/template"),
        &fixture_root.join("apply-run/destination"),
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("default capability report should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_envelope_apply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/envelope");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = apply_template_directory_session_envelope_with_default_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("apply envelope should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_status_apply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/status");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = apply_template_directory_session_envelope_with_default_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("apply status should succeed");
    assert_eq!(
        serde_json::to_value(report_template_directory_session_status(&actual))
            .expect("status should serialize"),
        fixture["expected"]
    );
}

fn assert_session_diagnostics_apply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/diagnostics");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = apply_template_directory_session_diagnostics_with_default_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("apply diagnostics should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_outcome_apply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/outcome");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let allowed_families = fixture["allowed_families"].as_array().map(|families| {
        families
            .iter()
            .map(|family| family.as_str().expect("family should be string"))
            .collect::<Vec<_>>()
    });
    let actual = apply_template_directory_session_outcome_with_default_registry_to_directory(
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families.as_deref(),
        &default_template_token_config(),
    )
    .expect("apply outcome should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_runner_apply_case(
    fixture: &Value,
    fixture_root: &std::path::Path,
    mode: ast_template::DirectorySessionMode,
) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/runner");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    if mode == ast_template::DirectorySessionMode::Reapply {
        let _ = run_template_directory_session_with_default_registry_to_directory(
            ast_template::DirectorySessionMode::Apply,
            &fixture_root.join("apply-run/template"),
            &temp_root,
            &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
                .expect("context should deserialize"),
            serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
                .expect("strategy should deserialize"),
            &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
                .expect("overrides should deserialize"),
            &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
                .expect("replacements should deserialize"),
            None,
            &default_template_token_config(),
        )
        .expect("apply priming should succeed");
    }

    let actual = run_template_directory_session_with_default_registry_to_directory(
        mode,
        &fixture_root.join("apply-run/template"),
        &temp_root,
        &serde_json::from_value::<TemplateDestinationContext>(fixture["context"].clone())
            .expect("context should deserialize"),
        serde_json::from_value::<TemplateStrategy>(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        &serde_json::from_value::<Vec<TemplateStrategyOverride>>(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        &serde_json::from_value::<HashMap<String, String>>(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        None,
        &default_template_token_config(),
    )
    .expect("runner should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn multi_family_merge_callback(
    entry: &TemplateExecutionPlanEntry,
) -> ast_merge::MergeResult<String> {
    ast_template::merge_prepared_content_from_registry(
        &HashMap::from([
            ("markdown".to_string(), markdown_adapter as FamilyMergeAdapter),
            ("ruby".to_string(), ruby_adapter as FamilyMergeAdapter),
            ("toml".to_string(), toml_adapter as FamilyMergeAdapter),
        ]),
        entry,
    )
}

fn markdown_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_markdown(&template, &destination, MarkdownDialect::Markdown)
}

fn toml_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_toml(&template, &destination, TomlDialect::Toml, None)
}

fn ruby_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_ruby(&template, &destination, RubyDialect::Ruby)
}
