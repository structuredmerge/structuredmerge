//! Compiled common-operation declarations, not registry state or availability.
//! This Rust build-tool surface is not yet part of the generated host facade.
use crate::{
    MergeParserRequirements, MergeProviderDescriptor, MergeProviderRole, OperationKind,
    OperationProfileCatalog, ParserProviderDescriptor,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tree_haver::{language_pack_provider::LanguagePackProvider, service::ParserProvider};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CompiledProviderInventory {
    pub schema: String,
    pub kernel_version: String,
    pub scope: String,
    pub workflows: Vec<MergeProviderDescriptor>,
    pub parsers: Vec<ParserProviderDescriptor>,
    pub profiles: OperationProfileCatalog,
    pub runtime_availability_checked: bool,
}

/// Describe the common-operation kernel and cached-only parser constructors.
/// Never registers/probes a provider, loads a grammar, reads a cache, or inspects
/// installed host packages. Native-extension requirements remain declarations.
/// This is not an exhaustive inventory of legacy CLI/benchmark implementations.
pub fn compiled_provider_inventory() -> CompiledProviderInventory {
    let profiles = crate::operation_profile_catalog();
    let mut workflows = Vec::new();
    let mut parser_languages = BTreeSet::new();
    for profile in &profiles.profiles {
        let languages: BTreeSet<_> = std::iter::once(None)
            .chain(profile.explicit_dialects.iter().map(|dialect| Some(dialect.as_str())))
            .map(|dialect| {
                crate::profiles::profile_parser_language(&profile.family, dialect)
                    .unwrap()
                    .to_owned()
            })
            .collect();
        if profile.required_native_extension.is_none() {
            parser_languages.extend(languages.iter().cloned());
        }
        let mut operations: Vec<String> = profile
            .operations
            .iter()
            .map(|operation| {
                match operation {
                    OperationKind::Analyze => "analyze",
                    OperationKind::Diff2 => "diff2",
                    OperationKind::Merge2 => "merge2",
                    OperationKind::Merge3 => "merge3",
                }
                .into()
            })
            .collect();
        operations.sort();
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "required_native_extension".into(),
            serde_json::json!(profile.required_native_extension),
        );
        metadata.insert("syntax_scope".into(), serde_json::json!(profile.syntax_scope));
        metadata.insert("limitations".into(), serde_json::json!(profile.limitations));
        metadata.insert("experimental".into(), serde_json::json!(profile.experimental));
        workflows.push(MergeProviderDescriptor {
            provider_id: profile.provider_id.clone(),
            family: profile.family.clone(),
            role: MergeProviderRole::Backend,
            operations: operations.clone(),
            dialects: profile.explicit_dialects.clone(),
            profiles: vec![profile.id.clone()],
            capabilities: operations,
            preservation_guarantees: vec![],
            priority: 0,
            parser_requirements: MergeParserRequirements {
                languages: languages.into_iter().collect(),
                contracts: vec![profile.parser_contract.clone()],
                ..MergeParserRequirements::default()
            },
            allowed_delegation_targets: vec![],
            runtime: profile.semantic_runtime.clone(),
            package: env!("CARGO_PKG_NAME").into(),
            package_version: env!("CARGO_PKG_VERSION").into(),
            metadata,
            extensions: vec![],
        });
    }
    workflows.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    let parsers = parser_languages
        .into_iter()
        .map(|language| {
            LanguagePackProvider::new_cached_only(format!("kernel.tslp.{language}"), language)
                .expect("compiled profile has a nonempty parser language")
                .descriptor()
                .clone()
        })
        .collect();
    CompiledProviderInventory {
        schema: "structuredmerge.compiled-provider-inventory/v1".into(),
        kernel_version: env!("CARGO_PKG_VERSION").into(),
        scope: "typed-common-operation-kernel".into(),
        workflows,
        parsers,
        profiles,
        runtime_availability_checked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_uses_typed_descriptors_and_does_not_mutate_registries() {
        let before = crate::parser_registry_inventory().unwrap();
        let hosts = crate::workflow_registry_inventory().unwrap();
        let inventory = compiled_provider_inventory();
        assert_eq!(inventory, compiled_provider_inventory());
        assert_eq!(before, crate::parser_registry_inventory().unwrap());
        assert_eq!(hosts, crate::workflow_registry_inventory().unwrap());
        assert!(!inventory.runtime_availability_checked);
        assert_eq!(inventory.workflows.len(), inventory.profiles.profiles.len());
        let registry = crate::provider_registry::MergeProviderRegistry::<()>::default();
        for workflow in &inventory.workflows {
            registry.register(workflow.clone(), std::sync::Arc::new(())).unwrap();
            let profile = inventory
                .profiles
                .profiles
                .iter()
                .find(|p| p.provider_id == workflow.provider_id)
                .unwrap();
            assert_eq!(workflow.profiles.as_slice(), std::slice::from_ref(&profile.id));
            assert_eq!(
                workflow.parser_requirements.contracts.as_slice(),
                std::slice::from_ref(&profile.parser_contract)
            );
        }
        assert_eq!(
            registry.snapshot().unwrap().inventory().providers.len(),
            inventory.workflows.len()
        );
        for parser in &inventory.parsers {
            let original = LanguagePackProvider::new_cached_only(
                parser.id.clone(),
                parser.languages[0].clone(),
            )
            .unwrap();
            assert_eq!(parser, original.descriptor());
            assert_eq!(parser.metadata["grammar_policy"], "cached-only");
            assert!(!matches!(parser.languages[0].as_str(), "python" | "yaml"));
        }
        for id in ["kernel.python", "kernel.yaml"] {
            let workflow = inventory.workflows.iter().find(|w| w.provider_id == id).unwrap();
            assert!(workflow.metadata["required_native_extension"].is_string());
        }
    }
}
