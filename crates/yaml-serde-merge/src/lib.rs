use std::sync::Once;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeResult, ParseResult,
};
use serde_json::Value;
use tree_haver::{BackendReference, register_backend};
use yaml_merge::{
    YamlAnalysis, YamlBackendFeatureProfile, YamlDialect, YamlFeatureProfile, YamlOwnerMatchResult,
    analyze_yaml_value, match_yaml_owners as match_yaml_owners_with_substrate,
    merge_yaml_with_parser, yaml_feature_profile,
};

pub const PACKAGE_NAME: &str = "yaml-serde-merge";
pub const BACKEND_ID: &str = "yaml_serde";

fn ensure_backend_registered() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_backend(BackendReference {
            id: BACKEND_ID.to_string(),
            family: "native".to_string(),
        });
    });
}

fn unsupported_feature(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::UnsupportedFeature,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

fn parse_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ParseError,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn available_yaml_backends() -> Vec<String> {
    ensure_backend_registered();
    vec![BACKEND_ID.to_string()]
}

pub fn yaml_backend_feature_profile() -> YamlBackendFeatureProfile {
    ensure_backend_registered();
    YamlBackendFeatureProfile {
        family: "yaml",
        supported_dialects: vec![YamlDialect::Yaml],
        supported_policies: yaml_feature_profile().supported_policies,
        backend: BACKEND_ID.to_string(),
    }
}

pub fn yaml_plan_context() -> ConformanceFamilyPlanContext {
    ensure_backend_registered();
    ConformanceFamilyPlanContext {
        family_profile: ast_merge::FamilyFeatureProfile {
            family: "yaml".to_string(),
            supported_dialects: vec!["yaml".to_string()],
            supported_policies: yaml_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: BACKEND_ID.to_string(),
            supports_dialects: true,
            supported_policies: yaml_feature_profile().supported_policies,
        }),
    }
}

pub fn provider_yaml_feature_profile() -> YamlFeatureProfile {
    yaml_feature_profile()
}

pub fn parse_yaml(
    source: &str,
    dialect: YamlDialect,
    backend: Option<&str>,
) -> ParseResult<YamlAnalysis> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported YAML backend {requested}."
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    if dialect != YamlDialect::Yaml {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature("Unsupported YAML dialect.")],
            analysis: None,
            policies: vec![],
        };
    }

    match yaml_serde::from_str::<Value>(source) {
        Ok(parsed) => analyze_yaml_value(parsed, dialect),
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&error.to_string())],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn match_yaml_owners(
    template: &YamlAnalysis,
    destination: &YamlAnalysis,
) -> YamlOwnerMatchResult {
    match_yaml_owners_with_substrate(template, destination)
}

pub fn merge_yaml(
    template_source: &str,
    destination_source: &str,
    dialect: YamlDialect,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported YAML backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_yaml_with_parser(template_source, destination_source, dialect, |source, parse_dialect| {
        parse_yaml(source, parse_dialect, None)
    })
}
