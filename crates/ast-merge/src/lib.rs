use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const PACKAGE_NAME: &str = "ast-merge";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    ParseError,
    DestinationParseError,
    UnsupportedFeature,
    FallbackApplied,
    Ambiguity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParseResult<TAnalysis> {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub analysis: Option<TAnalysis>,
    pub policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MergeResult<TOutput> {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub output: Option<TOutput>,
    pub policies: Vec<PolicyReference>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySurface {
    Fallback,
    Array,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolicyReference {
    pub surface: PolicySurface,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FamilyFeatureProfile {
    pub family: String,
    pub supported_dialects: Vec<String>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseRef {
    pub family: String,
    pub role: String,
    pub case: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseResult {
    #[serde(rename = "ref")]
    pub ref_: ConformanceCaseRef,
    pub outcome: ConformanceOutcome,
    pub messages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseRequirements {
    pub dialect: Option<String>,
    #[serde(default)]
    pub policies: Vec<PolicyReference>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceSelectionStatus {
    Selected,
    Skipped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseSelection {
    #[serde(rename = "ref")]
    pub ref_: ConformanceCaseRef,
    pub status: ConformanceSelectionStatus,
    pub messages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceFeatureProfileView {
    pub backend: String,
    pub supports_dialects: bool,
    #[serde(default)]
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseRun {
    #[serde(rename = "ref")]
    pub ref_: ConformanceCaseRef,
    pub requirements: ConformanceCaseRequirements,
    pub family_profile: FamilyFeatureProfile,
    pub feature_profile: Option<ConformanceFeatureProfileView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceCaseExecution {
    pub outcome: ConformanceOutcome,
    pub messages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestEntry {
    pub role: String,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceFamilyFeatureProfileEntry {
    pub family: String,
    pub role: String,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifest {
    pub family_feature_profiles: Vec<ConformanceFamilyFeatureProfileEntry>,
    pub families: HashMap<String, Vec<ConformanceManifestEntry>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteReport {
    pub results: Vec<ConformanceCaseResult>,
    pub summary: ConformanceSuiteSummary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuitePlanEntry {
    #[serde(rename = "ref")]
    pub ref_: ConformanceCaseRef,
    pub path: Vec<String>,
    pub run: ConformanceCaseRun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuitePlan {
    pub family: String,
    pub entries: Vec<ConformanceSuitePlanEntry>,
    pub missing_roles: Vec<String>,
}

fn includes_policy(supported_policies: &[PolicyReference], policy: &PolicyReference) -> bool {
    supported_policies.iter().any(|supported_policy| supported_policy == policy)
}

fn is_default_dialect(family_profile: &FamilyFeatureProfile, dialect: &str) -> bool {
    dialect == family_profile.family
}

pub fn conformance_family_entries<'a>(
    manifest: &'a ConformanceManifest,
    family: &str,
) -> &'a [ConformanceManifestEntry] {
    manifest.families.get(family).map(Vec::as_slice).unwrap_or(&[])
}

pub fn conformance_fixture_path<'a>(
    manifest: &'a ConformanceManifest,
    family: &str,
    role: &str,
) -> Option<&'a [String]> {
    conformance_family_entries(manifest, family)
        .iter()
        .find(|entry| entry.role == role)
        .map(|entry| entry.path.as_slice())
}

pub fn conformance_family_feature_profile_path<'a>(
    manifest: &'a ConformanceManifest,
    family: &str,
) -> Option<&'a [String]> {
    manifest
        .family_feature_profiles
        .iter()
        .find(|entry| entry.family == family)
        .map(|entry| entry.path.as_slice())
}

pub fn summarize_conformance_results(results: &[ConformanceCaseResult]) -> ConformanceSuiteSummary {
    results.iter().fold(
        ConformanceSuiteSummary { total: 0, passed: 0, failed: 0, skipped: 0 },
        |summary, result| ConformanceSuiteSummary {
            total: summary.total + 1,
            passed: summary.passed
                + usize::from(matches!(result.outcome, ConformanceOutcome::Passed)),
            failed: summary.failed
                + usize::from(matches!(result.outcome, ConformanceOutcome::Failed)),
            skipped: summary.skipped
                + usize::from(matches!(result.outcome, ConformanceOutcome::Skipped)),
        },
    )
}

pub fn select_conformance_case(
    ref_: ConformanceCaseRef,
    requirements: &ConformanceCaseRequirements,
    family_profile: &FamilyFeatureProfile,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> ConformanceCaseSelection {
    let mut messages = Vec::new();

    if let Some(dialect) = &requirements.dialect {
        if !family_profile
            .supported_dialects
            .iter()
            .any(|supported_dialect| supported_dialect == dialect)
        {
            messages.push(format!(
                "family {} does not support dialect {}.",
                family_profile.family, dialect
            ));
        } else if let Some(feature_profile) = feature_profile {
            if !feature_profile.supports_dialects && !is_default_dialect(family_profile, dialect) {
                messages.push(format!(
                    "backend {} does not support dialect {} for family {}.",
                    feature_profile.backend, dialect, family_profile.family
                ));
            }
        }
    }

    for policy in &requirements.policies {
        if !includes_policy(&family_profile.supported_policies, policy) {
            messages.push(format!(
                "family {} does not support policy {}.",
                family_profile.family, policy.name
            ));
            continue;
        }

        if let Some(feature_profile) = feature_profile {
            if !includes_policy(&feature_profile.supported_policies, policy) {
                messages.push(format!(
                    "backend {} does not support policy {}.",
                    feature_profile.backend, policy.name
                ));
            }
        }
    }

    ConformanceCaseSelection {
        ref_,
        status: if messages.is_empty() {
            ConformanceSelectionStatus::Selected
        } else {
            ConformanceSelectionStatus::Skipped
        },
        messages,
    }
}

pub fn run_conformance_case(
    run: &ConformanceCaseRun,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution,
) -> ConformanceCaseResult {
    let selection = select_conformance_case(
        run.ref_.clone(),
        &run.requirements,
        &run.family_profile,
        run.feature_profile.as_ref(),
    );

    if matches!(selection.status, ConformanceSelectionStatus::Skipped) {
        return ConformanceCaseResult {
            ref_: run.ref_.clone(),
            outcome: ConformanceOutcome::Skipped,
            messages: selection.messages,
        };
    }

    let execution = execute(run);
    ConformanceCaseResult {
        ref_: run.ref_.clone(),
        outcome: execution.outcome,
        messages: execution.messages,
    }
}

pub fn run_conformance_suite(
    runs: &[ConformanceCaseRun],
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> Vec<ConformanceCaseResult> {
    runs.iter().map(|run| run_conformance_case(run, execute)).collect()
}

pub fn run_planned_conformance_suite(
    plan: &ConformanceSuitePlan,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> Vec<ConformanceCaseResult> {
    plan.entries.iter().map(|entry| run_conformance_case(&entry.run, execute)).collect()
}

pub fn report_planned_conformance_suite(
    plan: &ConformanceSuitePlan,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> ConformanceSuiteReport {
    report_conformance_suite(&run_planned_conformance_suite(plan, execute))
}

pub fn report_conformance_suite(results: &[ConformanceCaseResult]) -> ConformanceSuiteReport {
    ConformanceSuiteReport {
        results: results.to_vec(),
        summary: summarize_conformance_results(results),
    }
}

pub fn plan_conformance_suite(
    manifest: &ConformanceManifest,
    family: &str,
    roles: &[String],
    family_profile: &FamilyFeatureProfile,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> ConformanceSuitePlan {
    let mut entries = Vec::new();
    let mut missing_roles = Vec::new();

    for role in roles {
        let Some(path) = conformance_fixture_path(manifest, family, role) else {
            missing_roles.push(role.clone());
            continue;
        };

        let ref_ = ConformanceCaseRef {
            family: family.to_string(),
            role: role.clone(),
            case: role.clone(),
        };

        entries.push(ConformanceSuitePlanEntry {
            ref_: ref_.clone(),
            path: path.to_vec(),
            run: ConformanceCaseRun {
                ref_,
                requirements: ConformanceCaseRequirements { dialect: None, policies: Vec::new() },
                family_profile: family_profile.clone(),
                feature_profile: feature_profile.cloned(),
            },
        });
    }

    ConformanceSuitePlan { family: family.to_string(), entries, missing_roles }
}
