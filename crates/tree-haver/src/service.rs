//! Typed in-process parser dispatch (Slices 1024, 1026, 1030).
//! Host runtime executors and encoded-frame limits belong to the binding bridge;
//! this service does not claim to make arbitrary host objects thread-safe.

use std::{
    collections::{BTreeMap, BTreeSet},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    parsed::{
        NativeExtension, ParseOutput, ParseValidationError, ParseValidationLimits, ParsedDocument,
    },
    source::{SourceDocument, SourceError, SourceErrorCode, SourceInput},
};

pub const PARSE_REQUEST_SCHEMA: &str = "structuredmerge.parse-request/v1";
pub const PARSE_RESULT_SCHEMA: &str = "structuredmerge.parse-result/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserSelection {
    pub backend_id: Option<String>,
    pub preference: Vec<String>,
    pub required_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParserProviderDescriptor {
    pub id: String,
    pub family: String,
    pub runtime: String,
    pub package: String,
    pub package_version: String,
    pub parser: String,
    pub parser_version: String,
    pub grammar: Option<String>,
    pub grammar_version: Option<String>,
    pub languages: Vec<String>,
    pub dialects: Vec<String>,
    pub contracts: Vec<String>,
    pub capabilities: Vec<String>,
    pub probe_id: String,
    pub priority: i32,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub extensions: Vec<NativeExtension>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParseOptions {
    pub comments: bool,
    pub tokens: bool,
    pub diagnostics: bool,
    pub native_extensions: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseRequest {
    pub schema: String,
    pub request_id: String,
    pub source: SourceInput,
    pub language: String,
    pub dialect: Option<String>,
    pub selection: ParserSelection,
    pub options: ParseOptions,
    pub metadata: BTreeMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserProbeRequest {
    pub language: String,
    pub dialect: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserProbeResult {
    pub available: bool,
    pub loadable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderFault {
    pub code: String,
    pub message: String,
}

/// Implementors must support concurrent calls. A host wrapper may implement this
/// only after providing its own verified runtime-affine executor.
pub trait ParserProvider: Send + Sync {
    fn descriptor(&self) -> &ParserProviderDescriptor;
    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault>;
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault>;
}

#[derive(Clone, Debug)]
pub struct ExecutionContext {
    pub cancelled: Arc<AtomicBool>,
    pub deadline: Option<Instant>,
    pub max_batch_items: usize,
    pub max_input_bytes: u64,
    pub max_nodes: usize,
    pub max_diagnostics: usize,
}

impl ExecutionContext {
    pub fn check(&self) -> Result<(), ServiceError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ServiceError::Cancelled);
        }
        if self.deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(ServiceError::DeadlineExceeded);
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Registration {
    descriptor: ParserProviderDescriptor,
    provider: Arc<dyn ParserProvider>,
}

#[derive(Clone, Default)]
struct RegistryState {
    generation: u64,
    entries: BTreeMap<String, Registration>,
}

#[derive(Default)]
pub struct ParserRegistry {
    state: RwLock<RegistryState>,
}

#[derive(Clone)]
pub struct ParserRegistrySnapshot {
    state: RegistryState,
    digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationError {
    InvalidDescriptor,
    DuplicateId,
    UnknownId,
    StaleGeneration,
    GenerationExhausted,
    Poisoned,
    ProviderPanic,
}

impl ParserRegistry {
    /// Descriptor callbacks run before the registry lock. Identity is cached.
    pub fn register(&self, provider: Arc<dyn ParserProvider>) -> Result<u64, RegistrationError> {
        let descriptor = catch_unwind(AssertUnwindSafe(|| provider.descriptor().clone()))
            .map_err(|_| RegistrationError::ProviderPanic)?;
        validate_descriptor(&descriptor)?;
        let mut state = self.state.write().map_err(|_| RegistrationError::Poisoned)?;
        if state.entries.contains_key(&descriptor.id) {
            return Err(RegistrationError::DuplicateId);
        }
        let generation =
            state.generation.checked_add(1).ok_or(RegistrationError::GenerationExhausted)?;
        state.entries.insert(descriptor.id.clone(), Registration { descriptor, provider });
        state.generation = generation;
        Ok(generation)
    }

    /// Removal changes future snapshots only; provider destruction runs unlocked.
    pub fn unregister(&self, id: &str, expected_generation: u64) -> Result<u64, RegistrationError> {
        let (removed, generation) = {
            let mut state = self.state.write().map_err(|_| RegistrationError::Poisoned)?;
            if state.generation != expected_generation {
                return Err(RegistrationError::StaleGeneration);
            }
            if !state.entries.contains_key(id) {
                return Err(RegistrationError::UnknownId);
            }
            let generation =
                state.generation.checked_add(1).ok_or(RegistrationError::GenerationExhausted)?;
            let removed = state.entries.remove(id);
            state.generation = generation;
            (removed, generation)
        };
        drop(removed);
        Ok(generation)
    }

    pub fn snapshot(&self) -> Result<ParserRegistrySnapshot, RegistrationError> {
        let state = self.state.read().map_err(|_| RegistrationError::Poisoned)?.clone();
        let descriptors: Vec<_> = state.entries.values().map(|entry| &entry.descriptor).collect();
        // BTreeMaps and validated sorted sets make the descriptor digest independent
        // of registration order. Generation separately identifies lifecycle changes.
        let bytes =
            serde_json::to_vec(&descriptors).map_err(|_| RegistrationError::InvalidDescriptor)?;
        let digest = format!("{:x}", Sha256::digest(bytes));
        Ok(ParserRegistrySnapshot { state, digest })
    }
}

impl ParserRegistrySnapshot {
    pub fn generation(&self) -> u64 {
        self.state.generation
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }
}

fn set_is_valid(values: &[String]) -> bool {
    values.iter().all(|value| !value.is_empty()) && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_descriptor(descriptor: &ParserProviderDescriptor) -> Result<(), RegistrationError> {
    crate::parsed::validate_extensions(&descriptor.extensions)
        .map_err(|_| RegistrationError::InvalidDescriptor)?;
    if [
        &descriptor.id,
        &descriptor.family,
        &descriptor.runtime,
        &descriptor.package,
        &descriptor.package_version,
        &descriptor.parser,
        &descriptor.parser_version,
        &descriptor.probe_id,
    ]
    .iter()
    .any(|value| value.is_empty())
        || descriptor.languages.is_empty()
        || !descriptor.contracts.iter().any(|contract| contract == PARSE_RESULT_SCHEMA)
        || [
            &descriptor.languages,
            &descriptor.dialects,
            &descriptor.contracts,
            &descriptor.capabilities,
        ]
        .iter()
        .any(|values| !set_is_valid(values))
        || descriptor.grammar.as_ref().is_some_and(String::is_empty)
        || descriptor.grammar_version.as_ref().is_some_and(String::is_empty)
    {
        return Err(RegistrationError::InvalidDescriptor);
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserCandidate {
    pub backend_id: String,
    pub rejections: Vec<String>,
    pub available: Option<bool>,
    pub loadable: Option<bool>,
    pub probe_fault: Option<String>,
    pub request_preference: Option<usize>,
    pub profile_preference: Option<usize>,
    pub priority: i32,
    pub selected: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelectionReport {
    pub requested: ParserSelection,
    pub generation: u64,
    pub digest: String,
    pub candidates: Vec<ParserCandidate>,
    pub selected_backend: Option<String>,
}

#[derive(Clone)]
pub struct SelectedParser {
    registration: Registration,
    pub report: SelectionReport,
}

#[derive(Clone, Debug)]
pub struct ParsedResult {
    pub schema: String,
    pub selection: SelectionReport,
    pub backend: ParserProviderDescriptor,
    pub document: ParsedDocument,
    pub source: SourceDocument,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceError {
    InvalidRequest,
    LimitExceeded,
    Cancelled,
    DeadlineExceeded,
    Source(SourceError),
    Selection(Box<SelectionReport>),
    Provider { backend_id: String, fault: ProviderFault },
    ProviderPanic { backend_id: String },
    InvalidBatch { backend_id: String },
    InvalidResult { backend_id: String, error: ParseValidationError },
}

/// Language-profile preferences are explicit service configuration, never a
/// built-in package-name order. This initial implementation has no availability cache.
#[derive(Default)]
pub struct TreeHaverParseService {
    profile_preferences: BTreeMap<String, Vec<String>>,
}

pub trait ParseService: Send + Sync {
    fn parser_for(
        &self,
        request: &ParseRequest,
        snapshot: &ParserRegistrySnapshot,
        context: &ExecutionContext,
    ) -> Result<SelectedParser, ServiceError>;
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        snapshot: &ParserRegistrySnapshot,
        context: &ExecutionContext,
    ) -> Result<Vec<ParsedResult>, ServiceError>;
}

impl TreeHaverParseService {
    pub fn with_profile_preferences(
        preferences: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, ServiceError> {
        if preferences.iter().any(|(language, ids)| language.is_empty() || !ordered_ids_valid(ids))
        {
            return Err(ServiceError::InvalidRequest);
        }
        Ok(Self { profile_preferences: preferences })
    }
}

impl ParseService for TreeHaverParseService {
    fn parser_for(
        &self,
        request: &ParseRequest,
        snapshot: &ParserRegistrySnapshot,
        context: &ExecutionContext,
    ) -> Result<SelectedParser, ServiceError> {
        context.check()?;
        validate_request(request)?;
        let mut report = SelectionReport {
            requested: request.selection.clone(),
            generation: snapshot.generation(),
            digest: snapshot.digest.clone(),
            candidates: vec![],
            selected_backend: None,
        };
        for entry in snapshot.state.entries.values() {
            let descriptor = &entry.descriptor;
            let mut candidate = ParserCandidate {
                backend_id: descriptor.id.clone(),
                rejections: vec![],
                available: None,
                loadable: None,
                probe_fault: None,
                request_preference: request
                    .selection
                    .preference
                    .iter()
                    .position(|id| id == &descriptor.id),
                profile_preference: self
                    .profile_preferences
                    .get(&request.language)
                    .and_then(|ids| ids.iter().position(|id| id == &descriptor.id)),
                priority: descriptor.priority,
                selected: false,
            };
            if request.selection.backend_id.as_ref().is_some_and(|id| id != &descriptor.id) {
                candidate.rejections.push("explicit_backend_mismatch".into());
            }
            if !descriptor.languages.contains(&request.language) {
                candidate.rejections.push("unsupported_language".into());
            }
            if request
                .dialect
                .as_ref()
                .is_some_and(|dialect| !descriptor.dialects.contains(dialect))
            {
                candidate.rejections.push("unsupported_dialect".into());
            }
            let mut required = request.selection.required_capabilities.clone();
            for (enabled, capability) in [
                (request.options.comments, "comments"),
                (request.options.tokens, "tokens"),
                (request.options.diagnostics, "diagnostics"),
                (request.options.native_extensions, "native_extensions"),
            ] {
                if enabled {
                    required.push(capability.into());
                }
            }
            for capability in required {
                if !descriptor.capabilities.contains(&capability) {
                    candidate.rejections.push(format!("missing_capability:{capability}"));
                }
            }
            if candidate.rejections.is_empty() {
                context.check()?;
                let probe = catch_unwind(AssertUnwindSafe(|| {
                    entry.provider.probe(&ParserProbeRequest {
                        language: request.language.clone(),
                        dialect: request.dialect.clone(),
                    })
                }));
                context.check()?;
                match probe {
                    Ok(Ok(probe)) => {
                        candidate.available = Some(probe.available);
                        candidate.loadable = Some(probe.loadable);
                        if !probe.available {
                            candidate.rejections.push("unavailable".into());
                        }
                        if !probe.loadable {
                            candidate.rejections.push("not_loadable".into());
                        }
                    }
                    Ok(Err(fault)) => {
                        candidate.probe_fault = Some(fault.code);
                        candidate.rejections.push("probe_fault".into());
                    }
                    Err(_) => {
                        candidate.rejections.push("probe_panic".into());
                    }
                }
            }
            report.candidates.push(candidate);
        }
        report.candidates.sort_by(|left, right| {
            (
                left.request_preference.unwrap_or(usize::MAX),
                left.profile_preference.unwrap_or(usize::MAX),
            )
                .cmp(&(
                    right.request_preference.unwrap_or(usize::MAX),
                    right.profile_preference.unwrap_or(usize::MAX),
                ))
                .then_with(|| right.priority.cmp(&left.priority))
                .then_with(|| left.backend_id.cmp(&right.backend_id))
        });
        let Some(winner) =
            report.candidates.iter_mut().find(|candidate| candidate.rejections.is_empty())
        else {
            return Err(ServiceError::Selection(Box::new(report)));
        };
        winner.selected = true;
        let id = winner.backend_id.clone();
        report.selected_backend = Some(id.clone());
        Ok(SelectedParser { registration: snapshot.state.entries[&id].clone(), report })
    }

    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        snapshot: &ParserRegistrySnapshot,
        context: &ExecutionContext,
    ) -> Result<Vec<ParsedResult>, ServiceError> {
        context.check()?;
        if requests.is_empty() {
            return Err(ServiceError::InvalidRequest);
        }
        if requests.len() > context.max_batch_items {
            return Err(ServiceError::LimitExceeded);
        }
        let mut sources = BTreeMap::new();
        let mut source_ids = BTreeSet::new();
        let mut remaining = context.max_input_bytes;
        // Validate the whole batch before the first callback, including probes.
        for request in &requests {
            validate_request(request)?;
            if sources.contains_key(&request.request_id) {
                return Err(ServiceError::InvalidRequest);
            }
            if !source_ids.insert(request.source.descriptor.source_id.clone()) {
                return Err(ServiceError::Source(SourceError {
                    code: SourceErrorCode::DuplicateId,
                    source_id: request.source.descriptor.source_id.clone(),
                }));
            }
            let source = SourceDocument::validate(request.source.clone(), remaining)
                .map_err(ServiceError::Source)?;
            remaining -= source.descriptor().byte_length;
            sources.insert(request.request_id.clone(), source);
        }
        let order: Vec<_> = requests.iter().map(|request| request.request_id.clone()).collect();
        let mut groups: BTreeMap<String, (Registration, Vec<ParseRequest>)> = BTreeMap::new();
        let mut selections = BTreeMap::new();
        for request in requests {
            let selected = self.parser_for(&request, snapshot, context)?;
            selections.insert(request.request_id.clone(), selected.report);
            groups
                .entry(selected.registration.descriptor.id.clone())
                .or_insert_with(|| (selected.registration, vec![]))
                .1
                .push(request);
        }
        let mut results = BTreeMap::new();
        for (backend_id, (registration, requests)) in groups {
            context.check()?;
            let mut expected: BTreeSet<_> =
                requests.iter().map(|request| request.request_id.clone()).collect();
            let outputs = catch_unwind(AssertUnwindSafe(|| {
                registration.provider.parse_batch(requests, context)
            }))
            .map_err(|_| ServiceError::ProviderPanic { backend_id: backend_id.clone() })?
            .map_err(|fault| ServiceError::Provider { backend_id: backend_id.clone(), fault })?;
            context.check()?;
            if outputs.len() != expected.len() {
                return Err(ServiceError::InvalidBatch { backend_id });
            }
            for output in outputs {
                let id = output.request_id.clone();
                if !expected.remove(&id) {
                    return Err(ServiceError::InvalidBatch { backend_id });
                }
                let source = sources.remove(&id).ok_or(ServiceError::InvalidRequest)?;
                let capabilities = &registration.descriptor.capabilities;
                let document = ParsedDocument::validate(
                    output,
                    &id,
                    &source,
                    ParseValidationLimits {
                        max_nodes: context.max_nodes,
                        max_diagnostics: context.max_diagnostics,
                        partial_tree_allowed: capabilities
                            .iter()
                            .any(|capability| capability == "partial_trees"),
                        comments_supported: capabilities
                            .iter()
                            .any(|capability| capability == "comments"),
                    },
                )
                .map_err(|error| ServiceError::InvalidResult {
                    backend_id: backend_id.clone(),
                    error,
                })?;
                results.insert(
                    id.clone(),
                    ParsedResult {
                        schema: PARSE_RESULT_SCHEMA.into(),
                        selection: selections.remove(&id).ok_or(ServiceError::InvalidRequest)?,
                        backend: registration.descriptor.clone(),
                        document,
                        source,
                    },
                );
            }
        }
        context.check()?;
        order
            .into_iter()
            .map(|id| results.remove(&id).ok_or(ServiceError::InvalidRequest))
            .collect()
    }
}

fn ordered_ids_valid(values: &[String]) -> bool {
    values.iter().all(|value| !value.is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn validate_request(request: &ParseRequest) -> Result<(), ServiceError> {
    if request.schema != PARSE_REQUEST_SCHEMA
        || request.request_id.is_empty()
        || request.language.is_empty()
        || request.dialect.as_ref().is_some_and(String::is_empty)
        || request.selection.backend_id.as_ref().is_some_and(String::is_empty)
        || !ordered_ids_valid(&request.selection.preference)
        || !set_is_valid(&request.selection.required_capabilities)
    {
        return Err(ServiceError::InvalidRequest);
    }
    Ok(())
}
