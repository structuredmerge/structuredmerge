use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    io::Write,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::{Duration, Instant},
};

use ast_merge_git::{Merge3Request as GitMerge3Request, merge3 as merge_git_three_way};
use go_merge::{GoDialect, merge_go, merge_go_three_way as merge_go_three_way_impl, parse_go};
use json_merge::{
    JsonDialect, merge_json_source_preserving,
    merge_json_three_way as merge_json_three_way_source_preserving, parse_json,
};
use parking_lot::{Mutex, RwLock};
use rust_merge::{
    RustDialect, merge_rust, merge_rust_three_way as merge_rust_three_way_impl, parse_rust,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tree_haver::{ParserRequest, parse_normalized_with_language_pack, parse_with_language_pack};
use typescript_merge::{
    TypeScriptDialect, merge_typescript as merge_typescript_impl,
    merge_typescript_three_way as merge_typescript_three_way_impl, parse_typescript,
};

pub const PACKAGE_NAME: &str = "structuredmerge-host-prototype-core";
const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
const MAX_CAPABILITIES: usize = 64;
const MAX_PROVIDER_NAME_BYTES: usize = 256;
const MAX_PROVIDER_VERSION_BYTES: usize = 256;
const MAX_BATCH_ITEMS: usize = 32;

type IdentityWorkerResult = Result<Vec<u8>, HostPrototypeError>;

struct IdentityWorkerTask {
    receiver: Receiver<IdentityWorkerResult>,
    cancelled: Arc<AtomicBool>,
}

struct PreparedIdentityWorker {
    provider: Arc<ProviderEntry<dyn WorkflowHost>>,
    request: Vec<u8>,
    cancelled: Arc<AtomicBool>,
}

static IDENTITY_WORKERS: OnceLock<Mutex<BTreeMap<u64, IdentityWorkerTask>>> = OnceLock::new();
static PREPARED_IDENTITY_WORKERS: OnceLock<Mutex<BTreeMap<u64, PreparedIdentityWorker>>> =
    OnceLock::new();
static DETACHED_IDENTITY_WORKERS: OnceLock<Mutex<BTreeMap<u64, Arc<AtomicBool>>>> = OnceLock::new();
static NEXT_IDENTITY_WORKER_ID: AtomicU64 = AtomicU64::new(1);
static RUNTIME_ACCEPTING_PROVIDERS: AtomicBool = AtomicBool::new(true);
const MAX_SHUTDOWN_TIMEOUT_MILLIS: u64 = 30_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPrototypeError {
    message: String,
}

impl HostPrototypeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostPrototypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HostPrototypeError {}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct HostSourceSegment {
    pub source_id: String,
    pub offset: u64,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct HostBatchRequest {
    pub schema: String,
    pub items: Vec<HostSourceSegment>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct DetachedBlob {
    path: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct DetachedBatchEnvelope {
    schema: String,
    transport: String,
    items: Vec<HostSourceSegment>,
    blob: DetachedBlob,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    fn version(&self) -> Result<String, HostPrototypeError>;

    fn initialize(&self) -> Result<(), HostPrototypeError>;

    fn shutdown(&self) -> Result<(), HostPrototypeError>;
}

#[async_trait::async_trait]
pub trait WorkflowHost: Plugin {
    fn descriptor(&self) -> Result<String, HostPrototypeError>;

    fn execute_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;

    fn execute_cancellable_batch(
        &self,
        task_id: u64,
        request: Vec<u8>,
    ) -> Result<Vec<u8>, HostPrototypeError>;

    async fn execute_async_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;

    fn execute_typed_batch(
        &self,
        request: HostBatchRequest,
        source: Vec<u8>,
    ) -> Result<Vec<u8>, HostPrototypeError>;

    fn execute_detached_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;
}

pub trait ParserHost: Plugin {
    fn descriptor(&self) -> Result<String, HostPrototypeError>;

    fn probe_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;

    fn parse_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;
}

struct TreeHaverLanguagePackParserHost {
    name: String,
    language: String,
}

impl TreeHaverLanguagePackParserHost {
    fn parse(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
        let source = String::from_utf8(request.clone()).map_err(|error| {
            HostPrototypeError::new(format!(
                "tree-haver TSLP provider {} requires UTF-8 source: {error}",
                self.name
            ))
        })?;
        let result = parse_with_language_pack(&ParserRequest {
            source,
            language: self.language.clone(),
            dialect: None,
        });
        if result.ok {
            return Ok(request);
        }

        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("; ");
        Err(HostPrototypeError::new(format!(
            "tree-haver TSLP parse failed for {}: {diagnostics}",
            self.language
        )))
    }
}

impl Plugin for TreeHaverLanguagePackParserHost {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> Result<String, HostPrototypeError> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }

    fn initialize(&self) -> Result<(), HostPrototypeError> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), HostPrototypeError> {
        Ok(())
    }
}

impl ParserHost for TreeHaverLanguagePackParserHost {
    fn descriptor(&self) -> Result<String, HostPrototypeError> {
        serde_json::to_string(&serde_json::json!({
            "id": self.name,
            "implementation": "tree-haver",
            "backend": "kreuzberg-language-pack",
            "language": self.language,
            "capabilities": ["probe", "parse", "source-bytes"],
        }))
        .map_err(|error| {
            HostPrototypeError::new(format!("failed to serialize descriptor: {error}"))
        })
    }

    fn probe_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
        self.parse(request)
    }

    fn parse_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
        self.parse(request)
    }
}

struct ProviderEntry<P: Plugin + ?Sized> {
    provider: Arc<P>,
    version: String,
    descriptor: Value,
    finalized: AtomicBool,
}

impl<P: Plugin + ?Sized> ProviderEntry<P> {
    fn new(provider: Arc<P>, version: String, descriptor: Value) -> Self {
        Self { provider, version, descriptor, finalized: AtomicBool::new(false) }
    }

    fn finalize(&self) -> Result<(), HostPrototypeError> {
        if self.finalized.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        self.provider.shutdown()
    }
}

impl<P: Plugin + ?Sized> Drop for ProviderEntry<P> {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}

fn finalize_if_unleased<P: Plugin + ?Sized>(
    entry: Arc<ProviderEntry<P>>,
) -> Result<(), HostPrototypeError> {
    if Arc::strong_count(&entry) == 1 { entry.finalize() } else { Ok(()) }
}

fn replace_provider_if_same<P: Plugin + ?Sized>(
    providers: &mut BTreeMap<String, Arc<ProviderEntry<P>>>,
    expected: &Arc<ProviderEntry<P>>,
    provider: Arc<P>,
    version: String,
    descriptor: Value,
) -> Result<Arc<ProviderEntry<P>>, HostPrototypeError> {
    let name = provider.name().to_owned();
    let current = providers
        .get(&name)
        .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))?;
    if !Arc::ptr_eq(current, expected) {
        return Err(HostPrototypeError::new(format!(
            "provider changed during replacement: {name}"
        )));
    }
    Ok(providers
        .insert(name, Arc::new(ProviderEntry::new(provider, version, descriptor)))
        .expect("existing provider checked above"))
}

#[derive(Default)]
pub struct WorkflowHostRegistry {
    providers: BTreeMap<String, Arc<ProviderEntry<dyn WorkflowHost>>>,
}

#[derive(Default)]
pub struct ParserHostRegistry {
    providers: BTreeMap<String, Arc<ProviderEntry<dyn ParserHost>>>,
}

impl ParserHostRegistry {
    #[cfg(test)]
    fn insert(&mut self, provider: Arc<dyn ParserHost>) -> Result<(), HostPrototypeError> {
        let name = provider.name().to_owned();
        let version = provider.version()?;
        let descriptor = provider.descriptor()?;
        let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
        self.insert_with_metadata(provider, version, descriptor)
    }

    fn insert_with_metadata(
        &mut self,
        provider: Arc<dyn ParserHost>,
        version: String,
        descriptor: Value,
    ) -> Result<(), HostPrototypeError> {
        let name = provider.name().to_owned();
        if name.is_empty() {
            return Err(HostPrototypeError::new("provider name cannot be empty"));
        }
        if self.providers.contains_key(&name) {
            return Err(HostPrototypeError::new(format!("provider already registered: {name}")));
        }
        self.providers.insert(name, Arc::new(ProviderEntry::new(provider, version, descriptor)));
        Ok(())
    }

    fn get(&self, name: &str) -> Result<Arc<ProviderEntry<dyn ParserHost>>, HostPrototypeError> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn replace_if_same(
        &mut self,
        expected: &Arc<ProviderEntry<dyn ParserHost>>,
        provider: Arc<dyn ParserHost>,
        version: String,
        descriptor: Value,
    ) -> Result<Arc<ProviderEntry<dyn ParserHost>>, HostPrototypeError> {
        replace_provider_if_same(&mut self.providers, expected, provider, version, descriptor)
    }

    fn remove(
        &mut self,
        name: &str,
    ) -> Result<Arc<ProviderEntry<dyn ParserHost>>, HostPrototypeError> {
        self.providers
            .remove(name)
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn drain(&mut self) -> Vec<Arc<ProviderEntry<dyn ParserHost>>> {
        std::mem::take(&mut self.providers).into_values().collect()
    }

    fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    fn entries(&self) -> Vec<Arc<ProviderEntry<dyn ParserHost>>> {
        self.providers.values().cloned().collect()
    }

    fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

impl WorkflowHostRegistry {
    #[cfg(test)]
    fn insert(&mut self, provider: Arc<dyn WorkflowHost>) -> Result<(), HostPrototypeError> {
        let name = provider.name().to_owned();
        let version = provider.version()?;
        let descriptor = provider.descriptor()?;
        let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
        self.insert_with_metadata(provider, version, descriptor)
    }

    fn insert_with_metadata(
        &mut self,
        provider: Arc<dyn WorkflowHost>,
        version: String,
        descriptor: Value,
    ) -> Result<(), HostPrototypeError> {
        let name = provider.name().to_owned();
        if name.is_empty() {
            return Err(HostPrototypeError::new("provider name cannot be empty"));
        }
        if self.providers.contains_key(&name) {
            return Err(HostPrototypeError::new(format!("provider already registered: {name}")));
        }
        self.providers.insert(name, Arc::new(ProviderEntry::new(provider, version, descriptor)));
        Ok(())
    }

    fn get(&self, name: &str) -> Result<Arc<ProviderEntry<dyn WorkflowHost>>, HostPrototypeError> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn replace_if_same(
        &mut self,
        expected: &Arc<ProviderEntry<dyn WorkflowHost>>,
        provider: Arc<dyn WorkflowHost>,
        version: String,
        descriptor: Value,
    ) -> Result<Arc<ProviderEntry<dyn WorkflowHost>>, HostPrototypeError> {
        replace_provider_if_same(&mut self.providers, expected, provider, version, descriptor)
    }

    fn remove(
        &mut self,
        name: &str,
    ) -> Result<Arc<ProviderEntry<dyn WorkflowHost>>, HostPrototypeError> {
        self.providers
            .remove(name)
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn drain(&mut self) -> Vec<Arc<ProviderEntry<dyn WorkflowHost>>> {
        std::mem::take(&mut self.providers).into_values().collect()
    }

    fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    fn entries(&self) -> Vec<Arc<ProviderEntry<dyn WorkflowHost>>> {
        self.providers.values().cloned().collect()
    }

    fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

fn validate_provider_name(name: &str) -> Result<(), HostPrototypeError> {
    if name.is_empty() {
        return Err(HostPrototypeError::new("provider name cannot be empty"));
    }
    if name.len() > MAX_PROVIDER_NAME_BYTES {
        return Err(HostPrototypeError::new(format!(
            "provider name exceeds {MAX_PROVIDER_NAME_BYTES} bytes"
        )));
    }
    Ok(())
}

fn ensure_runtime_accepting_providers() -> Result<(), HostPrototypeError> {
    if RUNTIME_ACCEPTING_PROVIDERS.load(Ordering::Acquire) {
        Ok(())
    } else {
        Err(HostPrototypeError::new("host runtime is shutting down"))
    }
}

fn validate_provider_metadata(
    name: &str,
    version: &str,
    descriptor: &str,
) -> Result<Value, HostPrototypeError> {
    if version.is_empty() {
        return Err(HostPrototypeError::new("provider version cannot be empty"));
    }
    if version.len() > MAX_PROVIDER_VERSION_BYTES {
        return Err(HostPrototypeError::new(format!(
            "provider version exceeds {MAX_PROVIDER_VERSION_BYTES} bytes"
        )));
    }
    if descriptor.len() > MAX_DESCRIPTOR_BYTES {
        return Err(HostPrototypeError::new(format!(
            "provider descriptor exceeds {MAX_DESCRIPTOR_BYTES} bytes"
        )));
    }

    let parsed: Value = serde_json::from_str(descriptor).map_err(|error| {
        HostPrototypeError::new(format!("invalid provider descriptor JSON: {error}"))
    })?;
    let object = parsed
        .as_object()
        .ok_or_else(|| HostPrototypeError::new("provider descriptor must be a JSON object"))?;
    let descriptor_id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| HostPrototypeError::new("provider descriptor requires a string id"))?;
    if descriptor_id != name {
        return Err(HostPrototypeError::new(format!(
            "provider descriptor id {descriptor_id:?} does not match registration name {name:?}"
        )));
    }

    if let Some(capabilities) = object.get("capabilities") {
        let capabilities = capabilities.as_array().ok_or_else(|| {
            HostPrototypeError::new("provider descriptor capabilities must be an array")
        })?;
        if capabilities.len() > MAX_CAPABILITIES {
            return Err(HostPrototypeError::new(format!(
                "provider descriptor exceeds {MAX_CAPABILITIES} capabilities"
            )));
        }
        if capabilities.iter().any(|capability| capability.as_str().is_none()) {
            return Err(HostPrototypeError::new(
                "provider descriptor capabilities must contain only strings",
            ));
        }
    }

    Ok(parsed)
}

fn initialization_failure<P: Plugin + ?Sized>(
    provider: &P,
    initialize_error: HostPrototypeError,
) -> HostPrototypeError {
    match provider.shutdown() {
        Ok(()) => initialize_error,
        Err(shutdown_error) => HostPrototypeError::new(format!(
            "{initialize_error}; cleanup shutdown failed: {shutdown_error}"
        )),
    }
}

fn publication_failure<P: Plugin + ?Sized>(
    provider: &P,
    publish_error: HostPrototypeError,
) -> HostPrototypeError {
    match provider.shutdown() {
        Ok(()) => publish_error,
        Err(shutdown_error) => HostPrototypeError::new(format!(
            "{publish_error}; cleanup shutdown failed: {shutdown_error}"
        )),
    }
}

pub mod registry {
    use super::{OnceLock, ParserHostRegistry, RwLock, WorkflowHostRegistry};

    static WORKFLOW_HOSTS: OnceLock<RwLock<WorkflowHostRegistry>> = OnceLock::new();
    static PARSER_HOSTS: OnceLock<RwLock<ParserHostRegistry>> = OnceLock::new();

    pub fn get_workflow_host_registry() -> &'static RwLock<WorkflowHostRegistry> {
        WORKFLOW_HOSTS.get_or_init(|| RwLock::new(WorkflowHostRegistry::default()))
    }

    pub fn get_parser_host_registry() -> &'static RwLock<ParserHostRegistry> {
        PARSER_HOSTS.get_or_init(|| RwLock::new(ParserHostRegistry::default()))
    }
}

pub mod workflow_host {
    use super::{HostPrototypeError, finalize_if_unleased, registry};

    pub fn unregister_workflow_host(name: &str) -> Result<(), HostPrototypeError> {
        let provider = registry::get_workflow_host_registry().write().remove(name)?;
        finalize_if_unleased(provider)
    }

    pub fn clear_workflow_hosts() -> Result<(), HostPrototypeError> {
        let providers = registry::get_workflow_host_registry().write().drain();
        let mut failures = Vec::new();
        for provider in providers {
            let name = provider.provider.name().to_owned();
            if let Err(error) = finalize_if_unleased(provider) {
                failures.push(format!("{name}: {error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(HostPrototypeError::new(format!(
                "provider shutdown failed: {}",
                failures.join(", ")
            )))
        }
    }
}

pub mod parser_host {
    use super::{HostPrototypeError, finalize_if_unleased, registry};

    pub fn unregister_parser_host(name: &str) -> Result<(), HostPrototypeError> {
        let provider = registry::get_parser_host_registry().write().remove(name)?;
        finalize_if_unleased(provider)
    }

    pub fn clear_parser_hosts() -> Result<(), HostPrototypeError> {
        let providers = registry::get_parser_host_registry().write().drain();
        let mut failures = Vec::new();
        for provider in providers {
            let name = provider.provider.name().to_owned();
            if let Err(error) = finalize_if_unleased(provider) {
                failures.push(format!("{name}: {error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(HostPrototypeError::new(format!(
                "provider shutdown failed: {}",
                failures.join(", ")
            )))
        }
    }
}

pub fn register_workflow_host(provider: Arc<dyn WorkflowHost>) -> Result<(), HostPrototypeError> {
    ensure_runtime_accepting_providers()?;
    let name = provider.name().to_owned();
    validate_provider_name(&name)?;
    if registry::get_workflow_host_registry().read().contains(&name) {
        return Err(HostPrototypeError::new(format!("provider already registered: {name}")));
    }

    let version = provider.version()?;
    let descriptor = provider.descriptor()?;
    let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
    if let Err(error) = provider.initialize() {
        return Err(initialization_failure(provider.as_ref(), error));
    }

    let publication = {
        let mut registry = registry::get_workflow_host_registry().write();
        ensure_runtime_accepting_providers().and_then(|()| {
            registry.insert_with_metadata(Arc::clone(&provider), version, descriptor)
        })
    };
    publication.map_err(|error| publication_failure(provider.as_ref(), error))
}

pub fn replace_workflow_host(
    provider: Arc<dyn WorkflowHost>,
    name: String,
) -> Result<(), HostPrototypeError> {
    ensure_runtime_accepting_providers()?;
    validate_provider_name(&name)?;
    if provider.name() != name {
        return Err(HostPrototypeError::new(format!(
            "replacement provider name {:?} does not match requested name {name:?}",
            provider.name()
        )));
    }
    let expected = registry::get_workflow_host_registry().read().get(&name)?;
    let version = provider.version()?;
    let descriptor = provider.descriptor()?;
    let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
    if let Err(error) = provider.initialize() {
        return Err(initialization_failure(provider.as_ref(), error));
    }

    let replaced = {
        let mut registry = registry::get_workflow_host_registry().write();
        ensure_runtime_accepting_providers().and_then(|()| {
            registry.replace_if_same(&expected, Arc::clone(&provider), version, descriptor)
        })
    };
    let replaced = match replaced {
        Ok(replaced) => replaced,
        Err(error) => return Err(publication_failure(provider.as_ref(), error)),
    };
    drop(expected);
    finalize_if_unleased(replaced).map_err(|error| {
        HostPrototypeError::new(format!(
            "replacement published; prior provider shutdown failed: {error}"
        ))
    })
}

pub fn register_parser_host(provider: Arc<dyn ParserHost>) -> Result<(), HostPrototypeError> {
    ensure_runtime_accepting_providers()?;
    let name = provider.name().to_owned();
    validate_provider_name(&name)?;
    if registry::get_parser_host_registry().read().contains(&name) {
        return Err(HostPrototypeError::new(format!("provider already registered: {name}")));
    }

    let version = provider.version()?;
    let descriptor = provider.descriptor()?;
    let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
    if let Err(error) = provider.initialize() {
        return Err(initialization_failure(provider.as_ref(), error));
    }

    let publication = {
        let mut registry = registry::get_parser_host_registry().write();
        ensure_runtime_accepting_providers().and_then(|()| {
            registry.insert_with_metadata(Arc::clone(&provider), version, descriptor)
        })
    };
    publication.map_err(|error| publication_failure(provider.as_ref(), error))
}

pub fn register_tslp_parser_host(
    provider_name: String,
    language: String,
) -> Result<(), HostPrototypeError> {
    if language.trim().is_empty() {
        return Err(HostPrototypeError::new("parser language cannot be empty"));
    }
    register_parser_host(Arc::new(TreeHaverLanguagePackParserHost {
        name: provider_name,
        language,
    }))
}

pub fn replace_parser_host(
    provider: Arc<dyn ParserHost>,
    name: String,
) -> Result<(), HostPrototypeError> {
    ensure_runtime_accepting_providers()?;
    validate_provider_name(&name)?;
    if provider.name() != name {
        return Err(HostPrototypeError::new(format!(
            "replacement provider name {:?} does not match requested name {name:?}",
            provider.name()
        )));
    }
    let expected = registry::get_parser_host_registry().read().get(&name)?;
    let version = provider.version()?;
    let descriptor = provider.descriptor()?;
    let descriptor = validate_provider_metadata(&name, &version, &descriptor)?;
    if let Err(error) = provider.initialize() {
        return Err(initialization_failure(provider.as_ref(), error));
    }

    let replaced = {
        let mut registry = registry::get_parser_host_registry().write();
        ensure_runtime_accepting_providers().and_then(|()| {
            registry.replace_if_same(&expected, Arc::clone(&provider), version, descriptor)
        })
    };
    let replaced = match replaced {
        Ok(replaced) => replaced,
        Err(error) => return Err(publication_failure(provider.as_ref(), error)),
    };
    drop(expected);
    finalize_if_unleased(replaced).map_err(|error| {
        HostPrototypeError::new(format!(
            "replacement published; prior provider shutdown failed: {error}"
        ))
    })
}

pub mod plugins {
    pub use crate::parser_host::{clear_parser_hosts, unregister_parser_host};
    pub use crate::workflow_host::{clear_workflow_hosts, unregister_workflow_host};
    pub use crate::{register_parser_host, register_workflow_host};
}

pub fn execute_identity(
    provider_name: String,
    request: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    provider.provider.execute_batch(request)
}

pub async fn execute_async_identity(
    provider_name: String,
    request: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    provider.provider.execute_async_batch(request).await
}

pub fn start_identity_worker(
    provider_name: String,
    request: Vec<u8>,
) -> Result<u64, HostPrototypeError> {
    let task_id = prepare_identity_worker(provider_name, request)?;
    dispatch_identity_worker(task_id)?;
    Ok(task_id)
}

pub fn prepare_identity_worker(
    provider_name: String,
    request: Vec<u8>,
) -> Result<u64, HostPrototypeError> {
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    let task_id = NEXT_IDENTITY_WORKER_ID.fetch_add(1, Ordering::Relaxed);
    if task_id == 0 {
        return Err(HostPrototypeError::new("identity worker task ID space exhausted"));
    }

    PREPARED_IDENTITY_WORKERS.get_or_init(Default::default).lock().insert(
        task_id,
        PreparedIdentityWorker { provider, request, cancelled: Arc::new(AtomicBool::new(false)) },
    );
    Ok(task_id)
}

fn execute_identity_worker(
    task_id: u64,
    provider: Arc<ProviderEntry<dyn WorkflowHost>>,
    request: Vec<u8>,
    cancelled: &AtomicBool,
) -> IdentityWorkerResult {
    if cancelled.load(Ordering::Acquire) {
        return Err(HostPrototypeError::new(format!(
            "identity worker cancelled before invocation: {task_id}"
        )));
    }

    let result = provider.provider.execute_cancellable_batch(task_id, request);
    if cancelled.load(Ordering::Acquire) {
        Err(HostPrototypeError::new(format!(
            "identity worker cancelled after invocation: {task_id}"
        )))
    } else {
        result
    }
}

pub fn dispatch_identity_worker(task_id: u64) -> Result<(), HostPrototypeError> {
    let prepared = PREPARED_IDENTITY_WORKERS
        .get_or_init(Default::default)
        .lock()
        .remove(&task_id)
        .ok_or_else(|| {
            HostPrototypeError::new(format!("prepared identity worker not found: {task_id}"))
        })?;

    let (sender, receiver) = mpsc::sync_channel(1);
    let worker_cancelled = Arc::clone(&prepared.cancelled);
    IDENTITY_WORKERS.get_or_init(Default::default).lock().insert(
        task_id,
        IdentityWorkerTask { receiver, cancelled: Arc::clone(&prepared.cancelled) },
    );

    if prepared.cancelled.load(Ordering::Acquire) {
        let _ = sender.send(Err(HostPrototypeError::new(format!(
            "identity worker cancelled before enqueue: {task_id}"
        ))));
        return Ok(());
    }

    let spawn_result = std::thread::Builder::new()
        .name(format!("structuredmerge-host-{task_id}"))
        .spawn(move || {
            let result = execute_identity_worker(
                task_id,
                prepared.provider,
                prepared.request,
                worker_cancelled.as_ref(),
            );
            let _ = sender.send(result);
            DETACHED_IDENTITY_WORKERS.get_or_init(Default::default).lock().remove(&task_id);
        });
    if let Err(error) = spawn_result {
        IDENTITY_WORKERS.get_or_init(Default::default).lock().remove(&task_id);
        return Err(HostPrototypeError::new(format!("failed to start identity worker: {error}")));
    }
    Ok(())
}

pub fn cancel_identity_worker(task_id: u64) -> Result<(), HostPrototypeError> {
    if let Some(task) = PREPARED_IDENTITY_WORKERS.get_or_init(Default::default).lock().get(&task_id)
    {
        task.cancelled.store(true, Ordering::Release);
        return Ok(());
    }
    if let Some(task) = IDENTITY_WORKERS.get_or_init(Default::default).lock().get(&task_id) {
        task.cancelled.store(true, Ordering::Release);
        return Ok(());
    }
    Err(HostPrototypeError::new(format!("identity worker not found: {task_id}")))
}

pub fn identity_worker_cancelled(task_id: u64) -> Result<bool, HostPrototypeError> {
    if let Some(task) = PREPARED_IDENTITY_WORKERS.get_or_init(Default::default).lock().get(&task_id)
    {
        return Ok(task.cancelled.load(Ordering::Acquire));
    }
    if let Some(task) = IDENTITY_WORKERS.get_or_init(Default::default).lock().get(&task_id) {
        return Ok(task.cancelled.load(Ordering::Acquire));
    }
    if let Some(cancelled) =
        DETACHED_IDENTITY_WORKERS.get_or_init(Default::default).lock().get(&task_id)
    {
        return Ok(cancelled.load(Ordering::Acquire));
    }
    Err(HostPrototypeError::new(format!("identity worker not found: {task_id}")))
}

pub fn poll_identity_worker(task_id: u64) -> Result<Option<Vec<u8>>, HostPrototypeError> {
    let mut workers = IDENTITY_WORKERS.get_or_init(Default::default).lock();
    let outcome = workers
        .get(&task_id)
        .ok_or_else(|| HostPrototypeError::new(format!("identity worker not found: {task_id}")))?
        .receiver
        .try_recv();

    match outcome {
        Ok(result) => {
            workers.remove(&task_id);
            result.map(Some)
        }
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => {
            workers.remove(&task_id);
            Err(HostPrototypeError::new(format!(
                "identity worker disconnected without a result: {task_id}"
            )))
        }
    }
}

fn cancel_prepared_identity_workers() {
    let prepared =
        std::mem::take(&mut *PREPARED_IDENTITY_WORKERS.get_or_init(Default::default).lock());
    for task in prepared.values() {
        task.cancelled.store(true, Ordering::Release);
    }
    drop(prepared);
}

fn cancel_active_identity_workers() {
    for task in IDENTITY_WORKERS.get_or_init(Default::default).lock().values() {
        task.cancelled.store(true, Ordering::Release);
    }
}

fn reap_finished_identity_workers() -> bool {
    let mut workers = IDENTITY_WORKERS.get_or_init(Default::default).lock();
    let finished: Vec<u64> = workers
        .iter()
        .filter_map(|(task_id, task)| match task.receiver.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => Some(*task_id),
            Err(TryRecvError::Empty) => None,
        })
        .collect();
    for task_id in finished {
        workers.remove(&task_id);
    }
    workers.is_empty()
}

fn force_detach_identity_workers() -> Vec<u64> {
    let mut workers = IDENTITY_WORKERS.get_or_init(Default::default).lock();
    let mut detached = DETACHED_IDENTITY_WORKERS.get_or_init(Default::default).lock();
    let task_ids: Vec<u64> = workers.keys().copied().collect();
    for (task_id, task) in workers.iter() {
        task.cancelled.store(true, Ordering::Release);
        detached.insert(*task_id, Arc::clone(&task.cancelled));
    }
    workers.clear();
    task_ids
}

pub fn start_host_runtime() -> Result<(), HostPrototypeError> {
    let has_workflow_hosts = !registry::get_workflow_host_registry().read().names().is_empty();
    let has_parser_hosts = !registry::get_parser_host_registry().read().names().is_empty();
    let has_prepared = !PREPARED_IDENTITY_WORKERS.get_or_init(Default::default).lock().is_empty();
    let has_active = !IDENTITY_WORKERS.get_or_init(Default::default).lock().is_empty();
    let has_detached = !DETACHED_IDENTITY_WORKERS.get_or_init(Default::default).lock().is_empty();
    if has_workflow_hosts || has_parser_hosts || has_prepared || has_active || has_detached {
        return Err(HostPrototypeError::new(
            "host runtime cannot start while providers or tasks remain",
        ));
    }
    RUNTIME_ACCEPTING_PROVIDERS.store(true, Ordering::Release);
    Ok(())
}

pub async fn shutdown_host_runtime(
    timeout_millis: u64,
    force: bool,
) -> Result<Vec<u64>, HostPrototypeError> {
    if timeout_millis > MAX_SHUTDOWN_TIMEOUT_MILLIS {
        return Err(HostPrototypeError::new(format!(
            "shutdown timeout exceeds {MAX_SHUTDOWN_TIMEOUT_MILLIS} milliseconds"
        )));
    }
    RUNTIME_ACCEPTING_PROVIDERS.store(false, Ordering::Release);

    let mut failures = Vec::new();
    if let Err(error) = workflow_host::clear_workflow_hosts() {
        failures.push(error.to_string());
    }
    if let Err(error) = parser_host::clear_parser_hosts() {
        failures.push(error.to_string());
    }
    cancel_prepared_identity_workers();
    cancel_active_identity_workers();

    let deadline = Instant::now() + Duration::from_millis(timeout_millis);
    let detached = loop {
        if reap_finished_identity_workers() {
            break Vec::new();
        }
        if Instant::now() >= deadline {
            if force {
                break force_detach_identity_workers();
            }
            let task_ids: Vec<u64> =
                IDENTITY_WORKERS.get_or_init(Default::default).lock().keys().copied().collect();
            failures.push(format!("shutdown timed out with active tasks: {task_ids:?}"));
            break Vec::new();
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    };

    if failures.is_empty() {
        Ok(detached)
    } else {
        Err(HostPrototypeError::new(failures.join("; ")))
    }
}

fn validated_batch_request(
    source_ids: Vec<String>,
    source_lengths: Vec<u64>,
    source_digests: Vec<String>,
    source: &[u8],
) -> Result<HostBatchRequest, HostPrototypeError> {
    let item_count = source_ids.len();
    if item_count == 0 || item_count > MAX_BATCH_ITEMS {
        return Err(HostPrototypeError::new(format!(
            "typed batch item count must be between 1 and {MAX_BATCH_ITEMS}"
        )));
    }
    if source_lengths.len() != item_count || source_digests.len() != item_count {
        return Err(HostPrototypeError::new(
            "typed batch source ids, lengths, and digests must have equal lengths",
        ));
    }

    let mut offset = 0usize;
    let mut items = Vec::with_capacity(item_count);
    for ((source_id, byte_length), expected_digest) in
        source_ids.into_iter().zip(source_lengths).zip(source_digests)
    {
        let byte_length = usize::try_from(byte_length).map_err(|_| {
            HostPrototypeError::new("typed batch source length exceeds this platform")
        })?;
        let end = offset
            .checked_add(byte_length)
            .ok_or_else(|| HostPrototypeError::new("typed batch source range overflow"))?;
        let bytes = source.get(offset..end).ok_or_else(|| {
            HostPrototypeError::new("typed batch source range exceeds source bytes")
        })?;
        let actual_digest = format!("{:x}", Sha256::digest(bytes));
        if actual_digest != expected_digest {
            return Err(HostPrototypeError::new(format!(
                "typed batch digest mismatch for source {source_id:?}"
            )));
        }
        items.push(HostSourceSegment {
            source_id,
            offset: u64::try_from(offset)
                .map_err(|_| HostPrototypeError::new("typed batch source offset exceeds u64"))?,
            byte_length: u64::try_from(byte_length)
                .map_err(|_| HostPrototypeError::new("typed batch source length exceeds u64"))?,
            sha256: expected_digest,
        });
        offset = end;
    }
    if offset != source.len() {
        return Err(HostPrototypeError::new(
            "typed batch source lengths do not consume all source bytes",
        ));
    }

    Ok(HostBatchRequest { schema: "structuredmerge.host-batch/v1".to_owned(), items })
}

pub fn execute_typed_identity(
    provider_name: String,
    source_ids: Vec<String>,
    source_lengths: Vec<u64>,
    source_digests: Vec<String>,
    source: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let request = validated_batch_request(source_ids, source_lengths, source_digests, &source)?;
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    let result = provider.provider.execute_typed_batch(request, source.clone())?;
    if result != source {
        return Err(HostPrototypeError::new("typed identity host changed source bytes"));
    }
    Ok(result)
}

pub fn execute_typed_workflow(
    provider_name: String,
    source_ids: Vec<String>,
    source_lengths: Vec<u64>,
    source_digests: Vec<String>,
    source: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let request = validated_batch_request(source_ids, source_lengths, source_digests, &source)?;
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    provider.provider.execute_typed_batch(request, source)
}

pub fn execute_in_process_identity(
    source_ids: Vec<String>,
    source_lengths: Vec<u64>,
    source_digests: Vec<String>,
    source: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    validated_batch_request(source_ids, source_lengths, source_digests, &source)?;
    Ok(source)
}

pub fn execute_detached_identity(
    provider_name: String,
    source_ids: Vec<String>,
    source_lengths: Vec<u64>,
    source_digests: Vec<String>,
    source: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let request = validated_batch_request(source_ids, source_lengths, source_digests, &source)?;
    let mut detached = tempfile::NamedTempFile::new().map_err(|error| {
        HostPrototypeError::new(format!("failed to create detached blob: {error}"))
    })?;
    detached.write_all(&source).and_then(|()| detached.flush()).map_err(|error| {
        HostPrototypeError::new(format!("failed to write detached blob: {error}"))
    })?;
    let detached_path = detached.path().to_string_lossy().into_owned();
    let envelope = DetachedBatchEnvelope {
        schema: "structuredmerge.host-batch/v1".to_owned(),
        transport: "detached_local_file".to_owned(),
        items: request.items,
        blob: DetachedBlob {
            path: detached_path.clone(),
            byte_length: u64::try_from(source.len())
                .map_err(|_| HostPrototypeError::new("detached blob length exceeds u64"))?,
            sha256: format!("{:x}", Sha256::digest(&source)),
        },
    };
    let envelope_bytes = serde_json::to_vec(&envelope).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize detached envelope: {error}"))
    })?;

    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    let result_bytes = provider.provider.execute_detached_batch(envelope_bytes.clone())?;
    let result: DetachedBatchEnvelope = serde_json::from_slice(&result_bytes).map_err(|error| {
        HostPrototypeError::new(format!("invalid detached result envelope: {error}"))
    })?;
    if result_bytes != envelope_bytes
        || result.schema != envelope.schema
        || result.transport != envelope.transport
        || result.blob.path != detached_path
        || result.blob.byte_length != envelope.blob.byte_length
        || result.blob.sha256 != envelope.blob.sha256
    {
        return Err(HostPrototypeError::new("detached identity host changed the envelope"));
    }

    let result_source = std::fs::read(detached.path()).map_err(|error| {
        HostPrototypeError::new(format!("failed to read detached result blob: {error}"))
    })?;
    if result_source.len() != source.len()
        || format!("{:x}", Sha256::digest(&result_source)) != envelope.blob.sha256
        || result_source != source
    {
        return Err(HostPrototypeError::new("detached identity host changed source bytes"));
    }
    Ok(result_source)
}

pub fn registered_workflow_hosts() -> Vec<String> {
    registry::get_workflow_host_registry().read().names()
}

pub fn probe_with_parser(
    provider_name: String,
    request: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let provider = registry::get_parser_host_registry().read().get(&provider_name)?;
    provider.provider.probe_batch(request)
}

pub fn parse_with_parser(
    provider_name: String,
    request: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let provider = registry::get_parser_host_registry().read().get(&provider_name)?;
    provider.provider.parse_batch(request)
}

pub fn parse_normalized_with_tslp(
    language: String,
    source: String,
    dialect: Option<String>,
) -> Result<String, HostPrototypeError> {
    if language.trim().is_empty() {
        return Err(HostPrototypeError::new("parser language cannot be empty"));
    }

    serde_json::to_string(&parse_normalized_with_language_pack(&ParserRequest {
        source,
        language,
        dialect,
    }))
    .map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize normalized parse: {error}"))
    })
}

pub fn registered_parser_hosts() -> Vec<String> {
    registry::get_parser_host_registry().read().names()
}

fn registered_provider_manifest<P: Plugin + ?Sized>(entry: &ProviderEntry<P>) -> Value {
    serde_json::json!({
        "id": entry.provider.name(),
        "version": entry.version,
        "descriptor": entry.descriptor,
    })
}

pub fn capability_manifest() -> Result<String, HostPrototypeError> {
    let parser_entries = registry::get_parser_host_registry().read().entries();
    let workflow_entries = registry::get_workflow_host_registry().read().entries();
    let parser_hosts = parser_entries
        .iter()
        .map(|entry| registered_provider_manifest(entry.as_ref()))
        .collect::<Vec<_>>();
    let workflow_hosts = workflow_entries
        .iter()
        .map(|entry| registered_provider_manifest(entry.as_ref()))
        .collect::<Vec<_>>();
    let manifest = serde_json::json!({
        "schema": "structuredmerge.capability-manifest/v1",
        "bundle": {
            "id": "structuredmerge-host-prototype",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "selection_policy": {
            "mode": "explicit",
            "registered_provider_id": "exact_match",
            "unavailable_provider": "error",
            "implicit_provider_fallback": false,
            "registration_order_affects_selection": false,
        },
        "operations": [
            {
                "id": "rust.json.tslp.merge2",
                "operation": "merge2",
                "family": "json",
                "dialects": ["json", "jsonc", "json5"],
                "merge_provider": "json-merge",
                "parser_provider": "rust.tslp",
                "tree_haver_backend": "kreuzberg-language-pack",
                "support_status": "production",
                "required_extension_schemas": [],
            },
            {
                "id": "rust.json.tslp.merge3",
                "operation": "merge3",
                "family": "json",
                "dialects": ["json", "jsonc", "json5"],
                "merge_provider": "json-merge",
                "parser_provider": "rust.tslp",
                "tree_haver_backend": "kreuzberg-language-pack",
                "support_status": "production",
                "required_extension_schemas": [],
            },
        ],
        "parser_provider_factories": [
            {
                "id": "rust.tslp",
                "operations": ["probe", "parse"],
                "languages": "on_demand",
                "tree_haver_backend": "kreuzberg-language-pack",
                "support_status": "host_runtime_only",
                "constraints": ["UTF-8 source"],
            },
        ],
        "registered_providers": {
            "parser": parser_hosts,
            "workflow": workflow_hosts,
        },
    });
    serde_json::to_string(&manifest).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize capability manifest: {error}"))
    })
}

pub fn merge_json_two_way(
    incoming_source: String,
    current_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let result =
        merge_json_source_preserving(&incoming_source, &current_source, json_dialect(&dialect)?);
    serialize_merge_result(&result)
}

pub fn merge_json_three_way(
    base_source: String,
    ours_source: String,
    theirs_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let result = merge_json_three_way_source_preserving(
        &base_source,
        &ours_source,
        &theirs_source,
        json_dialect(&dialect)?,
    );
    serialize_merge_result(&result)
}

pub fn merge_ast_merge_git_json(
    base_source: String,
    ours_source: String,
    theirs_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let request = GitMerge3Request {
        base_source,
        ours_source,
        theirs_source,
        path_name: Some(format!("merge-driver.{dialect}")),
        language: Some(dialect.clone()),
        dialect: Some(dialect),
        profile_id: Some("source_preserving".to_string()),
        fallback_policy: Some("none".to_string()),
        conflict_marker_size: Some(7),
        render_policy: Some("source_preserving_edits".to_string()),
    };
    serde_json::to_string(&merge_git_three_way(&request)).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize ast-merge-git merge: {error}"))
    })
}

pub fn parse_json_analysis(source: String, dialect: String) -> Result<String, HostPrototypeError> {
    serde_json::to_string(&parse_json(&source, json_dialect(&dialect)?)).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize JSON analysis: {error}"))
    })
}

pub fn parse_go_analysis(source: String, dialect: String) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("go") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Go dialect {dialect:?}; expected go"
        )));
    }
    serde_json::to_string(&parse_go(&source, GoDialect::Go)).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize Go analysis: {error}"))
    })
}

pub fn merge_go_three_way(
    base_source: String,
    ours_source: String,
    theirs_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("go") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Go dialect {dialect:?}; expected go"
        )));
    }
    serde_json::to_string(&merge_go_three_way_impl(
        &base_source,
        &ours_source,
        &theirs_source,
        GoDialect::Go,
    ))
    .map_err(|error| HostPrototypeError::new(format!("failed to serialize Go merge: {error}")))
}

pub fn merge_go_two_way(
    template_source: String,
    destination_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("go") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Go dialect {dialect:?}; expected go"
        )));
    }
    serde_json::to_string(&merge_go(&template_source, &destination_source, GoDialect::Go))
        .map_err(|error| HostPrototypeError::new(format!("failed to serialize Go merge: {error}")))
}

pub fn parse_rust_analysis(source: String, dialect: String) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("rust") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Rust dialect {dialect:?}; expected rust"
        )));
    }
    serde_json::to_string(&parse_rust(&source, RustDialect::Rust)).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize Rust analysis: {error}"))
    })
}

pub fn merge_rust_three_way(
    base_source: String,
    ours_source: String,
    theirs_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("rust") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Rust dialect {dialect:?}; expected rust"
        )));
    }
    serde_json::to_string(&merge_rust_three_way_impl(
        &base_source,
        &ours_source,
        &theirs_source,
        RustDialect::Rust,
    ))
    .map_err(|error| HostPrototypeError::new(format!("failed to serialize Rust merge: {error}")))
}

pub fn merge_rust_two_way(
    template_source: String,
    destination_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    if !dialect.trim().eq_ignore_ascii_case("rust") {
        return Err(HostPrototypeError::new(format!(
            "unsupported Rust dialect {dialect:?}; expected rust"
        )));
    }
    serde_json::to_string(&merge_rust(&template_source, &destination_source, RustDialect::Rust))
        .map_err(|error| {
            HostPrototypeError::new(format!("failed to serialize Rust merge: {error}"))
        })
}

pub fn parse_typescript_analysis(
    source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let dialect = typescript_dialect(&dialect)?;
    serde_json::to_string(&parse_typescript(&source, dialect)).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize TypeScript analysis: {error}"))
    })
}

pub fn merge_typescript_three_way(
    base_source: String,
    ours_source: String,
    theirs_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let dialect = typescript_dialect(&dialect)?;
    serde_json::to_string(&merge_typescript_three_way_impl(
        &base_source,
        &ours_source,
        &theirs_source,
        dialect,
    ))
    .map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize TypeScript merge: {error}"))
    })
}

pub fn merge_typescript_two_way(
    template_source: String,
    destination_source: String,
    dialect: String,
) -> Result<String, HostPrototypeError> {
    let dialect = typescript_dialect(&dialect)?;
    serde_json::to_string(&merge_typescript_impl(&template_source, &destination_source, dialect))
        .map_err(|error| {
            HostPrototypeError::new(format!("failed to serialize TypeScript merge: {error}"))
        })
}

fn json_dialect(dialect: &str) -> Result<JsonDialect, HostPrototypeError> {
    match dialect.trim().to_ascii_lowercase().as_str() {
        "json" => Ok(JsonDialect::Json),
        "jsonc" => Ok(JsonDialect::Jsonc),
        "json5" => Ok(JsonDialect::Json5),
        _ => Err(HostPrototypeError::new(format!(
            "unsupported JSON dialect {dialect:?}; expected json, jsonc, or json5"
        ))),
    }
}

fn typescript_dialect(dialect: &str) -> Result<TypeScriptDialect, HostPrototypeError> {
    match dialect.trim().to_ascii_lowercase().as_str() {
        "typescript" | "ts" => Ok(TypeScriptDialect::TypeScript),
        "tsx" => Ok(TypeScriptDialect::Tsx),
        _ => Err(HostPrototypeError::new(format!(
            "unsupported TypeScript dialect {dialect:?}; expected typescript or tsx"
        ))),
    }
}

fn serialize_merge_result<T: serde::Serialize>(result: &T) -> Result<String, HostPrototypeError> {
    serde_json::to_string(result).map_err(|error| {
        HostPrototypeError::new(format!("failed to serialize merge result: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdentityHost {
        name: String,
        typed_response: Option<Vec<u8>>,
    }

    impl Plugin for IdentityHost {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> Result<String, HostPrototypeError> {
            Ok("test".to_owned())
        }

        fn initialize(&self) -> Result<(), HostPrototypeError> {
            Ok(())
        }

        fn shutdown(&self) -> Result<(), HostPrototypeError> {
            Ok(())
        }
    }

    impl ParserHost for IdentityHost {
        fn descriptor(&self) -> Result<String, HostPrototypeError> {
            Ok(format!(r#"{{"id":"{}"}}"#, self.name))
        }

        fn probe_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }

        fn parse_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }
    }

    #[async_trait::async_trait]
    impl WorkflowHost for IdentityHost {
        fn descriptor(&self) -> Result<String, HostPrototypeError> {
            Ok(format!(r#"{{"id":"{}"}}"#, self.name))
        }

        fn execute_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }

        fn execute_cancellable_batch(
            &self,
            _task_id: u64,
            request: Vec<u8>,
        ) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }

        async fn execute_async_batch(
            &self,
            request: Vec<u8>,
        ) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }

        fn execute_typed_batch(
            &self,
            _request: HostBatchRequest,
            source: Vec<u8>,
        ) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(self.typed_response.clone().unwrap_or(source))
        }

        fn execute_detached_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }
    }

    fn identity(name: &str) -> Arc<dyn WorkflowHost> {
        Arc::new(IdentityHost { name: name.to_owned(), typed_response: None })
    }

    fn identity_parser(name: &str) -> Arc<dyn ParserHost> {
        Arc::new(IdentityHost { name: name.to_owned(), typed_response: None })
    }

    #[test]
    fn identity_provider_preserves_arbitrary_bytes() {
        let mut registry = WorkflowHostRegistry::default();
        registry.insert(identity("identity")).unwrap();

        let input = vec![0, 0xff, b'\r', b'\n', b'a', 0];
        let output =
            registry.get("identity").unwrap().provider.execute_batch(input.clone()).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn typed_workflow_returns_provider_result_bytes() {
        let provider_name = "workflow.typed-result";
        let _ = workflow_host::unregister_workflow_host(provider_name);
        let response =
            br#"{"schema":"https://structuredmerge.org/schemas/provider-result/v1.json"}"#.to_vec();
        register_workflow_host(Arc::new(IdentityHost {
            name: provider_name.to_owned(),
            typed_response: Some(response.clone()),
        }))
        .unwrap();
        let source = b"operation-requestyaml: true\n".to_vec();
        let parts = [b"operation-request".as_slice(), b"yaml: true\n".as_slice()];

        let result = execute_typed_workflow(
            provider_name.to_owned(),
            vec!["operation-request".to_owned(), "source:yaml".to_owned()],
            parts.iter().map(|part| part.len() as u64).collect(),
            parts.iter().map(|part| format!("{:x}", Sha256::digest(part))).collect(),
            source,
        )
        .unwrap();

        assert_eq!(result, response);
        workflow_host::unregister_workflow_host(provider_name).unwrap();
    }

    #[test]
    fn duplicate_names_fail_closed() {
        let mut registry = WorkflowHostRegistry::default();
        registry.insert(identity("identity")).unwrap();

        let error = registry.insert(identity("identity")).unwrap_err();

        assert_eq!(error.message(), "provider already registered: identity");
    }

    #[test]
    fn removing_a_provider_excludes_it_from_future_lookups() {
        let mut registry = WorkflowHostRegistry::default();
        registry.insert(identity("identity")).unwrap();

        let provider = registry.remove("identity").unwrap();

        assert_eq!(provider.provider.name(), "identity");
        let error = match registry.get("identity") {
            Ok(_) => panic!("removed provider remained available"),
            Err(error) => error,
        };
        assert_eq!(error.message(), "provider not registered: identity");
    }

    #[test]
    fn parser_provider_preserves_probe_and_parse_bytes() {
        let mut registry = ParserHostRegistry::default();
        registry.insert(identity_parser("identity.parser")).unwrap();
        let payload = vec![0, 0xff, b'\r', b'\n', b'a', 0];

        let provider = registry.get("identity.parser").unwrap();

        assert_eq!(provider.provider.probe_batch(payload.clone()).unwrap(), payload);
        assert_eq!(provider.provider.parse_batch(payload.clone()).unwrap(), payload);
    }

    #[test]
    fn json_two_way_boundary_preserves_current_layout() {
        let response = merge_json_two_way(
            "{\n  \"managed\": true\n}\n".to_owned(),
            "{\r\n  \"managed\": true,\r\n\r\n  \"local\": true\r\n}\r\n".to_owned(),
            "json".to_owned(),
        )
        .unwrap();
        let result: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(result["ok"], true);
        assert_eq!(result["output"], "{\r\n  \"managed\": true,\r\n\r\n  \"local\": true\r\n}\r\n");
    }

    #[test]
    fn json_three_way_boundary_combines_independent_edits() {
        let response = merge_json_three_way(
            r#"{"left":1,"right":1}"#.to_owned(),
            r#"{"left":2,"right":1}"#.to_owned(),
            r#"{"left":1,"right":2}"#.to_owned(),
            "json".to_owned(),
        )
        .unwrap();
        let result: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(result["outcome"], "clean");
        assert_eq!(result["output"], r#"{"left":2,"right":2}"#);
    }

    #[test]
    fn ast_merge_git_boundary_preserves_clean_and_conflict_envelopes() {
        let clean = merge_ast_merge_git_json(
            r#"{"shared":true}"#.to_owned(),
            r#"{"shared":true,"ours":1}"#.to_owned(),
            r#"{"shared":true,"theirs":2}"#.to_owned(),
            "json".to_owned(),
        )
        .unwrap();
        let clean: serde_json::Value = serde_json::from_str(&clean).unwrap();
        assert_eq!(clean["ok"], true);
        assert!(clean["merged_source"].as_str().unwrap().contains("\"ours\":1"));

        let conflict = merge_ast_merge_git_json(
            r#"{"enabled":true}"#.to_owned(),
            r#"{"enabled":false}"#.to_owned(),
            r#"{"enabled":"yes"}"#.to_owned(),
            "json".to_owned(),
        )
        .unwrap();
        let conflict: serde_json::Value = serde_json::from_str(&conflict).unwrap();
        assert_eq!(conflict["ok"], false);
        assert!(conflict["conflicted_source"].as_str().unwrap().contains("<<<<<<< ours"));
        assert_eq!(conflict["conflicts"][0]["path"], "/enabled");
    }

    #[test]
    fn json_analysis_boundary_preserves_owner_contract() {
        let response =
            parse_json_analysis(r#"{"answer":42}"#.to_owned(), "json".to_owned()).unwrap();
        let result: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(result["ok"], true);
        assert_eq!(result["analysis"]["root_kind"], "object");
        assert_eq!(result["analysis"]["owners"][0]["path"], "/answer");
        assert_eq!(result["analysis"]["owners"][0]["owner_kind"], "member");
    }

    #[test]
    fn json_boundary_rejects_unknown_dialects() {
        let error =
            merge_json_two_way(String::new(), String::new(), "yaml".to_owned()).unwrap_err();

        assert_eq!(
            error.message(),
            "unsupported JSON dialect \"yaml\"; expected json, jsonc, or json5"
        );
    }

    #[test]
    fn typescript_boundary_serializes_declaration_kinds_and_merge3() {
        let analysis = parse_typescript_analysis(
            "interface User { name: string }\nfunction answer(): number { return 42; }\n"
                .to_owned(),
            "typescript".to_owned(),
        )
        .unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&analysis).unwrap();
        assert_eq!(analysis["ok"], true);
        let declarations = analysis["analysis"]["declarations"].as_array().unwrap();
        assert!(
            declarations.iter().any(|declaration| declaration["declaration_kind"] == "function")
        );
        assert!(
            declarations.iter().any(|declaration| declaration["declaration_kind"] == "interface")
        );

        let merged = merge_typescript_three_way(
            "function left(): number { return 1; }\nfunction right(): number { return 1; }\n"
                .to_owned(),
            "function left(): number { return 2; }\nfunction right(): number { return 1; }\n"
                .to_owned(),
            "function left(): number { return 1; }\nfunction right(): number { return 3; }\n"
                .to_owned(),
            "typescript".to_owned(),
        )
        .unwrap();
        let merged: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(merged["outcome"], "clean");
        assert!(
            merged["output"].as_str().is_some_and(|output| {
                output.contains("return 2") && output.contains("return 3")
            })
        );

        let merged = merge_typescript_two_way(
            "function incoming(): number { return 4; }\n".to_owned(),
            "function existing(): number { return 1; }\n".to_owned(),
            "typescript".to_owned(),
        )
        .unwrap();
        let merged: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(merged["ok"], true);
        assert!(
            merged["output"].as_str().is_some_and(|output| {
                output.contains("existing") && output.contains("incoming")
            })
        );
    }

    #[test]
    fn normalized_tslp_boundary_serializes_tree_haver_parse_results() {
        let response =
            parse_normalized_with_tslp("json".to_owned(), "{\"answer\":42}".to_owned(), None)
                .unwrap();
        let result: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(result["ok"], true);
        assert_eq!(result["backend_capability"]["backend_ref"]["id"], "kreuzberg-language-pack");
        assert!(result["root_id"].as_str().is_some_and(|id| !id.is_empty()));
        assert_eq!(result["nodes"][0]["id"], result["root_id"]);
        assert_eq!(result["nodes"][0]["kind"], "document");
        assert_eq!(result["nodes"][0]["span"]["range"]["start_byte"], 0);
    }

    #[test]
    fn normalized_tslp_boundary_rejects_empty_languages() {
        let error = parse_normalized_with_tslp(" ".to_owned(), "{}".to_owned(), None).unwrap_err();

        assert_eq!(error.message(), "parser language cannot be empty");
    }

    #[test]
    fn capability_manifest_declares_compiled_operations_and_provider_factory() {
        let manifest: serde_json::Value =
            serde_json::from_str(&capability_manifest().unwrap()).unwrap();

        assert_eq!(manifest["schema"], "structuredmerge.capability-manifest/v1");
        assert_eq!(manifest["selection_policy"]["implicit_provider_fallback"], false);
        assert_eq!(manifest["selection_policy"]["unavailable_provider"], "error");
        assert_eq!(manifest["operations"][0]["id"], "rust.json.tslp.merge2");
        assert_eq!(manifest["operations"][0]["tree_haver_backend"], "kreuzberg-language-pack");
        assert_eq!(manifest["operations"][1]["id"], "rust.json.tslp.merge3");
        assert_eq!(manifest["parser_provider_factories"][0]["id"], "rust.tslp");
        assert_eq!(manifest["parser_provider_factories"][0]["support_status"], "host_runtime_only");
    }

    #[test]
    fn native_identity_worker_returns_arbitrary_bytes() {
        let provider_name = "identity.native-worker";
        let _ = workflow_host::unregister_workflow_host(provider_name);
        register_workflow_host(identity(provider_name)).unwrap();
        let payload = vec![0, 0xff, b'\r', b'\n', b'a', 0];

        let task_id = start_identity_worker(provider_name.to_owned(), payload.clone()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let result = loop {
            if let Some(result) = poll_identity_worker(task_id).unwrap() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline, "worker should return");
            std::thread::yield_now();
        };

        assert_eq!(result, payload);
        workflow_host::unregister_workflow_host(provider_name).unwrap();
    }

    #[test]
    fn native_identity_worker_rechecks_cancellation_before_invocation() {
        let mut registry = WorkflowHostRegistry::default();
        registry.insert(identity("identity.cancelled")).unwrap();
        let provider = registry.get("identity.cancelled").unwrap();
        let cancelled = AtomicBool::new(true);

        let error = execute_identity_worker(42, provider, vec![0, 0xff], &cancelled).unwrap_err();

        assert_eq!(error.message(), "identity worker cancelled before invocation: 42");
    }

    #[test]
    fn provider_metadata_accepts_matching_bounded_descriptor() {
        let descriptor = r#"{"id":"identity","capabilities":["parse","bytes"]}"#;

        validate_provider_metadata("identity", "1.0.0", descriptor).unwrap();
    }

    #[test]
    fn provider_metadata_rejects_identity_drift() {
        let error = validate_provider_metadata("registered", "1.0.0", r#"{"id":"descriptor"}"#)
            .unwrap_err();

        assert_eq!(
            error.message(),
            "provider descriptor id \"descriptor\" does not match registration name \"registered\""
        );
    }

    #[test]
    fn provider_metadata_rejects_unbounded_capabilities() {
        let capabilities = vec![Value::String("parse".to_owned()); MAX_CAPABILITIES + 1];
        let descriptor =
            serde_json::json!({"id": "identity", "capabilities": capabilities}).to_string();

        let error = validate_provider_metadata("identity", "1.0.0", &descriptor).unwrap_err();

        assert_eq!(error.message(), "provider descriptor exceeds 64 capabilities");
    }
}
