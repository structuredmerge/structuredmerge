use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

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
    KindMismatch,
    UnsupportedVersion,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditStructureProfile {
    pub owner_scope: String,
    pub owner_selector: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_selector_family: Option<String>,
    pub known_owner_selector: bool,
    pub supported_comment_regions: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditSelectionProfile {
    pub owner_scope: String,
    pub owner_selector: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_selector_family: Option<String>,
    pub selector_kind: String,
    pub selection_intent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_intent_family: Option<String>,
    pub known_selection_intent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_region: Option<String>,
    pub include_trailing_gap: bool,
    pub comment_anchored: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditMatchProfile {
    pub start_boundary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_boundary_family: Option<String>,
    pub known_start_boundary: bool,
    pub end_boundary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_boundary_family: Option<String>,
    pub known_end_boundary: bool,
    pub payload_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_family: Option<String>,
    pub known_payload_kind: bool,
    pub comment_anchored: bool,
    pub trailing_gap_extended: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditOperationProfile {
    pub operation_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_family: Option<String>,
    pub known_operation_kind: bool,
    pub source_requirement: String,
    pub destination_requirement: String,
    pub replacement_source: String,
    pub captures_source_text: bool,
    pub supports_if_missing: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditDestinationProfile {
    pub resolution_kind: String,
    pub resolution_source: String,
    pub anchor_boundary: String,
    pub resolution_family: String,
    pub resolution_source_family: String,
    pub anchor_boundary_family: String,
    pub known_resolution_kind: bool,
    pub known_resolution_source: bool,
    pub known_anchor_boundary: bool,
    pub used_if_missing: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditRequest {
    pub operation_kind: String,
    pub content: String,
    pub source_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_selector_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_selector_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_missing: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditResult {
    pub operation_kind: String,
    pub updated_content: String,
    pub changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<usize>,
    pub operation_profile: StructuredEditOperationProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_profile: Option<StructuredEditDestinationProfile>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditApplication {
    pub request: StructuredEditRequest,
    pub result: StructuredEditResult,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredEditTransportImportErrorCategory {
    KindMismatch,
    UnsupportedVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditTransportImportError {
    pub category: StructuredEditTransportImportErrorCategory,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditApplicationEnvelope {
    pub kind: String,
    pub version: u32,
    pub application: StructuredEditApplication,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditExecutionReport {
    pub application: StructuredEditApplication,
    pub provider_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_backend: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionRequest {
    pub request: StructuredEditRequest,
    pub provider_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_backend: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionRequestEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_request: StructuredEditProviderExecutionRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionPlan {
    pub execution_request: StructuredEditProviderExecutionRequest,
    pub executor_resolution: StructuredEditProviderExecutorResolution,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionPlanEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_plan: StructuredEditProviderExecutionPlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionHandoff {
    pub execution_plan: StructuredEditProviderExecutionPlan,
    pub execution_dispatch: StructuredEditProviderExecutionDispatch,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionHandoffEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_handoff: StructuredEditProviderExecutionHandoff,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionInvocation {
    pub execution_handoff: StructuredEditProviderExecutionHandoff,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionInvocationEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_invocation: StructuredEditProviderExecutionInvocation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionInvocation {
    pub invocations: Vec<StructuredEditProviderExecutionInvocation>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionInvocationEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_invocation: StructuredEditProviderBatchExecutionInvocation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionRunResult {
    pub execution_invocation: StructuredEditProviderExecutionInvocation,
    pub outcome: StructuredEditProviderExecutionOutcome,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionRunResultEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_run_result: StructuredEditProviderExecutionRunResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionRunResult {
    pub run_results: Vec<StructuredEditProviderExecutionRunResult>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionRunResultEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_run_result: StructuredEditProviderBatchExecutionRunResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceipt {
    pub run_result: StructuredEditProviderExecutionRunResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<StructuredEditProviderExecutionProvenance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_bundle: Option<StructuredEditProviderExecutionReplayBundle>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution_receipt: StructuredEditProviderExecutionReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceipt {
    pub receipts: Vec<StructuredEditProviderExecutionReceipt>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_receipt: StructuredEditProviderBatchExecutionReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayRequest {
    pub execution_receipt: StructuredEditProviderExecutionReceipt,
    pub replay_mode: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayRequestEnvelope {
    pub kind: String,
    pub version: u32,
    pub receipt_replay_request: StructuredEditProviderExecutionReceiptReplayRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayRequest {
    pub requests: Vec<StructuredEditProviderExecutionReceiptReplayRequest>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_receipt_replay_request: StructuredEditProviderBatchExecutionReceiptReplayRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayApplication {
    pub receipt_replay_request: StructuredEditProviderExecutionReceiptReplayRequest,
    pub run_result: StructuredEditProviderExecutionRunResult,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayApplicationEnvelope {
    pub kind: String,
    pub version: u32,
    pub receipt_replay_application: StructuredEditProviderExecutionReceiptReplayApplication,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayApplication {
    pub applications: Vec<StructuredEditProviderExecutionReceiptReplayApplication>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_receipt_replay_application:
        StructuredEditProviderBatchExecutionReceiptReplayApplication,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplaySession {
    pub receipt_replay_application: StructuredEditProviderExecutionReceiptReplayApplication,
    pub execution_receipt: StructuredEditProviderExecutionReceipt,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplaySessionEnvelope {
    pub kind: String,
    pub version: u32,
    pub receipt_replay_session: StructuredEditProviderExecutionReceiptReplaySession,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplaySession {
    pub sessions: Vec<StructuredEditProviderExecutionReceiptReplaySession>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_receipt_replay_session: StructuredEditProviderBatchExecutionReceiptReplaySession,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayWorkflow {
    pub receipt_replay_session: StructuredEditProviderExecutionReceiptReplaySession,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope {
    pub kind: String,
    pub version: u32,
    pub receipt_replay_workflow: StructuredEditProviderExecutionReceiptReplayWorkflow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayWorkflow {
    pub workflows: Vec<StructuredEditProviderExecutionReceiptReplayWorkflow>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_receipt_replay_workflow: StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReceiptReplayWorkflowResult {
    pub receipt_replay_workflow: StructuredEditProviderExecutionReceiptReplayWorkflow,
    pub receipt_replay_application: StructuredEditProviderExecutionReceiptReplayApplication,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditExecutionReportEnvelope {
    pub kind: String,
    pub version: u32,
    pub report: StructuredEditExecutionReport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionApplication {
    pub execution_request: StructuredEditProviderExecutionRequest,
    pub report: StructuredEditExecutionReport,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionDispatch {
    pub execution_request: StructuredEditProviderExecutionRequest,
    pub resolved_provider_family: String,
    pub resolved_provider_backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_label: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionApplicationEnvelope {
    pub kind: String,
    pub version: u32,
    pub provider_execution_application: StructuredEditProviderExecutionApplication,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionDispatchEnvelope {
    pub kind: String,
    pub version: u32,
    pub provider_execution_dispatch: StructuredEditProviderExecutionDispatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionOutcome {
    pub dispatch: StructuredEditProviderExecutionDispatch,
    pub application: StructuredEditProviderExecutionApplication,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionOutcomeEnvelope {
    pub kind: String,
    pub version: u32,
    pub provider_execution_outcome: StructuredEditProviderExecutionOutcome,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionOutcome {
    pub outcomes: Vec<StructuredEditProviderExecutionOutcome>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionOutcomeEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_outcome: StructuredEditProviderBatchExecutionOutcome,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionProvenance {
    pub dispatch: StructuredEditProviderExecutionDispatch,
    pub outcome: StructuredEditProviderExecutionOutcome,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionProvenanceEnvelope {
    pub kind: String,
    pub version: u32,
    pub provenance: StructuredEditProviderExecutionProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionProvenance {
    pub provenances: Vec<StructuredEditProviderExecutionProvenance>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionProvenanceEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_provenance: StructuredEditProviderBatchExecutionProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReplayBundle {
    pub execution_request: StructuredEditProviderExecutionRequest,
    pub provenance: StructuredEditProviderExecutionProvenance,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutionReplayBundleEnvelope {
    pub kind: String,
    pub version: u32,
    pub replay_bundle: StructuredEditProviderExecutionReplayBundle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReplayBundle {
    pub replay_bundles: Vec<StructuredEditProviderExecutionReplayBundle>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReplayBundleEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_replay_bundle: StructuredEditProviderBatchExecutionReplayBundle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorProfile {
    pub provider_family: String,
    pub provider_backend: String,
    pub executor_label: String,
    pub structure_profile: StructuredEditStructureProfile,
    pub selection_profile: StructuredEditSelectionProfile,
    pub match_profile: StructuredEditMatchProfile,
    pub operation_profiles: Vec<StructuredEditOperationProfile>,
    pub destination_profile: StructuredEditDestinationProfile,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorProfileEnvelope {
    pub kind: String,
    pub version: u32,
    pub executor_profile: StructuredEditProviderExecutorProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorRegistry {
    pub executor_profiles: Vec<StructuredEditProviderExecutorProfile>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorRegistryEnvelope {
    pub kind: String,
    pub version: u32,
    pub executor_registry: StructuredEditProviderExecutorRegistry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorSelectionPolicy {
    pub provider_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_label: Option<String>,
    pub selection_mode: String,
    pub allow_registry_fallback: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorSelectionPolicyEnvelope {
    pub kind: String,
    pub version: u32,
    pub selection_policy: StructuredEditProviderExecutorSelectionPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorResolution {
    pub executor_registry: StructuredEditProviderExecutorRegistry,
    pub selection_policy: StructuredEditProviderExecutorSelectionPolicy,
    pub selected_executor_profile: StructuredEditProviderExecutorProfile,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderExecutorResolutionEnvelope {
    pub kind: String,
    pub version: u32,
    pub executor_resolution: StructuredEditProviderExecutorResolution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditBatchRequest {
    pub requests: Vec<StructuredEditRequest>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionRequest {
    pub requests: Vec<StructuredEditProviderExecutionRequest>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionRequestEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_request: StructuredEditProviderBatchExecutionRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionHandoff {
    pub handoffs: Vec<StructuredEditProviderExecutionHandoff>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionHandoffEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_handoff: StructuredEditProviderBatchExecutionHandoff,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionPlan {
    pub plans: Vec<StructuredEditProviderExecutionPlan>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionPlanEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_execution_plan: StructuredEditProviderBatchExecutionPlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionDispatch {
    pub dispatches: Vec<StructuredEditProviderExecutionDispatch>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionDispatchEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_dispatch: StructuredEditProviderBatchExecutionDispatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditBatchReport {
    pub reports: Vec<StructuredEditExecutionReport>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditBatchReportEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_report: StructuredEditBatchReport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReport {
    pub applications: Vec<StructuredEditProviderExecutionApplication>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredEditProviderBatchExecutionReportEnvelope {
    pub kind: String,
    pub version: u32,
    pub batch_report: StructuredEditProviderBatchExecutionReport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateTargetClassification {
    pub destination_path: String,
    pub file_type: String,
    pub family: String,
    pub dialect: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDestinationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateTokenConfig {
    pub pre: String,
    pub post: String,
    pub separators: Vec<String>,
    pub min_segments: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_segments: Option<usize>,
    pub segment_pattern: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStrategy {
    Merge,
    AcceptTemplate,
    KeepDestination,
    RawCopy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateStrategyOverride {
    pub path: String,
    pub strategy: TemplateStrategy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplatePlanEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    pub classification: TemplateTargetClassification,
    pub strategy: TemplateStrategy,
    pub action: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplatePlanStateEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    pub classification: TemplateTargetClassification,
    pub strategy: TemplateStrategy,
    pub action: String,
    pub destination_exists: bool,
    pub write_action: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplatePlanBlockReason {
    UnresolvedTokens,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplatePlanTokenStateEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    pub classification: TemplateTargetClassification,
    pub strategy: TemplateStrategy,
    pub action: String,
    pub destination_exists: bool,
    pub write_action: String,
    pub token_keys: Vec<String>,
    pub unresolved_token_keys: Vec<String>,
    pub token_resolution_required: bool,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<TemplatePlanBlockReason>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplatePreparationAction {
    Blocked,
    ResolveTokens,
    PassThrough,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplatePreparedEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    pub classification: TemplateTargetClassification,
    pub strategy: TemplateStrategy,
    pub action: String,
    pub destination_exists: bool,
    pub write_action: String,
    pub token_keys: Vec<String>,
    pub unresolved_token_keys: Vec<String>,
    pub token_resolution_required: bool,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<TemplatePlanBlockReason>,
    pub template_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_template_content: Option<String>,
    pub preparation_action: TemplatePreparationAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateExecutionAction {
    Blocked,
    Omit,
    Keep,
    RawCopy,
    WritePreparedContent,
    MergePreparedContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateExecutionPlanEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    pub classification: TemplateTargetClassification,
    pub strategy: TemplateStrategy,
    pub action: String,
    pub destination_exists: bool,
    pub write_action: String,
    pub token_keys: Vec<String>,
    pub unresolved_token_keys: Vec<String>,
    pub token_resolution_required: bool,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<TemplatePlanBlockReason>,
    pub template_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_template_content: Option<String>,
    pub preparation_action: TemplatePreparationAction,
    pub execution_action: TemplateExecutionAction,
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_content: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplatePreviewResult {
    pub result_files: HashMap<String, String>,
    pub created_paths: Vec<String>,
    pub updated_paths: Vec<String>,
    pub kept_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub omitted_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateApplyResult {
    pub result_files: HashMap<String, String>,
    pub created_paths: Vec<String>,
    pub updated_paths: Vec<String>,
    pub kept_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub omitted_paths: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateConvergenceResult {
    pub converged: bool,
    pub pending_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateTreeRunResult {
    pub execution_plan: Vec<TemplateExecutionPlanEntry>,
    pub apply_result: TemplateApplyResult,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateTreeRunStatus {
    Created,
    Updated,
    Kept,
    Blocked,
    Omitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateTreeRunReportEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    pub destination_path: Option<String>,
    pub execution_action: TemplateExecutionAction,
    pub status: TemplateTreeRunStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateTreeRunReportSummary {
    pub created: usize,
    pub updated: usize,
    pub kept: usize,
    pub blocked: usize,
    pub omitted: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateTreeRunReport {
    pub entries: Vec<TemplateTreeRunReportEntry>,
    pub summary: TemplateTreeRunReportSummary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryApplyReportEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    pub destination_path: Option<String>,
    pub execution_action: TemplateExecutionAction,
    pub status: TemplateTreeRunStatus,
    pub written: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryApplyReportSummary {
    pub created: usize,
    pub updated: usize,
    pub kept: usize,
    pub blocked: usize,
    pub omitted: usize,
    pub written: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryApplyReport {
    pub entries: Vec<TemplateDirectoryApplyReportEntry>,
    pub summary: TemplateDirectoryApplyReportSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateDirectoryPlanStatus {
    Create,
    Update,
    Keep,
    Blocked,
    Omitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryPlanReportEntry {
    pub template_source_path: String,
    pub logical_destination_path: String,
    pub destination_path: Option<String>,
    pub execution_action: TemplateExecutionAction,
    pub write_action: String,
    pub status: TemplateDirectoryPlanStatus,
    pub previewable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryPlanReportSummary {
    pub create: usize,
    pub update: usize,
    pub keep: usize,
    pub blocked: usize,
    pub omitted: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryPlanReport {
    pub entries: Vec<TemplateDirectoryPlanReportEntry>,
    pub summary: TemplateDirectoryPlanReportSummary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateDirectoryRunnerReport {
    pub plan_report: TemplateDirectoryPlanReport,
    pub preview: Option<TemplatePreviewResult>,
    pub run_report: Option<TemplateTreeRunReport>,
    pub apply_report: Option<TemplateDirectoryApplyReport>,
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
pub struct DelegatedChildSurfaceOutput {
    pub surface_address: String,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppliedDelegatedChildOutput {
    pub operation_id: String,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildOutputResolutionOptions {
    pub default_family: String,
    pub request_id_prefix: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedChildOutputResolution {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_plan: Option<DelegatedChildApplyPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_children: Option<Vec<AppliedDelegatedChildOutput>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NestedMergeDiscoveryResult {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<DelegatedChildOperation>>,
}

pub struct NestedMergeExecutionCallbacks<
    TOutput,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
> where
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    pub merge_parent: MergeParent,
    pub discover_operations: DiscoverOperations,
    pub apply_resolved_outputs: ApplyResolvedOutputs,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewReplayBundle {
    pub replay_context: ReviewReplayContext,
    pub decisions: Vec<ReviewDecision>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reviewed_nested_executions: Vec<ReviewedNestedExecution>,
}

pub const REVIEW_TRANSPORT_VERSION: u32 = 1;
pub const STRUCTURED_EDIT_TRANSPORT_VERSION: u32 = 1;

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
pub struct ReviewedNestedExecution {
    pub family: String,
    pub review_state: DelegatedChildGroupReviewState,
    pub applied_children: Vec<AppliedDelegatedChildOutput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewedNestedExecutionResult<TOutput> {
    pub execution: ReviewedNestedExecution,
    pub result: MergeResult<TOutput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewedNestedExecutionApplication<TOutput> {
    pub diagnostics: Vec<Diagnostic>,
    pub results: Vec<ReviewedNestedExecutionResult<TOutput>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConformanceManifestReviewedNestedApplication<TOutput> {
    pub state: ConformanceManifestReviewState,
    pub results: Vec<ReviewedNestedExecutionResult<TOutput>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewedNestedExecutionEnvelope {
    pub kind: String,
    pub version: u32,
    pub execution: ReviewedNestedExecution,
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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reviewed_nested_executions: Vec<ReviewedNestedExecution>,
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

pub fn normalize_template_source_path(path: &str) -> String {
    if let Some(stripped) = path.strip_suffix(".no-osc.example") {
        return stripped.to_string();
    }

    if let Some(stripped) = path.strip_suffix(".example") {
        return stripped.to_string();
    }

    path.to_string()
}

pub fn classify_template_target_path(path: &str) -> TemplateTargetClassification {
    let normalized_path = path.trim_start_matches("./");
    let lower_path = normalized_path.to_ascii_lowercase();
    let base = normalized_path.rsplit('/').next().unwrap_or(normalized_path);
    let lower_base = base.to_ascii_lowercase();

    let classify = |file_type: &str, family: &str, dialect: &str| TemplateTargetClassification {
        destination_path: path.to_string(),
        file_type: file_type.to_string(),
        family: family.to_string(),
        dialect: dialect.to_string(),
    };

    match normalized_path {
        ".git-hooks/commit-msg" => return classify("ruby", "ruby", "ruby"),
        ".git-hooks/prepare-commit-msg" => return classify("bash", "bash", "bash"),
        _ => {}
    }

    match base {
        "Gemfile" | "Appraisal.root.gemfile" => return classify("gemfile", "ruby", "ruby"),
        "Appraisals" => return classify("appraisals", "ruby", "ruby"),
        "Rakefile" | ".simplecov" => return classify("ruby", "ruby", "ruby"),
        ".envrc" => return classify("bash", "bash", "bash"),
        ".tool-versions" => return classify("tool_versions", "text", "tool_versions"),
        "CITATION.cff" => return classify("yaml", "yaml", "yaml"),
        _ => {}
    }

    if lower_base.ends_with(".gemspec") {
        return classify("gemspec", "ruby", "ruby");
    }
    if lower_base.ends_with(".gemfile") {
        return classify("gemfile", "ruby", "ruby");
    }
    if lower_base.ends_with(".rb") || lower_base.ends_with(".rake") {
        return classify("ruby", "ruby", "ruby");
    }
    if lower_path.ends_with(".yml") || lower_path.ends_with(".yaml") {
        return classify("yaml", "yaml", "yaml");
    }
    if lower_path.ends_with(".md") || lower_path.ends_with(".markdown") {
        return classify("markdown", "markdown", "markdown");
    }
    if lower_path.ends_with(".sh") || lower_path.ends_with(".bash") {
        return classify("bash", "bash", "bash");
    }
    if lower_base == ".env" || lower_base.starts_with(".env.") {
        return classify("dotenv", "dotenv", "dotenv");
    }
    if lower_path.ends_with(".jsonc") {
        return classify("json", "json", "jsonc");
    }
    if lower_path.ends_with(".json") {
        return classify("json", "json", "json");
    }
    if lower_path.ends_with(".toml") {
        return classify("toml", "toml", "toml");
    }
    if lower_path.ends_with(".rbs") {
        return classify("rbs", "rbs", "rbs");
    }

    classify("text", "text", "text")
}

pub fn resolve_template_destination_path(
    path: &str,
    context: &TemplateDestinationContext,
) -> Option<String> {
    match path {
        ".kettle-jem.yml" => None,
        ".env.local" => Some(".env.local.example".to_string()),
        "gem.gemspec" => context
            .project_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| format!("{name}.gemspec"))
            .or_else(|| Some(path.to_string())),
        _ => Some(path.to_string()),
    }
}

pub fn default_template_token_config() -> TemplateTokenConfig {
    TemplateTokenConfig {
        pre: "{".to_string(),
        post: "}".to_string(),
        separators: vec!["|".to_string(), ":".to_string()],
        min_segments: 2,
        max_segments: None,
        segment_pattern: "[A-Za-z0-9_]".to_string(),
    }
}

fn template_token_separator_at(config: &TemplateTokenConfig, boundary_index: usize) -> &str {
    if boundary_index < config.separators.len() {
        &config.separators[boundary_index]
    } else {
        config
            .separators
            .last()
            .map(String::as_str)
            .expect("template token config should have at least one separator")
    }
}

fn segment_pattern_matches(pattern: &str, ch: char) -> bool {
    if !pattern.starts_with('[') || !pattern.ends_with(']') {
        return false;
    }

    let chars = pattern[1..pattern.len() - 1].chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if index + 2 < chars.len() && chars[index + 1] == '-' {
            if chars[index] <= ch && ch <= chars[index + 2] {
                return true;
            }
            index += 3;
            continue;
        }

        if chars[index] == ch {
            return true;
        }
        index += 1;
    }

    false
}

fn valid_template_token_key(key: &str, config: &TemplateTokenConfig) -> bool {
    if key.is_empty() {
        return false;
    }

    let mut index = 0;
    let mut segments = 0;
    let mut boundary_index = 0;

    while index < key.len() {
        let segment_start = index;
        while index < key.len() {
            let ch = key[index..].chars().next().expect("character should exist");
            if !segment_pattern_matches(&config.segment_pattern, ch) {
                break;
            }
            index += ch.len_utf8();
        }

        if index == segment_start {
            return false;
        }

        segments += 1;
        if index == key.len() {
            break;
        }

        let separator = template_token_separator_at(config, boundary_index);
        if !key[index..].starts_with(separator) {
            return false;
        }

        index += separator.len();
        boundary_index += 1;
    }

    if segments < config.min_segments {
        return false;
    }

    config.max_segments.is_none_or(|max| segments <= max)
}

pub fn template_token_keys(content: &str, config: &TemplateTokenConfig) -> Vec<String> {
    if content.is_empty() || !content.contains(&config.pre) {
        return Vec::new();
    }

    let mut keys = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0;

    while offset < content.len() {
        let Some(token_start) = content[offset..].find(&config.pre) else {
            break;
        };
        let token_start = offset + token_start;
        let content_start = token_start + config.pre.len();
        let Some(token_end) = content[content_start..].find(&config.post) else {
            break;
        };
        let token_end = content_start + token_end;
        let key = &content[content_start..token_end];

        if valid_template_token_key(key, config) && seen.insert(key.to_string()) {
            keys.push(key.to_string());
        }

        offset = token_end + config.post.len();
    }

    keys
}

pub fn unresolved_template_token_keys(
    content: &str,
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> Vec<String> {
    template_token_keys(content, config)
        .into_iter()
        .filter(|key| !replacements.contains_key(key))
        .collect()
}

pub fn resolve_template_tokens(
    content: &str,
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> String {
    let mut resolved = content.to_string();
    for key in template_token_keys(content, config) {
        let Some(replacement) = replacements.get(&key) else {
            continue;
        };
        resolved = resolved.replace(&format!("{}{}{}", config.pre, key, config.post), replacement);
    }
    resolved
}

pub fn select_template_strategy(
    path: &str,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
) -> TemplateStrategy {
    let normalized_path = path.trim_start_matches("./");
    for entry in overrides {
        if entry.path.trim_start_matches("./") == normalized_path {
            return entry.strategy;
        }
    }

    default_strategy
}

pub fn plan_template_entries(
    template_source_paths: &[String],
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
) -> Vec<TemplatePlanEntry> {
    template_source_paths
        .iter()
        .map(|template_source_path| {
            let logical_destination_path = normalize_template_source_path(template_source_path);
            let destination_path =
                resolve_template_destination_path(&logical_destination_path, context);
            let classification = classify_template_target_path(&logical_destination_path);
            let strategy =
                select_template_strategy(&logical_destination_path, default_strategy, overrides);
            let action = if destination_path.is_none() {
                "omit".to_string()
            } else {
                serde_json::to_value(strategy)
                    .expect("strategy should serialize")
                    .as_str()
                    .expect("strategy should serialize as string")
                    .to_string()
            };

            TemplatePlanEntry {
                template_source_path: template_source_path.clone(),
                logical_destination_path,
                destination_path,
                classification,
                strategy,
                action,
            }
        })
        .collect()
}

pub fn enrich_template_plan_entries(
    entries: &[TemplatePlanEntry],
    existing_destination_paths: &[String],
) -> Vec<TemplatePlanStateEntry> {
    let existing = existing_destination_paths.iter().collect::<std::collections::HashSet<_>>();
    entries
        .iter()
        .map(|entry| {
            let destination_exists =
                entry.destination_path.as_ref().is_some_and(|path| existing.contains(&path));
            let write_action = match entry.destination_path.as_ref() {
                None => "omit".to_string(),
                Some(_) if entry.strategy == TemplateStrategy::KeepDestination => {
                    "keep".to_string()
                }
                Some(_) if destination_exists => "update".to_string(),
                Some(_) => "create".to_string(),
            };

            TemplatePlanStateEntry {
                template_source_path: entry.template_source_path.clone(),
                logical_destination_path: entry.logical_destination_path.clone(),
                destination_path: entry.destination_path.clone(),
                classification: entry.classification.clone(),
                strategy: entry.strategy,
                action: entry.action.clone(),
                destination_exists,
                write_action,
            }
        })
        .collect()
}

pub fn enrich_template_plan_entries_with_token_state(
    entries: &[TemplatePlanStateEntry],
    template_contents: &HashMap<String, String>,
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> Vec<TemplatePlanTokenStateEntry> {
    entries
        .iter()
        .map(|entry| {
            let content = template_contents
                .get(&entry.template_source_path)
                .map(String::as_str)
                .unwrap_or("");
            let token_keys = template_token_keys(content, config);
            let unresolved_token_keys = token_keys
                .iter()
                .filter(|key| !replacements.contains_key(*key))
                .cloned()
                .collect::<Vec<_>>();
            let token_resolution_required = entry.destination_path.is_some()
                && entry.strategy != TemplateStrategy::KeepDestination
                && entry.strategy != TemplateStrategy::RawCopy;
            let blocked = token_resolution_required && !unresolved_token_keys.is_empty();

            TemplatePlanTokenStateEntry {
                template_source_path: entry.template_source_path.clone(),
                logical_destination_path: entry.logical_destination_path.clone(),
                destination_path: entry.destination_path.clone(),
                classification: entry.classification.clone(),
                strategy: entry.strategy,
                action: entry.action.clone(),
                destination_exists: entry.destination_exists,
                write_action: entry.write_action.clone(),
                token_keys,
                unresolved_token_keys,
                token_resolution_required,
                blocked,
                block_reason: blocked.then_some(TemplatePlanBlockReason::UnresolvedTokens),
            }
        })
        .collect()
}

pub fn prepare_template_entries(
    entries: &[TemplatePlanTokenStateEntry],
    template_contents: &HashMap<String, String>,
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> Vec<TemplatePreparedEntry> {
    entries
        .iter()
        .map(|entry| {
            let template_content =
                template_contents.get(&entry.template_source_path).cloned().unwrap_or_default();
            if entry.blocked {
                return TemplatePreparedEntry {
                    template_source_path: entry.template_source_path.clone(),
                    logical_destination_path: entry.logical_destination_path.clone(),
                    destination_path: entry.destination_path.clone(),
                    classification: entry.classification.clone(),
                    strategy: entry.strategy,
                    action: entry.action.clone(),
                    destination_exists: entry.destination_exists,
                    write_action: entry.write_action.clone(),
                    token_keys: entry.token_keys.clone(),
                    unresolved_token_keys: entry.unresolved_token_keys.clone(),
                    token_resolution_required: entry.token_resolution_required,
                    blocked: entry.blocked,
                    block_reason: entry.block_reason,
                    template_content,
                    prepared_template_content: None,
                    preparation_action: TemplatePreparationAction::Blocked,
                };
            }

            let prepared_template_content = if entry.token_resolution_required {
                Some(resolve_template_tokens(&template_content, replacements, config))
            } else {
                Some(template_content.clone())
            };

            TemplatePreparedEntry {
                template_source_path: entry.template_source_path.clone(),
                logical_destination_path: entry.logical_destination_path.clone(),
                destination_path: entry.destination_path.clone(),
                classification: entry.classification.clone(),
                strategy: entry.strategy,
                action: entry.action.clone(),
                destination_exists: entry.destination_exists,
                write_action: entry.write_action.clone(),
                token_keys: entry.token_keys.clone(),
                unresolved_token_keys: entry.unresolved_token_keys.clone(),
                token_resolution_required: entry.token_resolution_required,
                blocked: entry.blocked,
                block_reason: entry.block_reason,
                template_content,
                prepared_template_content,
                preparation_action: if entry.token_resolution_required {
                    TemplatePreparationAction::ResolveTokens
                } else {
                    TemplatePreparationAction::PassThrough
                },
            }
        })
        .collect()
}

pub fn plan_template_execution(
    entries: &[TemplatePreparedEntry],
    destination_contents: &HashMap<String, String>,
) -> Vec<TemplateExecutionPlanEntry> {
    entries
        .iter()
        .map(|entry| {
            let destination_content = entry
                .destination_path
                .as_ref()
                .and_then(|path| destination_contents.get(path).cloned());
            let execution_action = if entry.blocked {
                TemplateExecutionAction::Blocked
            } else if entry.destination_path.is_none() {
                TemplateExecutionAction::Omit
            } else if entry.write_action == "keep" {
                TemplateExecutionAction::Keep
            } else if entry.strategy == TemplateStrategy::RawCopy {
                TemplateExecutionAction::RawCopy
            } else if entry.strategy == TemplateStrategy::AcceptTemplate {
                TemplateExecutionAction::WritePreparedContent
            } else {
                TemplateExecutionAction::MergePreparedContent
            };
            let ready = !matches!(
                execution_action,
                TemplateExecutionAction::Blocked
                    | TemplateExecutionAction::Omit
                    | TemplateExecutionAction::Keep
            );

            TemplateExecutionPlanEntry {
                template_source_path: entry.template_source_path.clone(),
                logical_destination_path: entry.logical_destination_path.clone(),
                destination_path: entry.destination_path.clone(),
                classification: entry.classification.clone(),
                strategy: entry.strategy,
                action: entry.action.clone(),
                destination_exists: entry.destination_exists,
                write_action: entry.write_action.clone(),
                token_keys: entry.token_keys.clone(),
                unresolved_token_keys: entry.unresolved_token_keys.clone(),
                token_resolution_required: entry.token_resolution_required,
                blocked: entry.blocked,
                block_reason: entry.block_reason,
                template_content: entry.template_content.clone(),
                prepared_template_content: entry.prepared_template_content.clone(),
                preparation_action: entry.preparation_action,
                execution_action,
                ready,
                destination_content,
            }
        })
        .collect()
}

pub fn plan_template_tree_execution(
    template_source_paths: &[String],
    template_contents: &HashMap<String, String>,
    existing_destination_paths: &[String],
    destination_contents: &HashMap<String, String>,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> Vec<TemplateExecutionPlanEntry> {
    let planned_entries =
        plan_template_entries(template_source_paths, context, default_strategy, overrides);
    let stateful_entries =
        enrich_template_plan_entries(&planned_entries, existing_destination_paths);
    let token_state_entries = enrich_template_plan_entries_with_token_state(
        &stateful_entries,
        template_contents,
        replacements,
        config,
    );
    let prepared_entries =
        prepare_template_entries(&token_state_entries, template_contents, replacements, config);

    plan_template_execution(&prepared_entries, destination_contents)
}

pub fn preview_template_execution(entries: &[TemplateExecutionPlanEntry]) -> TemplatePreviewResult {
    let mut result = TemplatePreviewResult {
        result_files: HashMap::new(),
        created_paths: Vec::new(),
        updated_paths: Vec::new(),
        kept_paths: Vec::new(),
        blocked_paths: Vec::new(),
        omitted_paths: Vec::new(),
    };

    for entry in entries {
        match entry.execution_action {
            TemplateExecutionAction::Blocked => {
                if let Some(destination_path) = entry.destination_path.as_ref() {
                    result.blocked_paths.push(destination_path.clone());
                }
            }
            TemplateExecutionAction::Omit => {
                result.omitted_paths.push(entry.logical_destination_path.clone());
            }
            TemplateExecutionAction::Keep => {
                if let (Some(destination_path), Some(destination_content)) =
                    (entry.destination_path.as_ref(), entry.destination_content.as_ref())
                {
                    result
                        .result_files
                        .insert(destination_path.clone(), destination_content.clone());
                    result.kept_paths.push(destination_path.clone());
                }
            }
            TemplateExecutionAction::RawCopy | TemplateExecutionAction::WritePreparedContent => {
                if let (Some(destination_path), Some(prepared_template_content)) =
                    (entry.destination_path.as_ref(), entry.prepared_template_content.as_ref())
                {
                    result
                        .result_files
                        .insert(destination_path.clone(), prepared_template_content.clone());
                    if entry.destination_exists
                        && entry.destination_content.as_ref().is_some_and(|destination_content| {
                            destination_content == prepared_template_content
                        })
                    {
                        result.kept_paths.push(destination_path.clone());
                    } else if entry.destination_exists {
                        result.updated_paths.push(destination_path.clone());
                    } else {
                        result.created_paths.push(destination_path.clone());
                    }
                }
            }
            TemplateExecutionAction::MergePreparedContent => {
                if let (Some(destination_path), Some(prepared_template_content), None) = (
                    entry.destination_path.as_ref(),
                    entry.prepared_template_content.as_ref(),
                    entry.destination_content.as_ref(),
                ) {
                    result
                        .result_files
                        .insert(destination_path.clone(), prepared_template_content.clone());
                    if entry.destination_exists {
                        result.updated_paths.push(destination_path.clone());
                    } else {
                        result.created_paths.push(destination_path.clone());
                    }
                }
            }
        }
    }

    result
}

pub fn apply_template_execution<F>(
    entries: &[TemplateExecutionPlanEntry],
    merge_prepared_content: F,
) -> TemplateApplyResult
where
    F: Fn(&TemplateExecutionPlanEntry) -> MergeResult<String>,
{
    let mut result = TemplateApplyResult {
        result_files: HashMap::new(),
        created_paths: Vec::new(),
        updated_paths: Vec::new(),
        kept_paths: Vec::new(),
        blocked_paths: Vec::new(),
        omitted_paths: Vec::new(),
        diagnostics: Vec::new(),
    };

    for entry in entries {
        match entry.execution_action {
            TemplateExecutionAction::Blocked => {
                if let Some(destination_path) = entry.destination_path.as_ref() {
                    result.blocked_paths.push(destination_path.clone());
                }
            }
            TemplateExecutionAction::Omit => {
                result.omitted_paths.push(entry.logical_destination_path.clone());
            }
            TemplateExecutionAction::Keep => {
                if let (Some(destination_path), Some(destination_content)) =
                    (entry.destination_path.as_ref(), entry.destination_content.as_ref())
                {
                    result
                        .result_files
                        .insert(destination_path.clone(), destination_content.clone());
                    result.kept_paths.push(destination_path.clone());
                }
            }
            TemplateExecutionAction::RawCopy | TemplateExecutionAction::WritePreparedContent => {
                if let (Some(destination_path), Some(prepared_template_content)) =
                    (entry.destination_path.as_ref(), entry.prepared_template_content.as_ref())
                {
                    record_template_apply_output(
                        &mut result,
                        entry,
                        destination_path.clone(),
                        prepared_template_content.clone(),
                    );
                }
            }
            TemplateExecutionAction::MergePreparedContent => {
                let Some(destination_path) = entry.destination_path.as_ref() else {
                    continue;
                };
                let Some(prepared_template_content) = entry.prepared_template_content.as_ref()
                else {
                    continue;
                };

                if entry.destination_content.is_none() {
                    record_template_apply_output(
                        &mut result,
                        entry,
                        destination_path.clone(),
                        prepared_template_content.clone(),
                    );
                    continue;
                }

                let merge_result = merge_prepared_content(entry);
                result.diagnostics.extend(merge_result.diagnostics.clone());
                let Some(output) = merge_result.output else {
                    result.blocked_paths.push(destination_path.clone());
                    continue;
                };
                if !merge_result.ok {
                    result.blocked_paths.push(destination_path.clone());
                    continue;
                }

                record_template_apply_output(&mut result, entry, destination_path.clone(), output);
            }
        }
    }

    result
}

pub fn evaluate_template_tree_convergence(
    template_source_paths: &[String],
    template_contents: &HashMap<String, String>,
    destination_contents: &HashMap<String, String>,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> TemplateConvergenceResult {
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let execution_plan = plan_template_tree_execution(
        template_source_paths,
        template_contents,
        &existing_destination_paths,
        destination_contents,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    );
    let pending_paths = execution_plan
        .iter()
        .filter(|entry| {
            if entry.blocked {
                return true;
            }
            if !entry.ready {
                return false;
            }

            !matches!(
                (
                    entry.destination_content.as_ref(),
                    entry.prepared_template_content.as_ref(),
                ),
                (Some(destination_content), Some(prepared_template_content))
                    if destination_content == prepared_template_content
            )
        })
        .map(|entry| {
            entry.destination_path.clone().unwrap_or_else(|| entry.logical_destination_path.clone())
        })
        .collect::<Vec<_>>();

    TemplateConvergenceResult { converged: pending_paths.is_empty(), pending_paths }
}

pub fn run_template_tree_execution<F>(
    template_source_paths: &[String],
    template_contents: &HashMap<String, String>,
    destination_contents: &HashMap<String, String>,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    merge_prepared_content: F,
    config: &TemplateTokenConfig,
) -> TemplateTreeRunResult
where
    F: Fn(&TemplateExecutionPlanEntry) -> MergeResult<String>,
{
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();
    let execution_plan = plan_template_tree_execution(
        template_source_paths,
        template_contents,
        &existing_destination_paths,
        destination_contents,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    );

    TemplateTreeRunResult {
        apply_result: apply_template_execution(&execution_plan, merge_prepared_content),
        execution_plan,
    }
}

pub fn read_relative_file_tree(root: &Path) -> io::Result<HashMap<String, String>> {
    let mut files = HashMap::new();
    let metadata = match fs::metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(files),
        Err(error) => return Err(error),
    };
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", root.display()),
        ));
    }

    fn walk(root: &Path, current: &Path, files: &mut HashMap<String, String>) -> io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, files)?;
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .expect("path should be under root")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative_path, fs::read_to_string(&path)?);
        }
        Ok(())
    }

    walk(root, root, &mut files)?;
    Ok(files)
}

pub fn write_relative_file_tree(root: &Path, files: &HashMap<String, String>) -> io::Result<()> {
    fs::create_dir_all(root)?;

    let mut paths = files.keys().cloned().collect::<Vec<_>>();
    paths.sort();
    for relative_path in paths {
        let full_path =
            root.join(PathBuf::from(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR)));
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, files.get(&relative_path).expect("path should exist"))?;
    }

    Ok(())
}

pub fn run_template_tree_execution_from_directories<F>(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    merge_prepared_content: F,
    config: &TemplateTokenConfig,
) -> io::Result<TemplateTreeRunResult>
where
    F: Fn(&TemplateExecutionPlanEntry) -> MergeResult<String>,
{
    let template_contents = read_relative_file_tree(template_root)?;
    let destination_contents = read_relative_file_tree(destination_root)?;
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();

    Ok(run_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &destination_contents,
        context,
        default_strategy,
        overrides,
        replacements,
        merge_prepared_content,
        config,
    ))
}

pub fn plan_template_tree_execution_from_directories(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    config: &TemplateTokenConfig,
) -> io::Result<Vec<TemplateExecutionPlanEntry>> {
    let template_contents = read_relative_file_tree(template_root)?;
    let destination_contents = read_relative_file_tree(destination_root)?;
    let mut template_source_paths = template_contents.keys().cloned().collect::<Vec<_>>();
    template_source_paths.sort();
    let mut existing_destination_paths = destination_contents.keys().cloned().collect::<Vec<_>>();
    existing_destination_paths.sort();

    Ok(plan_template_tree_execution(
        &template_source_paths,
        &template_contents,
        &existing_destination_paths,
        &destination_contents,
        context,
        default_strategy,
        overrides,
        replacements,
        config,
    ))
}

pub fn apply_template_tree_execution_to_directory<F>(
    template_root: &Path,
    destination_root: &Path,
    context: &TemplateDestinationContext,
    default_strategy: TemplateStrategy,
    overrides: &[TemplateStrategyOverride],
    replacements: &HashMap<String, String>,
    merge_prepared_content: F,
    config: &TemplateTokenConfig,
) -> io::Result<TemplateTreeRunResult>
where
    F: Fn(&TemplateExecutionPlanEntry) -> MergeResult<String>,
{
    let run_result = run_template_tree_execution_from_directories(
        template_root,
        destination_root,
        context,
        default_strategy,
        overrides,
        replacements,
        merge_prepared_content,
        config,
    )?;

    let mut files_to_write = HashMap::new();
    for path in run_result
        .apply_result
        .created_paths
        .iter()
        .chain(run_result.apply_result.updated_paths.iter())
    {
        if let Some(content) = run_result.apply_result.result_files.get(path) {
            files_to_write.insert(path.clone(), content.clone());
        }
    }
    write_relative_file_tree(destination_root, &files_to_write)?;

    Ok(run_result)
}

pub fn report_template_tree_run(result: &TemplateTreeRunResult) -> TemplateTreeRunReport {
    let created =
        result.apply_result.created_paths.iter().cloned().collect::<std::collections::HashSet<_>>();
    let updated =
        result.apply_result.updated_paths.iter().cloned().collect::<std::collections::HashSet<_>>();
    let kept =
        result.apply_result.kept_paths.iter().cloned().collect::<std::collections::HashSet<_>>();
    let blocked =
        result.apply_result.blocked_paths.iter().cloned().collect::<std::collections::HashSet<_>>();
    let omitted =
        result.apply_result.omitted_paths.iter().cloned().collect::<std::collections::HashSet<_>>();

    let mut summary =
        TemplateTreeRunReportSummary { created: 0, updated: 0, kept: 0, blocked: 0, omitted: 0 };
    let entries = result
        .execution_plan
        .iter()
        .map(|entry| {
            let status = if entry.execution_action == TemplateExecutionAction::Omit
                || omitted.contains(&entry.logical_destination_path)
            {
                TemplateTreeRunStatus::Omitted
            } else if entry.destination_path.as_ref().is_some_and(|path| blocked.contains(path)) {
                TemplateTreeRunStatus::Blocked
            } else if entry.destination_path.as_ref().is_some_and(|path| kept.contains(path)) {
                TemplateTreeRunStatus::Kept
            } else if entry.destination_path.as_ref().is_some_and(|path| updated.contains(path)) {
                TemplateTreeRunStatus::Updated
            } else if entry.destination_path.as_ref().is_some_and(|path| created.contains(path)) {
                TemplateTreeRunStatus::Created
            } else {
                TemplateTreeRunStatus::Created
            };

            match status {
                TemplateTreeRunStatus::Created => summary.created += 1,
                TemplateTreeRunStatus::Updated => summary.updated += 1,
                TemplateTreeRunStatus::Kept => summary.kept += 1,
                TemplateTreeRunStatus::Blocked => summary.blocked += 1,
                TemplateTreeRunStatus::Omitted => summary.omitted += 1,
            }

            TemplateTreeRunReportEntry {
                template_source_path: entry.template_source_path.clone(),
                logical_destination_path: entry.logical_destination_path.clone(),
                destination_path: entry.destination_path.clone(),
                execution_action: entry.execution_action,
                status,
            }
        })
        .collect::<Vec<_>>();

    TemplateTreeRunReport { entries, summary }
}

pub fn report_template_directory_apply(
    result: &TemplateTreeRunResult,
) -> TemplateDirectoryApplyReport {
    let run_report = report_template_tree_run(result);
    let created =
        result.apply_result.created_paths.iter().cloned().collect::<std::collections::HashSet<_>>();
    let updated =
        result.apply_result.updated_paths.iter().cloned().collect::<std::collections::HashSet<_>>();

    let mut entries = Vec::with_capacity(run_report.entries.len());
    let mut summary = TemplateDirectoryApplyReportSummary {
        created: 0,
        updated: 0,
        kept: 0,
        blocked: 0,
        omitted: 0,
        written: 0,
    };
    for entry in run_report.entries {
        let written = entry
            .destination_path
            .as_ref()
            .is_some_and(|path| created.contains(path) || updated.contains(path));
        if written {
            summary.written += 1;
        }

        match entry.status {
            TemplateTreeRunStatus::Created => summary.created += 1,
            TemplateTreeRunStatus::Updated => summary.updated += 1,
            TemplateTreeRunStatus::Kept => summary.kept += 1,
            TemplateTreeRunStatus::Blocked => summary.blocked += 1,
            TemplateTreeRunStatus::Omitted => summary.omitted += 1,
        }

        entries.push(TemplateDirectoryApplyReportEntry {
            template_source_path: entry.template_source_path,
            logical_destination_path: entry.logical_destination_path,
            destination_path: entry.destination_path,
            execution_action: entry.execution_action,
            status: entry.status,
            written,
        });
    }

    TemplateDirectoryApplyReport { entries, summary }
}

pub fn report_template_directory_plan(
    entries: &[TemplateExecutionPlanEntry],
) -> TemplateDirectoryPlanReport {
    let mut report_entries = Vec::with_capacity(entries.len());
    let mut summary = TemplateDirectoryPlanReportSummary {
        create: 0,
        update: 0,
        keep: 0,
        blocked: 0,
        omitted: 0,
    };

    for entry in entries {
        let (status, previewable) = match entry.execution_action {
            TemplateExecutionAction::Blocked => (TemplateDirectoryPlanStatus::Blocked, false),
            TemplateExecutionAction::Omit => (TemplateDirectoryPlanStatus::Omitted, true),
            TemplateExecutionAction::Keep => (TemplateDirectoryPlanStatus::Keep, true),
            TemplateExecutionAction::RawCopy | TemplateExecutionAction::WritePreparedContent => (
                if entry.write_action == "create" {
                    TemplateDirectoryPlanStatus::Create
                } else {
                    TemplateDirectoryPlanStatus::Update
                },
                true,
            ),
            TemplateExecutionAction::MergePreparedContent => (
                if entry.write_action == "create" {
                    TemplateDirectoryPlanStatus::Create
                } else {
                    TemplateDirectoryPlanStatus::Update
                },
                entry.write_action == "create",
            ),
        };

        match status {
            TemplateDirectoryPlanStatus::Create => summary.create += 1,
            TemplateDirectoryPlanStatus::Update => summary.update += 1,
            TemplateDirectoryPlanStatus::Keep => summary.keep += 1,
            TemplateDirectoryPlanStatus::Blocked => summary.blocked += 1,
            TemplateDirectoryPlanStatus::Omitted => summary.omitted += 1,
        }

        report_entries.push(TemplateDirectoryPlanReportEntry {
            template_source_path: entry.template_source_path.clone(),
            logical_destination_path: entry.logical_destination_path.clone(),
            destination_path: entry.destination_path.clone(),
            execution_action: entry.execution_action,
            write_action: entry.write_action.clone(),
            status,
            previewable,
        });
    }

    TemplateDirectoryPlanReport { entries: report_entries, summary }
}

pub fn report_template_directory_runner(
    entries: &[TemplateExecutionPlanEntry],
    result: Option<&TemplateTreeRunResult>,
) -> TemplateDirectoryRunnerReport {
    TemplateDirectoryRunnerReport {
        plan_report: report_template_directory_plan(entries),
        preview: Some(preview_template_execution(entries)),
        run_report: result.map(report_template_tree_run),
        apply_report: result.map(report_template_directory_apply),
    }
}

fn record_template_apply_output(
    result: &mut TemplateApplyResult,
    entry: &TemplateExecutionPlanEntry,
    destination_path: String,
    output: String,
) {
    result.result_files.insert(destination_path.clone(), output.clone());
    if entry.destination_exists
        && entry
            .destination_content
            .as_ref()
            .is_some_and(|destination_content| destination_content == &output)
    {
        result.kept_paths.push(destination_path);
    } else if entry.destination_exists {
        result.updated_paths.push(destination_path);
    } else {
        result.created_paths.push(destination_path);
    }
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

pub fn conformance_suite_selectors(
    manifest: &ConformanceManifest,
) -> Vec<ConformanceSuiteSelector> {
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

pub fn resolve_delegated_child_outputs(
    operations: &[DelegatedChildOperation],
    nested_outputs: &[DelegatedChildSurfaceOutput],
    options: &DelegatedChildOutputResolutionOptions,
) -> DelegatedChildOutputResolution {
    let operations_by_surface_address: HashMap<&str, &DelegatedChildOperation> = operations
        .iter()
        .map(|operation| (operation.surface.address.as_str(), operation))
        .collect();

    for nested_output in nested_outputs {
        if operations_by_surface_address.contains_key(nested_output.surface_address.as_str()) {
            continue;
        }

        return DelegatedChildOutputResolution {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::ConfigurationError,
                message: format!(
                    "missing delegated child surface {}.",
                    nested_output.surface_address
                ),
                path: None,
                review: None,
            }],
            apply_plan: None,
            applied_children: None,
        };
    }

    let mut entries = Vec::with_capacity(nested_outputs.len());
    let mut applied_children = Vec::with_capacity(nested_outputs.len());
    for (index, nested_output) in nested_outputs.iter().enumerate() {
        let operation = operations_by_surface_address
            .get(nested_output.surface_address.as_str())
            .expect("surface should be validated before resolution");
        let request_id = format!("{}:{}", options.request_id_prefix, index);
        let family = operation
            .surface
            .metadata
            .get("family")
            .and_then(|value| value.as_str())
            .unwrap_or(options.default_family.as_str())
            .to_string();

        entries.push(DelegatedChildApplyPlanEntry {
            request_id: request_id.clone(),
            family,
            delegated_group: ProjectedChildReviewGroup {
                delegated_apply_group: request_id.clone(),
                parent_operation_id: operation.parent_operation_id.clone(),
                child_operation_id: operation.operation_id.clone(),
                delegated_runtime_surface_path: nested_output.surface_address.clone(),
                case_ids: vec![],
                delegated_case_ids: vec![],
            },
            decision: ReviewDecision {
                request_id,
                action: ReviewDecisionAction::ApplyDelegatedChildGroup,
                context: None,
            },
        });
        applied_children.push(AppliedDelegatedChildOutput {
            operation_id: operation.operation_id.clone(),
            output: nested_output.output.clone(),
        });
    }

    DelegatedChildOutputResolution {
        ok: true,
        diagnostics: vec![],
        apply_plan: Some(DelegatedChildApplyPlan { entries }),
        applied_children: Some(applied_children),
    }
}

pub fn execute_nested_merge<TOutput, MergeParent, DiscoverOperations, ApplyResolvedOutputs>(
    nested_outputs: &[DelegatedChildSurfaceOutput],
    options: &DelegatedChildOutputResolutionOptions,
    callbacks: NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
) -> MergeResult<TOutput>
where
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    let merged = (callbacks.merge_parent)();
    let Some(merged_output) = merged.output.as_ref() else {
        return merged;
    };
    if !merged.ok {
        return merged;
    }

    let discovery = (callbacks.discover_operations)(merged_output);
    let Some(operations) = discovery.operations.as_ref() else {
        return MergeResult {
            ok: false,
            diagnostics: discovery.diagnostics,
            output: None,
            policies: vec![],
        };
    };
    if !discovery.ok {
        return MergeResult {
            ok: false,
            diagnostics: discovery.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let resolution = resolve_delegated_child_outputs(operations, nested_outputs, options);
    let Some(apply_plan) = resolution.apply_plan.as_ref() else {
        return MergeResult {
            ok: false,
            diagnostics: resolution.diagnostics,
            output: None,
            policies: vec![],
        };
    };
    let Some(applied_children) = resolution.applied_children.as_ref() else {
        return MergeResult {
            ok: false,
            diagnostics: resolution.diagnostics,
            output: None,
            policies: vec![],
        };
    };
    if !resolution.ok {
        return MergeResult {
            ok: false,
            diagnostics: resolution.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    (callbacks.apply_resolved_outputs)(merged_output, operations, apply_plan, applied_children)
}

pub fn execute_delegated_child_apply_plan<
    TOutput,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    apply_plan: &DelegatedChildApplyPlan,
    applied_children: &[AppliedDelegatedChildOutput],
    callbacks: NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
) -> MergeResult<TOutput>
where
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    let merged = (callbacks.merge_parent)();
    let Some(merged_output) = merged.output.as_ref() else {
        return merged;
    };
    if !merged.ok {
        return merged;
    }

    let discovery = (callbacks.discover_operations)(merged_output);
    let Some(operations) = discovery.operations.as_ref() else {
        return MergeResult {
            ok: false,
            diagnostics: discovery.diagnostics,
            output: None,
            policies: vec![],
        };
    };
    if !discovery.ok {
        return MergeResult {
            ok: false,
            diagnostics: discovery.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    (callbacks.apply_resolved_outputs)(merged_output, operations, apply_plan, applied_children)
}

pub fn execute_reviewed_nested_merge<
    TOutput,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    state: &DelegatedChildGroupReviewState,
    family: &str,
    applied_children: &[AppliedDelegatedChildOutput],
    callbacks: NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
) -> MergeResult<TOutput>
where
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    let apply_plan = delegated_child_apply_plan(state, family);
    execute_delegated_child_apply_plan(&apply_plan, applied_children, callbacks)
}

pub fn reviewed_nested_execution(
    family: &str,
    review_state: &DelegatedChildGroupReviewState,
    applied_children: &[AppliedDelegatedChildOutput],
) -> ReviewedNestedExecution {
    ReviewedNestedExecution {
        family: family.to_string(),
        review_state: review_state.clone(),
        applied_children: applied_children.to_vec(),
    }
}

pub fn execute_reviewed_nested_execution<
    TOutput,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    execution: &ReviewedNestedExecution,
    callbacks: NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
) -> MergeResult<TOutput>
where
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    execute_reviewed_nested_merge(
        &execution.review_state,
        &execution.family,
        &execution.applied_children,
        callbacks,
    )
}

pub fn execute_reviewed_nested_executions<
    TOutput,
    CallbacksForExecution,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    executions: &[ReviewedNestedExecution],
    callbacks_for_execution: CallbacksForExecution,
) -> Vec<ReviewedNestedExecutionResult<TOutput>>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    executions
        .iter()
        .enumerate()
        .map(|(index, execution)| ReviewedNestedExecutionResult {
            execution: execution.clone(),
            result: execute_reviewed_nested_execution(
                execution,
                callbacks_for_execution(execution, index),
            ),
        })
        .collect()
}

pub fn execute_review_replay_bundle_reviewed_nested_executions<
    TOutput,
    CallbacksForExecution,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    bundle: &ReviewReplayBundle,
    callbacks_for_execution: CallbacksForExecution,
) -> Vec<ReviewedNestedExecutionResult<TOutput>>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    execute_reviewed_nested_executions(&bundle.reviewed_nested_executions, callbacks_for_execution)
}

pub fn execute_review_replay_bundle_envelope_reviewed_nested_executions<
    TOutput,
    CallbacksForExecution,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    envelope: &ReviewReplayBundleEnvelope,
    callbacks_for_execution: CallbacksForExecution,
) -> ReviewedNestedExecutionApplication<TOutput>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    match import_review_replay_bundle_envelope(envelope) {
        Ok(bundle) => ReviewedNestedExecutionApplication {
            diagnostics: Vec::new(),
            results: execute_review_replay_bundle_reviewed_nested_executions(
                &bundle,
                callbacks_for_execution,
            ),
        },
        Err(error) => ReviewedNestedExecutionApplication {
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: match error.category {
                    ReviewTransportImportErrorCategory::KindMismatch => {
                        DiagnosticCategory::KindMismatch
                    }
                    ReviewTransportImportErrorCategory::UnsupportedVersion => {
                        DiagnosticCategory::UnsupportedVersion
                    }
                },
                message: error.message,
                path: None,
                review: None,
            }],
            results: Vec::new(),
        },
    }
}

pub fn execute_review_state_reviewed_nested_executions<
    TOutput,
    CallbacksForExecution,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    state: &ConformanceManifestReviewState,
    callbacks_for_execution: CallbacksForExecution,
) -> Vec<ReviewedNestedExecutionResult<TOutput>>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    execute_reviewed_nested_executions(&state.reviewed_nested_executions, callbacks_for_execution)
}

pub fn execute_review_state_envelope_reviewed_nested_executions<
    TOutput,
    CallbacksForExecution,
    MergeParent,
    DiscoverOperations,
    ApplyResolvedOutputs,
>(
    envelope: &ConformanceManifestReviewStateEnvelope,
    callbacks_for_execution: CallbacksForExecution,
) -> ReviewedNestedExecutionApplication<TOutput>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        MergeParent,
        DiscoverOperations,
        ApplyResolvedOutputs,
    >,
    MergeParent: Fn() -> MergeResult<TOutput>,
    DiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    ApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
{
    match import_conformance_manifest_review_state_envelope(envelope) {
        Ok(state) => ReviewedNestedExecutionApplication {
            diagnostics: Vec::new(),
            results: execute_review_state_reviewed_nested_executions(
                &state,
                callbacks_for_execution,
            ),
        },
        Err(error) => ReviewedNestedExecutionApplication {
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: match error.category {
                    ReviewTransportImportErrorCategory::KindMismatch => {
                        DiagnosticCategory::KindMismatch
                    }
                    ReviewTransportImportErrorCategory::UnsupportedVersion => {
                        DiagnosticCategory::UnsupportedVersion
                    }
                },
                message: error.message,
                path: None,
                review: None,
            }],
            results: Vec::new(),
        },
    }
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
    let mut families = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for selector in conformance_suite_selectors(manifest) {
        let Some(definition) = conformance_suite_definition(manifest, &selector) else {
            continue;
        };
        if !seen.insert(definition.subject.grammar.clone()) {
            continue;
        }
        families.push(definition.subject.grammar.clone());
    }

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
) -> (Option<ReviewReplayContext>, Vec<ReviewDecision>, Vec<ReviewedNestedExecution>) {
    if let Some(bundle) = &options.review_replay_bundle {
        return (
            Some(bundle.replay_context.clone()),
            bundle.decisions.clone(),
            bundle.reviewed_nested_executions.clone(),
        );
    }

    (options.review_replay_context.clone(), options.review_decisions.clone(), Vec::new())
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

pub fn reviewed_nested_execution_envelope(
    execution: &ReviewedNestedExecution,
) -> ReviewedNestedExecutionEnvelope {
    ReviewedNestedExecutionEnvelope {
        kind: "reviewed_nested_execution".to_string(),
        version: REVIEW_TRANSPORT_VERSION,
        execution: execution.clone(),
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

pub fn import_reviewed_nested_execution_envelope(
    envelope: &ReviewedNestedExecutionEnvelope,
) -> Result<ReviewedNestedExecution, ReviewTransportImportError> {
    if envelope.kind != "reviewed_nested_execution" {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::KindMismatch,
            message: "expected reviewed_nested_execution envelope kind.".to_string(),
        });
    }

    if envelope.version != REVIEW_TRANSPORT_VERSION {
        return Err(ReviewTransportImportError {
            category: ReviewTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported reviewed_nested_execution envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution.clone())
}

pub fn structured_edit_application_envelope(
    application: &StructuredEditApplication,
) -> StructuredEditApplicationEnvelope {
    StructuredEditApplicationEnvelope {
        kind: "structured_edit_application".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        application: application.clone(),
    }
}

pub fn import_structured_edit_application_envelope(
    envelope: &StructuredEditApplicationEnvelope,
) -> Result<StructuredEditApplication, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_application" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_application envelope kind.".to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_application envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.application.clone())
}

pub fn structured_edit_provider_execution_request_envelope(
    execution_request: &StructuredEditProviderExecutionRequest,
) -> StructuredEditProviderExecutionRequestEnvelope {
    StructuredEditProviderExecutionRequestEnvelope {
        kind: "structured_edit_provider_execution_request".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_request: execution_request.clone(),
    }
}

pub fn import_structured_edit_provider_execution_request_envelope(
    envelope: &StructuredEditProviderExecutionRequestEnvelope,
) -> Result<StructuredEditProviderExecutionRequest, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_request" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_request envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_request envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_request.clone())
}

pub fn structured_edit_provider_execution_plan_envelope(
    execution_plan: &StructuredEditProviderExecutionPlan,
) -> StructuredEditProviderExecutionPlanEnvelope {
    StructuredEditProviderExecutionPlanEnvelope {
        kind: "structured_edit_provider_execution_plan".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_plan: execution_plan.clone(),
    }
}

pub fn import_structured_edit_provider_execution_plan_envelope(
    envelope: &StructuredEditProviderExecutionPlanEnvelope,
) -> Result<StructuredEditProviderExecutionPlan, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_plan" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_plan envelope kind.".to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_plan envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_plan.clone())
}

pub fn structured_edit_provider_execution_handoff_envelope(
    execution_handoff: &StructuredEditProviderExecutionHandoff,
) -> StructuredEditProviderExecutionHandoffEnvelope {
    StructuredEditProviderExecutionHandoffEnvelope {
        kind: "structured_edit_provider_execution_handoff".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_handoff: execution_handoff.clone(),
    }
}

pub fn import_structured_edit_provider_execution_handoff_envelope(
    envelope: &StructuredEditProviderExecutionHandoffEnvelope,
) -> Result<StructuredEditProviderExecutionHandoff, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_handoff" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_handoff envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_handoff envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_handoff.clone())
}

pub fn structured_edit_provider_execution_invocation_envelope(
    execution_invocation: &StructuredEditProviderExecutionInvocation,
) -> StructuredEditProviderExecutionInvocationEnvelope {
    StructuredEditProviderExecutionInvocationEnvelope {
        kind: "structured_edit_provider_execution_invocation".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_invocation: execution_invocation.clone(),
    }
}

pub fn import_structured_edit_provider_execution_invocation_envelope(
    envelope: &StructuredEditProviderExecutionInvocationEnvelope,
) -> Result<StructuredEditProviderExecutionInvocation, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_invocation" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_invocation envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_invocation envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_invocation.clone())
}

pub fn structured_edit_provider_batch_execution_invocation_envelope(
    batch_execution_invocation: &StructuredEditProviderBatchExecutionInvocation,
) -> StructuredEditProviderBatchExecutionInvocationEnvelope {
    StructuredEditProviderBatchExecutionInvocationEnvelope {
        kind: "structured_edit_provider_batch_execution_invocation".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_invocation: batch_execution_invocation.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_invocation_envelope(
    envelope: &StructuredEditProviderBatchExecutionInvocationEnvelope,
) -> Result<StructuredEditProviderBatchExecutionInvocation, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_invocation" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_invocation envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_invocation envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_invocation.clone())
}

pub fn structured_edit_provider_execution_run_result_envelope(
    execution_run_result: &StructuredEditProviderExecutionRunResult,
) -> StructuredEditProviderExecutionRunResultEnvelope {
    StructuredEditProviderExecutionRunResultEnvelope {
        kind: "structured_edit_provider_execution_run_result".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_run_result: execution_run_result.clone(),
    }
}

pub fn import_structured_edit_provider_execution_run_result_envelope(
    envelope: &StructuredEditProviderExecutionRunResultEnvelope,
) -> Result<StructuredEditProviderExecutionRunResult, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_run_result" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_run_result envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_run_result envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_run_result.clone())
}

pub fn structured_edit_provider_batch_execution_run_result_envelope(
    batch_execution_run_result: &StructuredEditProviderBatchExecutionRunResult,
) -> StructuredEditProviderBatchExecutionRunResultEnvelope {
    StructuredEditProviderBatchExecutionRunResultEnvelope {
        kind: "structured_edit_provider_batch_execution_run_result".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_run_result: batch_execution_run_result.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_run_result_envelope(
    envelope: &StructuredEditProviderBatchExecutionRunResultEnvelope,
) -> Result<StructuredEditProviderBatchExecutionRunResult, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_run_result" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_run_result envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_run_result envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_run_result.clone())
}

pub fn structured_edit_provider_execution_receipt_envelope(
    execution_receipt: &StructuredEditProviderExecutionReceipt,
) -> StructuredEditProviderExecutionReceiptEnvelope {
    StructuredEditProviderExecutionReceiptEnvelope {
        kind: "structured_edit_provider_execution_receipt".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        execution_receipt: execution_receipt.clone(),
    }
}

pub fn import_structured_edit_provider_execution_receipt_envelope(
    envelope: &StructuredEditProviderExecutionReceiptEnvelope,
) -> Result<StructuredEditProviderExecutionReceipt, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_receipt" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_receipt envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_receipt envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.execution_receipt.clone())
}

pub fn structured_edit_provider_batch_execution_receipt_envelope(
    batch_execution_receipt: &StructuredEditProviderBatchExecutionReceipt,
) -> StructuredEditProviderBatchExecutionReceiptEnvelope {
    StructuredEditProviderBatchExecutionReceiptEnvelope {
        kind: "structured_edit_provider_batch_execution_receipt".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_receipt: batch_execution_receipt.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_receipt_envelope(
    envelope: &StructuredEditProviderBatchExecutionReceiptEnvelope,
) -> Result<StructuredEditProviderBatchExecutionReceipt, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_receipt" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_receipt envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_receipt envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_receipt.clone())
}

pub fn structured_edit_provider_execution_receipt_replay_request_envelope(
    receipt_replay_request: &StructuredEditProviderExecutionReceiptReplayRequest,
) -> StructuredEditProviderExecutionReceiptReplayRequestEnvelope {
    StructuredEditProviderExecutionReceiptReplayRequestEnvelope {
        kind: "structured_edit_provider_execution_receipt_replay_request".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        receipt_replay_request: receipt_replay_request.clone(),
    }
}

pub fn import_structured_edit_provider_execution_receipt_replay_request_envelope(
    envelope: &StructuredEditProviderExecutionReceiptReplayRequestEnvelope,
) -> Result<StructuredEditProviderExecutionReceiptReplayRequest, StructuredEditTransportImportError>
{
    if envelope.kind != "structured_edit_provider_execution_receipt_replay_request" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_execution_receipt_replay_request envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_receipt_replay_request envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.receipt_replay_request.clone())
}

pub fn structured_edit_provider_batch_execution_receipt_replay_request_envelope(
    batch_receipt_replay_request: &StructuredEditProviderBatchExecutionReceiptReplayRequest,
) -> StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope {
    StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope {
        kind: "structured_edit_provider_batch_execution_receipt_replay_request".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_receipt_replay_request: batch_receipt_replay_request.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_receipt_replay_request_envelope(
    envelope: &StructuredEditProviderBatchExecutionReceiptReplayRequestEnvelope,
) -> Result<
    StructuredEditProviderBatchExecutionReceiptReplayRequest,
    StructuredEditTransportImportError,
> {
    if envelope.kind != "structured_edit_provider_batch_execution_receipt_replay_request" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_receipt_replay_request envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_receipt_replay_request envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_receipt_replay_request.clone())
}

pub fn structured_edit_provider_execution_receipt_replay_application_envelope(
    receipt_replay_application: &StructuredEditProviderExecutionReceiptReplayApplication,
) -> StructuredEditProviderExecutionReceiptReplayApplicationEnvelope {
    StructuredEditProviderExecutionReceiptReplayApplicationEnvelope {
        kind: "structured_edit_provider_execution_receipt_replay_application".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        receipt_replay_application: receipt_replay_application.clone(),
    }
}

pub fn import_structured_edit_provider_execution_receipt_replay_application_envelope(
    envelope: &StructuredEditProviderExecutionReceiptReplayApplicationEnvelope,
) -> Result<
    StructuredEditProviderExecutionReceiptReplayApplication,
    StructuredEditTransportImportError,
> {
    if envelope.kind != "structured_edit_provider_execution_receipt_replay_application" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_execution_receipt_replay_application envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_receipt_replay_application envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.receipt_replay_application.clone())
}

pub fn structured_edit_provider_batch_execution_receipt_replay_application_envelope(
    batch_receipt_replay_application: &StructuredEditProviderBatchExecutionReceiptReplayApplication,
) -> StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope {
    StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope {
        kind: "structured_edit_provider_batch_execution_receipt_replay_application".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_receipt_replay_application: batch_receipt_replay_application.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_receipt_replay_application_envelope(
    envelope: &StructuredEditProviderBatchExecutionReceiptReplayApplicationEnvelope,
) -> Result<
    StructuredEditProviderBatchExecutionReceiptReplayApplication,
    StructuredEditTransportImportError,
> {
    if envelope.kind != "structured_edit_provider_batch_execution_receipt_replay_application" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_receipt_replay_application envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_receipt_replay_application envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_receipt_replay_application.clone())
}

pub fn structured_edit_provider_execution_receipt_replay_session_envelope(
    receipt_replay_session: &StructuredEditProviderExecutionReceiptReplaySession,
) -> StructuredEditProviderExecutionReceiptReplaySessionEnvelope {
    StructuredEditProviderExecutionReceiptReplaySessionEnvelope {
        kind: "structured_edit_provider_execution_receipt_replay_session".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        receipt_replay_session: receipt_replay_session.clone(),
    }
}

pub fn import_structured_edit_provider_execution_receipt_replay_session_envelope(
    envelope: &StructuredEditProviderExecutionReceiptReplaySessionEnvelope,
) -> Result<StructuredEditProviderExecutionReceiptReplaySession, StructuredEditTransportImportError>
{
    if envelope.kind != "structured_edit_provider_execution_receipt_replay_session" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_execution_receipt_replay_session envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_receipt_replay_session envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.receipt_replay_session.clone())
}

pub fn structured_edit_provider_batch_execution_receipt_replay_session_envelope(
    batch_receipt_replay_session: &StructuredEditProviderBatchExecutionReceiptReplaySession,
) -> StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope {
    StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope {
        kind: "structured_edit_provider_batch_execution_receipt_replay_session".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_receipt_replay_session: batch_receipt_replay_session.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_receipt_replay_session_envelope(
    envelope: &StructuredEditProviderBatchExecutionReceiptReplaySessionEnvelope,
) -> Result<
    StructuredEditProviderBatchExecutionReceiptReplaySession,
    StructuredEditTransportImportError,
> {
    if envelope.kind != "structured_edit_provider_batch_execution_receipt_replay_session" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_receipt_replay_session envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_receipt_replay_session envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_receipt_replay_session.clone())
}

pub fn structured_edit_provider_execution_receipt_replay_workflow_envelope(
    receipt_replay_workflow: &StructuredEditProviderExecutionReceiptReplayWorkflow,
) -> StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope {
    StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope {
        kind: "structured_edit_provider_execution_receipt_replay_workflow".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        receipt_replay_workflow: receipt_replay_workflow.clone(),
    }
}

pub fn import_structured_edit_provider_execution_receipt_replay_workflow_envelope(
    envelope: &StructuredEditProviderExecutionReceiptReplayWorkflowEnvelope,
) -> Result<StructuredEditProviderExecutionReceiptReplayWorkflow, StructuredEditTransportImportError>
{
    if envelope.kind != "structured_edit_provider_execution_receipt_replay_workflow" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_execution_receipt_replay_workflow envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_receipt_replay_workflow envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.receipt_replay_workflow.clone())
}

pub fn structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
    batch_receipt_replay_workflow: &StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
) -> StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope {
    StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope {
        kind: "structured_edit_provider_batch_execution_receipt_replay_workflow".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_receipt_replay_workflow: batch_receipt_replay_workflow.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_receipt_replay_workflow_envelope(
    envelope: &StructuredEditProviderBatchExecutionReceiptReplayWorkflowEnvelope,
) -> Result<
    StructuredEditProviderBatchExecutionReceiptReplayWorkflow,
    StructuredEditTransportImportError,
> {
    if envelope.kind != "structured_edit_provider_batch_execution_receipt_replay_workflow" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_batch_execution_receipt_replay_workflow envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_receipt_replay_workflow envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_receipt_replay_workflow.clone())
}

pub fn structured_edit_provider_batch_execution_handoff_envelope(
    batch_execution_handoff: &StructuredEditProviderBatchExecutionHandoff,
) -> StructuredEditProviderBatchExecutionHandoffEnvelope {
    StructuredEditProviderBatchExecutionHandoffEnvelope {
        kind: "structured_edit_provider_batch_execution_handoff".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_handoff: batch_execution_handoff.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_handoff_envelope(
    envelope: &StructuredEditProviderBatchExecutionHandoffEnvelope,
) -> Result<StructuredEditProviderBatchExecutionHandoff, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_handoff" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_handoff envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_handoff envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_handoff.clone())
}

pub fn structured_edit_provider_batch_execution_plan_envelope(
    batch_execution_plan: &StructuredEditProviderBatchExecutionPlan,
) -> StructuredEditProviderBatchExecutionPlanEnvelope {
    StructuredEditProviderBatchExecutionPlanEnvelope {
        kind: "structured_edit_provider_batch_execution_plan".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_plan: batch_execution_plan.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_plan_envelope(
    envelope: &StructuredEditProviderBatchExecutionPlanEnvelope,
) -> Result<StructuredEditProviderBatchExecutionPlan, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_plan" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_plan envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_plan envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_plan.clone())
}

pub fn structured_edit_execution_report_envelope(
    report: &StructuredEditExecutionReport,
) -> StructuredEditExecutionReportEnvelope {
    StructuredEditExecutionReportEnvelope {
        kind: "structured_edit_execution_report".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        report: report.clone(),
    }
}

pub fn import_structured_edit_execution_report_envelope(
    envelope: &StructuredEditExecutionReportEnvelope,
) -> Result<StructuredEditExecutionReport, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_execution_report" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_execution_report envelope kind.".to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_execution_report envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.report.clone())
}

pub fn structured_edit_provider_execution_application_envelope(
    provider_execution_application: &StructuredEditProviderExecutionApplication,
) -> StructuredEditProviderExecutionApplicationEnvelope {
    StructuredEditProviderExecutionApplicationEnvelope {
        kind: "structured_edit_provider_execution_application".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        provider_execution_application: provider_execution_application.clone(),
    }
}

pub fn import_structured_edit_provider_execution_application_envelope(
    envelope: &StructuredEditProviderExecutionApplicationEnvelope,
) -> Result<StructuredEditProviderExecutionApplication, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_application" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_application envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_application envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.provider_execution_application.clone())
}

pub fn structured_edit_provider_execution_dispatch_envelope(
    provider_execution_dispatch: &StructuredEditProviderExecutionDispatch,
) -> StructuredEditProviderExecutionDispatchEnvelope {
    StructuredEditProviderExecutionDispatchEnvelope {
        kind: "structured_edit_provider_execution_dispatch".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        provider_execution_dispatch: provider_execution_dispatch.clone(),
    }
}

pub fn import_structured_edit_provider_execution_dispatch_envelope(
    envelope: &StructuredEditProviderExecutionDispatchEnvelope,
) -> Result<StructuredEditProviderExecutionDispatch, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_dispatch" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_dispatch envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_dispatch envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.provider_execution_dispatch.clone())
}

pub fn structured_edit_provider_execution_outcome_envelope(
    provider_execution_outcome: &StructuredEditProviderExecutionOutcome,
) -> StructuredEditProviderExecutionOutcomeEnvelope {
    StructuredEditProviderExecutionOutcomeEnvelope {
        kind: "structured_edit_provider_execution_outcome".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        provider_execution_outcome: provider_execution_outcome.clone(),
    }
}

pub fn import_structured_edit_provider_execution_outcome_envelope(
    envelope: &StructuredEditProviderExecutionOutcomeEnvelope,
) -> Result<StructuredEditProviderExecutionOutcome, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_outcome" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_outcome envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_outcome envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.provider_execution_outcome.clone())
}

pub fn structured_edit_provider_batch_execution_outcome_envelope(
    batch_outcome: &StructuredEditProviderBatchExecutionOutcome,
) -> StructuredEditProviderBatchExecutionOutcomeEnvelope {
    StructuredEditProviderBatchExecutionOutcomeEnvelope {
        kind: "structured_edit_provider_batch_execution_outcome".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_outcome: batch_outcome.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_outcome_envelope(
    envelope: &StructuredEditProviderBatchExecutionOutcomeEnvelope,
) -> Result<StructuredEditProviderBatchExecutionOutcome, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_outcome" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_outcome envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_outcome envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_outcome.clone())
}

pub fn structured_edit_provider_execution_provenance_envelope(
    provenance: &StructuredEditProviderExecutionProvenance,
) -> StructuredEditProviderExecutionProvenanceEnvelope {
    StructuredEditProviderExecutionProvenanceEnvelope {
        kind: "structured_edit_provider_execution_provenance".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        provenance: provenance.clone(),
    }
}

pub fn import_structured_edit_provider_execution_provenance_envelope(
    envelope: &StructuredEditProviderExecutionProvenanceEnvelope,
) -> Result<StructuredEditProviderExecutionProvenance, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_provenance" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_provenance envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_provenance envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.provenance.clone())
}

pub fn structured_edit_provider_batch_execution_provenance_envelope(
    batch_provenance: &StructuredEditProviderBatchExecutionProvenance,
) -> StructuredEditProviderBatchExecutionProvenanceEnvelope {
    StructuredEditProviderBatchExecutionProvenanceEnvelope {
        kind: "structured_edit_provider_batch_execution_provenance".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_provenance: batch_provenance.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_provenance_envelope(
    envelope: &StructuredEditProviderBatchExecutionProvenanceEnvelope,
) -> Result<StructuredEditProviderBatchExecutionProvenance, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_provenance" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_provenance envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_provenance envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_provenance.clone())
}

pub fn structured_edit_provider_execution_replay_bundle_envelope(
    replay_bundle: &StructuredEditProviderExecutionReplayBundle,
) -> StructuredEditProviderExecutionReplayBundleEnvelope {
    StructuredEditProviderExecutionReplayBundleEnvelope {
        kind: "structured_edit_provider_execution_replay_bundle".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        replay_bundle: replay_bundle.clone(),
    }
}

pub fn import_structured_edit_provider_execution_replay_bundle_envelope(
    envelope: &StructuredEditProviderExecutionReplayBundleEnvelope,
) -> Result<StructuredEditProviderExecutionReplayBundle, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_execution_replay_bundle" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_execution_replay_bundle envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_execution_replay_bundle envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.replay_bundle.clone())
}

pub fn structured_edit_provider_batch_execution_replay_bundle_envelope(
    batch_replay_bundle: &StructuredEditProviderBatchExecutionReplayBundle,
) -> StructuredEditProviderBatchExecutionReplayBundleEnvelope {
    StructuredEditProviderBatchExecutionReplayBundleEnvelope {
        kind: "structured_edit_provider_batch_execution_replay_bundle".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_replay_bundle: batch_replay_bundle.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_replay_bundle_envelope(
    envelope: &StructuredEditProviderBatchExecutionReplayBundleEnvelope,
) -> Result<StructuredEditProviderBatchExecutionReplayBundle, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_replay_bundle" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message:
                "expected structured_edit_provider_batch_execution_replay_bundle envelope kind."
                    .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_replay_bundle envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_replay_bundle.clone())
}

pub fn structured_edit_provider_executor_profile_envelope(
    executor_profile: &StructuredEditProviderExecutorProfile,
) -> StructuredEditProviderExecutorProfileEnvelope {
    StructuredEditProviderExecutorProfileEnvelope {
        kind: "structured_edit_provider_executor_profile".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        executor_profile: executor_profile.clone(),
    }
}

pub fn import_structured_edit_provider_executor_profile_envelope(
    envelope: &StructuredEditProviderExecutorProfileEnvelope,
) -> Result<StructuredEditProviderExecutorProfile, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_executor_profile" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_executor_profile envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_executor_profile envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.executor_profile.clone())
}

pub fn structured_edit_provider_executor_registry_envelope(
    executor_registry: &StructuredEditProviderExecutorRegistry,
) -> StructuredEditProviderExecutorRegistryEnvelope {
    StructuredEditProviderExecutorRegistryEnvelope {
        kind: "structured_edit_provider_executor_registry".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        executor_registry: executor_registry.clone(),
    }
}

pub fn import_structured_edit_provider_executor_registry_envelope(
    envelope: &StructuredEditProviderExecutorRegistryEnvelope,
) -> Result<StructuredEditProviderExecutorRegistry, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_executor_registry" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_executor_registry envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_executor_registry envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.executor_registry.clone())
}

pub fn structured_edit_provider_executor_selection_policy_envelope(
    selection_policy: &StructuredEditProviderExecutorSelectionPolicy,
) -> StructuredEditProviderExecutorSelectionPolicyEnvelope {
    StructuredEditProviderExecutorSelectionPolicyEnvelope {
        kind: "structured_edit_provider_executor_selection_policy".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        selection_policy: selection_policy.clone(),
    }
}

pub fn import_structured_edit_provider_executor_selection_policy_envelope(
    envelope: &StructuredEditProviderExecutorSelectionPolicyEnvelope,
) -> Result<StructuredEditProviderExecutorSelectionPolicy, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_executor_selection_policy" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_executor_selection_policy envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_executor_selection_policy envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.selection_policy.clone())
}

pub fn structured_edit_provider_executor_resolution_envelope(
    executor_resolution: &StructuredEditProviderExecutorResolution,
) -> StructuredEditProviderExecutorResolutionEnvelope {
    StructuredEditProviderExecutorResolutionEnvelope {
        kind: "structured_edit_provider_executor_resolution".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        executor_resolution: executor_resolution.clone(),
    }
}

pub fn import_structured_edit_provider_executor_resolution_envelope(
    envelope: &StructuredEditProviderExecutorResolutionEnvelope,
) -> Result<StructuredEditProviderExecutorResolution, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_executor_resolution" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_executor_resolution envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_executor_resolution envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.executor_resolution.clone())
}

pub fn structured_edit_provider_batch_execution_request_envelope(
    batch_execution_request: &StructuredEditProviderBatchExecutionRequest,
) -> StructuredEditProviderBatchExecutionRequestEnvelope {
    StructuredEditProviderBatchExecutionRequestEnvelope {
        kind: "structured_edit_provider_batch_execution_request".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_execution_request: batch_execution_request.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_request_envelope(
    envelope: &StructuredEditProviderBatchExecutionRequestEnvelope,
) -> Result<StructuredEditProviderBatchExecutionRequest, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_request" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_request envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_request envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_execution_request.clone())
}

pub fn structured_edit_provider_batch_execution_dispatch_envelope(
    batch_dispatch: &StructuredEditProviderBatchExecutionDispatch,
) -> StructuredEditProviderBatchExecutionDispatchEnvelope {
    StructuredEditProviderBatchExecutionDispatchEnvelope {
        kind: "structured_edit_provider_batch_execution_dispatch".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_dispatch: batch_dispatch.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_dispatch_envelope(
    envelope: &StructuredEditProviderBatchExecutionDispatchEnvelope,
) -> Result<StructuredEditProviderBatchExecutionDispatch, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_dispatch" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_dispatch envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_dispatch envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_dispatch.clone())
}

pub fn structured_edit_provider_batch_execution_report_envelope(
    batch_report: &StructuredEditProviderBatchExecutionReport,
) -> StructuredEditProviderBatchExecutionReportEnvelope {
    StructuredEditProviderBatchExecutionReportEnvelope {
        kind: "structured_edit_provider_batch_execution_report".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_report: batch_report.clone(),
    }
}

pub fn import_structured_edit_provider_batch_execution_report_envelope(
    envelope: &StructuredEditProviderBatchExecutionReportEnvelope,
) -> Result<StructuredEditProviderBatchExecutionReport, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_provider_batch_execution_report" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_provider_batch_execution_report envelope kind."
                .to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_provider_batch_execution_report envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_report.clone())
}

pub fn structured_edit_batch_report_envelope(
    batch_report: &StructuredEditBatchReport,
) -> StructuredEditBatchReportEnvelope {
    StructuredEditBatchReportEnvelope {
        kind: "structured_edit_batch_report".to_string(),
        version: STRUCTURED_EDIT_TRANSPORT_VERSION,
        batch_report: batch_report.clone(),
    }
}

pub fn import_structured_edit_batch_report_envelope(
    envelope: &StructuredEditBatchReportEnvelope,
) -> Result<StructuredEditBatchReport, StructuredEditTransportImportError> {
    if envelope.kind != "structured_edit_batch_report" {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::KindMismatch,
            message: "expected structured_edit_batch_report envelope kind.".to_string(),
        });
    }

    if envelope.version != STRUCTURED_EDIT_TRANSPORT_VERSION {
        return Err(StructuredEditTransportImportError {
            category: StructuredEditTransportImportErrorCategory::UnsupportedVersion,
            message: format!(
                "unsupported structured_edit_batch_report envelope version {}.",
                envelope.version
            ),
        });
    }

    Ok(envelope.batch_report.clone())
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
    let results =
        run_named_conformance_suite(manifest, selector, family_profile, execute, feature_profile)?;
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
    let (replay_input_context, replay_input_decisions, mut reviewed_nested_executions) =
        review_replay_bundle_inputs(options);
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
        reviewed_nested_executions.clear();
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
        reviewed_nested_executions.clear();
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

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &selector, &context) else {
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
        reviewed_nested_executions,
    }
}

pub fn review_conformance_manifest_with_replay_bundle_envelope(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestReviewOptions,
    replay_bundle_envelope: &ReviewReplayBundleEnvelope,
    execute: impl Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
) -> ConformanceManifestReviewState {
    match import_review_replay_bundle_envelope(replay_bundle_envelope) {
        Ok(replay_bundle) => {
            let mut envelope_options = options.clone();
            envelope_options.review_replay_bundle = Some(replay_bundle);
            review_conformance_manifest(manifest, &envelope_options, execute)
        }
        Err(error) => {
            let mut fallback_options = options.clone();
            fallback_options.review_replay_bundle = None;
            let mut state = review_conformance_manifest(manifest, &fallback_options, execute);
            state.diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: match error.category {
                    ReviewTransportImportErrorCategory::KindMismatch => {
                        DiagnosticCategory::KindMismatch
                    }
                    ReviewTransportImportErrorCategory::UnsupportedVersion => {
                        DiagnosticCategory::UnsupportedVersion
                    }
                },
                message: error.message,
                path: None,
                review: None,
            });
            state
        }
    }
}

pub fn review_and_execute_conformance_manifest_with_replay_bundle_envelope<
    TOutput,
    CallbacksForExecution,
    TMergeParent,
    TDiscoverOperations,
    TApplyResolvedOutputs,
    TExecute,
>(
    manifest: &ConformanceManifest,
    options: &ConformanceManifestReviewOptions,
    replay_bundle_envelope: &ReviewReplayBundleEnvelope,
    execute: TExecute,
    callbacks_for_execution: CallbacksForExecution,
) -> ConformanceManifestReviewedNestedApplication<TOutput>
where
    TOutput: Clone,
    CallbacksForExecution: Fn(
        &ReviewedNestedExecution,
        usize,
    ) -> NestedMergeExecutionCallbacks<
        TOutput,
        TMergeParent,
        TDiscoverOperations,
        TApplyResolvedOutputs,
    >,
    TMergeParent: Fn() -> MergeResult<TOutput>,
    TDiscoverOperations: Fn(&TOutput) -> NestedMergeDiscoveryResult,
    TApplyResolvedOutputs: Fn(
        &TOutput,
        &[DelegatedChildOperation],
        &DelegatedChildApplyPlan,
        &[AppliedDelegatedChildOutput],
    ) -> MergeResult<TOutput>,
    TExecute: Fn(&ConformanceCaseRun) -> ConformanceCaseExecution + Copy,
{
    let state = review_conformance_manifest_with_replay_bundle_envelope(
        manifest,
        options,
        replay_bundle_envelope,
        execute,
    );

    ConformanceManifestReviewedNestedApplication {
        results: execute_review_state_reviewed_nested_executions(&state, callbacks_for_execution),
        state,
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

        let Some(entry) = plan_named_conformance_suite_entry(manifest, &selector, &context) else {
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
