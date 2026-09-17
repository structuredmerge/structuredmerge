//! Merge-behavior registration, separate from TreeHaver parser registration.
//! Declarations and snapshots do not establish availability, selection, execution
//! ownership or default authority. The typed facade supplies the executable handle.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    sync::{Arc, RwLock},
};
use tree_haver::parsed::NativeExtension;

pub const MERGE_REGISTRY_SCHEMA: &str = "structuredmerge.merge-provider-inventory/v1";
const MAX_PROVIDERS: usize = 1024;
const MAX_DESCRIPTOR_BYTES: usize = 65536;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeProviderRole {
    Workflow,
    Backend,
}

/// Constraints for the eventual TreeHaver negotiation, never direct parser calls.
/// Empty sets add no constraint. These sets have no preference-order semantics.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MergeParserRequirements {
    pub allowed_backend_ids: Vec<String>,
    pub forbidden_backend_ids: Vec<String>,
    pub allowed_backend_families: Vec<String>,
    pub forbidden_backend_families: Vec<String>,
    pub languages: Vec<String>,
    pub dialects: Vec<String>,
    pub contracts: Vec<String>,
    pub capabilities: Vec<String>,
    pub profiles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MergeProviderDescriptor {
    pub provider_id: String,
    pub family: String,
    pub role: MergeProviderRole,
    /// Operation names from the common operation envelope, not parser methods.
    pub operations: Vec<String>,
    pub dialects: Vec<String>,
    pub profiles: Vec<String>,
    pub capabilities: Vec<String>,
    pub preservation_guarantees: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    pub parser_requirements: MergeParserRequirements,
    pub allowed_delegation_targets: Vec<String>,
    pub runtime: String,
    pub package: String,
    pub package_version: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub extensions: Vec<NativeExtension>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MergeProviderInventory {
    pub schema: String,
    /// Meaningful only within the originating registry instance.
    pub generation: u64,
    /// Hash of normalized declarations, not executable code or provider liveness.
    pub descriptor_digest: String,
    pub providers: Vec<MergeProviderDescriptor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeRegistrationError {
    InvalidDescriptor,
    DuplicateId,
    UnknownId,
    StaleGeneration,
    GenerationExhausted,
    CapacityExceeded,
    Poisoned,
}
impl std::fmt::Display for MergeRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for MergeRegistrationError {}

struct Entry<P: ?Sized> {
    descriptor: MergeProviderDescriptor,
    provider: Arc<P>,
}
struct State<P: ?Sized> {
    generation: u64,
    entries: BTreeMap<String, Arc<Entry<P>>>,
}

/// One merge-behavior registry can retain native or trait-bridge executors behind
/// the same facade-owned handle type. This is not a second parser registry.
pub struct MergeProviderRegistry<P: ?Sized + Send + Sync> {
    state: RwLock<State<P>>,
}
impl<P: ?Sized + Send + Sync> Default for MergeProviderRegistry<P> {
    fn default() -> Self {
        Self { state: RwLock::new(State { generation: 0, entries: BTreeMap::new() }) }
    }
}

pub struct MergeProviderSnapshot<P: ?Sized + Send + Sync> {
    generation: u64,
    digest: String,
    entries: BTreeMap<String, Arc<Entry<P>>>,
}
impl<P: ?Sized + Send + Sync> Clone for MergeProviderSnapshot<P> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation,
            digest: self.digest.clone(),
            entries: self.entries.clone(),
        }
    }
}

impl<P: ?Sized + Send + Sync> MergeProviderRegistry<P> {
    /// Caller obtains the descriptor before registration: no provider callback
    /// or retired-provider destruction runs while a registry lock is held.
    pub fn register(
        &self,
        descriptor: MergeProviderDescriptor,
        provider: Arc<P>,
    ) -> Result<u64, MergeRegistrationError> {
        let descriptor = normalize(descriptor)?;
        let mut state = self.state.write().map_err(|_| MergeRegistrationError::Poisoned)?;
        if state.entries.contains_key(&descriptor.provider_id) {
            return Err(MergeRegistrationError::DuplicateId);
        }
        if state.entries.len() >= MAX_PROVIDERS {
            return Err(MergeRegistrationError::CapacityExceeded);
        }
        let generation = next_generation(state.generation)?;
        state
            .entries
            .insert(descriptor.provider_id.clone(), Arc::new(Entry { descriptor, provider }));
        state.generation = generation;
        Ok(generation)
    }

    pub fn replace(
        &self,
        descriptor: MergeProviderDescriptor,
        provider: Arc<P>,
        expected_generation: u64,
    ) -> Result<u64, MergeRegistrationError> {
        let descriptor = normalize(descriptor)?;
        let (retired, generation) = {
            let mut state = self.state.write().map_err(|_| MergeRegistrationError::Poisoned)?;
            check_generation(state.generation, expected_generation)?;
            if !state.entries.contains_key(&descriptor.provider_id) {
                return Err(MergeRegistrationError::UnknownId);
            }
            let generation = next_generation(state.generation)?;
            let retired = state
                .entries
                .insert(descriptor.provider_id.clone(), Arc::new(Entry { descriptor, provider }));
            state.generation = generation;
            (retired, generation)
        };
        drop(retired);
        Ok(generation)
    }

    pub fn unregister(
        &self,
        id: &str,
        expected_generation: u64,
    ) -> Result<u64, MergeRegistrationError> {
        let (retired, generation) = {
            let mut state = self.state.write().map_err(|_| MergeRegistrationError::Poisoned)?;
            check_generation(state.generation, expected_generation)?;
            if !state.entries.contains_key(id) {
                return Err(MergeRegistrationError::UnknownId);
            }
            let generation = next_generation(state.generation)?;
            let retired = state.entries.remove(id);
            state.generation = generation;
            (retired, generation)
        };
        drop(retired);
        Ok(generation)
    }

    /// Explicit scoped-test/owner reset. Existing snapshots stay executable.
    pub fn clear(&self, expected_generation: u64) -> Result<u64, MergeRegistrationError> {
        let (retired, generation) = {
            let mut state = self.state.write().map_err(|_| MergeRegistrationError::Poisoned)?;
            check_generation(state.generation, expected_generation)?;
            let generation = next_generation(state.generation)?;
            let retired = std::mem::take(&mut state.entries);
            state.generation = generation;
            (retired, generation)
        };
        drop(retired);
        Ok(generation)
    }

    pub fn snapshot(&self) -> Result<MergeProviderSnapshot<P>, MergeRegistrationError> {
        let (generation, entries) = {
            let state = self.state.read().map_err(|_| MergeRegistrationError::Poisoned)?;
            (state.generation, state.entries.clone())
        };
        let descriptors: Vec<_> = entries.values().map(|entry| &entry.descriptor).collect();
        let mut hash = HashWriter(Sha256::new());
        serde_json::to_writer(&mut hash, &descriptors)
            .map_err(|_| MergeRegistrationError::InvalidDescriptor)?;
        Ok(MergeProviderSnapshot {
            generation,
            digest: format!("{:x}", hash.0.finalize()),
            entries,
        })
    }
}

impl<P: ?Sized + Send + Sync> MergeProviderSnapshot<P> {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn descriptor(&self, id: &str) -> Option<&MergeProviderDescriptor> {
        self.entries.get(id).map(|entry| &entry.descriptor)
    }
    /// Exact identity lookup, NOT capability selection or authorization to run.
    pub fn provider(&self, id: &str) -> Option<Arc<P>> {
        self.entries.get(id).map(|entry| entry.provider.clone())
    }
    pub fn inventory(&self) -> MergeProviderInventory {
        MergeProviderInventory {
            schema: MERGE_REGISTRY_SCHEMA.into(),
            generation: self.generation,
            descriptor_digest: self.digest.clone(),
            providers: self.entries.values().map(|e| e.descriptor.clone()).collect(),
        }
    }
}

fn next_generation(current: u64) -> Result<u64, MergeRegistrationError> {
    current.checked_add(1).ok_or(MergeRegistrationError::GenerationExhausted)
}
fn check_generation(current: u64, expected: u64) -> Result<(), MergeRegistrationError> {
    if current == expected { Ok(()) } else { Err(MergeRegistrationError::StaleGeneration) }
}
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}
fn normalize_set(values: &mut [String]) -> Result<(), MergeRegistrationError> {
    if values.len() > 256 || values.iter().any(|value| !valid_name(value)) {
        return Err(MergeRegistrationError::InvalidDescriptor);
    }
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(MergeRegistrationError::InvalidDescriptor);
    }
    Ok(())
}
fn normalize(
    mut descriptor: MergeProviderDescriptor,
) -> Result<MergeProviderDescriptor, MergeRegistrationError> {
    use MergeRegistrationError::InvalidDescriptor;
    for name in [
        &descriptor.provider_id,
        &descriptor.family,
        &descriptor.runtime,
        &descriptor.package,
        &descriptor.package_version,
    ] {
        if !valid_name(name) {
            return Err(InvalidDescriptor);
        }
    }
    if descriptor.operations.is_empty()
        || descriptor.operations.iter().any(|operation| {
            !matches!(operation.as_str(), "analyze" | "diff2" | "merge2" | "merge3")
        })
    {
        return Err(InvalidDescriptor);
    }
    let requirements = &mut descriptor.parser_requirements;
    for values in [
        &mut descriptor.operations,
        &mut descriptor.dialects,
        &mut descriptor.profiles,
        &mut descriptor.capabilities,
        &mut descriptor.preservation_guarantees,
        &mut descriptor.allowed_delegation_targets,
        &mut requirements.allowed_backend_ids,
        &mut requirements.forbidden_backend_ids,
        &mut requirements.allowed_backend_families,
        &mut requirements.forbidden_backend_families,
        &mut requirements.languages,
        &mut requirements.dialects,
        &mut requirements.contracts,
        &mut requirements.capabilities,
        &mut requirements.profiles,
    ] {
        normalize_set(values)?;
    }
    for (allowed, forbidden) in [
        (&requirements.allowed_backend_ids, &requirements.forbidden_backend_ids),
        (&requirements.allowed_backend_families, &requirements.forbidden_backend_families),
    ] {
        let forbidden: BTreeSet<_> = forbidden.iter().collect();
        if allowed.iter().any(|id| forbidden.contains(id)) {
            return Err(InvalidDescriptor);
        }
    }
    if descriptor.allowed_delegation_targets.contains(&descriptor.provider_id) {
        return Err(InvalidDescriptor);
    }
    // Bound encoded declarations without allocating a second metadata-sized buffer.
    serde_json::to_writer(BoundedWriter(MAX_DESCRIPTOR_BYTES), &descriptor)
        .map_err(|_| InvalidDescriptor)?;
    Ok(descriptor)
}

struct BoundedWriter(usize);
impl io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 =
            self.0.checked_sub(bytes.len()).ok_or_else(|| io::Error::other("descriptor limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct HashWriter(Sha256);
impl io::Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generation_never_wraps() {
        assert_eq!(next_generation(u64::MAX), Err(MergeRegistrationError::GenerationExhausted));
    }
}
