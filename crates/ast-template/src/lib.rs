use ast_merge::{
    TemplateDestinationContext, TemplateExecutionPlanEntry, TemplateStrategy,
    TemplateStrategyOverride, TemplateTokenConfig, TemplateTreeRunResult,
    apply_template_tree_execution_to_directory, plan_template_tree_execution_from_directories,
    report_template_directory_runner,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
