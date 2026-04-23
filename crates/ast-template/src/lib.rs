use ast_merge::{
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplateStrategy,
    TemplateStrategyOverride, TemplateTokenConfig, TemplateTreeRunResult,
    apply_template_tree_execution_to_directory, default_template_token_config,
    plan_template_tree_execution_from_directories, report_template_directory_runner,
};
use markdown_merge::{MarkdownDialect, merge_markdown};
use ruby_merge::{RubyDialect, merge_ruby};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
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
pub struct SessionStatusReport {
    pub mode: DirectorySessionMode,
    pub ready: bool,
    pub missing_families: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub planned_write_count: usize,
    pub written_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDiagnostic {
    pub severity: ast_merge::DiagnosticSeverity,
    pub category: ast_merge::DiagnosticCategory,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDiagnosticsReport {
    pub mode: DirectorySessionMode,
    pub ready: bool,
    pub diagnostics: Vec<SessionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionOutcomeReport<T> {
    pub session_report: T,
    pub status: SessionStatusReport,
    pub diagnostics: SessionDiagnosticsReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRequestReport {
    pub request_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    pub mode: DirectorySessionMode,
    pub ready: bool,
    pub diagnostics: Vec<SessionDiagnostic>,
    pub resolved_options: Option<DirectorySessionOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRunnerRequest {
    pub request_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRunnerInput {
    pub request_kind: String,
    #[serde(default)]
    pub profile_name: Option<String>,
    pub mode: DirectorySessionMode,
    pub template_root: String,
    pub destination_root: String,
    #[serde(default)]
    pub context: TemplateDestinationContext,
    #[serde(default = "default_template_strategy")]
    pub default_strategy: TemplateStrategy,
    #[serde(default)]
    pub overrides: Vec<TemplateStrategyOverride>,
    #[serde(default)]
    pub replacements: HashMap<String, String>,
    #[serde(default)]
    pub allowed_families: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnySessionOutcomeReport {
    Plan(SessionOutcomeReport<TemplateDirectorySessionReport>),
    Registry(SessionOutcomeReport<TemplateDirectoryRegistrySessionReport>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorySessionOptions {
    pub mode: DirectorySessionMode,
    pub template_root: PathBuf,
    pub destination_root: PathBuf,
    pub context: TemplateDestinationContext,
    pub default_strategy: TemplateStrategy,
    pub overrides: Vec<TemplateStrategyOverride>,
    pub replacements: HashMap<String, String>,
    pub allowed_families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TemplateTokenConfig>,
}

impl Default for DirectorySessionOptions {
    fn default() -> Self {
        Self {
            mode: DirectorySessionMode::Plan,
            template_root: PathBuf::new(),
            destination_root: PathBuf::new(),
            context: TemplateDestinationContext::default(),
            default_strategy: TemplateStrategy::Merge,
            overrides: Vec::new(),
            replacements: HashMap::new(),
            allowed_families: None,
            config: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorySessionProfile {
    pub mode: DirectorySessionMode,
    pub context: TemplateDestinationContext,
    pub default_strategy: TemplateStrategy,
    pub overrides: Vec<TemplateStrategyOverride>,
    pub replacements: HashMap<String, String>,
    pub allowed_families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TemplateTokenConfig>,
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

pub fn report_template_directory_session_status<T>(
    envelope: &SessionEnvelopeReport<T>,
) -> SessionStatusReport
where
    T: SessionReportView,
{
    let session_report = envelope.session_report.runner_report();
    let mut blocked_paths = session_report
        .plan_report
        .entries
        .iter()
        .filter(|entry| entry.status == ast_merge::TemplateDirectoryPlanStatus::Blocked)
        .filter_map(|entry| entry.destination_path.clone())
        .collect::<Vec<_>>();
    if let Some(apply_report) = &session_report.apply_report {
        blocked_paths.extend(
            apply_report
                .entries
                .iter()
                .filter(|entry| entry.status == ast_merge::TemplateTreeRunStatus::Blocked)
                .filter_map(|entry| entry.destination_path.clone()),
        );
    }
    blocked_paths.sort();
    blocked_paths.dedup();
    let mut missing_families = envelope.adapter_capabilities.missing_families.clone();
    missing_families.sort();
    SessionStatusReport {
        mode: envelope.session_report.mode(),
        ready: envelope.adapter_capabilities.ready && blocked_paths.is_empty(),
        missing_families,
        blocked_paths,
        planned_write_count: session_report.plan_report.summary.create
            + session_report.plan_report.summary.update,
        written_count: session_report
            .apply_report
            .as_ref()
            .map(|report| report.summary.written)
            .unwrap_or(0),
    }
}

pub trait SessionReportView {
    fn mode(&self) -> DirectorySessionMode;
    fn runner_report(&self) -> &ast_merge::TemplateDirectoryRunnerReport;
}

impl SessionReportView for TemplateDirectorySessionReport {
    fn mode(&self) -> DirectorySessionMode {
        self.mode
    }

    fn runner_report(&self) -> &ast_merge::TemplateDirectoryRunnerReport {
        &self.runner_report
    }
}

impl SessionReportView for TemplateDirectoryRegistrySessionReport {
    fn mode(&self) -> DirectorySessionMode {
        self.mode
    }

    fn runner_report(&self) -> &ast_merge::TemplateDirectoryRunnerReport {
        &self.runner_report
    }
}

pub fn report_template_directory_session_diagnostics(
    mode: DirectorySessionMode,
    entries: &[TemplateExecutionPlanEntry],
    capabilities: &AdapterCapabilityReport,
    result: Option<&TemplateTreeRunResult>,
) -> SessionDiagnosticsReport {
    let missing_families = capabilities.missing_families.iter().cloned().collect::<Vec<_>>();
    let blocked_apply_paths =
        result.map(|value| value.apply_result.blocked_paths.clone()).unwrap_or_default();
    let mut diagnostics = entries
        .iter()
        .flat_map(|entry| {
            let path = entry
                .destination_path
                .clone()
                .unwrap_or_else(|| entry.logical_destination_path.clone());
            let mut output = Vec::new();
            if entry.blocked
                && entry.block_reason == Some(ast_merge::TemplatePlanBlockReason::UnresolvedTokens)
            {
                output.push(SessionDiagnostic {
                    severity: ast_merge::DiagnosticSeverity::Error,
                    category: ast_merge::DiagnosticCategory::ConfigurationError,
                    reason: "unresolved_tokens".to_string(),
                    path: Some(path.clone()),
                    family: None,
                    message: format!("unresolved template tokens block {path}"),
                });
            }
            if entry.execution_action == ast_merge::TemplateExecutionAction::MergePreparedContent
                && missing_families.contains(&entry.classification.family)
                && (result.is_none()
                    || blocked_apply_paths.is_empty()
                    || blocked_apply_paths.contains(&path))
            {
                output.push(SessionDiagnostic {
                    severity: ast_merge::DiagnosticSeverity::Error,
                    category: ast_merge::DiagnosticCategory::ConfigurationError,
                    reason: "missing_family_adapter".to_string(),
                    path: Some(path.clone()),
                    family: Some(entry.classification.family.clone()),
                    message: format!(
                        "missing family adapter for {} blocks {path}",
                        entry.classification.family
                    ),
                });
            }
            output
        })
        .collect::<Vec<_>>();
    diagnostics.sort_by(|a, b| {
        a.path.cmp(&b.path).then(a.reason.cmp(&b.reason)).then(a.family.cmp(&b.family))
    });
    SessionDiagnosticsReport { mode, ready: diagnostics.is_empty(), diagnostics }
}

pub fn plan_template_directory_session_diagnostics_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionDiagnosticsReport> {
    let entries = plan_template_tree_execution_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    )?;
    let registry = default_family_merge_adapter_registry(allowed_families);
    let capabilities = report_adapter_capabilities(&entries, &registry);
    Ok(report_template_directory_session_diagnostics(
        DirectorySessionMode::Plan,
        &entries,
        &capabilities,
        None,
    ))
}

pub fn apply_template_directory_session_diagnostics_with_default_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionDiagnosticsReport> {
    let registry = default_family_merge_adapter_registry(allowed_families);
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        |entry| merge_prepared_content_from_registry(&registry, entry),
        config,
    )?;
    let capabilities = report_adapter_capabilities(&result.execution_plan, &registry);
    Ok(report_template_directory_session_diagnostics(
        DirectorySessionMode::Apply,
        &result.execution_plan,
        &capabilities,
        Some(&result),
    ))
}

pub fn report_template_directory_session_outcome<T>(
    session_report: T,
    status: SessionStatusReport,
    diagnostics: SessionDiagnosticsReport,
) -> SessionOutcomeReport<T> {
    SessionOutcomeReport { session_report, status, diagnostics }
}

pub fn plan_template_directory_session_outcome_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionOutcomeReport<TemplateDirectorySessionReport>> {
    let session_report = plan_template_directory_session_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    )?;
    let envelope = plan_template_directory_session_envelope_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        allowed_families,
        config,
    )?;
    let diagnostics = plan_template_directory_session_diagnostics_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        allowed_families,
        config,
    )?;
    Ok(report_template_directory_session_outcome(
        session_report,
        report_template_directory_session_status(&envelope),
        diagnostics,
    ))
}

pub fn apply_template_directory_session_outcome_with_default_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionOutcomeReport<TemplateDirectoryRegistrySessionReport>> {
    let registry = default_family_merge_adapter_registry(allowed_families);
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        |entry| merge_prepared_content_from_registry(&registry, entry),
        config,
    )?;
    let session_report = report_template_directory_registry_session(
        DirectorySessionMode::Apply,
        &result.execution_plan,
        Some(&result),
        &registry,
    );
    let capabilities = report_adapter_capabilities(&result.execution_plan, &registry);
    let status = report_template_directory_session_status(
        &report_template_directory_session_envelope(session_report.clone(), capabilities.clone()),
    );
    let diagnostics = report_template_directory_session_diagnostics(
        DirectorySessionMode::Apply,
        &result.execution_plan,
        &capabilities,
        Some(&result),
    );
    Ok(report_template_directory_session_outcome(session_report, status, diagnostics))
}

pub fn reapply_template_directory_session_outcome_with_default_registry_to_directory(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<SessionOutcomeReport<TemplateDirectoryRegistrySessionReport>> {
    let registry = default_family_merge_adapter_registry(allowed_families);
    let result = apply_template_tree_execution_to_directory(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        |entry| merge_prepared_content_from_registry(&registry, entry),
        config,
    )?;
    let session_report = report_template_directory_registry_session(
        DirectorySessionMode::Reapply,
        &result.execution_plan,
        Some(&result),
        &registry,
    );
    let capabilities = report_adapter_capabilities(&result.execution_plan, &registry);
    let status = report_template_directory_session_status(
        &report_template_directory_session_envelope(session_report.clone(), capabilities.clone()),
    );
    let diagnostics = report_template_directory_session_diagnostics(
        DirectorySessionMode::Reapply,
        &result.execution_plan,
        &capabilities,
        Some(&result),
    );
    Ok(report_template_directory_session_outcome(session_report, status, diagnostics))
}

pub fn run_template_directory_session_with_default_registry_to_directory(
    mode: DirectorySessionMode,
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    allowed_families: Option<&[&str]>,
    config: &TemplateTokenConfig,
) -> std::io::Result<AnySessionOutcomeReport> {
    match mode {
        DirectorySessionMode::Plan => Ok(AnySessionOutcomeReport::Plan(
            plan_template_directory_session_outcome_from_directories(
                template_root,
                destination_root,
                context,
                default_strategy,
                overrides,
                replacements,
                allowed_families,
                config,
            )?,
        )),
        DirectorySessionMode::Apply => Ok(AnySessionOutcomeReport::Registry(
            apply_template_directory_session_outcome_with_default_registry_to_directory(
                template_root,
                destination_root,
                context,
                default_strategy,
                overrides,
                replacements,
                allowed_families,
                config,
            )?,
        )),
        DirectorySessionMode::Reapply => Ok(AnySessionOutcomeReport::Registry(
            reapply_template_directory_session_outcome_with_default_registry_to_directory(
                template_root,
                destination_root,
                context,
                default_strategy,
                overrides,
                replacements,
                allowed_families,
                config,
            )?,
        )),
    }
}

pub fn run_template_directory_session_with_options(
    options: &DirectorySessionOptions,
) -> std::io::Result<AnySessionOutcomeReport> {
    let request = report_template_directory_session_options_request(options);
    if !request.ready {
        return Ok(report_template_directory_session_configuration_outcome(
            request.mode,
            SessionDiagnosticsReport {
                mode: request.mode,
                ready: request.ready,
                diagnostics: request.diagnostics,
            },
        ));
    }
    let resolved = request
        .resolved_options
        .expect("ready options request should resolve options");
    let allowed = resolved
        .allowed_families
        .as_ref()
        .map(|families| families.iter().map(|family| family.as_str()).collect::<Vec<_>>());
    let default_config = default_template_token_config();
    run_template_directory_session_with_default_registry_to_directory(
        resolved.mode,
        &resolved.template_root,
        &resolved.destination_root,
        &resolved.context,
        resolved.default_strategy,
        &resolved.overrides,
        &resolved.replacements,
        allowed.as_deref(),
        resolved.config.as_ref().unwrap_or(&default_config),
    )
}

fn normalize_session_mode(mode: DirectorySessionMode) -> DirectorySessionMode {
    match mode {
        DirectorySessionMode::Apply | DirectorySessionMode::Reapply => mode,
        DirectorySessionMode::Plan => DirectorySessionMode::Plan,
    }
}

pub fn report_template_directory_session_options_configuration(
    options: &DirectorySessionOptions,
) -> SessionDiagnosticsReport {
    let mut diagnostics = Vec::new();
    if options.destination_root.as_os_str().is_empty() {
        diagnostics.push(SessionDiagnostic {
            severity: ast_merge::DiagnosticSeverity::Error,
            category: ast_merge::DiagnosticCategory::ConfigurationError,
            reason: "missing_destination_root".to_string(),
            path: None,
            family: None,
            message: "missing destination_root for template session".to_string(),
        });
    }
    if options.template_root.as_os_str().is_empty() {
        diagnostics.push(SessionDiagnostic {
            severity: ast_merge::DiagnosticSeverity::Error,
            category: ast_merge::DiagnosticCategory::ConfigurationError,
            reason: "missing_template_root".to_string(),
            path: None,
            family: None,
            message: "missing template_root for template session".to_string(),
        });
    }
    diagnostics.sort_by(|a, b| a.reason.cmp(&b.reason));
    SessionDiagnosticsReport {
        mode: normalize_session_mode(options.mode),
        ready: diagnostics.is_empty(),
        diagnostics,
    }
}

pub fn report_template_directory_session_profile_configuration(
    profiles: &HashMap<String, DirectorySessionProfile>,
    profile_name: &str,
    overrides: &DirectorySessionOptions,
) -> SessionDiagnosticsReport {
    let mut report = report_template_directory_session_options_configuration(overrides);
    if let Some(profile) = profiles.get(profile_name) {
        report.mode = normalize_session_mode(match overrides.mode {
            DirectorySessionMode::Plan if !matches!(profile.mode, DirectorySessionMode::Plan) => {
                profile.mode
            }
            _ => overrides.mode,
        });
    } else {
        report.diagnostics.push(SessionDiagnostic {
            severity: ast_merge::DiagnosticSeverity::Error,
            category: ast_merge::DiagnosticCategory::ConfigurationError,
            reason: "missing_profile".to_string(),
            path: None,
            family: None,
            message: format!("unknown template session profile: {profile_name}"),
        });
    }
    report.diagnostics.sort_by(|a, b| a.reason.cmp(&b.reason));
    report.ready = report.diagnostics.is_empty();
    report
}

pub fn report_template_directory_session_options_request(
    options: &DirectorySessionOptions,
) -> SessionRequestReport {
    let configuration = report_template_directory_session_options_configuration(options);
    SessionRequestReport {
        request_kind: "options".to_string(),
        profile_name: None,
        mode: configuration.mode,
        ready: configuration.ready,
        diagnostics: configuration.diagnostics,
        resolved_options: configuration.ready.then(|| options.clone()),
    }
}

pub fn report_template_directory_session_profile_request(
    profiles: &HashMap<String, DirectorySessionProfile>,
    profile_name: &str,
    overrides: &DirectorySessionOptions,
) -> SessionRequestReport {
    let configuration =
        report_template_directory_session_profile_configuration(profiles, profile_name, overrides);
    let resolved_options = if configuration.ready {
        resolve_template_directory_session_options(profiles, profile_name, overrides)
    } else {
        None
    };
    SessionRequestReport {
        request_kind: "profile".to_string(),
        profile_name: Some(profile_name.to_string()),
        mode: configuration.mode,
        ready: configuration.ready,
        diagnostics: configuration.diagnostics,
        resolved_options,
    }
}

fn report_template_directory_session_configuration_outcome(
    mode: DirectorySessionMode,
    diagnostics: SessionDiagnosticsReport,
) -> AnySessionOutcomeReport {
    AnySessionOutcomeReport::Plan(report_template_directory_session_outcome(
        report_template_directory_session(mode, &[], None),
        SessionStatusReport {
            mode,
            ready: false,
            missing_families: Vec::new(),
            blocked_paths: Vec::new(),
            planned_write_count: 0,
            written_count: 0,
        },
        diagnostics,
    ))
}

pub fn run_template_directory_session_request(
    request: &SessionRequestReport,
) -> std::io::Result<AnySessionOutcomeReport> {
    if !request.ready {
        return Ok(report_template_directory_session_configuration_outcome(
            request.mode,
            SessionDiagnosticsReport {
                mode: request.mode,
                ready: request.ready,
                diagnostics: request.diagnostics.clone(),
            },
        ));
    }
    let options = request
        .resolved_options
        .clone()
        .expect("ready session request should include resolved options");
    run_template_directory_session_with_options(&options)
}

pub fn run_template_directory_session_runner_request(
    request: &SessionRunnerRequest,
    profiles: &HashMap<String, DirectorySessionProfile>,
) -> std::io::Result<AnySessionOutcomeReport> {
    if request.request_kind == "profile" {
        let report = report_template_directory_session_profile_request(
            profiles,
            request.profile_name.as_deref().unwrap_or(""),
            &decode_session_runner_options(request.overrides.as_ref()),
        );
        return run_template_directory_session_request(&report);
    }
    let report = report_template_directory_session_options_request(
        &decode_session_runner_options(request.options.as_ref()),
    );
    run_template_directory_session_request(&report)
}

pub fn report_template_directory_session_runner_input(
    input: &SessionRunnerInput,
) -> SessionRunnerRequest {
    if input.request_kind == "profile" {
        SessionRunnerRequest {
            request_kind: input.request_kind.clone(),
            profile_name: input.profile_name.clone(),
            options: None,
            overrides: Some(session_runner_input_overrides_value(input)),
        }
    } else {
        SessionRunnerRequest {
            request_kind: input.request_kind.clone(),
            profile_name: None,
            options: Some(session_runner_input_options_value(input)),
            overrides: None,
        }
    }
}

fn session_runner_input_options_value(input: &SessionRunnerInput) -> Value {
    let mut options = Map::new();
    options.insert("mode".to_string(), serde_json::to_value(input.mode).expect("mode should serialize"));
    options.insert("template_root".to_string(), Value::String(input.template_root.clone()));
    options.insert("destination_root".to_string(), Value::String(input.destination_root.clone()));
    options.insert("context".to_string(), serde_json::to_value(&input.context).expect("context should serialize"));
    options.insert(
        "default_strategy".to_string(),
        serde_json::to_value(if input.default_strategy == default_template_strategy() {
            TemplateStrategy::Merge
        } else {
            input.default_strategy
        })
        .expect("strategy should serialize"),
    );
    options.insert(
        "overrides".to_string(),
        serde_json::to_value(&input.overrides).expect("overrides should serialize"),
    );
    options.insert(
        "replacements".to_string(),
        serde_json::to_value(&input.replacements).expect("replacements should serialize"),
    );
    options.insert(
        "allowed_families".to_string(),
        serde_json::to_value(&input.allowed_families).expect("families should serialize"),
    );
    Value::Object(options)
}

fn session_runner_input_overrides_value(input: &SessionRunnerInput) -> Value {
    let mut overrides = Map::new();
    overrides.insert("mode".to_string(), serde_json::to_value(input.mode).expect("mode should serialize"));
    overrides.insert("template_root".to_string(), Value::String(input.template_root.clone()));
    overrides.insert("destination_root".to_string(), Value::String(input.destination_root.clone()));
    if input.context != TemplateDestinationContext::default() {
        overrides.insert("context".to_string(), serde_json::to_value(&input.context).expect("context should serialize"));
    }
    if input.default_strategy != default_template_strategy() {
        overrides.insert(
            "default_strategy".to_string(),
            serde_json::to_value(input.default_strategy).expect("strategy should serialize"),
        );
    }
    if !input.overrides.is_empty() {
        overrides.insert(
            "overrides".to_string(),
            serde_json::to_value(&input.overrides).expect("overrides should serialize"),
        );
    }
    if !input.replacements.is_empty() {
        overrides.insert(
            "replacements".to_string(),
            serde_json::to_value(&input.replacements).expect("replacements should serialize"),
        );
    }
    if let Some(allowed_families) = &input.allowed_families {
        overrides.insert(
            "allowed_families".to_string(),
            serde_json::to_value(allowed_families).expect("families should serialize"),
        );
    }
    Value::Object(overrides)
}

fn decode_session_runner_options(value: Option<&Value>) -> DirectorySessionOptions {
    let mut options = DirectorySessionOptions::default();
    if let Some(Value::Object(section)) = value {
        if let Some(mode) = section.get("mode") {
            options.mode = serde_json::from_value(mode.clone()).expect("mode should deserialize");
        }
        if let Some(template_root) = section.get("template_root").and_then(Value::as_str) {
            options.template_root = PathBuf::from(template_root);
        }
        if let Some(destination_root) = section.get("destination_root").and_then(Value::as_str) {
            options.destination_root = PathBuf::from(destination_root);
        }
        if let Some(context) = section.get("context") {
            options.context = serde_json::from_value(context.clone()).expect("context should deserialize");
        }
        if let Some(default_strategy) = section.get("default_strategy") {
            options.default_strategy =
                serde_json::from_value(default_strategy.clone()).expect("strategy should deserialize");
        }
        if let Some(overrides) = section.get("overrides") {
            options.overrides = serde_json::from_value(overrides.clone()).expect("overrides should deserialize");
        }
        if let Some(replacements) = section.get("replacements") {
            options.replacements =
                serde_json::from_value(replacements.clone()).expect("replacements should deserialize");
        }
        if let Some(allowed_families) = section.get("allowed_families") {
            options.allowed_families =
                serde_json::from_value(allowed_families.clone()).expect("families should deserialize");
        }
    }
    options
}

fn default_template_strategy() -> TemplateStrategy {
    TemplateStrategy::Merge
}

pub fn resolve_template_directory_session_options(
    profiles: &HashMap<String, DirectorySessionProfile>,
    profile_name: &str,
    overrides: &DirectorySessionOptions,
) -> Option<DirectorySessionOptions> {
    let profile = profiles.get(profile_name)?;
    Some(DirectorySessionOptions {
        mode: if matches!(overrides.mode, DirectorySessionMode::Plan)
            && !matches!(profile.mode, DirectorySessionMode::Plan)
        {
            profile.mode
        } else {
            overrides.mode
        },
        template_root: overrides.template_root.clone(),
        destination_root: overrides.destination_root.clone(),
        context: if overrides.context == TemplateDestinationContext::default() {
            profile.context.clone()
        } else {
            overrides.context.clone()
        },
        default_strategy: if overrides.default_strategy == TemplateStrategy::Merge
            && profile.default_strategy != TemplateStrategy::Merge
        {
            profile.default_strategy
        } else {
            overrides.default_strategy
        },
        overrides: if overrides.overrides.is_empty() {
            profile.overrides.clone()
        } else {
            overrides.overrides.clone()
        },
        replacements: if overrides.replacements.is_empty() {
            profile.replacements.clone()
        } else {
            overrides.replacements.clone()
        },
        allowed_families: overrides
            .allowed_families
            .clone()
            .or_else(|| profile.allowed_families.clone()),
        config: overrides.config.clone().or_else(|| profile.config.clone()),
    })
}

pub fn run_template_directory_session_with_profile(
    profiles: &HashMap<String, DirectorySessionProfile>,
    profile_name: &str,
    overrides: &DirectorySessionOptions,
) -> std::io::Result<AnySessionOutcomeReport> {
    let request = report_template_directory_session_profile_request(profiles, profile_name, overrides);
    if !request.ready {
        return Ok(report_template_directory_session_configuration_outcome(
            request.mode,
            SessionDiagnosticsReport {
                mode: request.mode,
                ready: request.ready,
                diagnostics: request.diagnostics,
            },
        ));
    }
    let options = request
        .resolved_options
        .expect("ready profile request should resolve options");
    run_template_directory_session_with_options(&options)
}
