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
    AssumedDefault,
    ConfigurationError,
    ReplayRejected,
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
    pub requirements: Option<ConformanceCaseRequirements>,
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
    #[serde(default)]
    pub suites: HashMap<String, ConformanceSuiteDefinition>,
    pub families: HashMap<String, Vec<ConformanceManifestEntry>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteDefinition {
    pub family: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuiteReport {
    pub suite: String,
    pub report: ConformanceSuiteReport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceFamilyPlanContext {
    pub family_profile: FamilyFeatureProfile,
    pub feature_profile: Option<ConformanceFeatureProfileView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuitePlan {
    pub suite: String,
    pub plan: ConformanceSuitePlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuiteResults {
    pub suite: String,
    pub results: Vec<ConformanceCaseResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuiteReportEnvelope {
    pub entries: Vec<NamedConformanceSuiteReport>,
    pub summary: ConformanceSuiteSummary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestPlanningOptions {
    #[serde(default)]
    pub contexts: HashMap<String, ConformanceFamilyPlanContext>,
    #[serde(default)]
    pub family_profiles: HashMap<String, FamilyFeatureProfile>,
    #[serde(default)]
    pub require_explicit_contexts: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestReport {
    pub report: NamedConformanceSuiteReportEnvelope,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRequestKind {
    FamilyContext,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecisionAction {
    AcceptDefaultContext,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewRequest {
    pub id: String,
    pub kind: ReviewRequestKind,
    pub family: String,
    pub message: String,
    pub blocking: bool,
    pub available_actions: Vec<ReviewDecisionAction>,
    pub default_action: Option<ReviewDecisionAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewDecision {
    pub request_id: String,
    pub action: ReviewDecisionAction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewReplayBundle {
    pub replay_context: ReviewReplayContext,
    pub decisions: Vec<ReviewDecision>,
}

pub const REVIEW_TRANSPORT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTransportImportErrorCategory {
    KindMismatch,
    UnsupportedVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewTransportImportError {
    pub category: ReviewTransportImportErrorCategory,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestReviewStateEnvelope {
    pub kind: String,
    pub version: u32,
    pub state: ConformanceManifestReviewState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewReplayBundleEnvelope {
    pub kind: String,
    pub version: u32,
    pub replay_bundle: ReviewReplayBundle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewHostHints {
    pub interactive: bool,
    pub require_explicit_contexts: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewReplayContext {
    pub surface: String,
    pub families: Vec<String>,
    pub require_explicit_contexts: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestReviewOptions {
    #[serde(default)]
    pub contexts: HashMap<String, ConformanceFamilyPlanContext>,
    #[serde(default)]
    pub family_profiles: HashMap<String, FamilyFeatureProfile>,
    #[serde(default)]
    pub require_explicit_contexts: bool,
    #[serde(default)]
    pub review_decisions: Vec<ReviewDecision>,
    pub review_replay_context: Option<ReviewReplayContext>,
    pub review_replay_bundle: Option<ReviewReplayBundle>,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestReviewState {
    pub report: NamedConformanceSuiteReportEnvelope,
    pub diagnostics: Vec<Diagnostic>,
    pub requests: Vec<ReviewRequest>,
    pub applied_decisions: Vec<ReviewDecision>,
    pub host_hints: ReviewHostHints,
    pub replay_context: ReviewReplayContext,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestPlan {
    pub entries: Vec<NamedConformanceSuitePlan>,
    pub diagnostics: Vec<Diagnostic>,
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

pub fn conformance_suite_definition<'a>(
    manifest: &'a ConformanceManifest,
    suite_name: &str,
) -> Option<&'a ConformanceSuiteDefinition> {
    manifest.suites.get(suite_name)
}

pub fn conformance_suite_names(manifest: &ConformanceManifest) -> Vec<String> {
    let mut names = manifest.suites.keys().cloned().collect::<Vec<_>>();
    names.sort();
    names
}

pub fn default_conformance_family_context(
    family_profile: &FamilyFeatureProfile,
) -> ConformanceFamilyPlanContext {
    ConformanceFamilyPlanContext { family_profile: family_profile.clone(), feature_profile: None }
}

pub fn review_request_id_for_family_context(family: &str) -> String {
    format!("family_context:{family}")
}

pub fn conformance_review_host_hints(
    options: &ConformanceManifestReviewOptions,
) -> ReviewHostHints {
    ReviewHostHints {
        interactive: options.interactive,
        require_explicit_contexts: options.require_explicit_contexts,
    }
}

pub fn conformance_manifest_replay_context(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestReviewOptions,
) -> ReviewReplayContext {
    let mut families: Vec<String> = conformance_suite_names(manifest)
        .into_iter()
        .filter_map(|suite_name| conformance_suite_definition(manifest, &suite_name))
        .map(|definition| definition.family.clone())
        .collect();
    families.sort();
    families.dedup();

    ReviewReplayContext {
        surface: "conformance_manifest".to_string(),
        families,
        require_explicit_contexts: options.require_explicit_contexts,
    }
}

pub fn review_replay_context_compatible(
    current: &ReviewReplayContext,
    candidate: Option<&ReviewReplayContext>,
) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };

    candidate.surface == current.surface
        && candidate.require_explicit_contexts == current.require_explicit_contexts
        && candidate.families == current.families
}

pub fn conformance_manifest_review_request_ids(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestReviewOptions,
) -> Vec<String> {
    if !options.require_explicit_contexts {
        return Vec::new();
    }

    let mut request_ids: Vec<String> = conformance_suite_names(manifest)
        .into_iter()
        .filter_map(|suite_name| conformance_suite_definition(manifest, &suite_name))
        .filter(|definition| !options.contexts.contains_key(&definition.family))
        .filter(|definition| options.family_profiles.contains_key(&definition.family))
        .map(|definition| review_request_id_for_family_context(&definition.family))
        .collect();
    request_ids.sort();
    request_ids.dedup();
    request_ids
}

pub fn review_replay_bundle_inputs(
    options: &ConformanceManifestReviewOptions,
) -> (Option<ReviewReplayContext>, Vec<ReviewDecision>) {
    if let Some(bundle) = &options.review_replay_bundle {
        return (Some(bundle.replay_context.clone()), bundle.decisions.clone());
    }

    (options.review_replay_context.clone(), options.review_decisions.clone())
}

pub fn conformance_manifest_review_state_envelope(
    state: &ConformanceManifestReviewState,
) -> ConformanceManifestReviewStateEnvelope {
    ConformanceManifestReviewStateEnvelope {
        kind: "conformance_manifest_review_state".to_string(),
        version: REVIEW_TRANSPORT_VERSION,
        state: state.clone(),
    }
}

pub fn review_replay_bundle_envelope(bundle: &ReviewReplayBundle) -> ReviewReplayBundleEnvelope {
    ReviewReplayBundleEnvelope {
        kind: "review_replay_bundle".to_string(),
        version: REVIEW_TRANSPORT_VERSION,
        replay_bundle: bundle.clone(),
    }
}

pub fn import_conformance_manifest_review_state_envelope(
    envelope: &ConformanceManifestReviewStateEnvelope,
) -> Result<ConformanceManifestReviewState, ReviewTransportImportError> {
    if envelope.kind != "conformance_manifest_review_state" {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::KindMismatch,
            message: "expected conformance_manifest_review_state envelope kind.".to_string(),
        });
    }

    if envelope.version != REVIEW_TRANSPORT_VERSION {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported conformance_manifest_review_state envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.state.clone())
}

pub fn import_review_replay_bundle_envelope(
    envelope: &ReviewReplayBundleEnvelope,
) -> Result<ReviewReplayBundle, ReviewTransportImportError> {
    if envelope.kind != "review_replay_bundle" {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::KindMismatch,
            message: "expected review_replay_bundle envelope kind.".to_string(),
        });
    }

    if envelope.version != REVIEW_TRANSPORT_VERSION {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported review_replay_bundle envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.replay_bundle.clone())
}

pub fn resolve_conformance_family_context(
    family: &str,
    options: &ConformanceManifestPlanningOptions,
) -> (Option<ConformanceFamilyPlanContext>, Vec<Diagnostic>) {
    if let Some(context) = options.contexts.get(family) {
        return (Some(context.clone()), Vec::new());
    }

    if options.require_explicit_contexts {
        return (
            None,
            vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!("missing explicit family context for {}.", family),
                path: None,
            }],
        );
    }

    if let Some(family_profile) = options.family_profiles.get(family) {
        return (
            Some(default_conformance_family_context(family_profile)),
            vec![Diagnostic {
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::AssumedDefault,
                message: format!("using default family context for {}.", family),
                path: None,
            }],
        );
    }

    (
        None,
        vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ConfigurationError,
            message: format!(
                "missing family context for {} and no default family profile is available.",
                family
            ),
            path: None,
        }],
    )
}

fn review_decision_for_family_context(
    family: &str,
    options: &ConformanceManifestReviewOptions,
) -> Option<ReviewDecision> {
    let request_id = review_request_id_for_family_context(family);
    options
        .review_decisions
        .iter()
        .find(|decision| {
            decision.request_id == request_id
                && decision.action == ReviewDecisionAction::AcceptDefaultContext
        })
        .cloned()
}

pub fn review_conformance_family_context(
    family: &str,
    options: &ConformanceManifestReviewOptions,
) -> (Option<ConformanceFamilyPlanContext>, Vec<Diagnostic>, Vec<ReviewRequest>, Vec<ReviewDecision>)
{
    if let Some(context) = options.contexts.get(family) {
        return (Some(context.clone()), Vec::new(), Vec::new(), Vec::new());
    }

    if !options.require_explicit_contexts {
        let planning_options = ConformanceManifestPlanningOptions {
            contexts: options.contexts.clone(),
            family_profiles: options.family_profiles.clone(),
            require_explicit_contexts: false,
        };
        let (context, diagnostics) = resolve_conformance_family_context(family, &planning_options);
        return (context, diagnostics, Vec::new(), Vec::new());
    }

    let Some(family_profile) = options.family_profiles.get(family) else {
        return (
            None,
            vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "missing family context for {family} and no default family profile is available."
                ),
                path: None,
            }],
            Vec::new(),
            Vec::new(),
        );
    };

    if let Some(decision) = review_decision_for_family_context(family, options) {
        return (
            Some(default_conformance_family_context(family_profile)),
            vec![Diagnostic {
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::AssumedDefault,
                message: format!("using default family context for {family}."),
                path: None,
            }],
            Vec::new(),
            vec![decision],
        );
    }

    (
        None,
        vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ConfigurationError,
            message: format!("missing explicit family context for {family}."),
            path: None,
        }],
        vec![ReviewRequest {
            id: review_request_id_for_family_context(family),
            kind: ReviewRequestKind::FamilyContext,
            family: family.to_string(),
            message: format!(
                "explicit family context is required for {family}; a synthesized default may be accepted by review."
            ),
            blocking: true,
            available_actions: vec![ReviewDecisionAction::AcceptDefaultContext],
            default_action: Some(ReviewDecisionAction::AcceptDefaultContext),
        }],
        Vec::new(),
    )
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

pub fn run_named_conformance_suite(
    manifest: &ConformanceManifest,
    suite_name: &str,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<Vec<ConformanceCaseResult>> {
    let plan = plan_named_conformance_suite(manifest, suite_name, family_profile, feature_profile)?;
    Some(run_planned_conformance_suite(&plan, execute))
}

pub fn run_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    suite_name: &str,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<NamedConformanceSuiteResults> {
    let results = run_named_conformance_suite(
        manifest,
        suite_name,
        family_profile,
        execute,
        feature_profile,
    )?;
    Some(NamedConformanceSuiteResults { suite: suite_name.to_string(), results })
}

pub fn run_planned_named_conformance_suites(
    entries: &[NamedConformanceSuitePlan],
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> Vec<NamedConformanceSuiteResults> {
    entries
        .iter()
        .map(|entry| NamedConformanceSuiteResults {
            suite: entry.suite.clone(),
            results: run_planned_conformance_suite(&entry.plan, execute),
        })
        .collect()
}

pub fn report_planned_conformance_suite(
    plan: &ConformanceSuitePlan,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> ConformanceSuiteReport {
    report_conformance_suite(&run_planned_conformance_suite(plan, execute))
}

pub fn report_named_conformance_suite(
    manifest: &ConformanceManifest,
    suite_name: &str,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<ConformanceSuiteReport> {
    let plan = plan_named_conformance_suite(manifest, suite_name, family_profile, feature_profile)?;
    Some(report_planned_conformance_suite(&plan, execute))
}

pub fn report_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    suite_name: &str,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<NamedConformanceSuiteReport> {
    let report = report_named_conformance_suite(
        manifest,
        suite_name,
        family_profile,
        execute,
        feature_profile,
    )?;
    Some(NamedConformanceSuiteReport { suite: suite_name.to_string(), report })
}

pub fn report_planned_named_conformance_suites(
    entries: &[NamedConformanceSuitePlan],
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> Vec<NamedConformanceSuiteReport> {
    entries
        .iter()
        .map(|entry| NamedConformanceSuiteReport {
            suite: entry.suite.clone(),
            report: report_planned_conformance_suite(&entry.plan, execute),
        })
        .collect()
}

pub fn summarize_named_conformance_suite_reports(
    entries: &[NamedConformanceSuiteReport],
) -> ConformanceSuiteSummary {
    entries.iter().fold(
        ConformanceSuiteSummary { total: 0, passed: 0, failed: 0, skipped: 0 },
        |summary, entry| ConformanceSuiteSummary {
            total: summary.total + entry.report.summary.total,
            passed: summary.passed + entry.report.summary.passed,
            failed: summary.failed + entry.report.summary.failed,
            skipped: summary.skipped + entry.report.summary.skipped,
        },
    )
}

pub fn report_named_conformance_suite_envelope(
    entries: &[NamedConformanceSuiteReport],
) -> NamedConformanceSuiteReportEnvelope {
    NamedConformanceSuiteReportEnvelope {
        entries: entries.to_vec(),
        summary: summarize_named_conformance_suite_reports(entries),
    }
}

pub fn report_named_conformance_suite_manifest(
    manifest: &ConformanceManifest,
    contexts: &HashMap<String, ConformanceFamilyPlanContext>,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> NamedConformanceSuiteReportEnvelope {
    let entries = report_planned_named_conformance_suites(
        &plan_named_conformance_suites(manifest, contexts),
        execute,
    );
    report_named_conformance_suite_envelope(&entries)
}

pub fn report_conformance_manifest(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestPlanningOptions,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> ConformanceManifestReport {
    let planned = plan_named_conformance_suites_with_diagnostics(manifest, options);
    ConformanceManifestReport {
        report: report_named_conformance_suite_envelope(&report_planned_named_conformance_suites(
            &planned.entries,
            execute,
        )),
        diagnostics: planned.diagnostics,
    }
}

pub fn review_conformance_manifest(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestReviewOptions,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> ConformanceManifestReviewState {
    let replay_context = conformance_manifest_replay_context(manifest, options);
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut requests = Vec::new();
    let mut applied_decisions = Vec::new();
    let mut effective_options = options.clone();
    let (replay_input_context, replay_input_decisions) = review_replay_bundle_inputs(options);
    if !replay_input_decisions.is_empty() && replay_input_context.is_none() {
        diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ReplayRejected,
            message: "review decisions were provided without replay context.".to_string(),
            path: None,
        });
        effective_options.review_replay_bundle = None;
        effective_options.review_replay_context = None;
        effective_options.review_decisions.clear();
    } else if !replay_input_decisions.is_empty()
        && !review_replay_context_compatible(&replay_context, replay_input_context.as_ref())
    {
        diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ReplayRejected,
            message: "review replay context does not match the current conformance manifest state."
                .to_string(),
            path: None,
        });
        effective_options.review_replay_bundle = None;
        effective_options.review_replay_context = None;
        effective_options.review_decisions.clear();
    } else if !replay_input_decisions.is_empty() {
        let allowed_request_ids: HashMap<String, bool> =
            conformance_manifest_review_request_ids(manifest, options)
                .into_iter()
                .map(|request_id| (request_id, true))
                .collect();
        effective_options.review_replay_bundle = None;
        effective_options.review_replay_context = replay_input_context;
        effective_options.review_decisions = replay_input_decisions
            .iter()
            .filter_map(|decision| {
                if allowed_request_ids.contains_key(&decision.request_id) {
                    Some(decision.clone())
                } else {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        category: DiagnosticCategory::ReplayRejected,
                        message: format!(
                            "review decision {} does not match any current review request.",
                            decision.request_id
                        ),
                        path: None,
                    });
                    None
                }
            })
            .collect();
    }
    let mut resolved_contexts: HashMap<String, Option<ConformanceFamilyPlanContext>> =
        HashMap::new();

    for suite_name in conformance_suite_names(manifest) {
        let Some(definition) = conformance_suite_definition(manifest, &suite_name) else {
            continue;
        };

        let context = if let Some(context) = resolved_contexts.get(&definition.family) {
            context.clone()
        } else {
            let (context, mut context_diagnostics, mut context_requests, mut context_decisions) =
                review_conformance_family_context(&definition.family, &effective_options);
            diagnostics.append(&mut context_diagnostics);
            requests.append(&mut context_requests);
            applied_decisions.append(&mut context_decisions);
            resolved_contexts.insert(definition.family.clone(), context.clone());
            context
        };

        let Some(context) = context else {
            continue;
        };

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &suite_name, &context)
        else {
            continue;
        };

        if !entry.plan.missing_roles.is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "suite {} declares missing roles: {}.",
                    suite_name,
                    entry.plan.missing_roles.join(", ")
                ),
                path: None,
            });
            continue;
        }

        entries.push(entry);
    }

    ConformanceManifestReviewState {
        report: report_named_conformance_suite_envelope(&report_planned_named_conformance_suites(
            &entries, execute,
        )),
        diagnostics,
        requests,
        applied_decisions,
        host_hints: conformance_review_host_hints(options),
        replay_context,
    }
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
        let Some(entry) =
            conformance_family_entries(manifest, family).iter().find(|entry| entry.role == *role)
        else {
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
            path: entry.path.clone(),
            run: ConformanceCaseRun {
                ref_,
                requirements: entry
                    .requirements
                    .clone()
                    .unwrap_or(ConformanceCaseRequirements { dialect: None, policies: Vec::new() }),
                family_profile: family_profile.clone(),
                feature_profile: feature_profile.cloned(),
            },
        });
    }

    ConformanceSuitePlan { family: family.to_string(), entries, missing_roles }
}

pub fn plan_named_conformance_suite(
    manifest: &ConformanceManifest,
    suite_name: &str,
    family_profile: &FamilyFeatureProfile,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<ConformanceSuitePlan> {
    let definition = conformance_suite_definition(manifest, suite_name)?;
    Some(plan_conformance_suite(
        manifest,
        &definition.family,
        &definition.roles,
        family_profile,
        feature_profile,
    ))
}

pub fn plan_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    suite_name: &str,
    context: &ConformanceFamilyPlanContext,
) -> Option<NamedConformanceSuitePlan> {
    let plan = plan_named_conformance_suite(
        manifest,
        suite_name,
        &context.family_profile,
        context.feature_profile.as_ref(),
    )?;
    Some(NamedConformanceSuitePlan { suite: suite_name.to_string(), plan })
}

pub fn plan_named_conformance_suites(
    manifest: &ConformanceManifest,
    contexts: &HashMap<String, ConformanceFamilyPlanContext>,
) -> Vec<NamedConformanceSuitePlan> {
    conformance_suite_names(manifest)
        .into_iter()
        .filter_map(|suite_name| {
            let definition = conformance_suite_definition(manifest, &suite_name)?;
            let context = contexts.get(&definition.family)?;
            plan_named_conformance_suite_entry(manifest, &suite_name, context)
        })
        .collect()
}

pub fn plan_named_conformance_suites_with_diagnostics(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestPlanningOptions,
) -> ConformanceManifestPlan {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut resolved_contexts: HashMap<String, Option<ConformanceFamilyPlanContext>> =
        HashMap::new();

    for suite_name in conformance_suite_names(manifest) {
        let Some(definition) = conformance_suite_definition(manifest, &suite_name) else {
            continue;
        };

        let context = if let Some(context) = resolved_contexts.get(&definition.family) {
            context.clone()
        } else {
            let (context, mut context_diagnostics) =
                resolve_conformance_family_context(&definition.family, options);
            diagnostics.append(&mut context_diagnostics);
            resolved_contexts.insert(definition.family.clone(), context.clone());
            context
        };

        let Some(context) = context else {
            continue;
        };

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &suite_name, &context)
        else {
            continue;
        };

        if !entry.plan.missing_roles.is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "suite {} declares missing roles: {}.",
                    suite_name,
                    entry.plan.missing_roles.join(", ")
                ),
                path: None,
            });
            continue;
        }

        entries.push(entry);
    }

    ConformanceManifestPlan { entries, diagnostics }
}
