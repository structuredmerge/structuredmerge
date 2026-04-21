use std::sync::Once;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeResult, ParseResult, PolicyReference, PolicySurface,
};
use pest::Parser;
use pest_grammars::toml::{Rule as PestTomlRule, TomlParser as PestTomlParser};
use tree_haver::{BackendReference, register_backend};
use toml_merge::{
    TomlAnalysis, TomlDialect, TomlFeatureProfile, TomlOwnerMatchResult, analyze_toml_source,
    match_toml_owners as match_toml_owners_with_substrate, merge_toml_with_parser,
    toml_feature_profile,
};

pub const PACKAGE_NAME: &str = "pest-toml-merge";
pub const BACKEND_ID: &str = "pest";

fn ensure_backend_registered() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_backend(BackendReference {
            id: BACKEND_ID.to_string(),
            family: "peg".to_string(),
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

pub fn available_toml_backends() -> Vec<String> {
    ensure_backend_registered();
    vec![BACKEND_ID.to_string()]
}

pub fn toml_backend_feature_profile() -> std::collections::BTreeMap<String, serde_json::Value> {
    ensure_backend_registered();
    let mut profile = serde_json::Map::new();
    profile.insert("family".to_string(), serde_json::Value::String("toml".to_string()));
    profile.insert(
        "supported_dialects".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("toml".to_string())]),
    );
    profile.insert(
        "supported_policies".to_string(),
        serde_json::Value::Array(vec![serde_json::json!({
            "surface": "array",
            "name": "destination_wins_array",
        })]),
    );
    profile.insert("backend".to_string(), serde_json::Value::String(BACKEND_ID.to_string()));
    profile.into_iter().collect()
}

pub fn toml_plan_context() -> ConformanceFamilyPlanContext {
    ensure_backend_registered();
    ConformanceFamilyPlanContext {
        family_profile: ast_merge::FamilyFeatureProfile {
            family: "toml".to_string(),
            supported_dialects: vec!["toml".to_string()],
            supported_policies: vec![PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            }],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: BACKEND_ID.to_string(),
            supports_dialects: false,
            supported_policies: vec![PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            }],
        }),
    }
}

pub fn provider_toml_feature_profile() -> TomlFeatureProfile {
    toml_feature_profile()
}

pub fn parse_toml(
    source: &str,
    dialect: TomlDialect,
    backend: Option<&str>,
) -> ParseResult<TomlAnalysis> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {requested}."
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    if let Err(error) = PestTomlParser::parse(PestTomlRule::toml, source) {
        return ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&error.to_string())],
            analysis: None,
            policies: vec![],
        };
    }

    analyze_toml_source(source, dialect)
}

pub fn match_toml_owners(
    template: &TomlAnalysis,
    destination: &TomlAnalysis,
) -> TomlOwnerMatchResult {
    match_toml_owners_with_substrate(template, destination)
}

pub fn merge_toml(
    template_source: &str,
    destination_source: &str,
    dialect: TomlDialect,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_toml_with_parser(template_source, destination_source, dialect, |source, parse_dialect| {
        parse_toml(source, parse_dialect, None)
    })
}
