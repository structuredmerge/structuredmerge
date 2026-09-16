//! Typed template reports. Only the explicitly named directory planner reads
//! files; none of these entry points applies a template or writes a destination.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use ast_merge::{
    TemplateDestinationContext, TemplateDirectoryApplyReport, TemplateDirectoryApplyReportEntry,
    TemplateDirectoryApplyReportSummary, TemplateDirectoryPlanReport,
    TemplateDirectoryPlanReportEntry, TemplateDirectoryPlanReportSummary,
    TemplateDirectoryPlanStatus, TemplateDirectoryRunnerReport, TemplateExecutionAction,
    TemplatePreviewResult, TemplateStrategy, TemplateStrategyOverride, TemplateTokenConfig,
    TemplateTreeRunReport, TemplateTreeRunReportEntry, TemplateTreeRunReportSummary,
    TemplateTreeRunStatus,
};
pub use ast_template::{DirectorySessionMode, DirectorySessionProfile, SessionDiagnostic};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateSessionPlanReport {
    pub mode: DirectorySessionMode,
    pub runner_report: TemplateDirectoryRunnerReport,
}

/// Portable string paths replace OS-specific PathBuf at the binding boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateSessionOptions {
    pub mode: DirectorySessionMode,
    pub template_root: String,
    pub destination_root: String,
    pub context: TemplateDestinationContext,
    pub default_strategy: TemplateStrategy,
    pub overrides: Vec<TemplateStrategyOverride>,
    pub replacements: HashMap<String, String>,
    pub allowed_families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TemplateTokenConfig>,
}

impl From<TemplateSessionOptions> for ast_template::DirectorySessionOptions {
    fn from(value: TemplateSessionOptions) -> Self {
        Self {
            mode: value.mode,
            template_root: value.template_root.into(),
            destination_root: value.destination_root.into(),
            context: value.context,
            default_strategy: value.default_strategy,
            overrides: value.overrides,
            replacements: value.replacements,
            allowed_families: value.allowed_families,
            config: value.config,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateSessionRequestReport {
    pub request_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    pub mode: DirectorySessionMode,
    pub ready: bool,
    pub diagnostics: Vec<SessionDiagnostic>,
    pub resolved_options: Option<TemplateSessionOptions>,
}

fn portable_report(report: ast_template::SessionRequestReport) -> TemplateSessionRequestReport {
    TemplateSessionRequestReport {
        request_kind: report.request_kind,
        profile_name: report.profile_name,
        mode: report.mode,
        ready: report.ready,
        diagnostics: report.diagnostics,
        // These paths originate exclusively from the UTF-8 request strings;
        // profile resolution only copies them, never discovers OS-native paths.
        resolved_options: report.resolved_options.map(|value| TemplateSessionOptions {
            mode: value.mode,
            template_root: value
                .template_root
                .into_os_string()
                .into_string()
                .expect("UTF-8 request path"),
            destination_root: value
                .destination_root
                .into_os_string()
                .into_string()
                .expect("UTF-8 request path"),
            context: value.context,
            default_strategy: value.default_strategy,
            overrides: value.overrides,
            replacements: value.replacements,
            allowed_families: value.allowed_families,
            config: value.config,
        }),
    }
}

pub fn report_template_options(options: TemplateSessionOptions) -> TemplateSessionRequestReport {
    portable_report(ast_template::report_template_directory_session_options_request(
        &options.into(),
    ))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateProfileRequest {
    pub profile_name: String,
    pub profiles: HashMap<String, DirectorySessionProfile>,
    pub options: TemplateSessionOptions,
}

pub fn report_template_profile(request: TemplateProfileRequest) -> TemplateSessionRequestReport {
    portable_report(ast_template::report_template_directory_session_profile_request(
        &request.profiles,
        &request.profile_name,
        &request.options.into(),
    ))
}

/// Read local template/destination directories and return a plan. No apply API
/// is exposed here. allowed_families retains its historical report-only meaning
/// and is not an access-control boundary. Callers must authorize both roots.
pub fn plan_template_directory(
    options: TemplateSessionOptions,
) -> Result<TemplateSessionPlanReport, crate::CoreError> {
    if options.mode != DirectorySessionMode::Plan {
        return Err(crate::CoreError {
            code: "template.request.invalid".into(),
            message: "directory planning requires mode=plan".into(),
        });
    }
    let options: ast_template::DirectorySessionOptions = options.into();
    let config = options.config.unwrap_or_else(ast_merge::default_template_token_config);
    ast_template::plan_template_directory_session_from_directories(
        &options.template_root,
        &options.destination_root,
        &options.context,
        options.default_strategy,
        &options.overrides,
        &options.replacements,
        &config,
    )
    .map(|report| TemplateSessionPlanReport {
        mode: report.mode,
        runner_report: report.runner_report,
    })
    .map_err(|error| crate::CoreError {
        code: "template.plan.failed".into(),
        message: error.to_string(),
    })
}
