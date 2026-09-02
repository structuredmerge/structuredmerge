use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CompactRuleset, CompactRulesetAtomicNode, CompactRulesetBackendDeclaration,
    CompactRulesetChildGroup, CompactRulesetNodeRole, compact_ruleset_feature_profile,
};

pub const RULESET_CONFIG_ENVELOPE_KIND: &str = "structuredmerge.ruleset-config";
pub const RULESET_CONFIG_ENVELOPE_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RulesetScalar {
    Boolean(bool),
    String(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetRepairPolicy {
    pub kind: String,
    pub handling: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetSurfaceDeclaration {
    pub name: String,
    pub selector: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetDelegationPolicy {
    pub surface_name: String,
    pub strategy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetRuntimeConfig {
    pub format: String,
    pub owner_selector: String,
    pub match_key: String,
    pub read_strategy: String,
    pub attachment_strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_strategy: Option<String>,
    pub backends: Vec<CompactRulesetBackendDeclaration>,
    pub node_roles: Vec<CompactRulesetNodeRole>,
    pub atomic_nodes: Vec<CompactRulesetAtomicNode>,
    pub child_groups: Vec<CompactRulesetChildGroup>,
    pub capabilities: BTreeMap<String, RulesetScalar>,
    pub logical_owners: BTreeMap<String, String>,
    pub repair_policies: Vec<RulesetRepairPolicy>,
    pub surfaces: Vec<RulesetSurfaceDeclaration>,
    pub delegation_policies: Vec<RulesetDelegationPolicy>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetSupportStyle {
    pub mode: String,
    pub source: String,
    pub capability: String,
    pub style: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetRuntimeDeclaration {
    pub read_strategy: String,
    pub attachment_strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_family: Option<String>,
    pub capabilities: BTreeMap<String, RulesetScalar>,
    pub logical_owners: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_style: Option<RulesetSupportStyle>,
    pub comment_free: bool,
    pub logical_owner: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetRuntimeFeatureProfile {
    pub owner_selector: String,
    pub match_key: String,
    pub declaration: RulesetRuntimeDeclaration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_strategy: Option<String>,
    pub backends: Vec<CompactRulesetBackendDeclaration>,
    pub node_roles: Vec<CompactRulesetNodeRole>,
    pub atomic_nodes: Vec<CompactRulesetAtomicNode>,
    pub child_groups: Vec<CompactRulesetChildGroup>,
    pub repair_policies: Vec<RulesetRepairPolicy>,
    pub surfaces: Vec<RulesetSurfaceDeclaration>,
    pub delegation_policies: Vec<RulesetDelegationPolicy>,
    pub layout_aware: bool,
    pub comment_aware: bool,
    pub structural_only: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetRuntimeTranslation {
    pub config: RulesetRuntimeConfig,
    pub declaration: RulesetRuntimeDeclaration,
    pub feature_profile: RulesetRuntimeFeatureProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulesetConfigEnvelope {
    pub kind: String,
    pub version: u32,
    pub config: RulesetRuntimeConfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulesetConfigImportErrorCategory {
    KindMismatch,
    UnsupportedVersion,
    InvalidConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RulesetConfigImportError {
    pub category: RulesetConfigImportErrorCategory,
    pub message: String,
}

pub fn translate_compact_ruleset(
    ruleset: &CompactRuleset,
    source: &str,
    capability: &str,
) -> Result<RulesetRuntimeTranslation, String> {
    let profile = compact_ruleset_feature_profile(ruleset);
    let config = RulesetRuntimeConfig {
        format: profile.format,
        owner_selector: profile.owners,
        match_key: profile.match_,
        read_strategy: profile.read,
        attachment_strategy: profile.attach,
        comment_style: nonempty(profile.comment_style),
        render_family: nonempty(profile.render),
        render_strategy: nonempty(profile.render_strategy),
        backends: profile.backends,
        node_roles: profile.node_roles,
        atomic_nodes: profile.atomic_nodes,
        child_groups: profile.child_groups,
        capabilities: profile
            .capabilities
            .into_iter()
            .map(|entry| (entry.name, ruleset_scalar(&entry.value)))
            .collect(),
        logical_owners: profile
            .logical_owners
            .into_iter()
            .map(|entry| (entry.name, entry.value))
            .collect(),
        repair_policies: profile
            .repairs
            .into_iter()
            .map(|entry| RulesetRepairPolicy { kind: entry.name, handling: entry.value })
            .collect(),
        surfaces: profile
            .surfaces
            .into_iter()
            .map(|entry| RulesetSurfaceDeclaration { name: entry.name, selector: entry.selector })
            .collect(),
        delegation_policies: profile
            .delegates
            .into_iter()
            .map(|entry| RulesetDelegationPolicy {
                surface_name: entry.surface,
                strategy: entry.policy,
            })
            .collect(),
    };
    validate_runtime_config(&config)?;
    Ok(translate_runtime_config(config, source, capability))
}

pub fn translate_runtime_config(
    config: RulesetRuntimeConfig,
    source: &str,
    capability: &str,
) -> RulesetRuntimeTranslation {
    let support_style = config.comment_style.as_ref().map(|style| RulesetSupportStyle {
        mode: config.read_strategy.clone(),
        source: source.to_string(),
        capability: capability.to_string(),
        style: style.clone(),
    });
    let declaration = RulesetRuntimeDeclaration {
        read_strategy: config.read_strategy.clone(),
        attachment_strategy: config.attachment_strategy.clone(),
        comment_style: config.comment_style.clone(),
        render_family: config.render_family.clone(),
        capabilities: config.capabilities.clone(),
        logical_owners: config.logical_owners.clone(),
        comment_free: support_style.is_none(),
        logical_owner: !config.logical_owners.is_empty(),
        support_style,
    };
    let comment_aware = declaration.support_style.is_some();
    let feature_profile = RulesetRuntimeFeatureProfile {
        owner_selector: config.owner_selector.clone(),
        match_key: config.match_key.clone(),
        declaration: declaration.clone(),
        render_strategy: config.render_strategy.clone(),
        backends: config.backends.clone(),
        node_roles: config.node_roles.clone(),
        atomic_nodes: config.atomic_nodes.clone(),
        child_groups: config.child_groups.clone(),
        repair_policies: config.repair_policies.clone(),
        surfaces: config.surfaces.clone(),
        delegation_policies: config.delegation_policies.clone(),
        layout_aware: true,
        comment_aware,
        structural_only: false,
    };
    RulesetRuntimeTranslation { config, declaration, feature_profile }
}

pub fn ruleset_config_envelope(config: RulesetRuntimeConfig) -> RulesetConfigEnvelope {
    RulesetConfigEnvelope {
        kind: RULESET_CONFIG_ENVELOPE_KIND.to_string(),
        version: RULESET_CONFIG_ENVELOPE_VERSION,
        config,
    }
}

pub fn import_ruleset_config_envelope(
    envelope: &RulesetConfigEnvelope,
) -> Result<RulesetRuntimeConfig, RulesetConfigImportError> {
    if envelope.kind != RULESET_CONFIG_ENVELOPE_KIND {
        return Err(RulesetConfigImportError {
            category: RulesetConfigImportErrorCategory::KindMismatch,
            message: format!("unsupported ruleset config envelope kind {:?}", envelope.kind),
        });
    }
    if envelope.version != RULESET_CONFIG_ENVELOPE_VERSION {
        return Err(RulesetConfigImportError {
            category: RulesetConfigImportErrorCategory::UnsupportedVersion,
            message: format!("unsupported ruleset config envelope version {}", envelope.version),
        });
    }
    validate_runtime_config(&envelope.config).map_err(|message| RulesetConfigImportError {
        category: RulesetConfigImportErrorCategory::InvalidConfiguration,
        message,
    })?;
    Ok(envelope.config.clone())
}

fn validate_runtime_config(config: &RulesetRuntimeConfig) -> Result<(), String> {
    for (name, value) in [
        ("format", &config.format),
        ("owner selector", &config.owner_selector),
        ("match key", &config.match_key),
    ] {
        if value.is_empty() {
            return Err(format!("ruleset {name} must not be empty"));
        }
    }
    if !matches!(
        config.read_strategy.as_str(),
        "source_augmented_portable_write" | "native_read_portable_write" | "native_mutation"
    ) {
        return Err(format!("unknown ruleset read strategy {:?}", config.read_strategy));
    }
    if !matches!(
        config.attachment_strategy.as_str(),
        "layout_only"
            | "tracker_layout_merge"
            | "augmenter_preferred_tracker_layout"
            | "normalize_tracked_layout_merge"
    ) {
        return Err(format!(
            "unknown ruleset attachment strategy {:?}",
            config.attachment_strategy
        ));
    }
    for policy in &config.logical_owners {
        if !matches!(
            policy.1.as_str(),
            "preserve_always" | "preserve_if_referenced" | "remove_unreferenced"
        ) {
            return Err(format!("unknown logical owner action {:?}", policy.1));
        }
    }
    for policy in &config.repair_policies {
        if !matches!(policy.handling.as_str(), "heal" | "warn" | "error" | "skip") {
            return Err(format!("unknown repair handling {:?}", policy.handling));
        }
    }
    for policy in &config.delegation_policies {
        if !config.surfaces.iter().any(|surface| surface.name == policy.surface_name) {
            return Err(format!(
                "delegation policy references undeclared surface {:?}",
                policy.surface_name
            ));
        }
    }
    Ok(())
}

fn ruleset_scalar(value: &str) -> RulesetScalar {
    match value {
        "true" => RulesetScalar::Boolean(true),
        "false" => RulesetScalar::Boolean(false),
        value => RulesetScalar::String(value.to_string()),
    }
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_compact_ruleset;

    const RULESET: &str = "format markdown\n\
owners heading_sections\n\
match normalized_heading_path\n\
read source_augmented_portable_write\n\
attach tracker_layout_merge\n\
comment_style html_comment\n\
render markdown_sections\n\
render_strategy source_fragment_reuse\n\
backend kreuzberg-language-pack supported\n\
node_role section structural\n\
atomic code_span true\n\
child_group document sections ordered\n\
capability layout_aware true\n\
logical_owner link_definition preserve_if_referenced\n\
repair comment_ownership_overlap warn\n\
surface fenced_code_block language_tag\n\
delegate fenced_code_block by_language\n";

    #[test]
    fn translates_runtime_semantics_and_round_trips_a_versioned_config() {
        let parsed = parse_compact_ruleset(RULESET).analysis.unwrap();
        let translated = translate_compact_ruleset(&parsed, "fixture", "full").unwrap();

        assert_eq!(translated.declaration.read_strategy, "source_augmented_portable_write");
        assert!(translated.feature_profile.layout_aware);
        assert!(translated.feature_profile.comment_aware);
        assert_eq!(translated.feature_profile.backends[0].backend, "kreuzberg-language-pack");
        assert_eq!(translated.feature_profile.node_roles[0].role, "structural");
        assert!(translated.feature_profile.atomic_nodes[0].atomic);
        assert_eq!(translated.feature_profile.child_groups[0].policy, "ordered");
        assert_eq!(translated.config.capabilities["layout_aware"], RulesetScalar::Boolean(true));
        let envelope = ruleset_config_envelope(translated.config.clone());
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: RulesetConfigEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(import_ruleset_config_envelope(&decoded).unwrap(), translated.config);
    }

    #[test]
    fn keeps_comment_free_rulesets_comment_free() {
        let source = "format json\nowners mapping_entries\nmatch key_name\n\
read native_mutation\nattach layout_only\n";
        let parsed = parse_compact_ruleset(source).analysis.unwrap();
        let translated = translate_compact_ruleset(&parsed, "fixture", "full").unwrap();

        assert!(translated.declaration.comment_free);
        assert!(!translated.feature_profile.comment_aware);
        assert!(translated.declaration.support_style.is_none());
    }

    #[test]
    fn rejects_wrong_envelope_identity_and_tampered_runtime_values() {
        let parsed = parse_compact_ruleset(RULESET).analysis.unwrap();
        let config = translate_compact_ruleset(&parsed, "fixture", "full").unwrap().config;
        let mut envelope = ruleset_config_envelope(config.clone());
        envelope.version += 1;
        assert_eq!(
            import_ruleset_config_envelope(&envelope).unwrap_err().category,
            RulesetConfigImportErrorCategory::UnsupportedVersion
        );

        let mut invalid = config;
        invalid.read_strategy = "fallback_parser".to_string();
        let error = import_ruleset_config_envelope(&ruleset_config_envelope(invalid)).unwrap_err();
        assert_eq!(error.category, RulesetConfigImportErrorCategory::InvalidConfiguration);
    }
}
