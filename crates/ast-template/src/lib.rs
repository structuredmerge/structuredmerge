use ast_merge::{
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplateStrategy,
    TemplateStrategyOverride, TemplateTokenConfig, TemplateTreeRunResult,
    apply_template_tree_execution_to_directory, plan_template_tree_execution_from_directories,
    report_template_directory_runner,
};
use markdown_merge::{MarkdownDialect, merge_markdown};
use ruby_merge::{RubyDialect, merge_ruby};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use toml_merge::{TomlDialect, merge_toml};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectorySessionMode {
    Plan,
    Apply,
    Reapply,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDirectorySessionReport {
    pub mode: DirectorySessionMode,
    pub runner_report: ast_merge::TemplateDirectoryRunnerReport,
}

pub type FamilyMergeAdapter = fn(&TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String>;
pub type FamilyMergeAdapterRegistry = HashMap<String, FamilyMergeAdapter>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDirectoryRegistrySessionReport {
    pub mode: DirectorySessionMode,
    pub adapter_families: Vec<String>,
    pub diagnostics: Vec<TemplateDirectoryRegistryDiagnostic>,
    pub runner_report: ast_merge::TemplateDirectoryRunnerReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterCapabilityReport {
    pub required_families: Vec<String>,
    pub adapter_families: Vec<String>,
    pub missing_families: Vec<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEnvelopeReport<T> {
    pub session_report: T,
    pub adapter_capabilities: AdapterCapabilityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDirectoryRegistryDiagnostic {
    pub severity: ast_merge::DiagnosticSeverity,
    pub category: ast_merge::DiagnosticCategory,
    pub message: String,
}

pub fn report_template_directory_session(
    mode: DirectorySessionMode,
    entries: &[TemplateExecutionPlanEntry],
    result: Option<&TemplateTreeRunResult>,
) -> TemplateDirectorySessionReport {
    TemplateDirectorySessionReport {
        mode,
        runner_report: report_template_directory_runner(entries, result),
    }
}

pub fn plan_template_directory_session_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &std::collections::HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> std::io::Result<TemplateDirectorySessionReport> {
    let plan = plan_template_tree_execution_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    )?;
    Ok(report_template_directory_session(DirectorySessionMode::Plan, &plan, None))
}

pub fn apply_template_directory_session_to_directory<F>(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &std::collections::HashMap<String, String>,
    merge_prepared_content: F,
    config: &TemplateTokenConfig,
) -> std::io::Result<TemplateDirectorySessionReport>
where
    F: Fn(&TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String>,
{
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        merge_prepared_content,
        config,
    )?;
    Ok(report_template_directory_session(
        DirectorySessionMode::Apply,
        &result.execution_plan,
        Some(&result),
    ))
}

pub fn reapply_template_directory_session_to_directory<F>(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &std::collections::HashMap<String, String>,
    merge_prepared_content: F,
    config: &TemplateTokenConfig,
) -> std::io::Result<TemplateDirectorySessionReport>
where
    F: Fn(&TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String>,
{
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        merge_prepared_content,
        config,
    )?;
    Ok(report_template_directory_session(
        DirectorySessionMode::Reapply,
        &result.execution_plan,
        Some(&result),
    ))
}

pub fn merge_prepared_content_from_registry(
    registry: &FamilyMergeAdapterRegistry,
    entry: &TemplateExecutionPlanEntry,
) -> ast_merge::MergeResult<String> {
    let family = entry.classification.family.clone();
    match registry.get(&family) {
        Some(adapter) => adapter(entry),
        None => ast_merge::MergeResult {
            ok: false,
            diagnostics: vec![ast_merge::Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::ConfigurationError,
                message: format!("missing family adapter for {family}"),
                path: None,
                review: None,
            }],
            output: None,
            policies: vec![],
        },
    }
}

pub fn registered_adapter_families(registry: &FamilyMergeAdapterRegistry) -> Vec<String> {
    let mut families = registry.keys().cloned().collect::<Vec<_>>();
    families.sort();
    families
}

pub fn report_template_directory_registry_session(
    mode: DirectorySessionMode,
    entries: &[TemplateExecutionPlanEntry],
    result: Option<&TemplateTreeRunResult>,
    registry: &FamilyMergeAdapterRegistry,
) -> TemplateDirectoryRegistrySessionReport {
    TemplateDirectoryRegistrySessionReport {
        mode,
        adapter_families: registered_adapter_families(registry),
        diagnostics: result
            .map(|value| {
                value
                    .apply_result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| TemplateDirectoryRegistryDiagnostic {
                        severity: diagnostic.severity,
                        category: diagnostic.category,
                        message: diagnostic.message.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        runner_report: report_template_directory_runner(entries, result),
    }
}

pub fn apply_template_directory_session_with_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    registry: &FamilyMergeAdapterRegistry,
    config: &TemplateTokenConfig,
) -> std::io::Result<TemplateDirectoryRegistrySessionReport> {
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        |entry| merge_prepared_content_from_registry(registry, entry),
        config,
    )?;
    Ok(report_template_directory_registry_session(
        DirectorySessionMode::Apply,
        &result.execution_plan,
        Some(&result),
        registry,
    ))
}

pub fn default_family_merge_adapter_registry(
    allowed_families: Option<&[&str]>,
) -> FamilyMergeAdapterRegistry {
    let include =
        |family: &str| allowed_families.map(|families| families.contains(&family)).unwrap_or(true);

    let mut registry = FamilyMergeAdapterRegistry::new();
    if include("markdown") {
        registry.insert("markdown".to_string(), markdown_family_adapter);
    }
    if include("toml") {
        registry.insert("toml".to_string(), toml_family_adapter);
    }
    if include("ruby") {
        registry.insert("ruby".to_string(), ruby_family_adapter);
    }
    registry
}

pub fn apply_template_directory_session_with_default_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<TemplateDirectoryRegistrySessionReport> {
    let registry = default_family_merge_adapter_registry(allowed_families);
    apply_template_directory_session_with_registry_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        &registry,
        config,
    )
}

fn markdown_family_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_markdown(&template, &destination, MarkdownDialect::Markdown)
}

fn toml_family_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_toml(&template, &destination, TomlDialect::Toml, None)
}

fn ruby_family_adapter(entry: &TemplateExecutionPlanEntry) -> ast_merge::MergeResult<String> {
    let template = entry.prepared_template_content.clone().unwrap_or_default();
    let destination = entry.destination_content.clone().unwrap_or_default();
    merge_ruby(&template, &destination, RubyDialect::Ruby)
}

pub fn required_families(entries: &[TemplateExecutionPlanEntry]) -> Vec<String> {
    let mut families = entries
        .iter()
        .filter(|entry| {
            entry.execution_action == ast_merge::TemplateExecutionAction::MergePreparedContent
        })
        .map(|entry| entry.classification.family.clone())
        .collect::<Vec<_>>();
    families.sort();
    families.dedup();
    families
}

pub fn report_adapter_capabilities(
    entries: &[TemplateExecutionPlanEntry],
    registry: &FamilyMergeAdapterRegistry,
) -> AdapterCapabilityReport {
    let required = required_families(entries);
    let available = registered_adapter_families(registry);
    let missing =
        required.iter().filter(|family| !available.contains(family)).cloned().collect::<Vec<_>>();
    AdapterCapabilityReport {
        required_families: required,
        adapter_families: available,
        missing_families: missing.clone(),
        ready: missing.is_empty(),
    }
}

pub fn report_adapter_capabilities_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    registry: &FamilyMergeAdapterRegistry,
    config: &TemplateTokenConfig,
) -> std::io::Result<AdapterCapabilityReport> {
    let plan = plan_template_tree_execution_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    )?;
    Ok(report_adapter_capabilities(&plan, registry))
}

pub fn report_default_adapter_capabilities_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<AdapterCapabilityReport> {
    let registry = default_family_merge_adapter_registry(allowed_families);
    report_adapter_capabilities_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        &registry,
        config,
    )
}

pub fn report_template_directory_session_envelope<T>(
    session_report: T,
    adapter_capabilities: AdapterCapabilityReport,
) -> SessionEnvelopeReport<T> {
    SessionEnvelopeReport { session_report, adapter_capabilities }
}

pub fn plan_template_directory_session_envelope_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionEnvelopeReport<TemplateDirectorySessionReport>> {
    let session_report = plan_template_directory_session_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    )?;
    let adapter_capabilities = report_default_adapter_capabilities_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        allowed_families,
        config,
    )?;
    Ok(report_template_directory_session_envelope(session_report, adapter_capabilities))
}

pub fn apply_template_directory_session_envelope_with_default_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionEnvelopeReport<TemplateDirectoryRegistrySessionReport>> {
    let session_report = apply_template_directory_session_with_default_registry_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        allowed_families,
        config,
    )?;
    let adapter_capabilities = report_default_adapter_capabilities_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        allowed_families,
        config,
    )?;
    Ok(report_template_directory_session_envelope(session_report, adapter_capabilities))
}
