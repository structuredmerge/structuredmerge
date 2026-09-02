use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{Arc, OnceLock},
};

use parking_lot::RwLock;

pub const PACKAGE_NAME: &str = "structuredmerge-host-prototype-core";

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

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    fn version(&self) -> String;

    fn initialize(&self) -> Result<(), HostPrototypeError>;

    fn shutdown(&self) -> Result<(), HostPrototypeError>;
}

pub trait WorkflowHost: Plugin {
    fn descriptor(&self) -> Result<String, HostPrototypeError>;

    fn execute_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError>;
}

#[derive(Default)]
pub struct WorkflowHostRegistry {
    providers: BTreeMap<String, Arc<dyn WorkflowHost>>,
}

impl WorkflowHostRegistry {
    pub fn register(&mut self, provider: Arc<dyn WorkflowHost>) -> Result<(), HostPrototypeError> {
        let name = provider.name().to_owned();
        if name.is_empty() {
            return Err(HostPrototypeError::new("provider name cannot be empty"));
        }
        if self.providers.contains_key(&name) {
            return Err(HostPrototypeError::new(format!("provider already registered: {name}")));
        }
        self.providers.insert(name, provider);
        Ok(())
    }

    fn get(&self, name: &str) -> Result<Arc<dyn WorkflowHost>, HostPrototypeError> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn remove(&mut self, name: &str) -> Result<Arc<dyn WorkflowHost>, HostPrototypeError> {
        self.providers
            .remove(name)
            .ok_or_else(|| HostPrototypeError::new(format!("provider not registered: {name}")))
    }

    fn drain(&mut self) -> Vec<Arc<dyn WorkflowHost>> {
        std::mem::take(&mut self.providers).into_values().collect()
    }

    fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

pub mod registry {
    use super::{OnceLock, RwLock, WorkflowHostRegistry};

    static WORKFLOW_HOSTS: OnceLock<RwLock<WorkflowHostRegistry>> = OnceLock::new();

    pub fn get_workflow_host_registry() -> &'static RwLock<WorkflowHostRegistry> {
        WORKFLOW_HOSTS.get_or_init(|| RwLock::new(WorkflowHostRegistry::default()))
    }
}

pub mod workflow_host {
    use super::{HostPrototypeError, registry};

    pub fn unregister_workflow_host(name: &str) -> Result<(), HostPrototypeError> {
        let provider = registry::get_workflow_host_registry().write().remove(name)?;
        provider.shutdown()
    }

    pub fn clear_workflow_hosts() -> Result<(), HostPrototypeError> {
        let providers = registry::get_workflow_host_registry().write().drain();
        let mut failures = Vec::new();
        for provider in providers {
            if let Err(error) = provider.shutdown() {
                failures.push(format!("{}: {error}", provider.name()));
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

pub fn execute_identity(
    provider_name: String,
    request: Vec<u8>,
) -> Result<Vec<u8>, HostPrototypeError> {
    let provider = registry::get_workflow_host_registry().read().get(&provider_name)?;
    provider.execute_batch(request)
}

pub fn registered_workflow_hosts() -> Vec<String> {
    registry::get_workflow_host_registry().read().names()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdentityHost {
        name: String,
    }

    impl Plugin for IdentityHost {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> String {
            "test".to_owned()
        }

        fn initialize(&self) -> Result<(), HostPrototypeError> {
            Ok(())
        }

        fn shutdown(&self) -> Result<(), HostPrototypeError> {
            Ok(())
        }
    }

    impl WorkflowHost for IdentityHost {
        fn descriptor(&self) -> Result<String, HostPrototypeError> {
            Ok(format!(r#"{{"id":"{}"}}"#, self.name))
        }

        fn execute_batch(&self, request: Vec<u8>) -> Result<Vec<u8>, HostPrototypeError> {
            Ok(request)
        }
    }

    fn identity(name: &str) -> Arc<dyn WorkflowHost> {
        Arc::new(IdentityHost { name: name.to_owned() })
    }

    #[test]
    fn identity_provider_preserves_arbitrary_bytes() {
        let mut registry = WorkflowHostRegistry::default();
        registry.register(identity("identity")).unwrap();

        let input = vec![0, 0xff, b'\r', b'\n', b'a', 0];
        let output = registry.get("identity").unwrap().execute_batch(input.clone()).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn duplicate_names_fail_closed() {
        let mut registry = WorkflowHostRegistry::default();
        registry.register(identity("identity")).unwrap();

        let error = registry.register(identity("identity")).unwrap_err();

        assert_eq!(error.message(), "provider already registered: identity");
    }

    #[test]
    fn removing_a_provider_excludes_it_from_future_lookups() {
        let mut registry = WorkflowHostRegistry::default();
        registry.register(identity("identity")).unwrap();

        let provider = registry.remove("identity").unwrap();

        assert_eq!(provider.name(), "identity");
        let error = match registry.get("identity") {
            Ok(_) => panic!("removed provider remained available"),
            Err(error) => error,
        };
        assert_eq!(error.message(), "provider not registered: identity");
    }
}
