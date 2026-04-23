use ast_merge::{
    Diagnostic, DiagnosticCategory, DiagnosticSeverity, TemplateDestinationContext,
    TemplateExecutionPlanEntry, TemplateStrategy, TemplateStrategyOverride,
    default_template_token_config, read_relative_file_tree, write_relative_file_tree,
};
use ast_template::{
    apply_template_directory_session_to_directory,
    plan_template_directory_session_from_directories,
    reapply_template_directory_session_to_directory,
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root should resolve")
}

fn multi_family_merge_callback(
    entry: &TemplateExecutionPlanEntry,
) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    match entry.classification.family.as_str() {
        "markdown" => merge_markdown(&template, &destination, MarkdownDialect::Markdown),
        "toml" => merge_toml(&template, &destination, TomlDialect::Toml, None),
        "ruby" => merge_ruby(&template, &destination, RubyDialect::Ruby),
        family => ast_merge::MergeResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!("missing family merge adapter for {family}"),
                path: None,
                review: None,
            }],
            output: None,
            policies: vec![],
        },
    }
}
