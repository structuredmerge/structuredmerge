use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, ParseResult,
};
use markdown_merge::{
    MarkdownAnalysis, MarkdownDialect, collect_markdown_owners, markdown_feature_profile,
    match_markdown_owners as match_markdown_owners_with_substrate, normalize_markdown_source,
};
use pulldown_cmark::Parser;

pub const PACKAGE_NAME: &str = "pulldown-cmark-merge";
pub const BACKEND_ID: &str = "pulldown-cmark";

fn unsupported_feature(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::UnsupportedFeature,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn available_markdown_backends() -> Vec<String> {
    vec![BACKEND_ID.to_string()]
}

pub fn markdown_backend_feature_profile() -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut profile = serde_json::Map::new();
    profile.insert("family".to_string(), serde_json::Value::String("markdown".to_string()));
    profile.insert(
        "supported_dialects".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("markdown".to_string())]),
    );
    profile.insert("supported_policies".to_string(), serde_json::Value::Array(vec![]));
    profile.insert("backend".to_string(), serde_json::Value::String(BACKEND_ID.to_string()));
    profile.into_iter().collect()
}

pub fn markdown_plan_context() -> ConformanceFamilyPlanContext {
    ConformanceFamilyPlanContext {
        family_profile: ast_merge::FamilyFeatureProfile {
            family: "markdown".to_string(),
            supported_dialects: vec!["markdown".to_string()],
            supported_policies: vec![],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: BACKEND_ID.to_string(),
            supports_dialects: true,
            supported_policies: vec![],
        }),
    }
}

pub fn provider_markdown_feature_profile() -> markdown_merge::MarkdownFeatureProfile {
    markdown_feature_profile()
}

pub fn parse_markdown(
    source: &str,
    dialect: MarkdownDialect,
    backend: Option<&str>,
) -> ParseResult<MarkdownAnalysis> {
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    if dialect != MarkdownDialect::Markdown {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown dialect {:?}.",
                dialect
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    let parser = Parser::new(source);
    for _ in parser {}

    let normalized_source = normalize_markdown_source(source);
    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(MarkdownAnalysis {
            dialect,
            normalized_source: normalized_source.clone(),
            root_kind: markdown_merge::MarkdownRootKind::Document,
            owners: collect_markdown_owners(&normalized_source),
        }),
        policies: vec![],
    }
}

pub fn match_markdown_owners(
    template: MarkdownAnalysis,
    destination: MarkdownAnalysis,
) -> markdown_merge::MarkdownOwnerMatchResult {
    match_markdown_owners_with_substrate(template, destination)
}

pub fn markdown_embedded_families(
    analysis: &MarkdownAnalysis,
) -> Vec<markdown_merge::MarkdownEmbeddedFamilyCandidate> {
    markdown_merge::markdown_embedded_families(analysis)
}
