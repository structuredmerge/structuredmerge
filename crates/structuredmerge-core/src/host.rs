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
    sync::{Arc, OnceLock, atomic::AtomicBool},
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

pub fn register_parser_host(host: Arc<dyn ParserHost>) -> Result<(), CoreError> {
    // No registry lock spans a host callback. TreeHaver caches the descriptor.
    let descriptor =
        catch_unwind(AssertUnwindSafe(|| host.descriptor())).map_err(|_| CoreError {
            code: "registration_panic".into(),
            message: "parser descriptor panicked".into(),
        })??;
    registry()
        .register(Arc::new(HostParser { descriptor, host }))
        .map(|_| ())
        .map_err(|error| CoreError { code: "registration".into(), message: format!("{error:?}") })
}

pub fn unregister_parser_host(id: String) -> Result<(), CoreError> {
    let snapshot = registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    registry()
        .unregister(&id, snapshot.generation())
        .map(|_| ())
        .map_err(|error| CoreError { code: "registration".into(), message: format!("{error:?}") })
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
}

impl ParseLimits {
    pub(crate) fn context(self) -> ExecutionContext {
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: self.max_batch_items,
            max_input_bytes: self.max_input_bytes,
            max_nodes: self.max_nodes,
            max_diagnostics: self.max_diagnostics,
        }
    }
}

pub fn parse_sources(
    requests: Vec<ParseRequest>,
    limits: ParseLimits,
) -> Result<Vec<CoreParseResult>, CoreError> {
    let snapshot = registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    let context = limits.context();
    TreeHaverParseService::default()
        .parse_batch(requests, &snapshot, &context)
        .map(|results| results.into_iter().map(CoreParseResult::from).collect())
        .map_err(CoreError::from)
}
