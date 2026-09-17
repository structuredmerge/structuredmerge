use ast_merge::provider_registry::*;
use std::sync::{
    Arc, Barrier, Weak,
    atomic::{AtomicBool, Ordering},
};

fn descriptor(id: &str) -> MergeProviderDescriptor {
    MergeProviderDescriptor {
        provider_id: id.into(),
        family: "ruby".into(),
        role: MergeProviderRole::Workflow,
        operations: vec!["merge3".into(), "analyze".into()],
        dialects: vec!["ruby".into()],
        profiles: vec!["native.v1".into()],
        capabilities: vec!["ownership".into(), "comments".into()],
        preservation_guarantees: vec!["exact_bytes".into()],
        priority: 0,
        parser_requirements: MergeParserRequirements {
            languages: vec!["ruby".into()],
            contracts: vec!["structuredmerge.parse-result/v1".into()],
            ..MergeParserRequirements::default()
        },
        allowed_delegation_targets: vec![],
        runtime: "ruby".into(),
        package: "ruby-merge".into(),
        package_version: "0.2.0".into(),
        metadata: Default::default(),
        extensions: vec![],
    }
}

#[test]
fn declaration_digest_and_inventory_do_not_depend_on_registration_or_set_order() {
    let first = MergeProviderRegistry::default();
    let second = MergeProviderRegistry::default();
    for id in ["ruby.workflow", "ruby.backend"] {
        let mut declaration = descriptor(id);
        if id.ends_with("backend") {
            declaration.role = MergeProviderRole::Backend;
        }
        first.register(declaration, Arc::new(1)).unwrap();
    }
    for id in ["ruby.backend", "ruby.workflow"] {
        let mut declaration = descriptor(id);
        declaration.operations.reverse();
        declaration.capabilities.reverse();
        if id.ends_with("backend") {
            declaration.role = MergeProviderRole::Backend;
        }
        second.register(declaration, Arc::new(2)).unwrap();
    }
    let snapshot = first.snapshot().unwrap();
    let inventory = snapshot.inventory();
    assert_eq!(inventory, second.snapshot().unwrap().inventory());
    assert_eq!(inventory.schema, MERGE_REGISTRY_SCHEMA);
    assert_eq!(
        inventory.providers.iter().map(|d| d.provider_id.as_str()).collect::<Vec<_>>(),
        ["ruby.backend", "ruby.workflow"]
    );
    assert_eq!(inventory.providers[0].role, MergeProviderRole::Backend);
    let encoded = serde_json::to_value(&inventory).unwrap();
    assert!(encoded.get("selected_provider").is_none());
    assert!(encoded.get("approved_as_default").is_none());
    assert_eq!(serde_json::from_value::<MergeProviderInventory>(encoded).unwrap(), inventory);
    let mut copy = snapshot.inventory();
    copy.providers[0].family = "forged".into();
    assert_eq!(snapshot.inventory(), inventory);
    assert!(snapshot.provider("missing").is_none());
    assert_eq!(*snapshot.provider("ruby.workflow").unwrap(), 1);
}

#[test]
fn replacement_and_retirement_preserve_snapshot_handles_and_cached_identity() {
    let registry = MergeProviderRegistry::default();
    let provider = Arc::new(1);
    let old = Arc::downgrade(&provider);
    assert_eq!(registry.register(descriptor("host"), provider).unwrap(), 1);
    let snapshot = registry.snapshot().unwrap();
    let clone = snapshot.clone();
    let mut changed = descriptor("host");
    changed.package_version = "0.3.0".into();
    assert_eq!(registry.replace(changed, Arc::new(2), 1).unwrap(), 2);
    let replacement = registry.snapshot().unwrap();
    assert_ne!(snapshot.digest(), replacement.digest());
    assert_eq!(snapshot.descriptor("host").unwrap().package_version, "0.2.0");
    assert_eq!(*snapshot.provider("host").unwrap(), 1);
    assert_eq!(*replacement.provider("host").unwrap(), 2);
    assert_eq!(registry.unregister("host", 2).unwrap(), 3);
    assert!(registry.snapshot().unwrap().provider("host").is_none());
    assert!(old.upgrade().is_some());
    drop(snapshot);
    assert!(old.upgrade().is_some());
    drop(clone);
    assert!(old.upgrade().is_none());
    assert_eq!(*replacement.provider("host").unwrap(), 2);
    assert_eq!(registry.register(descriptor("host"), Arc::new(3)).unwrap(), 4);
    assert_eq!(*replacement.provider("host").unwrap(), 2);
    assert_eq!(*registry.snapshot().unwrap().provider("host").unwrap(), 3);
}

#[test]
fn failed_mutations_are_atomic_and_same_descriptor_replacement_still_advances_generation() {
    use MergeRegistrationError::*;
    let registry = MergeProviderRegistry::default();
    registry.register(descriptor("host"), Arc::new(1)).unwrap();
    let before = registry.snapshot().unwrap().inventory();
    assert_eq!(registry.register(descriptor("host"), Arc::new(2)), Err(DuplicateId));
    assert_eq!(registry.replace(descriptor("host"), Arc::new(2), 0), Err(StaleGeneration));
    assert_eq!(registry.replace(descriptor("missing"), Arc::new(2), 1), Err(UnknownId));
    assert_eq!(registry.unregister("host", 0), Err(StaleGeneration));
    assert_eq!(registry.unregister("missing", 1), Err(UnknownId));
    assert_eq!(registry.clear(0), Err(StaleGeneration));
    assert_eq!(registry.snapshot().unwrap().inventory(), before);
    assert_eq!(registry.replace(descriptor("host"), Arc::new(2), 1).unwrap(), 2);
    assert_eq!(registry.snapshot().unwrap().digest(), before.descriptor_digest);
    assert_eq!(registry.clear(2).unwrap(), 3);
    assert!(registry.snapshot().unwrap().inventory().providers.is_empty());
}

#[test]
fn invalid_declarations_never_mutate_registry() {
    let registry = MergeProviderRegistry::default();
    let mut cases = vec![];
    let mut value = descriptor("bad id");
    cases.push(value.clone());
    value = descriptor("host");
    value.operations.clear();
    cases.push(value.clone());
    value = descriptor("host");
    value.operations.push("parse".into());
    cases.push(value.clone());
    value = descriptor("host");
    value.operations.push("analyze".into());
    cases.push(value.clone());
    value = descriptor("host");
    value.runtime.clear();
    cases.push(value.clone());
    value = descriptor("host");
    value.allowed_delegation_targets.push("host".into());
    cases.push(value.clone());
    value = descriptor("host");
    value.parser_requirements.allowed_backend_ids.push("prism".into());
    value.parser_requirements.forbidden_backend_ids.push("prism".into());
    cases.push(value.clone());
    value = descriptor("host");
    value.parser_requirements.allowed_backend_families.push("native".into());
    value.parser_requirements.forbidden_backend_families.push("native".into());
    cases.push(value.clone());
    value = descriptor("host");
    value.metadata.insert("oversized".into(), "x".repeat(65536).into());
    cases.push(value);
    for descriptor in cases {
        assert_eq!(
            registry.register(descriptor, Arc::new(())),
            Err(MergeRegistrationError::InvalidDescriptor)
        );
        assert_eq!(registry.snapshot().unwrap().generation(), 0);
        assert!(registry.snapshot().unwrap().inventory().providers.is_empty());
    }
}

#[test]
fn concurrent_replacements_cannot_both_commit_the_same_observed_generation() {
    let registry = Arc::new(MergeProviderRegistry::default());
    registry.register(descriptor("host"), Arc::new(0)).unwrap();
    let old = registry.snapshot().unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let workers: Vec<_> = [1, 2]
        .into_iter()
        .map(|id| {
            let registry = registry.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                registry.replace(descriptor("host"), Arc::new(id), 1)
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|result| **result == Ok(2)).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(MergeRegistrationError::StaleGeneration))
            .count(),
        1
    );
    assert_eq!(*old.provider("host").unwrap(), 0);
    assert_eq!(registry.snapshot().unwrap().generation(), 2);
}

struct ReentrantDrop {
    registry: Weak<MergeProviderRegistry<ReentrantDrop>>,
    released: Arc<AtomicBool>,
}
impl Drop for ReentrantDrop {
    fn drop(&mut self) {
        let registry = self.registry.upgrade().unwrap();
        registry.snapshot().unwrap();
        self.released.store(true, Ordering::SeqCst);
    }
}

#[test]
fn retiring_provider_destructors_can_reenter_registry() {
    let registry = Arc::new(MergeProviderRegistry::default());
    for mutation in ["replace", "unregister", "clear"] {
        let released = Arc::new(AtomicBool::new(false));
        let provider = Arc::new(ReentrantDrop {
            registry: Arc::downgrade(&registry),
            released: released.clone(),
        });
        let generation = registry.register(descriptor("host"), provider).unwrap();
        match mutation {
            "replace" => {
                let replacement = Arc::new(ReentrantDrop {
                    registry: Arc::downgrade(&registry),
                    released: Arc::new(AtomicBool::new(false)),
                });
                let generation =
                    registry.replace(descriptor("host"), replacement, generation).unwrap();
                registry.clear(generation).unwrap();
            }
            "unregister" => {
                registry.unregister("host", generation).unwrap();
            }
            _ => {
                registry.clear(generation).unwrap();
            }
        }
        assert!(released.load(Ordering::SeqCst));
    }
}

#[test]
fn trait_object_handles_require_no_clone_or_serialization_implementation() {
    trait Provider: Send + Sync {
        fn identity(&self) -> usize;
    }
    struct Handle;
    impl Provider for Handle {
        fn identity(&self) -> usize {
            7
        }
    }
    let registry: MergeProviderRegistry<dyn Provider> = MergeProviderRegistry::default();
    registry.register(descriptor("host"), Arc::new(Handle)).unwrap();
    assert_eq!(registry.snapshot().unwrap().clone().provider("host").unwrap().identity(), 7);
}

#[test]
fn priority_defaults_to_zero_and_inventory_preserves_extension_data() {
    let mut encoded = serde_json::to_value(descriptor("host")).unwrap();
    encoded.as_object_mut().unwrap().remove("priority");
    let mut declaration: MergeProviderDescriptor = serde_json::from_value(encoded).unwrap();
    assert_eq!(declaration.priority, 0);
    declaration.metadata.insert("future".into(), serde_json::json!({"nested": [null, false, 7]}));
    let registry = MergeProviderRegistry::default();
    registry.register(declaration.clone(), Arc::new(())).unwrap();
    let before = registry.snapshot().unwrap();
    assert_eq!(before.inventory().providers[0].metadata, declaration.metadata);
    declaration.priority = 10;
    registry.replace(declaration, Arc::new(()), 1).unwrap();
    assert_ne!(registry.snapshot().unwrap().digest(), before.digest());
}

#[test]
fn registry_capacity_is_bounded_and_failed_registration_does_not_advance_generation() {
    let registry = MergeProviderRegistry::default();
    for index in 0..1024 {
        registry.register(descriptor(&format!("provider.{index}")), Arc::new(())).unwrap();
    }
    assert_eq!(
        registry.register(descriptor("overflow"), Arc::new(())),
        Err(MergeRegistrationError::CapacityExceeded)
    );
    let snapshot = registry.snapshot().unwrap();
    assert_eq!(snapshot.generation(), 1024);
    assert_eq!(snapshot.inventory().providers.len(), 1024);
    assert!(snapshot.provider("overflow").is_none());
    // Replacement is permitted at capacity because it adds no registration.
    assert_eq!(registry.replace(descriptor("provider.0"), Arc::new(()), 1024).unwrap(), 1025);
}
