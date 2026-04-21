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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDiagnosticReason {
    MissingRequiredPayload,
    FamilyMismatch,
    RequestNotFound,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewDiagnosticDetail {
    pub request_id: Option<String>,
    pub action: Option<ReviewDecisionAction>,
    pub reason: Option<ReviewDiagnosticReason>,
    pub payload_kind: Option<String>,
    pub expected_family: Option<String>,
    pub provided_family: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub path: Option<String>,
    pub review: Option<Box<ReviewDiagnosticDetail>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceOwnerKind {
    StructuralOwner,
    OwnedRegion,
    ParentSurface,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SurfaceOwnerRef {
    pub kind: SurfaceOwnerKind,
    pub address: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SurfaceSpan {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiscoveredSurface {
    pub surface_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_language: Option<String>,
    pub effective_language: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SurfaceSpan>,
    pub owner: SurfaceOwnerRef,
    pub reconstruction_strategy: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildOperation {
    pub operation_id: String,
    pub parent_operation_id: String,
    pub requested_strategy: String,
    pub language_chain: Vec<String>,
    pub surface: DiscoveredSurface,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectedChildReviewCase {
    pub case_id: String,
    pub parent_operation_id: String,
    pub child_operation_id: String,
    pub surface_path: String,
    pub delegated_case_id: String,
    pub delegated_apply_group: String,
    pub delegated_runtime_surface_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectedChildReviewGroup {
    pub delegated_apply_group: String,
    pub parent_operation_id: String,
    pub child_operation_id: String,
    pub delegated_runtime_surface_path: String,
    pub case_ids: Vec<String>,
    pub delegated_case_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectedChildReviewGroupProgress {
    pub delegated_apply_group: String,
    pub parent_operation_id: String,
    pub child_operation_id: String,
    pub delegated_runtime_surface_path: String,
    pub resolved_case_ids: Vec<String>,
    pub pending_case_ids: Vec<String>,
    pub complete: bool,
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
    pub backend: Option<String>,
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
    pub suite_descriptors: Vec<ConformanceSuiteDefinition>,
    pub families: HashMap<String, Vec<ConformanceManifestEntry>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteSubject {
    pub grammar: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteSelector {
    pub kind: String,
    pub subject: ConformanceSuiteSubject,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceSuiteDefinition {
    pub kind: String,
    pub subject: ConformanceSuiteSubject,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuiteReport {
    pub suite: ConformanceSuiteDefinition,
    pub report: ConformanceSuiteReport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceFamilyPlanContext {
    pub family_profile: FamilyFeatureProfile,
    pub feature_profile: Option<ConformanceFeatureProfileView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuitePlan {
    pub suite: ConformanceSuiteDefinition,
    pub plan: ConformanceSuitePlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedConformanceSuiteResults {
    pub suite: ConformanceSuiteDefinition,
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
    DelegatedChildGroup,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecisionAction {
    AcceptDefaultContext,
    ProvideExplicitContext,
    ApplyDelegatedChildGroup,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewActionOffer {
    pub action: ReviewDecisionAction,
    pub requires_context: bool,
    pub payload_kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewRequest {
    pub id: String,
    pub kind: ReviewRequestKind,
    pub family: String,
    pub message: String,
    pub blocking: bool,
    pub proposed_context: Option<ConformanceFamilyPlanContext>,
    pub delegated_group: Option<ProjectedChildReviewGroup>,
    pub action_offers: Vec<ReviewActionOffer>,
    pub default_action: Option<ReviewDecisionAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewDecision {
    pub request_id: String,
    pub action: ReviewDecisionAction,
    pub context: Option<ConformanceFamilyPlanContext>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildGroupReviewState {
    pub requests: Vec<ReviewRequest>,
    pub accepted_groups: Vec<ProjectedChildReviewGroup>,
    pub applied_decisions: Vec<ReviewDecision>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildApplyPlanEntry {
    pub request_id: String,
    pub family: String,
    pub delegated_group: ProjectedChildReviewGroup,
    pub decision: ReviewDecision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildApplyPlan {
    pub entries: Vec<DelegatedChildApplyPlanEntry>,
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
    selector: &ConformanceSuiteSelector,
) -> Option<&'a ConformanceSuiteDefinition> {
    manifest.suite_descriptors.iter().find(|definition| {
        definition.kind == selector.kind && definition.subject == selector.subject
    })
}

fn compare_conformance_suite_selectors(
    left: &ConformanceSuiteSelector,
    right: &ConformanceSuiteSelector,
) -> std::cmp::Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.subject.grammar.cmp(&right.subject.grammar))
        .then_with(|| left.subject.variant.cmp(&right.subject.variant))
}

pub fn conformance_suite_selectors(manifest: &ConformanceManifest) -> Vec<ConformanceSuiteSelector> {
    let mut selectors = manifest
        .suite_descriptors
        .iter()
        .map(|definition| ConformanceSuiteSelector {
            kind: definition.kind.clone(),
            subject: definition.subject.clone(),
        })
        .collect::<Vec<_>>();
    selectors.sort_by(compare_conformance_suite_selectors);
    selectors
}

fn conformance_suite_descriptor_string(definition: &ConformanceSuiteDefinition) -> String {
    serde_json::to_string(definition).unwrap_or_else(|_| {
        let mut rendered = format!(
            "{{\"kind\":\"{}\",\"subject\":{{\"grammar\":\"{}\"",
            definition.kind, definition.subject.grammar
        );
        if let Some(variant) = &definition.subject.variant {
            rendered.push_str(&format!(",\"variant\":\"{}\"", variant));
        }
        rendered.push_str("},\"roles\":[");
        rendered.push_str(
            &definition
                .roles
                .iter()
                .map(|role| format!("\"{}\"", role))
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push_str("]}");
        rendered
    })
}

pub fn default_conformance_family_context(
    family_profile: &FamilyFeatureProfile,
) -> ConformanceFamilyPlanContext {
    ConformanceFamilyPlanContext { family_profile: family_profile.clone(), feature_profile: None }
}

pub fn review_request_id_for_family_context(family: &str) -> String {
    format!("family_context:{family}")
}

pub fn group_projected_child_review_cases(
    cases: &[ProjectedChildReviewCase],
) -> Vec<ProjectedChildReviewGroup> {
    let mut groups: Vec<ProjectedChildReviewGroup> = Vec::new();

    for case in cases {
        if let Some(existing) = groups
            .iter_mut()
            .find(|group| group.delegated_apply_group == case.delegated_apply_group)
        {
            existing.case_ids.push(case.case_id.clone());
            existing.delegated_case_ids.push(case.delegated_case_id.clone());
            continue;
        }

        groups.push(ProjectedChildReviewGroup {
            delegated_apply_group: case.delegated_apply_group.clone(),
            parent_operation_id: case.parent_operation_id.clone(),
            child_operation_id: case.child_operation_id.clone(),
            delegated_runtime_surface_path: case.delegated_runtime_surface_path.clone(),
            case_ids: vec![case.case_id.clone()],
            delegated_case_ids: vec![case.delegated_case_id.clone()],
        });
    }

    groups
}

pub fn summarize_projected_child_review_group_progress(
    groups: &[ProjectedChildReviewGroup],
    resolved_case_ids: &[String],
) -> Vec<ProjectedChildReviewGroupProgress> {
    groups
        .iter()
        .map(|group| {
            let resolved = group
                .case_ids
                .iter()
                .filter(|case_id| resolved_case_ids.contains(*case_id))
                .cloned()
                .collect::<Vec<_>>();
            let pending = group
                .case_ids
                .iter()
                .filter(|case_id| !resolved_case_ids.contains(*case_id))
                .cloned()
                .collect::<Vec<_>>();

            ProjectedChildReviewGroupProgress {
                delegated_apply_group: group.delegated_apply_group.clone(),
                parent_operation_id: group.parent_operation_id.clone(),
                child_operation_id: group.child_operation_id.clone(),
                delegated_runtime_surface_path: group.delegated_runtime_surface_path.clone(),
                resolved_case_ids: resolved,
                pending_case_ids: pending.clone(),
                complete: pending.is_empty(),
            }
        })
        .collect()
}

pub fn select_projected_child_review_groups_ready_for_apply(
    groups: &[ProjectedChildReviewGroup],
    resolved_case_ids: &[String],
) -> Vec<ProjectedChildReviewGroup> {
    groups
        .iter()
        .filter(|group| group.case_ids.iter().all(|case_id| resolved_case_ids.contains(case_id)))
        .cloned()
        .collect()
}

pub fn review_request_id_for_projected_child_group(group: &ProjectedChildReviewGroup) -> String {
    format!("projected_child_group:{}", group.delegated_apply_group)
}

pub fn projected_child_group_review_request(
    group: &ProjectedChildReviewGroup,
    family: &str,
) -> ReviewRequest {
    ReviewRequest {
        id: review_request_id_for_projected_child_group(group),
        kind: ReviewRequestKind::DelegatedChildGroup,
        family: family.to_string(),
        message: format!(
            "delegated child group {} is ready to apply for {}.",
            group.delegated_apply_group, family
        ),
        blocking: true,
        proposed_context: None,
        delegated_group: Some(group.clone()),
        action_offers: vec![ReviewActionOffer {
            action: ReviewDecisionAction::ApplyDelegatedChildGroup,
            requires_context: false,
            payload_kind: None,
        }],
        default_action: Some(ReviewDecisionAction::ApplyDelegatedChildGroup),
    }
}

pub fn select_projected_child_review_groups_accepted_for_apply(
    groups: &[ProjectedChildReviewGroup],
    _family: &str,
    decisions: &[ReviewDecision],
) -> Vec<ProjectedChildReviewGroup> {
    let accepted_request_ids: Vec<String> = decisions
        .iter()
        .filter(|decision| decision.action == ReviewDecisionAction::ApplyDelegatedChildGroup)
        .map(|decision| decision.request_id.clone())
        .collect();

    groups
        .iter()
        .filter(|group| {
            accepted_request_ids.contains(&review_request_id_for_projected_child_group(group))
        })
        .cloned()
        .collect()
}

pub fn review_projected_child_groups(
    groups: &[ProjectedChildReviewGroup],
    family: &str,
    decisions: &[ReviewDecision],
) -> DelegatedChildGroupReviewState {
    let request_ids: Vec<String> =
        groups.iter().map(review_request_id_for_projected_child_group).collect();
    let mut applied_decisions = Vec::new();
    let mut diagnostics = Vec::new();

    for decision in decisions {
        if decision.action != ReviewDecisionAction::ApplyDelegatedChildGroup {
            continue;
        }

        if request_ids.contains(&decision.request_id) {
            applied_decisions.push(decision.clone());
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ReplayRejected,
                message: format!(
                    "review decision {} does not match any current delegated child review request.",
                    decision.request_id
                ),
                path: None,
                review: Some(Box::new(ReviewDiagnosticDetail {
                    request_id: Some(decision.request_id.clone()),
                    action: Some(decision.action),
                    reason: Some(ReviewDiagnosticReason::RequestNotFound),
                    payload_kind: None,
                    expected_family: None,
                    provided_family: None,
                })),
            });
        }
    }

    let accepted_groups =
        select_projected_child_review_groups_accepted_for_apply(groups, family, &applied_decisions);
    let accepted_request_ids: Vec<String> =
        accepted_groups.iter().map(review_request_id_for_projected_child_group).collect();
    let requests = groups
        .iter()
        .filter(|group| {
            !accepted_request_ids.contains(&review_request_id_for_projected_child_group(group))
        })
        .map(|group| projected_child_group_review_request(group, family))
        .collect();

    DelegatedChildGroupReviewState { requests, accepted_groups, applied_decisions, diagnostics }
}

pub fn delegated_child_apply_plan(
    state: &DelegatedChildGroupReviewState,
    family: &str,
) -> DelegatedChildApplyPlan {
    let entries = state
        .accepted_groups
        .iter()
        .filter_map(|group| {
            let request_id = review_request_id_for_projected_child_group(group);
            let decision = state
                .applied_decisions
                .iter()
                .find(|decision| decision.request_id == request_id)?
                .clone();

            Some(DelegatedChildApplyPlanEntry {
                request_id,
                family: family.to_string(),
                delegated_group: group.clone(),
                decision,
            })
        })
        .collect();

    DelegatedChildApplyPlan { entries }
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
    let mut families: Vec<String> = conformance_suite_selectors(manifest)
        .into_iter()
        .filter_map(|selector| conformance_suite_definition(manifest, &selector))
        .map(|definition| definition.subject.grammar.clone())
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

    let mut request_ids: Vec<String> = conformance_suite_selectors(manifest)
        .into_iter()
        .filter_map(|selector| conformance_suite_definition(manifest, &selector))
        .filter(|definition| !options.contexts.contains_key(&definition.subject.grammar))
        .filter(|definition| options.family_profiles.contains_key(&definition.subject.grammar))
        .map(|definition| review_request_id_for_family_context(&definition.subject.grammar))
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
                review: None,
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
                review: None,
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
            review: None,
        }],
    )
}

fn review_decision_for_family_context(
    family: &str,
    options: &ConformanceManifestReviewOptions,
) -> (Option<ConformanceFamilyPlanContext>, Option<ReviewDecision>, bool, Vec<Diagnostic>) {
    let request_id = review_request_id_for_family_context(family);
    let family_profile = options.family_profiles.get(family);

    for decision in &options.review_decisions {
        if decision.request_id != request_id {
            continue;
        }

        match decision.action {
            ReviewDecisionAction::AcceptDefaultContext => {
                let Some(family_profile) = family_profile else {
                    continue;
                };

                return (
                    Some(default_conformance_family_context(family_profile)),
                    Some(decision.clone()),
                    true,
                    Vec::new(),
                );
            }
            ReviewDecisionAction::ProvideExplicitContext => {
                let Some(context) = &decision.context else {
                    return (
                        None,
                        None,
                        false,
                        vec![Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            category: DiagnosticCategory::ConfigurationError,
                            message: format!(
                                "review decision {} requires explicit context payload.",
                                request_id
                            ),
                            path: None,
                            review: Some(Box::new(ReviewDiagnosticDetail {
                                request_id: Some(request_id.clone()),
                                action: Some(ReviewDecisionAction::ProvideExplicitContext),
                                reason: Some(ReviewDiagnosticReason::MissingRequiredPayload),
                                payload_kind: Some("conformance_family_context".to_string()),
                                expected_family: None,
                                provided_family: None,
                            })),
                        }],
                    );
                };

                if context.family_profile.family != family {
                    return (
                        None,
                        None,
                        false,
                        vec![Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            category: DiagnosticCategory::ConfigurationError,
                            message: format!(
                                "review decision {} provided context for {}, expected {}.",
                                request_id, context.family_profile.family, family
                            ),
                            path: None,
                            review: Some(Box::new(ReviewDiagnosticDetail {
                                request_id: Some(request_id.clone()),
                                action: Some(ReviewDecisionAction::ProvideExplicitContext),
                                reason: Some(ReviewDiagnosticReason::FamilyMismatch),
                                payload_kind: None,
                                expected_family: Some(family.to_string()),
                                provided_family: Some(context.family_profile.family.clone()),
                            })),
                        }],
                    );
                }

                return (Some(context.clone()), Some(decision.clone()), false, Vec::new());
            }
            ReviewDecisionAction::ApplyDelegatedChildGroup => continue,
        }
    }

    (None, None, false, Vec::new())
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
                review: None,
            }],
            Vec::new(),
            Vec::new(),
        );
    };

    let (decision_context, decision, assumed_default, decision_diagnostics) =
        review_decision_for_family_context(family, options);
    if let (Some(context), Some(decision)) = (decision_context, decision) {
        return (
            Some(context),
            if assumed_default {
                vec![Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::AssumedDefault,
                    message: format!("using default family context for {family}."),
                    path: None,
                    review: None,
                }]
            } else {
                Vec::new()
            },
            Vec::new(),
            vec![decision],
        );
    }

    (
        None,
        if decision_diagnostics.is_empty() {
            vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!("missing explicit family context for {family}."),
                path: None,
                review: None,
            }]
        } else {
            decision_diagnostics
        },
        vec![ReviewRequest {
            id: review_request_id_for_family_context(family),
            kind: ReviewRequestKind::FamilyContext,
            family: family.to_string(),
            message: format!(
                "explicit family context is required for {family}; a synthesized default may be accepted by review."
            ),
            blocking: true,
            proposed_context: Some(default_conformance_family_context(family_profile)),
            delegated_group: None,
            action_offers: vec![
                ReviewActionOffer {
                    action: ReviewDecisionAction::AcceptDefaultContext,
                    requires_context: false,
                    payload_kind: None,
                },
                ReviewActionOffer {
                    action: ReviewDecisionAction::ProvideExplicitContext,
                    requires_context: true,
                    payload_kind: Some("conformance_family_context".to_string()),
                },
            ],
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

    if let Some(required_backend) = &requirements.backend {
        match feature_profile {
            Some(feature_profile) if &feature_profile.backend != required_backend => {
                messages.push(format!(
                    "case requires backend {} but backend {} is active for family {}.",
                    required_backend, feature_profile.backend, family_profile.family
                ));
            }
            None => {
                messages.push(format!(
                    "case requires backend {} but no backend feature profile is available for family {}.",
                    required_backend, family_profile.family
                ));
            }
            _ => {}
        }
    }

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
    selector: &ConformanceSuiteSelector,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<Vec<ConformanceCaseResult>> {
    let plan = plan_named_conformance_suite(manifest, selector, family_profile, feature_profile)?;
    Some(run_planned_conformance_suite(&plan, execute))
}

pub fn run_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    selector: &ConformanceSuiteSelector,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<NamedConformanceSuiteResults> {
    let results = run_named_conformance_suite(
        manifest,
        selector,
        family_profile,
        execute,
        feature_profile,
    )?;
    let definition = conformance_suite_definition(manifest, selector)?;
    Some(NamedConformanceSuiteResults { suite: definition.clone(), results })
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
    selector: &ConformanceSuiteSelector,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<ConformanceSuiteReport> {
    let plan = plan_named_conformance_suite(manifest, selector, family_profile, feature_profile)?;
    Some(report_planned_conformance_suite(&plan, execute))
}

pub fn report_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    selector: &ConformanceSuiteSelector,
    family_profile: &FamilyFeatureProfile,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<NamedConformanceSuiteReport> {
    let report = report_named_conformance_suite(
        manifest,
        selector,
        family_profile,
        execute,
        feature_profile,
    )?;
    let definition = conformance_suite_definition(manifest, selector)?;
    Some(NamedConformanceSuiteReport { suite: definition.clone(), report })
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
            review: None,
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
            review: None,
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
                        review: Some(Box::new(ReviewDiagnosticDetail {
                            request_id: Some(decision.request_id.clone()),
                            action: Some(decision.action),
                            reason: Some(ReviewDiagnosticReason::RequestNotFound),
                            payload_kind: None,
                            expected_family: None,
                            provided_family: None,
                        })),
                    });
                    None
                }
            })
            .collect();
    }
    let mut resolved_contexts: HashMap<String, Option<ConformanceFamilyPlanContext>> =
        HashMap::new();

    for selector in conformance_suite_selectors(manifest) {
        let Some(definition) = conformance_suite_definition(manifest, &selector) else {
            continue;
        };

        let context = if let Some(context) = resolved_contexts.get(&definition.subject.grammar) {
            context.clone()
        } else {
            let (context, mut context_diagnostics, mut context_requests, mut context_decisions) =
                review_conformance_family_context(&definition.subject.grammar, &effective_options);
            diagnostics.append(&mut context_diagnostics);
            requests.append(&mut context_requests);
            applied_decisions.append(&mut context_decisions);
            resolved_contexts.insert(definition.subject.grammar.clone(), context.clone());
            context
        };

        let Some(context) = context else {
            continue;
        };

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &selector, &context)
        else {
            continue;
        };

        if !entry.plan.missing_roles.is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "suite {} declares missing roles: {}.",
                    conformance_suite_descriptor_string(&entry.suite),
                    entry.plan.missing_roles.join(", ")
                ),
                path: None,
                review: None,
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
                requirements: entry.requirements.clone().unwrap_or(ConformanceCaseRequirements {
                    backend: None,
                    dialect: None,
                    policies: Vec::new(),
                }),
                family_profile: family_profile.clone(),
                feature_profile: feature_profile.cloned(),
            },
        });
    }

    ConformanceSuitePlan { family: family.to_string(), entries, missing_roles }
}

pub fn plan_named_conformance_suite(
    manifest: &ConformanceManifest,
    selector: &ConformanceSuiteSelector,
    family_profile: &FamilyFeatureProfile,
    feature_profile: Option<&ConformanceFeatureProfileView>,
) -> Option<ConformanceSuitePlan> {
    let definition = conformance_suite_definition(manifest, selector)?;
    Some(plan_conformance_suite(
        manifest,
        &definition.subject.grammar,
        &definition.roles,
        family_profile,
        feature_profile,
    ))
}

pub fn plan_named_conformance_suite_entry(
    manifest: &ConformanceManifest,
    selector: &ConformanceSuiteSelector,
    context: &ConformanceFamilyPlanContext,
) -> Option<NamedConformanceSuitePlan> {
    let plan = plan_named_conformance_suite(
        manifest,
        selector,
        &context.family_profile,
        context.feature_profile.as_ref(),
    )?;
    let definition = conformance_suite_definition(manifest, selector)?;
    Some(NamedConformanceSuitePlan { suite: definition.clone(), plan })
}

pub fn plan_named_conformance_suites(
    manifest: &ConformanceManifest,
    contexts: &HashMap<String, ConformanceFamilyPlanContext>,
) -> Vec<NamedConformanceSuitePlan> {
    conformance_suite_selectors(manifest)
        .into_iter()
        .filter_map(|selector| {
            let definition = conformance_suite_definition(manifest, &selector)?;
            let context = contexts.get(&definition.subject.grammar)?;
            plan_named_conformance_suite_entry(manifest, &selector, context)
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

    for selector in conformance_suite_selectors(manifest) {
        let Some(definition) = conformance_suite_definition(manifest, &selector) else {
            continue;
        };

        let context = if let Some(context) = resolved_contexts.get(&definition.subject.grammar) {
            context.clone()
        } else {
            let (context, mut context_diagnostics) =
                resolve_conformance_family_context(&definition.subject.grammar, options);
            diagnostics.append(&mut context_diagnostics);
            resolved_contexts.insert(definition.subject.grammar.clone(), context.clone());
            context
        };

        let Some(context) = context else {
            continue;
        };

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &selector, &context)
        else {
            continue;
        };

        if !entry.plan.missing_roles.is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "suite {} declares missing roles: {}.",
                    conformance_suite_descriptor_string(&entry.suite),
                    entry.plan.missing_roles.join(", ")
                ),
                path: None,
                review: None,
            });
            continue;
        }

        entries.push(entry);
    }

    ConformanceManifestPlan { entries, diagnostics }
}
