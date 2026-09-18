//! One executable registry for compiled kernel and host-owned workflows.
//! TreeHaver owns parsing; ownership never implies default authority.

use crate::*;
use ast_merge::{
    provider_registry::{
        MergeProviderDescriptor, MergeProviderInventory, MergeProviderRegistry,
        MergeProviderSnapshot,
    },
    provider_selection::{MergeSelectionReport, MergeSelectionRequest, negotiate_merge_provider},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, OnceLock},
};
use tree_haver::service::{
    ExecutionContext, ParseService, ParserConstraints, ParserRegistrySnapshot,
    TreeHaverParseService,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowOperation {
    pub operation: OperationRequest,
    /// Parser language/dialect are explicit, not inferred from provider names.
    pub parser_language: String,
    pub parser_dialect: Option<String>,
    pub parse_options: ParseOptions,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowBatchRequest {
    pub items: Vec<WorkflowOperation>,
}
/// Caller-required registry state, not authentication or a grammar identity lease.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowRegistryExpectation {
    pub provider_generation: u64,
    pub provider_digest: String,
    pub parser_generation: u64,
    pub parser_digest: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PreparedWorkflowOperation {
    pub operation: OperationRequest,
    pub parses: Vec<CoreParseResult>,
    pub selection: MergeSelectionReport,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PreparedWorkflowBatch {
    pub items: Vec<PreparedWorkflowOperation>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowBatchResult {
    pub items: Vec<OperationResult>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowLimits {
    pub max_operations: usize,
    /// Serialized typed request/callback-response budgets, without extra buffers.
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    /// Input-byte budget is cumulative across all operations in this batch.
    pub parse: ParseLimits,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionOwner {
    Host,
    Kernel,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowExecution {
    pub provider: MergeProviderDescriptor,
    pub execution_owner: WorkflowExecutionOwner,
    pub approved_as_default: bool,
    pub selections: Vec<MergeSelectionReport>,
    pub results: Vec<OperationResult>,
}

pub trait WorkflowHost: Send + Sync {
    fn descriptor(&self) -> Result<MergeProviderDescriptor, CoreError>;
    fn execute_batch(
        &self,
        request: PreparedWorkflowBatch,
        control: OperationControl,
    ) -> Result<WorkflowBatchResult, CoreError>;
}
enum WorkflowExecutor {
    Kernel,
    Host(Arc<dyn WorkflowHost>),
}
static WORKFLOWS: OnceLock<MergeProviderRegistry<WorkflowExecutor>> = OnceLock::new();
fn registry() -> &'static MergeProviderRegistry<WorkflowExecutor> {
    WORKFLOWS.get_or_init(|| {
        let registry = MergeProviderRegistry::default();
        for descriptor in crate::artifact_inventory::compiled_provider_inventory().workflows {
            registry
                .register(descriptor, Arc::new(WorkflowExecutor::Kernel))
                .expect("compiled workflow descriptor is valid and unique");
        }
        registry
    })
}
fn require_host_id(id: &str) -> Result<(), CoreError> {
    if crate::operation_profile_catalog().profiles.iter().any(|p| p.provider_id == id) {
        return Err(error(
            "workflow.reserved_provider",
            "compiled provider IDs cannot be mutated through the host API",
        ));
    }
    Ok(())
}
fn error(code: &str, message: &str) -> CoreError {
    CoreError { code: code.into(), message: message.into() }
}
fn registration_error(error: ast_merge::provider_registry::MergeRegistrationError) -> CoreError {
    CoreError { code: "workflow.registration".into(), message: error.to_string() }
}
fn describe(host: &dyn WorkflowHost) -> Result<MergeProviderDescriptor, CoreError> {
    catch_unwind(AssertUnwindSafe(|| host.descriptor()))
        .map_err(|_| error("workflow.descriptor_panic", "workflow descriptor callback panicked"))?
        .map_err(|_| error("workflow.descriptor_fault", "workflow descriptor callback failed"))
}
pub fn register_workflow_host(host: Arc<dyn WorkflowHost>) -> Result<u64, CoreError> {
    let descriptor = describe(host.as_ref())?;
    require_host_id(&descriptor.provider_id)?;
    registry()
        .register(descriptor, Arc::new(WorkflowExecutor::Host(host)))
        .map_err(registration_error)
}
pub fn replace_workflow_host(
    host: Arc<dyn WorkflowHost>,
    expected_generation: u64,
) -> Result<u64, CoreError> {
    let descriptor = describe(host.as_ref())?;
    require_host_id(&descriptor.provider_id)?;
    registry()
        .replace(descriptor, Arc::new(WorkflowExecutor::Host(host)), expected_generation)
        .map_err(registration_error)
}
pub fn unregister_workflow_host(id: String, expected_generation: u64) -> Result<u64, CoreError> {
    require_host_id(&id)?;
    registry().unregister(&id, expected_generation).map_err(registration_error)
}
pub fn workflow_registry_inventory() -> Result<MergeProviderInventory, CoreError> {
    registry().snapshot().map(|snapshot| snapshot.inventory()).map_err(registration_error)
}

/// Source-free selection observations, not availability or preflight approval.
/// Probes follow registered parser policy and may load/acquire grammars: callers
/// requiring offline behavior must register cached-only providers. This is not
/// host health, authenticated availability, execution approval or a preflight lease.
pub fn workflow_selection_reports(
    queries: Vec<MergeSelectionRequest>,
    limits: WorkflowLimits,
) -> Result<Vec<MergeSelectionReport>, CoreError> {
    workflow_selection_reports_controlled(queries, limits, &OperationControl::new())
}

/// Controlled source-free selection observations, not preflight approval.
pub fn workflow_selection_reports_controlled(
    queries: Vec<MergeSelectionRequest>,
    limits: WorkflowLimits,
    control: &OperationControl,
) -> Result<Vec<MergeSelectionReport>, CoreError> {
    let context = limits.parse.clone().controlled_context(control)?;
    context.check().map_err(CoreError::from)?;
    let providers = registry().snapshot().map_err(registration_error)?;
    let parsers = crate::host::registry()
        .snapshot()
        .map_err(|_| error("registry", "parser registry unavailable"))?;
    observe_selection(queries, &limits, &context, &providers, &parsers)
}

fn observe_selection(
    queries: Vec<MergeSelectionRequest>,
    limits: &WorkflowLimits,
    context: &ExecutionContext,
    providers: &MergeProviderSnapshot<WorkflowExecutor>,
    parsers: &ParserRegistrySnapshot,
) -> Result<Vec<MergeSelectionReport>, CoreError> {
    context.check().map_err(CoreError::from)?;
    if queries.len() > limits.max_operations {
        return Err(error("resource.limit", "workflow selection queries exceed operation limit"));
    }
    bounded(&queries, limits.max_request_bytes)?;
    // Validate the entire batch before any provider probe.
    for query in &queries {
        context.check().map_err(CoreError::from)?;
        validate_selection_query(query, providers)?;
    }
    let mut reports = Vec::new();
    for query in &queries {
        reports.push(
            negotiate_merge_provider(
                query,
                providers,
                parsers,
                &TreeHaverParseService::default(),
                context,
            )
            .map_err(CoreError::from)?,
        );
        context.check().map_err(CoreError::from)?;
        bounded(&reports, limits.max_response_bytes)?;
    }
    context.check().map_err(CoreError::from)?;
    bounded(&reports, limits.max_response_bytes)?;
    Ok(reports)
}

fn validate_selection_query(
    query: &MergeSelectionRequest,
    providers: &MergeProviderSnapshot<WorkflowExecutor>,
) -> Result<(), CoreError> {
    query.validate().map_err(CoreError::from)?;
    let Some(id) = query.provider_id.as_deref() else { return Ok(()) };
    if !providers.provider(id).is_some_and(|p| matches!(p.as_ref(), WorkflowExecutor::Kernel)) {
        return Ok(());
    }
    let profile = crate::operation_profile_catalog()
        .profiles
        .into_iter()
        .find(|p| p.provider_id == id && Some(&p.id) == query.profile.as_ref())
        .ok_or_else(|| {
            error("workflow.invalid_request", "compiled workflow requires its explicit profile")
        })?;
    let operation = match query.operation.as_str() {
        "analyze" => OperationKind::Analyze,
        "diff2" => OperationKind::Diff2,
        "merge2" => OperationKind::Merge2,
        "merge3" => OperationKind::Merge3,
        _ => unreachable!("query was validated"),
    };
    if crate::profiles::profile_parser_language(&profile.family, query.dialect.as_deref())
        != Some(query.parser.language.as_str())
        || query.parser.dialect.is_some()
        || query.parser.options != crate::profiles::operation_parse_options(&profile.id, operation)
    {
        return Err(error(
            "workflow.invalid_request",
            "compiled workflow parser query differs from its profile",
        ));
    }
    Ok(())
}

/// The common facade retains explicit compiled-profile semantics, but resolves
/// its executor through the same registry as batches. Host workflows need the
/// batch API's explicit parser query; never infer that query from a host name.
pub(crate) fn execute_compiled_operation(
    request: &crate::ValidatedOperationRequest,
    parsers: &ParserRegistrySnapshot,
    context: &ExecutionContext,
) -> Result<OperationResult, CoreError> {
    let profile = request.request().provider_selection.profile_id.as_deref();
    let declaration = crate::operation_profile_catalog()
        .profiles
        .into_iter()
        .find(|p| Some(p.id.as_str()) == profile);
    if let Some(declaration) = declaration {
        let snapshot = registry().snapshot().map_err(registration_error)?;
        let executor = snapshot.provider(&declaration.provider_id).ok_or_else(|| {
            error("workflow.unknown_provider", "compiled executor is not registered")
        })?;
        if !matches!(executor.as_ref(), WorkflowExecutor::Kernel) {
            return Err(error(
                "workflow.invalid_executor",
                "compiled profile has no kernel executor",
            ));
        }
        return crate::native_operation::execute_native_operation(request, parsers, context);
    }
    // Preserve the existing portable unsupported-profile result, not a host
    // dispatch, text fallback, or an implicit provider choice.
    crate::native_operation::execute_native_operation(request, parsers, context)
}
pub fn execute_workflow_batch(
    provider_id: String,
    request: WorkflowBatchRequest,
    limits: WorkflowLimits,
) -> Result<WorkflowExecution, CoreError> {
    execute_workflow_batch_controlled(provider_id, request, limits, &OperationControl::new())
}
pub fn execute_workflow_batch_controlled(
    provider_id: String,
    request: WorkflowBatchRequest,
    limits: WorkflowLimits,
    control: &OperationControl,
) -> Result<WorkflowExecution, CoreError> {
    let context = limits.parse.clone().controlled_context(control)?;
    context.check().map_err(CoreError::from)?;
    let providers = registry().snapshot().map_err(registration_error)?;
    let parsers = crate::host::registry()
        .snapshot()
        .map_err(|_| error("registry", "parser registry unavailable"))?;
    execute(&provider_id, request, &limits, control, &context, &providers, &parsers)
}

/// Reject changed registry state before probes or execution; not full preflight.
pub fn execute_workflow_batch_at_registry(
    provider_id: String,
    request: WorkflowBatchRequest,
    expected: WorkflowRegistryExpectation,
    limits: WorkflowLimits,
) -> Result<WorkflowExecution, CoreError> {
    execute_workflow_batch_at_registry_controlled(
        provider_id,
        request,
        expected,
        limits,
        &OperationControl::new(),
    )
}

/// Controlled registry-state guard, not authentication or loaded-asset validation.
pub fn execute_workflow_batch_at_registry_controlled(
    provider_id: String,
    request: WorkflowBatchRequest,
    expected: WorkflowRegistryExpectation,
    limits: WorkflowLimits,
    control: &OperationControl,
) -> Result<WorkflowExecution, CoreError> {
    let context = limits.parse.clone().controlled_context(control)?;
    context.check().map_err(CoreError::from)?;
    bounded(&expected, limits.max_request_bytes)?;
    let providers = registry().snapshot().map_err(registration_error)?;
    let parsers = crate::host::registry()
        .snapshot()
        .map_err(|_| error("registry", "parser registry unavailable"))?;
    execute_at_registry(
        &provider_id,
        request,
        &expected,
        &limits,
        control,
        &context,
        (&providers, &parsers),
    )
}

fn execute_at_registry(
    provider_id: &str,
    request: WorkflowBatchRequest,
    expected: &WorkflowRegistryExpectation,
    limits: &WorkflowLimits,
    control: &OperationControl,
    context: &ExecutionContext,
    snapshots: (&MergeProviderSnapshot<WorkflowExecutor>, &ParserRegistrySnapshot),
) -> Result<WorkflowExecution, CoreError> {
    context.check().map_err(CoreError::from)?;
    // Count both typed inputs together, without building a serialized buffer.
    bounded(&(&request, expected), limits.max_request_bytes)?;
    let (providers, parsers) = snapshots;
    if expected.provider_generation != providers.generation()
        || expected.provider_digest != providers.digest()
        || expected.parser_generation != parsers.generation()
        || expected.parser_digest != parsers.digest()
    {
        return Err(error(
            "workflow.registry_stale",
            "registry state differs from caller expectation",
        ));
    }
    // The captured handles, not another snapshot or a new registry lookup, are
    // used throughout execution. Later mutation affects only subsequent calls.
    execute(provider_id, request, limits, control, context, providers, parsers)
}

fn execute(
    provider_id: &str,
    request: WorkflowBatchRequest,
    limits: &WorkflowLimits,
    control: &OperationControl,
    context: &ExecutionContext,
    providers: &MergeProviderSnapshot<WorkflowExecutor>,
    parsers: &ParserRegistrySnapshot,
) -> Result<WorkflowExecution, CoreError> {
    context.check().map_err(CoreError::from)?;
    if request.items.is_empty() {
        return Err(error("workflow.invalid_request", "workflow batch is empty"));
    }
    if request.items.len() > limits.max_operations {
        return Err(error("resource.limit", "workflow batch exceeds operation limit"));
    }
    bounded(&request, limits.max_request_bytes)?;
    let descriptor = providers.descriptor(provider_id).ok_or_else(|| {
        error("workflow.unknown_provider", "explicit workflow provider is not registered")
    })?;
    let executor = providers.provider(provider_id).unwrap();
    let mut remaining = context.max_input_bytes;
    let mut ids = BTreeSet::new();
    let mut validated = vec![];
    // Normalize the whole batch before any parser probes or host execution.
    for item in &request.items {
        context.check().map_err(CoreError::from)?;
        if !ids.insert(item.operation.request_id.clone())
            || item
                .operation
                .provider_selection
                .provider_id
                .as_deref()
                .is_some_and(|id| id != provider_id)
            || item.parser_language.is_empty()
            || item.parser_dialect.as_ref().is_some_and(String::is_empty)
        {
            return Err(error(
                "workflow.invalid_request",
                "workflow identity or parser query is invalid",
            ));
        }
        if item.operation.parser_selection.profile_id.is_some()
            || item.operation.parser_selection.language_version.is_some()
            || !item.operation.parser_selection.extra.is_empty()
            || !item.operation.provider_selection.extra.is_empty()
            || item.operation.extensions.iter().any(|extension| !extension.capabilities.is_empty())
        {
            return Err(error(
                "workflow.unsupported_parser_constraint",
                "requested selector or required extension constraint is not implemented",
            ));
        }
        let value = item
            .operation
            .clone()
            .validate(remaining, |_, _| Err("workflow requires checked inline sources".into()))
            .map_err(|e| {
                error(
                    if e.code == OperationRequestErrorCode::ResourceLimit {
                        "resource.limit"
                    } else {
                        "workflow.invalid_request"
                    },
                    "workflow source/request validation failed",
                )
            })?;
        if value.request().sources.len() > context.max_batch_items {
            return Err(error("resource.limit", "workflow source batch exceeds parse limit"));
        }
        for source in value.request().sources.values() {
            remaining -= source.byte_length;
        }
        validated.push(value);
    }
    let mut queries = vec![];
    for item in &request.items {
        let operation = &item.operation;
        let query = MergeSelectionRequest {
            provider_id: Some(provider_id.into()),
            family: operation
                .provider_selection
                .family
                .clone()
                .unwrap_or_else(|| descriptor.family.clone()),
            operation: match operation.operation.kind() {
                OperationKind::Analyze => "analyze",
                OperationKind::Diff2 => "diff2",
                OperationKind::Merge2 => "merge2",
                OperationKind::Merge3 => "merge3",
            }
            .into(),
            dialect: operation.provider_selection.dialect.clone(),
            profile: operation.provider_selection.profile_id.clone(),
            required_capabilities: operation.provider_selection.required_capabilities.clone(),
            required_preservation: vec![],
            parser: ParserSelectionRequest {
                language: item.parser_language.clone(),
                dialect: item.parser_dialect.clone(),
                options: item.parse_options.clone(),
                selection: ParserSelection {
                    backend_id: operation.parser_selection.backend.clone(),
                    preference: operation.parser_selection.preference.clone(),
                    required_capabilities: operation.parser_selection.required_capabilities.clone(),
                },
            },
        };
        validate_selection_query(&query, providers)?;
        queries.push(query);
    }
    let mut selections = vec![];
    for query in &queries {
        let selection = negotiate_merge_provider(
            query,
            providers,
            parsers,
            &TreeHaverParseService::default(),
            context,
        )
        .map_err(CoreError::from)?;
        if selection.selected_provider.as_deref() != Some(provider_id) {
            return Err(error(
                "workflow.no_eligible_provider",
                "explicit workflow provider or parser is ineligible",
            ));
        }
        selections.push(selection);
    }
    if matches!(executor.as_ref(), WorkflowExecutor::Kernel) {
        let mut results = Vec::new();
        for (value, selection) in validated.iter().zip(&selections) {
            context.check().map_err(CoreError::from)?;
            let selected = selection
                .candidates
                .iter()
                .find(|c| c.selected)
                .unwrap()
                .parser_report
                .as_ref()
                .unwrap()
                .selected_backend
                .as_ref();
            let service = TreeHaverParseService::default()
                .with_constraints(ParserConstraints {
                    allowed_backend_ids: vec![selected.unwrap().clone()],
                    ..ParserConstraints::default()
                })
                .map_err(CoreError::from)?;
            let result = crate::native_operation::execute_native_operation_with_service(
                value, parsers, context, &service,
            )?;
            if result.provider.provider_id.as_deref() != Some(provider_id)
                || result
                    .profile
                    .parser
                    .as_ref()
                    .is_some_and(|p| p.selected_backend.as_ref() != selected)
            {
                return Err(error(
                    "workflow.invalid_result",
                    "compiled workflow changed negotiated executor identity",
                ));
            }
            result.validate_against(value).map_err(|_| {
                error(
                    "workflow.invalid_result",
                    "compiled workflow result failed common validation",
                )
            })?;
            results.push(result);
            #[derive(Serialize)]
            struct Response<'a> {
                items: &'a [OperationResult],
            }
            bounded(&Response { items: &results }, limits.max_response_bytes)?;
        }
        context.check().map_err(CoreError::from)?;
        return Ok(WorkflowExecution {
            provider: descriptor.clone(),
            execution_owner: WorkflowExecutionOwner::Kernel,
            approved_as_default: false,
            selections,
            results,
        });
    }
    let WorkflowExecutor::Host(host) = executor.as_ref() else { unreachable!() };
    let requirements = &descriptor.parser_requirements;
    let service = TreeHaverParseService::default()
        .with_constraints(ParserConstraints {
            allowed_backend_ids: requirements.allowed_backend_ids.clone(),
            forbidden_backend_ids: requirements.forbidden_backend_ids.clone(),
            allowed_backend_families: requirements.allowed_backend_families.clone(),
            forbidden_backend_families: requirements.forbidden_backend_families.clone(),
            required_contracts: requirements.contracts.clone(),
            required_capabilities: requirements.capabilities.clone(),
        })
        .map_err(CoreError::from)?;
    let mut prepared = vec![];
    for ((item, value), selection) in request.items.into_iter().zip(&validated).zip(&selections) {
        let chosen = selection
            .candidates
            .iter()
            .find(|candidate| candidate.selected)
            .unwrap()
            .parser_report
            .as_ref()
            .unwrap();
        let mut parses = vec![];
        for role in item.operation.operation.kind().source_roles() {
            let source = &item.operation.sources[role];
            let document = value
                .sources()
                .get(&source.source_id)
                .map_err(|_| error("workflow.invalid_request", "validated source is missing"))?;
            parses.push(ParseRequest {
                schema: tree_haver::service::PARSE_REQUEST_SCHEMA.into(),
                request_id: format!("{}:{role:?}", item.operation.request_id),
                source: SourceInput {
                    descriptor: document.descriptor().clone(),
                    bytes: document.bytes().to_vec(),
                },
                language: item.parser_language.clone(),
                dialect: item.parser_dialect.clone(),
                options: item.parse_options.clone(),
                selection: ParserSelection {
                    backend_id: chosen.selected_backend.clone(),
                    preference: vec![],
                    required_capabilities: item
                        .operation
                        .parser_selection
                        .required_capabilities
                        .clone(),
                },
                metadata: item.operation.metadata.clone(),
                extra: Metadata::new(),
            });
        }
        let parses = service.parse_batch(parses, parsers, context).map_err(|failure| {
            let failure = ParserFailure::from(failure);
            error(&failure.code, "workflow parser service failed")
        })?;
        if parses.iter().any(|parsed| !parsed.document.output().ok) {
            return Err(error("workflow.input_parse_failed", "workflow input parse failed"));
        }
        prepared.push(PreparedWorkflowOperation {
            operation: item.operation,
            parses: parses.into_iter().map(CoreParseResult::from).collect(),
            selection: selection.clone(),
        });
    }
    let prepared = PreparedWorkflowBatch { items: prepared };
    bounded(&prepared, limits.max_request_bytes)?;
    context.check().map_err(CoreError::from)?;
    let callback = catch_unwind(AssertUnwindSafe(|| host.execute_batch(prepared, control.clone())));
    // Late results and faults never override execution control.
    context.check().map_err(CoreError::from)?;
    let response = callback
        .map_err(|_| error("workflow.provider_panic", "workflow callback panicked"))?
        .map_err(|_| error("workflow.provider_fault", "workflow callback failed"))?;
    bounded(&response, limits.max_response_bytes)?;
    // Flattened extension maps must not shadow reserved typed fields at the
    // portable boundary. Strict typed decoding rejects duplicate known keys;
    // the prior byte check bounds this validation buffer. This is an internal
    // wire-contract check, not a JSON-string callback interface.
    let encoded = serde_json::to_vec(&response)
        .map_err(|_| error("workflow.invalid_result", "workflow result is not serializable"))?;
    let restored: WorkflowBatchResult = serde_json::from_slice(&encoded).map_err(|_| {
        error("workflow.invalid_result", "workflow result has an invalid wire representation")
    })?;
    if restored != response {
        return Err(error(
            "workflow.invalid_result",
            "workflow result changes under wire encoding",
        ));
    }
    if response.items.len() != validated.len() {
        return Err(error("workflow.invalid_result", "workflow result cardinality mismatch"));
    }
    for ((result, input), selection) in response.items.iter().zip(&validated).zip(&selections) {
        context.check().map_err(CoreError::from)?;
        let parser = selection
            .candidates
            .iter()
            .find(|candidate| candidate.selected)
            .unwrap()
            .parser_report
            .as_ref()
            .unwrap();
        if result.provider.provider_id.as_deref() != Some(provider_id)
            || result.provider.family.as_deref() != Some(descriptor.family.as_str())
            || result.provider.delegation.as_ref().is_some_and(|chain| !chain.is_empty())
            || result
                .profile
                .profile_id
                .as_ref()
                .is_some_and(|profile| !descriptor.profiles.contains(profile))
            || result.profile.parser.as_ref().is_none_or(|profile| {
                profile.selected_backend != parser.selected_backend
                    || profile.requested_backend != input.request().parser_selection.backend
                    || profile.selection_mode.as_deref()
                        != Some(if input.request().parser_selection.backend.is_some() {
                            "explicit"
                        } else {
                            "policy"
                        })
            })
        {
            return Err(error(
                "workflow.invalid_result",
                "workflow returned inconsistent provider or parser identity",
            ));
        }
        result.validate_against(input).map_err(|_| {
            error("workflow.invalid_result", "workflow result failed common contract validation")
        })?;
    }
    context.check().map_err(CoreError::from)?;
    Ok(WorkflowExecution {
        provider: descriptor.clone(),
        execution_owner: WorkflowExecutionOwner::Host,
        approved_as_default: false,
        selections,
        results: response.items,
    })
}

fn bounded(value: &impl Serialize, limit: usize) -> Result<(), CoreError> {
    struct Budget(usize);
    impl io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| io::Error::other("workflow byte limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Budget(limit), value)
        .map_err(|_| error("resource.limit", "workflow typed envelope exceeds byte budget"))
}

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;
