//! Typed host adapter over TreeHaver's registry, not a separate parser registry.
use crate::{
    ParseOutput, ParseRequest, ParserProbeRequest, ParserProbeResult, ParserProviderDescriptor,
    SelectionReport,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tree_haver::service::{
    ExecutionContext, ParseService, ParserProvider, ParserRegistry, ProviderFault, ServiceError,
    TreeHaverParseService,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoreError {
    pub code: String,
    pub message: String,
}

impl CoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { code: "host_bridge".into(), message: message.into() }
    }
}
impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl Error for CoreError {}

impl From<ServiceError> for CoreError {
    fn from(error: ServiceError) -> Self {
        let (code, message) = match error {
            ServiceError::InvalidRequest => ("request.invalid", "invalid parser request".into()),
            ServiceError::LimitExceeded => {
                ("resource.limit", "parser resource limit exceeded".into())
            }
            ServiceError::Cancelled => ("execution.cancelled", "operation cancelled".into()),
            ServiceError::DeadlineExceeded => {
                ("execution.deadline_exceeded", "operation deadline exceeded".into())
            }
            ServiceError::Source(error) => (
                if error.code == crate::SourceErrorCode::LimitExceeded {
                    "resource.limit"
                } else {
                    "source.invalid"
                },
                error.to_string(),
            ),
            ServiceError::Selection(report) => (
                "selection.no_parser",
                match report.requested.backend_id {
                    Some(id) => format!("no eligible parser for explicit backend {id}"),
                    None => "no eligible parser for request".into(),
                },
            ),
            ServiceError::Provider { backend_id, fault } => (
                "parser.provider_fault",
                format!("parser {backend_id}: {}: {}", fault.code, fault.message),
            ),
            ServiceError::ProviderPanic { backend_id } => {
                ("parser.provider_panic", format!("parser {backend_id} panicked"))
            }
            ServiceError::InvalidBatch { backend_id } => {
                ("parser.invalid_batch", format!("parser {backend_id} returned an invalid batch"))
            }
            ServiceError::InvalidResult { backend_id, error } => (
                if error == crate::parsed::ParseValidationError::LimitExceeded {
                    "resource.limit"
                } else {
                    "parser.invalid_result"
                },
                format!("parser {backend_id} result failed validation: {error:?}"),
            ),
        };
        Self { code: code.into(), message }
    }
}

/// Structured service-failure evidence. Native codes remain separate from the
/// stable core code; this is not the complete portable diagnostic envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserFailure {
    pub code: String,
    pub message: String,
    pub backend_id: Option<String>,
    pub native_code: Option<String>,
    pub native_message: Option<String>,
    pub source_id: Option<String>,
    pub selection: Option<SelectionReport>,
}

impl From<ServiceError> for ParserFailure {
    fn from(error: ServiceError) -> Self {
        let core = CoreError::from(error.clone());
        let mut failure = Self {
            code: core.code,
            message: core.message,
            backend_id: None,
            native_code: None,
            native_message: None,
            source_id: None,
            selection: None,
        };
        match error {
            ServiceError::Provider { backend_id, fault } => {
                failure.backend_id = Some(backend_id);
                failure.native_code = Some(fault.code);
                failure.native_message = Some(fault.message);
            }
            ServiceError::ProviderPanic { backend_id }
            | ServiceError::InvalidBatch { backend_id }
            | ServiceError::InvalidResult { backend_id, .. } => {
                failure.backend_id = Some(backend_id);
            }
            ServiceError::Selection(report) => failure.selection = Some(*report),
            ServiceError::Source(error) => failure.source_id = Some(error.source_id),
            ServiceError::InvalidRequest
            | ServiceError::LimitExceeded
            | ServiceError::Cancelled
            | ServiceError::DeadlineExceeded => {}
        }
        failure
    }
}

/// Coarse owned parser calls. Generated wrappers must enforce their runtime's
/// thread-affinity rules; these Rust bounds alone do not establish host safety.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseBatchRequest {
    pub items: Vec<ParseRequest>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseBatchResult {
    pub items: Vec<ParseOutput>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeBatchRequest {
    pub items: Vec<ParserProbeRequest>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeBatchResult {
    pub items: Vec<ParserProbeResult>,
}

pub trait ParserHost: Send + Sync {
    fn descriptor(&self) -> Result<ParserProviderDescriptor, CoreError>;
    fn probe_batch(&self, request: ProbeBatchRequest) -> Result<ProbeBatchResult, CoreError>;
    fn parse_batch(&self, request: ParseBatchRequest) -> Result<ParseBatchResult, CoreError>;
}

struct HostParser {
    descriptor: ParserProviderDescriptor,
    host: Arc<dyn ParserHost>,
}

fn fault(error: CoreError) -> ProviderFault {
    ProviderFault { code: error.code, message: error.message }
}

impl ParserProvider for HostParser {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        let mut results = self
            .host
            .probe_batch(ProbeBatchRequest { items: vec![request.clone()] })
            .map_err(fault)?
            .items;
        if results.len() != 1 {
            return Err(ProviderFault {
                code: "invalid_probe_batch".into(),
                message: "expected one probe result".into(),
            });
        }
        Ok(results.remove(0))
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        context.check().map_err(|error| ProviderFault {
            code: "execution_control".into(),
            message: format!("{error:?}"),
        })?;
        self.host
            .parse_batch(ParseBatchRequest { items: requests })
            .map(|result| result.items)
            .map_err(fault)
    }
}

static PARSERS: OnceLock<ParserRegistry> = OnceLock::new();
pub(crate) fn registry() -> &'static ParserRegistry {
    PARSERS.get_or_init(ParserRegistry::default)
}

/// Observe registered parser declarations without loading or probing providers.
/// This is not an availability report, merge capability manifest or authority grant.
pub fn parser_registry_inventory() -> Result<crate::ParserRegistryInventory, CoreError> {
    registry()
        .snapshot()
        .map(|snapshot| snapshot.inventory())
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })
}

fn describe_host(host: Arc<dyn ParserHost>) -> Result<HostParser, CoreError> {
    // No registry lock spans a host callback. TreeHaver caches the descriptor.
    let descriptor =
        catch_unwind(AssertUnwindSafe(|| host.descriptor())).map_err(|_| CoreError {
            code: "registration_panic".into(),
            message: "parser descriptor panicked".into(),
        })??;
    Ok(HostParser { descriptor, host })
}

pub fn register_parser_host(host: Arc<dyn ParserHost>) -> Result<(), CoreError> {
    registry()
        .register(Arc::new(describe_host(host)?))
        .map(|_| ())
        .map_err(|error| CoreError { code: "registration".into(), message: format!("{error:?}") })
}

/// Atomically replace the host's existing provider ID at an observed generation.
/// Old snapshots retain their provider. The returned generation identifies the
/// replacement commit point; later/re-entrant mutations may advance it again.
pub fn replace_parser_host(
    host: Arc<dyn ParserHost>,
    expected_generation: u64,
) -> Result<u64, CoreError> {
    registry()
        .replace(Arc::new(describe_host(host)?), expected_generation)
        .map_err(|error| CoreError { code: "registration".into(), message: format!("{error:?}") })
}

/// Opt into a Rust-owned language-pack parser in TreeHaver's shared registry.
/// Registration does not load a grammar or replace an existing provider. Probe
/// and parse may load/download grammars under the configured language-pack policy.
pub fn register_language_pack_parser(
    id: String,
    language: String,
) -> Result<ParserProviderDescriptor, CoreError> {
    let provider = tree_haver::language_pack_provider::LanguagePackProvider::new(id, language)
        .map_err(|error| CoreError { code: error.code, message: error.message })?;
    register_language_pack_provider(provider)
}

/// Register a Rust-owned parser restricted to usable local or already-loaded
/// grammars. Missing or unusable grammars fail closed without acquisition during
/// probe/parse. Registration itself does not load grammars or replace providers.
pub fn register_cached_language_pack_parser(
    id: String,
    language: String,
) -> Result<ParserProviderDescriptor, CoreError> {
    let provider =
        tree_haver::language_pack_provider::LanguagePackProvider::new_cached_only(id, language)
            .map_err(|error| CoreError { code: error.code, message: error.message })?;
    register_language_pack_provider(provider)
}

fn register_language_pack_provider(
    provider: tree_haver::language_pack_provider::LanguagePackProvider,
) -> Result<ParserProviderDescriptor, CoreError> {
    let descriptor = provider.descriptor().clone();
    registry().register(Arc::new(provider)).map_err(|error| CoreError {
        code: "registration".into(),
        message: format!("{error:?}"),
    })?;
    Ok(descriptor)
}

/// Remove a native or host parser for future operations. Existing snapshots
/// remain valid. Concurrent registry mutations can cause StaleGeneration;
/// unknown IDs fail rather than silently succeeding.
pub fn unregister_parser_provider(id: String) -> Result<(), CoreError> {
    let snapshot = registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    registry()
        .unregister(&id, snapshot.generation())
        .map(|_| ())
        .map_err(|error| CoreError { code: "registration".into(), message: format!("{error:?}") })
}

/// Compatibility spelling; both parser kinds share the same registry.
pub fn unregister_parser_host(id: String) -> Result<(), CoreError> {
    unregister_parser_provider(id)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CoreParseResult {
    pub schema: String,
    pub selection: SelectionReport,
    pub backend: ParserProviderDescriptor,
    pub parsed: ParseOutput,
}

impl From<tree_haver::service::ParsedResult> for CoreParseResult {
    fn from(result: tree_haver::service::ParsedResult) -> Self {
        Self {
            schema: result.schema,
            selection: result.selection,
            backend: result.backend,
            parsed: result.document.output().clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParseLimits {
    pub max_batch_items: usize,
    pub max_input_bytes: u64,
    pub max_nodes: usize,
    pub max_diagnostics: usize,
    /// Monotonic operation budget, including selection and output verification.
    /// None disables the deadline; zero rejects before any parser callback.
    /// Callbacks are cooperative: late results are rejected, not preempted.
    #[serde(default)]
    pub timeout_millis: Option<u64>,
}

/// Shared, one-way cancellation signal. Clones observe the same state.
/// Not serializable: a process-local control is not request data.
#[derive(Clone, Debug)]
pub struct OperationControl {
    cancelled: Arc<AtomicBool>,
}

impl OperationControl {
    // A control is an identity-bearing capability, not a defaultable options DTO.
    // Require explicit construction so generated facades preserve its shared state.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { cancelled: Arc::new(AtomicBool::new(false)) }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub fn create_operation_control() -> OperationControl {
    OperationControl::new()
}

impl ParseLimits {
    pub(crate) fn controlled_context(
        self,
        control: &OperationControl,
    ) -> Result<ExecutionContext, CoreError> {
        let mut context = self.context()?;
        context.cancelled = control.cancelled.clone();
        Ok(context)
    }

    pub(crate) fn context(self) -> Result<ExecutionContext, CoreError> {
        let deadline = self
            .timeout_millis
            .map(|millis| {
                Instant::now().checked_add(Duration::from_millis(millis)).ok_or_else(|| CoreError {
                    code: "request.invalid".into(),
                    message: "operation timeout exceeds monotonic clock range".into(),
                })
            })
            .transpose()?;
        Ok(ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline,
            max_batch_items: self.max_batch_items,
            max_input_bytes: self.max_input_bytes,
            max_nodes: self.max_nodes,
            max_diagnostics: self.max_diagnostics,
        })
    }
}

pub fn parse_sources(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<Vec<CoreParseResult>, CoreError> {
    parse_sources_controlled(requests, limits, &OperationControl::new())
}

/// Probe source-free eligibility using the same snapshot/selection algorithm as
/// parse dispatch. No eligible parser is a report, not an exception or fallback.
/// Parse byte/node limits do not apply because no document is submitted; timeout
/// and cancellation still apply around callbacks, which are not forcibly stopped.
pub fn parser_selection_report(
    request: crate::ParserSelectionRequest,
    limits: ParseLimits,
) -> Result<SelectionReport, CoreError> {
    parser_selection_report_controlled(request, limits, &OperationControl::new())
}

pub fn parser_selection_report_controlled(
    request: crate::ParserSelectionRequest,
    limits: ParseLimits,
    control: &OperationControl,
) -> Result<SelectionReport, CoreError> {
    let context = limits.controlled_context(control)?;
    let snapshot = registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    TreeHaverParseService::default()
        .selection_report(&request, &snapshot, &context)
        .map_err(CoreError::from)
}

pub fn parse_sources_controlled(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
    control: &OperationControl,
) -> Result<Vec<CoreParseResult>, CoreError> {
    let context = limits.controlled_context(control)?;
    let snapshot = registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    TreeHaverParseService::default()
        .parse_batch(requests, &snapshot, &context)
        .map(|results| results.into_iter().map(CoreParseResult::from).collect())
        .map_err(CoreError::from)
}
