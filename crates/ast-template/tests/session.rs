use ast_merge::{
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplateStrategy,
    TemplateStrategyOverride, default_template_token_config, read_relative_file_tree,
    write_relative_file_tree,
};
use ast_template::{
    DirectorySessionOptions, DirectorySessionProfile, FamilyMergeAdapter,
    FamilyMergeAdapterRegistry,
    apply_template_directory_session_diagnostics_with_default_registry_to_directory,
    apply_template_directory_session_envelope_with_default_registry_to_directory,
    apply_template_directory_session_outcome_with_default_registry_to_directory,
    apply_template_directory_session_to_directory,
    apply_template_directory_session_with_default_registry_to_directory,
    apply_template_directory_session_with_registry_to_directory, import_session_command_envelope,
    import_session_invocation_envelope,
    plan_template_directory_session_diagnostics_from_directories,
    plan_template_directory_session_envelope_from_directories,
    plan_template_directory_session_from_directories,
    plan_template_directory_session_outcome_from_directories,
    reapply_template_directory_session_to_directory, report_adapter_capabilities_from_directories,
    report_default_adapter_capabilities_from_directories,
    report_template_directory_session_entrypoint, report_template_directory_session_inspection,
    report_template_directory_session_options_configuration,
    report_template_directory_session_options_request,
    report_template_directory_session_profile_configuration,
    report_template_directory_session_profile_request,
    report_template_directory_session_resolution, report_template_directory_session_runner_input,
    report_template_directory_session_runner_payload, report_template_directory_session_status,
    run_template_directory_session, run_template_directory_session_command,
    run_template_directory_session_command_payload, run_template_directory_session_dispatch,
    run_template_directory_session_entrypoint, run_template_directory_session_request,
    run_template_directory_session_runner_payload, run_template_directory_session_runner_request,
    run_template_directory_session_with_default_registry_to_directory,
    run_template_directory_session_with_options, run_template_directory_session_with_profile,
    session_command_envelope, session_invocation_envelope,
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

#[test]
fn conforms_to_template_directory_session_options_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-362-template-directory-session-options-report/template-directory-session-options-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let plan_options = decode_session_options(
        &fixture["plan_run"]["options"],
        fixture_root.join("dry-run/template"),
        fixture_root.join("dry-run/destination"),
    );
    let plan_actual = run_template_directory_session_with_options(&plan_options)
        .expect("plan options should succeed");
    assert_eq!(
        serde_json::to_value(plan_actual).expect("report should serialize"),
        fixture["plan_run"]["expected"]
    );

    assert_session_options_apply_case(&fixture["apply_run"], fixture_root);
    assert_session_options_reapply_case(&fixture["reapply_run"], fixture_root);
}

#[test]
fn conforms_to_template_directory_session_profile_report_fixture() {
    let fixture_path = repo_root()
        .join("fixtures/diagnostics/slice-363-template-directory-session-profile-report/template-directory-session-profile-report.json");
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let plan_overrides = DirectorySessionOptions {
        mode: ast_template::DirectorySessionMode::Plan,
        template_root: fixture_root.join("dry-run/template"),
        destination_root: fixture_root.join("dry-run/destination"),
        context: serde_json::from_value(serde_json::json!({})).expect("context should deserialize"),
        default_strategy: TemplateStrategy::Merge,
        overrides: vec![],
        replacements: HashMap::new(),
        allowed_families: None,
        config: None,
    };
    let plan_actual = run_template_directory_session_with_profile(
        &profiles,
        fixture["plan_run"]["profile"].as_str().expect("profile should be string"),
        &plan_overrides,
    )
    .expect("plan profile should succeed");
    assert_eq!(
        serde_json::to_value(plan_actual).expect("report should serialize"),
        fixture["plan_run"]["expected"]
    );

    assert_session_profile_apply_case(&fixture["apply_run"], fixture_root, &profiles);
    assert_session_profile_reapply_case(&fixture["reapply_run"], fixture_root, &profiles);
}

#[test]
fn conforms_to_template_directory_session_configuration_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-364-template-directory-session-configuration-report/template-directory-session-configuration-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let options_valid = decode_session_options_direct(&fixture["options_valid"]["options"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_options_configuration(
            &options_valid
        ))
        .expect("report should serialize"),
        fixture["options_valid"]["expected"]
    );

    let options_missing_roots =
        decode_session_options_direct(&fixture["options_missing_roots"]["options"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_options_configuration(
            &options_missing_roots
        ))
        .expect("report should serialize"),
        fixture["options_missing_roots"]["expected"]
    );

    let profile_valid = decode_session_options_direct(&fixture["profile_valid"]["overrides"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_profile_configuration(
            &profiles,
            fixture["profile_valid"]["profile"].as_str().expect("profile should be string"),
            &profile_valid,
        ))
        .expect("report should serialize"),
        fixture["profile_valid"]["expected"]
    );

    let profile_missing_profile =
        decode_session_options_direct(&fixture["profile_missing_profile"]["overrides"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_profile_configuration(
            &profiles,
            fixture["profile_missing_profile"]["profile"]
                .as_str()
                .expect("profile should be string"),
            &profile_missing_profile,
        ))
        .expect("report should serialize"),
        fixture["profile_missing_profile"]["expected"]
    );

    let profile_missing_roots =
        decode_session_options_direct(&fixture["profile_missing_roots"]["overrides"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_profile_configuration(
            &profiles,
            fixture["profile_missing_roots"]["profile"].as_str().expect("profile should be string"),
            &profile_missing_roots,
        ))
        .expect("report should serialize"),
        fixture["profile_missing_roots"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_profile_configuration_outcome_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-365-template-directory-session-profile-configuration-outcome-report/template-directory-session-profile-configuration-outcome-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let missing_profile = decode_session_options_direct(&fixture["missing_profile"]["overrides"]);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_with_profile(
                &profiles,
                fixture["missing_profile"]["profile"].as_str().expect("profile should be string"),
                &missing_profile,
            )
            .expect("missing profile outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["missing_profile"]["expected"]
    );

    let missing_roots = decode_session_options_direct(&fixture["missing_roots"]["overrides"]);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_with_profile(
                &profiles,
                fixture["missing_roots"]["profile"].as_str().expect("profile should be string"),
                &missing_roots,
            )
            .expect("missing roots outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["missing_roots"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_options_configuration_outcome_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-366-template-directory-session-options-configuration-outcome-report/template-directory-session-options-configuration-outcome-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");

    let missing_both_roots =
        decode_session_options_direct(&fixture["missing_both_roots"]["options"]);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_with_options(&missing_both_roots)
                .expect("missing both roots outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["missing_both_roots"]["expected"]
    );

    let missing_destination_root =
        decode_session_options_direct(&fixture["missing_destination_root"]["options"]);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_with_options(&missing_destination_root)
                .expect("missing destination root outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["missing_destination_root"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_request_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-367-template-directory-session-request-report/template-directory-session-request-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let options_valid = decode_session_options_direct(&fixture["options_valid"]["options"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_options_request(&options_valid))
            .expect("report should serialize"),
        fixture["options_valid"]["expected"]
    );

    let options_invalid = decode_session_options_direct(&fixture["options_invalid"]["options"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_options_request(&options_invalid))
            .expect("report should serialize"),
        fixture["options_invalid"]["expected"]
    );

    let profile_valid = decode_session_options_direct(&fixture["profile_valid"]["overrides"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_profile_request(
            &profiles,
            fixture["profile_valid"]["profile"].as_str().expect("profile should be string"),
            &profile_valid,
        ))
        .expect("report should serialize"),
        fixture["profile_valid"]["expected"]
    );

    let profile_invalid = decode_session_options_direct(&fixture["profile_invalid"]["overrides"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_profile_request(
            &profiles,
            fixture["profile_invalid"]["profile"].as_str().expect("profile should be string"),
            &profile_invalid,
        ))
        .expect("report should serialize"),
        fixture["profile_invalid"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_request_outcome_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-368-template-directory-session-request-outcome-report/template-directory-session-request-outcome-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let options_ready = decode_session_request_report_from_fixture(
        &fixture["options_ready"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_request(&options_ready)
                .expect("options ready outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["options_ready"]["expected"]
    );

    let options_blocked = decode_session_request_report_from_fixture(
        &fixture["options_blocked"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_request(&options_blocked)
                .expect("options blocked outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["options_blocked"]["expected"]
    );

    let profile_ready = decode_session_request_report_from_fixture(
        &fixture["profile_ready"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_request(&profile_ready)
                .expect("profile ready outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_ready"]["expected"]
    );

    let profile_blocked = decode_session_request_report_from_fixture(
        &fixture["profile_blocked"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_request(&profile_blocked)
                .expect("profile blocked outcome should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_request_runner_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-369-template-directory-session-request-runner-report/template-directory-session-request-runner-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let options_ready = decode_session_runner_request_from_fixture(
        &fixture["options_ready"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_request(&options_ready, &profiles)
                .expect("options ready runner should succeed")
        )
        .expect("report should serialize"),
        fixture["options_ready"]["expected"]
    );

    let options_blocked = decode_session_runner_request_from_fixture(
        &fixture["options_blocked"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_request(&options_blocked, &profiles)
                .expect("options blocked runner should succeed")
        )
        .expect("report should serialize"),
        fixture["options_blocked"]["expected"]
    );

    let profile_ready = decode_session_runner_request_from_fixture(
        &fixture["profile_ready"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_request(&profile_ready, &profiles)
                .expect("profile ready runner should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_ready"]["expected"]
    );

    let profile_blocked = decode_session_runner_request_from_fixture(
        &fixture["profile_blocked"]["request"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_request(&profile_blocked, &profiles)
                .expect("profile blocked runner should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_runner_input_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-370-template-directory-session-runner-input-report/template-directory-session-runner-input-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");

    let options_ready =
        decode_session_runner_input_from_fixture(&fixture["options_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_input(&options_ready))
            .expect("report should serialize"),
        fixture["options_ready"]["expected"]
    );

    let options_blocked = decode_session_runner_input_from_fixture(
        &fixture["options_blocked"]["input"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_input(&options_blocked))
            .expect("report should serialize"),
        fixture["options_blocked"]["expected"]
    );

    let profile_ready =
        decode_session_runner_input_from_fixture(&fixture["profile_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_input(&profile_ready))
            .expect("report should serialize"),
        fixture["profile_ready"]["expected"]
    );

    let profile_blocked = decode_session_runner_input_from_fixture(
        &fixture["profile_blocked"]["input"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_input(&profile_blocked))
            .expect("report should serialize"),
        fixture["profile_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_runner_payload_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-371-template-directory-session-runner-payload-report/template-directory-session-runner-payload-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");

    let options_explicit = decode_session_runner_payload(&fixture["options_explicit"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_payload(&options_explicit))
            .expect("report should serialize"),
        fixture["options_explicit"]["expected"]
    );

    let options_inferred = decode_session_runner_payload(&fixture["options_inferred"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_payload(&options_inferred))
            .expect("report should serialize"),
        fixture["options_inferred"]["expected"]
    );

    let profile_default_name =
        decode_session_runner_payload(&fixture["profile_default_name"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_payload(
            &profile_default_name
        ))
        .expect("report should serialize"),
        fixture["profile_default_name"]["expected"]
    );

    let profile_explicit_name =
        decode_session_runner_payload(&fixture["profile_explicit_name"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_runner_payload(
            &profile_explicit_name
        ))
        .expect("report should serialize"),
        fixture["profile_explicit_name"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_runner_payload_outcome_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-372-template-directory-session-runner-payload-outcome-report/template-directory-session-runner-payload-outcome-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let options_ready = decode_session_runner_payload_from_fixture(
        &fixture["options_ready"]["payload"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_payload(&options_ready, &profiles)
                .expect("options ready payload runner should succeed")
        )
        .expect("report should serialize"),
        fixture["options_ready"]["expected"]
    );

    let options_blocked = decode_session_runner_payload_from_fixture(
        &fixture["options_blocked"]["payload"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_payload(&options_blocked, &profiles)
                .expect("options blocked payload runner should succeed")
        )
        .expect("report should serialize"),
        fixture["options_blocked"]["expected"]
    );

    let profile_ready = decode_session_runner_payload_from_fixture(
        &fixture["profile_ready"]["payload"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_payload(&profile_ready, &profiles)
                .expect("profile ready payload runner should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_ready"]["expected"]
    );

    let profile_blocked = decode_session_runner_payload_from_fixture(
        &fixture["profile_blocked"]["payload"],
        fixture_root,
    );
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_runner_payload(&profile_blocked, &profiles)
                .expect("profile blocked payload runner should succeed")
        )
        .expect("report should serialize"),
        fixture["profile_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_entrypoint_outcome_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-373-template-directory-session-entrypoint-outcome-report/template-directory-session-entrypoint-outcome-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture should have parent");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let payload_ready =
        decode_session_entrypoint_from_fixture(&fixture["payload_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_entrypoint(&payload_ready, &profiles)
                .expect("payload ready entrypoint should succeed")
        )
        .expect("report should serialize"),
        fixture["payload_ready"]["expected"]
    );

    let request_blocked =
        decode_session_entrypoint_from_fixture(&fixture["request_blocked"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_entrypoint(&request_blocked, &profiles)
                .expect("request blocked entrypoint should succeed")
        )
        .expect("report should serialize"),
        fixture["request_blocked"]["expected"]
    );

    let request_ready =
        decode_session_entrypoint_from_fixture(&fixture["request_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_entrypoint(&request_ready, &profiles)
                .expect("request ready entrypoint should succeed")
        )
        .expect("report should serialize"),
        fixture["request_ready"]["expected"]
    );

    let payload_blocked =
        decode_session_entrypoint_from_fixture(&fixture["payload_blocked"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            run_template_directory_session_entrypoint(&payload_blocked, &profiles)
                .expect("payload blocked entrypoint should succeed")
        )
        .expect("report should serialize"),
        fixture["payload_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_entrypoint_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-374-template-directory-session-entrypoint-report/template-directory-session-entrypoint-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");

    let payload_ready = decode_session_entrypoint(&fixture["payload_ready"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_entrypoint(&payload_ready))
            .expect("report should serialize"),
        fixture["payload_ready"]["expected"]
    );

    let request_blocked = decode_session_entrypoint(&fixture["request_blocked"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_entrypoint(&request_blocked))
            .expect("report should serialize"),
        fixture["request_blocked"]["expected"]
    );

    let request_ready = decode_session_entrypoint(&fixture["request_ready"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_entrypoint(&request_ready))
            .expect("report should serialize"),
        fixture["request_ready"]["expected"]
    );

    let payload_blocked = decode_session_entrypoint(&fixture["payload_blocked"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_entrypoint(&payload_blocked))
            .expect("report should serialize"),
        fixture["payload_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_resolution_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-375-template-directory-session-resolution-report/template-directory-session-resolution-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let payload_ready = decode_session_entrypoint(&fixture["payload_ready"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_resolution(
            &payload_ready,
            &profiles
        ))
        .expect("report should serialize"),
        fixture["payload_ready"]["expected"]
    );

    let request_blocked = decode_session_entrypoint(&fixture["request_blocked"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_resolution(
            &request_blocked,
            &profiles
        ))
        .expect("report should serialize"),
        fixture["request_blocked"]["expected"]
    );

    let request_ready = decode_session_entrypoint(&fixture["request_ready"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_resolution(
            &request_ready,
            &profiles
        ))
        .expect("report should serialize"),
        fixture["request_ready"]["expected"]
    );

    let payload_blocked = decode_session_entrypoint(&fixture["payload_blocked"]["input"]);
    assert_eq!(
        serde_json::to_value(report_template_directory_session_resolution(
            &payload_blocked,
            &profiles
        ))
        .expect("report should serialize"),
        fixture["payload_blocked"]["expected"]
    );
}

#[test]
fn conforms_to_template_directory_session_inspection_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-376-template-directory-session-inspection-report/template-directory-session-inspection-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    let payload_ready =
        decode_session_entrypoint_from_fixture(&fixture["payload_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            report_template_directory_session_inspection(&payload_ready, &profiles)
                .expect("report should succeed")
        )
        .expect("report should serialize"),
        resolve_session_inspection_expected_paths(
            &fixture["payload_ready"]["expected"],
            fixture_root
        )
    );

    let request_blocked =
        decode_session_entrypoint_from_fixture(&fixture["request_blocked"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            report_template_directory_session_inspection(&request_blocked, &profiles)
                .expect("report should succeed")
        )
        .expect("report should serialize"),
        resolve_session_inspection_expected_paths(
            &fixture["request_blocked"]["expected"],
            fixture_root
        )
    );

    let request_ready =
        decode_session_entrypoint_from_fixture(&fixture["request_ready"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            report_template_directory_session_inspection(&request_ready, &profiles)
                .expect("report should succeed")
        )
        .expect("report should serialize"),
        resolve_session_inspection_expected_paths(
            &fixture["request_ready"]["expected"],
            fixture_root
        )
    );

    let payload_blocked =
        decode_session_entrypoint_from_fixture(&fixture["payload_blocked"]["input"], fixture_root);
    assert_eq!(
        serde_json::to_value(
            report_template_directory_session_inspection(&payload_blocked, &profiles)
                .expect("report should succeed")
        )
        .expect("report should serialize"),
        resolve_session_inspection_expected_paths(
            &fixture["payload_blocked"]["expected"],
            fixture_root
        )
    );
}

#[test]
fn conforms_to_template_directory_session_dispatch_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-377-template-directory-session-dispatch-report/template-directory-session-dispatch-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    for key in [
        "inspect_payload_ready",
        "inspect_request_blocked",
        "run_request_ready",
        "run_payload_blocked",
    ] {
        let case = &fixture[key];
        let input = case["input"].as_object().expect("input should be object");
        let operation = input["operation"].as_str().expect("operation should be string");
        let entrypoint = decode_session_entrypoint_from_fixture(&input["entrypoint"], fixture_root);
        assert_eq!(
            serde_json::to_value(
                run_template_directory_session_dispatch(operation, &entrypoint, &profiles)
                    .expect("dispatch should succeed")
            )
            .expect("report should serialize"),
            resolve_session_dispatch_expected_paths(&case["expected"], fixture_root)
        );
    }
}

#[test]
fn conforms_to_template_directory_session_command_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-378-template-directory-session-command-report/template-directory-session-command-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    for key in ["inspect_payload_ready", "run_request_ready", "run_payload_blocked"] {
        let case = &fixture[key];
        let command = decode_session_command_from_fixture(&case["input"], fixture_root);
        assert_eq!(
            serde_json::to_value(
                run_template_directory_session_command(&command, &profiles)
                    .expect("command should succeed")
            )
            .expect("command should serialize"),
            resolve_session_dispatch_expected_paths(&case["expected"], fixture_root)
        );
    }
}

#[test]
fn conforms_to_template_directory_session_command_payload_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-379-template-directory-session-command-payload-report/template-directory-session-command-payload-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    for key in ["inspect_ready", "run_profile_ready", "run_profile_blocked"] {
        let case = &fixture[key];
        let command = decode_session_command_payload_from_fixture(&case["input"], fixture_root);
        assert_eq!(
            serde_json::to_value(
                run_template_directory_session_command_payload(&command, &profiles)
                    .expect("command payload should succeed")
            )
            .expect("command payload should serialize"),
            resolve_session_dispatch_expected_paths(&case["expected"], fixture_root)
        );
    }
}

#[test]
fn conforms_to_template_directory_session_dispatch_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-380-template-directory-session-dispatch-rejection/template-directory-session-dispatch-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for case in fixture["cases"].as_array().expect("cases should be array") {
        let label = case["label"].as_str().expect("label should be string");
        let input = case["input"].as_object().expect("input should be object");
        let operation = input["operation"].as_str().expect("operation should be string");
        let entrypoint = decode_session_entrypoint_from_fixture(&input["entrypoint"], fixture_root);
        let error =
            run_template_directory_session_dispatch(operation, &entrypoint, &HashMap::new())
                .expect_err("dispatch rejection should fail");
        assert_eq!(
            error.to_string(),
            case["expected_error"].as_str().expect("expected_error should be string"),
            "{label}"
        );
    }
}

#[test]
fn conforms_to_template_directory_session_command_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-381-template-directory-session-command-rejection/template-directory-session-command-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for case in fixture["cases"].as_array().expect("cases should be array") {
        let label = case["label"].as_str().expect("label should be string");
        let command = decode_session_command_from_fixture(&case["input"], fixture_root);
        let error = run_template_directory_session_command(&command, &HashMap::new())
            .expect_err("command rejection should fail");
        assert_eq!(
            error.to_string(),
            case["expected_error"].as_str().expect("expected_error should be string"),
            "{label}"
        );
    }
}

#[test]
fn conforms_to_template_directory_session_command_payload_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-382-template-directory-session-command-payload-rejection/template-directory-session-command-payload-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for case in fixture["cases"].as_array().expect("cases should be array") {
        let label = case["label"].as_str().expect("label should be string");
        let command = decode_session_command_payload_from_fixture(&case["input"], fixture_root);
        let error = run_template_directory_session_command_payload(&command, &HashMap::new())
            .expect_err("command payload rejection should fail");
        assert_eq!(
            error.to_string(),
            case["expected_error"].as_str().expect("expected_error should be string"),
            "{label}"
        );
    }
}

#[test]
fn conforms_to_template_directory_session_command_transport_envelope_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-389-template-directory-session-command-transport-envelope/template-directory-session-command-envelope.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let command = decode_session_command_from_fixture(&test_case["input"], fixture_root);
        let expected: ast_template::SessionCommandEnvelope =
            serde_json::from_value(resolve_session_command_envelope_fixture_paths(
                &test_case["expected_envelope"],
                fixture_root,
            ))
            .expect("expected envelope should deserialize");

        assert_eq!(session_command_envelope(&command), expected);
        assert_eq!(import_session_command_envelope(&expected), Ok(command));
    }
}

#[test]
fn conforms_to_template_directory_session_command_transport_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-390-template-directory-session-command-transport-rejection/template-directory-session-command-envelope-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let envelope: ast_template::SessionCommandEnvelope = serde_json::from_value(
            resolve_session_command_envelope_fixture_paths(&test_case["envelope"], fixture_root),
        )
        .expect("envelope should deserialize");
        let expected: ast_template::SessionCommandTransportImportError =
            serde_json::from_value(test_case["expected_error"].clone())
                .expect("expected error should deserialize");

        assert_eq!(import_session_command_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_report_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-383-template-directory-session-invocation-report/template-directory-session-invocation-report.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    for key in
        ["inspect_nested_payload_ready", "run_nested_request_ready", "run_flat_profile_blocked"]
    {
        let case = &fixture[key];
        let invocation = decode_session_invocation_from_fixture(&case["input"], fixture_root);
        assert_eq!(
            serde_json::to_value(
                run_template_directory_session(&invocation, &profiles)
                    .expect("invocation should succeed")
            )
            .expect("invocation should serialize"),
            resolve_session_dispatch_expected_paths(&case["expected"], fixture_root)
        );
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-384-template-directory-session-invocation-rejection/template-directory-session-invocation-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let invocation = decode_session_invocation_from_fixture(&test_case["input"], fixture_root);
        let error = run_template_directory_session(&invocation, &HashMap::new())
            .expect_err("invocation rejection should fail");
        assert_eq!(
            error.to_string(),
            test_case["expected_error"].as_str().expect("expected error should be string")
        );
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_json_roundtrip_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-385-template-directory-session-invocation-json-roundtrip/template-directory-session-invocation-json-roundtrip.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let invocation = decode_session_invocation_from_fixture(&test_case["input"], fixture_root);
        let payload = serde_json::to_vec(&invocation).expect("invocation should serialize");
        let round_tripped: ast_template::SessionInvocation =
            serde_json::from_slice(&payload).expect("invocation should deserialize");
        assert_eq!(
            serde_json::to_value(round_tripped).expect("invocation should serialize"),
            serde_json::to_value(invocation).expect("invocation should serialize")
        );
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_transport_envelope_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-386-template-directory-session-invocation-transport-envelope/template-directory-session-invocation-envelope.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let invocation = decode_session_invocation_from_fixture(&test_case["input"], fixture_root);
        let expected: ast_template::SessionInvocationEnvelope =
            serde_json::from_value(resolve_session_invocation_envelope_fixture_paths(
                &test_case["expected_envelope"],
                fixture_root,
            ))
            .expect("expected envelope should deserialize");

        assert_eq!(session_invocation_envelope(&invocation), expected);
        assert_eq!(import_session_invocation_envelope(&expected), Ok(invocation));
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_transport_rejection_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-387-template-directory-session-invocation-transport-rejection/template-directory-session-invocation-envelope-rejection.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let envelope: ast_template::SessionInvocationEnvelope = serde_json::from_value(
            resolve_session_invocation_envelope_fixture_paths(&test_case["envelope"], fixture_root),
        )
        .expect("envelope should deserialize");
        let expected: ast_template::SessionInvocationTransportImportError =
            serde_json::from_value(test_case["expected_error"].clone())
                .expect("expected error should deserialize");

        assert_eq!(import_session_invocation_envelope(&envelope), Err(expected));
    }
}

#[test]
fn conforms_to_template_directory_session_invocation_envelope_application_fixture() {
    let fixture_path = repo_root().join(
        "fixtures/diagnostics/slice-388-template-directory-session-invocation-envelope-application/template-directory-session-invocation-envelope-application.json",
    );
    let fixture: Value =
        serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
            .expect("fixture should deserialize");
    let fixture_root = fixture_path.parent().expect("fixture root should exist");
    let profiles = decode_session_profiles(&fixture["profiles"]);

    for test_case in fixture["cases"].as_array().expect("cases should be array") {
        let envelope: ast_template::SessionInvocationEnvelope = serde_json::from_value(
            resolve_session_invocation_envelope_fixture_paths(&test_case["envelope"], fixture_root),
        )
        .expect("envelope should deserialize");
        let invocation =
            import_session_invocation_envelope(&envelope).expect("envelope import should succeed");
        let actual = run_template_directory_session(&invocation, &profiles)
            .expect("invocation envelope application should succeed");
        assert_eq!(
            serde_json::to_value(actual).expect("dispatch report should serialize"),
            resolve_session_dispatch_expected_paths(&test_case["expected"], fixture_root)
        );
    }

    for test_case in fixture["rejections"].as_array().expect("rejections should be array") {
        let envelope: ast_template::SessionInvocationEnvelope = serde_json::from_value(
            resolve_session_invocation_envelope_fixture_paths(&test_case["envelope"], fixture_root),
        )
        .expect("envelope should deserialize");
        let expected: ast_template::SessionInvocationTransportImportError =
            serde_json::from_value(test_case["expected_error"].clone())
                .expect("expected error should deserialize");
        assert_eq!(import_session_invocation_envelope(&envelope), Err(expected));
    }
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

fn assert_session_options_apply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/options");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let options = decode_session_options(
        &fixture["options"],
        fixture_root.join("apply-run/template"),
        temp_root.clone(),
    );
    let actual = run_template_directory_session_with_options(&options)
        .expect("apply options should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_options_reapply_case(fixture: &Value, fixture_root: &std::path::Path) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/options-reapply");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let priming_options = decode_session_options(
        &serde_json::json!({
            "mode": "apply",
            "context": fixture["options"]["context"].clone(),
            "default_strategy": fixture["options"]["default_strategy"].clone(),
            "overrides": fixture["options"]["overrides"].clone(),
            "replacements": fixture["options"]["replacements"].clone(),
            "allowed_families": fixture["options"]["allowed_families"].clone()
        }),
        fixture_root.join("apply-run/template"),
        temp_root.clone(),
    );
    let _ = run_template_directory_session_with_options(&priming_options)
        .expect("apply priming should succeed");

    let options = decode_session_options(
        &fixture["options"],
        fixture_root.join("apply-run/template"),
        temp_root.clone(),
    );
    let actual = run_template_directory_session_with_options(&options)
        .expect("reapply options should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn decode_session_options(
    fixture: &Value,
    template_root: PathBuf,
    destination_root: PathBuf,
) -> DirectorySessionOptions {
    DirectorySessionOptions {
        mode: serde_json::from_value(fixture["mode"].clone()).expect("mode should deserialize"),
        template_root,
        destination_root,
        context: serde_json::from_value(fixture["context"].clone())
            .expect("context should deserialize"),
        default_strategy: serde_json::from_value(fixture["default_strategy"].clone())
            .expect("strategy should deserialize"),
        overrides: serde_json::from_value(fixture["overrides"].clone())
            .expect("overrides should deserialize"),
        replacements: serde_json::from_value(fixture["replacements"].clone())
            .expect("replacements should deserialize"),
        allowed_families: fixture["allowed_families"].as_array().map(|families| {
            families
                .iter()
                .map(|family| family.as_str().expect("family should be string").to_string())
                .collect()
        }),
        config: None,
    }
}

fn decode_session_options_direct(fixture: &Value) -> DirectorySessionOptions {
    DirectorySessionOptions {
        mode: serde_json::from_value(fixture["mode"].clone()).expect("mode should deserialize"),
        template_root: PathBuf::from(
            fixture["template_root"].as_str().expect("template_root should be string"),
        ),
        destination_root: PathBuf::from(
            fixture["destination_root"].as_str().expect("destination_root should be string"),
        ),
        context: fixture
            .get("context")
            .map(|value| serde_json::from_value(value.clone()).expect("context should deserialize"))
            .unwrap_or_default(),
        default_strategy: fixture
            .get("default_strategy")
            .map(|value| {
                serde_json::from_value(value.clone()).expect("strategy should deserialize")
            })
            .unwrap_or(TemplateStrategy::Merge),
        overrides: fixture
            .get("overrides")
            .map(|value| {
                serde_json::from_value(value.clone()).expect("overrides should deserialize")
            })
            .unwrap_or_default(),
        replacements: fixture
            .get("replacements")
            .map(|value| {
                serde_json::from_value(value.clone()).expect("replacements should deserialize")
            })
            .unwrap_or_default(),
        allowed_families: fixture.get("allowed_families").and_then(|value| value.as_array()).map(
            |families| {
                families
                    .iter()
                    .map(|family| family.as_str().expect("family should be string").to_string())
                    .collect()
            },
        ),
        config: None,
    }
}

fn assert_session_profile_apply_case(
    fixture: &Value,
    fixture_root: &std::path::Path,
    profiles: &HashMap<String, DirectorySessionProfile>,
) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/profiles");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let overrides = DirectorySessionOptions {
        template_root: fixture_root.join("apply-run/template"),
        destination_root: temp_root.clone(),
        ..Default::default()
    };
    let actual = run_template_directory_session_with_profile(
        profiles,
        fixture["profile"].as_str().expect("profile should be string"),
        &overrides,
    )
    .expect("apply profile should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn assert_session_profile_reapply_case(
    fixture: &Value,
    fixture_root: &std::path::Path,
    profiles: &HashMap<String, DirectorySessionProfile>,
) {
    let temp_root = repo_root().join("rust/crates/ast-template/tmp/profiles-reapply");
    let _ = fs::remove_dir_all(&temp_root);
    write_relative_file_tree(
        &temp_root,
        &read_relative_file_tree(&fixture_root.join("apply-run/destination"))
            .expect("apply-run destination should read"),
    )
    .expect("apply-run destination should write");

    let priming = DirectorySessionOptions {
        template_root: fixture_root.join("apply-run/template"),
        destination_root: temp_root.clone(),
        ..Default::default()
    };
    let _ = run_template_directory_session_with_profile(profiles, "apply_run", &priming)
        .expect("apply priming should succeed");

    let mut overrides = decode_session_options(
        &serde_json::json!({
            "mode": fixture["overrides"]["mode"].clone(),
            "context": {},
            "default_strategy": "merge",
            "overrides": [],
            "replacements": {},
            "allowed_families": null
        }),
        fixture_root.join("apply-run/template"),
        temp_root.clone(),
    );
    overrides.template_root = fixture_root.join("apply-run/template");
    overrides.destination_root = temp_root;
    let actual = run_template_directory_session_with_profile(
        profiles,
        fixture["profile"].as_str().expect("profile should be string"),
        &overrides,
    )
    .expect("reapply profile should succeed");
    assert_eq!(serde_json::to_value(actual).expect("report should serialize"), fixture["expected"]);
}

fn decode_session_profiles(fixture: &Value) -> HashMap<String, DirectorySessionProfile> {
    fixture
        .as_object()
        .expect("profiles should be object")
        .iter()
        .map(|(name, profile)| {
            (
                name.clone(),
                serde_json::from_value(profile.clone()).expect("profile should deserialize"),
            )
        })
        .collect()
}

fn decode_session_request_report(fixture: &Value) -> ast_template::SessionRequestReport {
    ast_template::SessionRequestReport {
        request_kind: fixture["request_kind"]
            .as_str()
            .expect("request_kind should be string")
            .to_string(),
        profile_name: fixture["profile_name"].as_str().map(|value| value.to_string()),
        mode: serde_json::from_value(fixture["mode"].clone()).expect("mode should deserialize"),
        ready: fixture["ready"].as_bool().expect("ready should be boolean"),
        diagnostics: serde_json::from_value(fixture["diagnostics"].clone())
            .expect("diagnostics should deserialize"),
        resolved_options: fixture
            .get("resolved_options")
            .and_then(|value| (!value.is_null()).then(|| decode_session_options_direct(value))),
    }
}

fn decode_session_request_report_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionRequestReport {
    let mut report = decode_session_request_report(fixture);
    if let Some(options) = report.resolved_options.as_mut() {
        options.template_root = fixture_root.join(&options.template_root);
        options.destination_root = fixture_root.join(&options.destination_root);
    }
    report
}

fn decode_session_runner_request_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionRunnerRequest {
    ast_template::SessionRunnerRequest {
        request_kind: fixture["request_kind"]
            .as_str()
            .expect("request_kind should be string")
            .to_string(),
        profile_name: fixture["profile_name"].as_str().map(|value| value.to_string()),
        options: fixture.get("options").and_then(|value| {
            (!value.is_null()).then(|| resolve_runner_request_fixture_paths(value, fixture_root))
        }),
        overrides: fixture.get("overrides").and_then(|value| {
            (!value.is_null()).then(|| resolve_runner_request_fixture_paths(value, fixture_root))
        }),
    }
}

fn decode_session_runner_request(fixture: &Value) -> ast_template::SessionRunnerRequest {
    ast_template::SessionRunnerRequest {
        request_kind: fixture["request_kind"]
            .as_str()
            .expect("request_kind should be string")
            .to_string(),
        profile_name: fixture["profile_name"].as_str().map(|value| value.to_string()),
        options: fixture.get("options").and_then(|value| (!value.is_null()).then(|| value.clone())),
        overrides: fixture
            .get("overrides")
            .and_then(|value| (!value.is_null()).then(|| value.clone())),
    }
}

fn decode_session_runner_input_from_fixture(
    fixture: &Value,
    _fixture_root: &std::path::Path,
) -> ast_template::SessionRunnerInput {
    serde_json::from_value(fixture.clone()).expect("runner input should deserialize")
}

fn decode_session_runner_payload(fixture: &Value) -> ast_template::SessionRunnerPayload {
    serde_json::from_value(fixture.clone()).expect("runner payload should deserialize")
}

fn decode_session_runner_payload_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionRunnerPayload {
    let mut payload: ast_template::SessionRunnerPayload =
        serde_json::from_value(fixture.clone()).expect("runner payload should deserialize");
    if !payload.template_root.is_empty() {
        payload.template_root =
            fixture_root.join(&payload.template_root).to_string_lossy().into_owned();
    }
    if !payload.destination_root.is_empty() {
        payload.destination_root =
            fixture_root.join(&payload.destination_root).to_string_lossy().into_owned();
    }
    payload
}

fn decode_session_entrypoint_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionEntrypoint {
    ast_template::SessionEntrypoint {
        payload: fixture.get("payload").and_then(|value| {
            (!value.is_null())
                .then(|| decode_session_runner_payload_from_fixture(value, fixture_root))
        }),
        request: fixture.get("request").and_then(|value| {
            (!value.is_null())
                .then(|| decode_session_runner_request_from_fixture(value, fixture_root))
        }),
    }
}

fn decode_session_entrypoint(fixture: &Value) -> ast_template::SessionEntrypoint {
    ast_template::SessionEntrypoint {
        payload: fixture
            .get("payload")
            .and_then(|value| (!value.is_null()).then(|| decode_session_runner_payload(value))),
        request: fixture
            .get("request")
            .and_then(|value| (!value.is_null()).then(|| decode_session_runner_request(value))),
    }
}

fn decode_session_command_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionCommand {
    ast_template::SessionCommand {
        operation: fixture.get("operation").and_then(Value::as_str).unwrap_or_default().to_string(),
        payload: fixture.get("payload").and_then(|value| {
            (!value.is_null())
                .then(|| decode_session_runner_payload_from_fixture(value, fixture_root))
        }),
        request: fixture.get("request").and_then(|value| {
            (!value.is_null())
                .then(|| decode_session_runner_request_from_fixture(value, fixture_root))
        }),
    }
}

fn resolve_session_command_envelope_fixture_paths(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> Value {
    let mut resolved = fixture.clone();
    if let Some(section) = resolved.as_object_mut() {
        if let Some(command) = section.get_mut("command") {
            *command =
                serde_json::to_value(decode_session_command_from_fixture(command, fixture_root))
                    .expect("command should serialize");
        }
    }
    resolved
}

fn decode_session_command_payload_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionCommandPayload {
    let mut payload: ast_template::SessionCommandPayload =
        serde_json::from_value(fixture.clone()).expect("command payload should deserialize");
    if !payload.template_root.is_empty() {
        payload.template_root =
            fixture_root.join(&payload.template_root).to_string_lossy().into_owned();
    }
    if !payload.destination_root.is_empty() {
        payload.destination_root =
            fixture_root.join(&payload.destination_root).to_string_lossy().into_owned();
    }
    payload
}

fn decode_session_invocation_from_fixture(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> ast_template::SessionInvocation {
    let mut invocation: ast_template::SessionInvocation =
        serde_json::from_value(fixture.clone()).expect("session invocation should deserialize");
    if let Some(payload) = invocation.payload.as_mut() {
        if !payload.template_root.is_empty() {
            payload.template_root =
                fixture_root.join(&payload.template_root).to_string_lossy().into_owned();
        }
        if !payload.destination_root.is_empty() {
            payload.destination_root =
                fixture_root.join(&payload.destination_root).to_string_lossy().into_owned();
        }
    }
    if let Some(request) = invocation.request.as_mut() {
        if let Some(options) = request.options.as_mut() {
            *options = resolve_runner_request_fixture_paths(options, fixture_root);
        }
        if let Some(overrides) = request.overrides.as_mut() {
            *overrides = resolve_runner_request_fixture_paths(overrides, fixture_root);
        }
    }
    if !invocation.template_root.is_empty() {
        invocation.template_root =
            fixture_root.join(&invocation.template_root).to_string_lossy().into_owned();
    }
    if !invocation.destination_root.is_empty() {
        invocation.destination_root =
            fixture_root.join(&invocation.destination_root).to_string_lossy().into_owned();
    }
    invocation
}

fn resolve_session_invocation_envelope_fixture_paths(
    fixture: &Value,
    fixture_root: &std::path::Path,
) -> Value {
    let mut resolved = fixture.clone();
    if let Some(section) = resolved.as_object_mut() {
        if let Some(invocation) = section.get_mut("invocation") {
            *invocation = serde_json::to_value(decode_session_invocation_from_fixture(
                invocation,
                fixture_root,
            ))
            .expect("invocation should serialize");
        }
    }
    resolved
}

fn resolve_runner_request_fixture_paths(value: &Value, fixture_root: &std::path::Path) -> Value {
    let mut resolved = value.clone();
    if let Some(section) = resolved.as_object_mut() {
        if let Some(options) = section.get_mut("options") {
            if !options.is_null() {
                *options = resolve_runner_request_fixture_paths(options, fixture_root);
            }
        }
        if let Some(overrides) = section.get_mut("overrides") {
            if !overrides.is_null() {
                *overrides = resolve_runner_request_fixture_paths(overrides, fixture_root);
            }
        }
        if let Some(template_root) = section.get_mut("template_root") {
            if let Some(path) = template_root.as_str() {
                if !path.is_empty() {
                    *template_root =
                        Value::String(fixture_root.join(path).to_string_lossy().into_owned());
                }
            }
        }
        if let Some(destination_root) = section.get_mut("destination_root") {
            if let Some(path) = destination_root.as_str() {
                if !path.is_empty() {
                    *destination_root =
                        Value::String(fixture_root.join(path).to_string_lossy().into_owned());
                }
            }
        }
    }
    resolved
}

fn resolve_session_inspection_expected_paths(
    value: &Value,
    fixture_root: &std::path::Path,
) -> Value {
    let mut resolved = value.clone();
    if let Some(section) = resolved.as_object_mut() {
        if let Some(entrypoint_report) =
            section.get_mut("entrypoint_report").and_then(Value::as_object_mut)
        {
            if let Some(runner_request) = entrypoint_report.get_mut("runner_request") {
                *runner_request =
                    resolve_runner_request_fixture_paths(runner_request, fixture_root);
            }
        }
        if let Some(session_resolution) =
            section.get_mut("session_resolution").and_then(Value::as_object_mut)
        {
            if let Some(runner_request) = session_resolution.get_mut("runner_request") {
                *runner_request =
                    resolve_runner_request_fixture_paths(runner_request, fixture_root);
            }
            if let Some(session_request) =
                session_resolution.get_mut("session_request").and_then(Value::as_object_mut)
            {
                if let Some(resolved_options) = session_request.get_mut("resolved_options") {
                    if !resolved_options.is_null() {
                        *resolved_options =
                            resolve_runner_request_fixture_paths(resolved_options, fixture_root);
                    }
                }
            }
        }
    }
    resolved
}

fn resolve_session_dispatch_expected_paths(value: &Value, fixture_root: &std::path::Path) -> Value {
    let mut resolved = value.clone();
    if let Some(section) = resolved.as_object_mut() {
        if let Some(inspection) = section.get_mut("inspection") {
            if !inspection.is_null() {
                *inspection = resolve_session_inspection_expected_paths(inspection, fixture_root);
            }
        }
    }
    resolved
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
