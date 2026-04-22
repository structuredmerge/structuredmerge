use std::sync::Once;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, ConformanceManifestReviewState,
    ConformanceManifestReviewStateEnvelope, DelegatedChildGroupReviewState, Diagnostic,
    DiagnosticCategory, DiagnosticSeverity, MergeResult, ParseResult, ReviewReplayBundle,
    ReviewReplayBundleEnvelope,
};
use markdown_merge::{
    AppliedChildOutput, MarkdownAnalysis, MarkdownDialect, collect_markdown_owners,
    markdown_feature_profile, merge_markdown as merge_markdown_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope as merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle as merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope as merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_review_state as merge_markdown_with_reviewed_nested_outputs_from_review_state_with_substrate,
    merge_markdown_with_reviewed_nested_outputs as merge_markdown_with_reviewed_nested_outputs_with_substrate,
    match_markdown_owners as match_markdown_owners_with_substrate, normalize_markdown_source,
};
use pulldown_cmark::Parser;
use tree_haver::{BackendReference, register_backend};

pub const PACKAGE_NAME: &str = "pulldown-cmark-merge";
pub const BACKEND_ID: &str = "pulldown-cmark";

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

pub fn available_markdown_backends() -> Vec<String> {
    ensure_backend_registered();
    vec![BACKEND_ID.to_string()]
}

pub fn markdown_backend_feature_profile() -> std::collections::BTreeMap<String, serde_json::Value> {
    ensure_backend_registered();
    let mut profile = serde_json::Map::new();
    profile.insert("family".to_string(), serde_json::Value::String("markdown".to_string()));
    profile.insert(
        "supported_dialects".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("markdown".to_string())]),
    );
    profile.insert("supported_policies".to_string(), serde_json::Value::Array(vec![]));
    profile.insert("backend".to_string(), serde_json::Value::String(BACKEND_ID.to_string()));
    profile.insert(
        "backend_ref".to_string(),
        serde_json::json!({
            "id": BACKEND_ID,
            "family": "native",
        }),
    );
    profile.into_iter().collect()
}

pub fn markdown_plan_context() -> ConformanceFamilyPlanContext {
    ensure_backend_registered();
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
    ensure_backend_registered();
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

pub fn merge_markdown(
  template_source: &str,
  destination_source: &str,
    dialect: MarkdownDialect,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_substrate(
        template_source,
        destination_source,
        dialect,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &DelegatedChildGroupReviewState,
    applied_children: &[AppliedChildOutput],
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_reviewed_nested_outputs_with_substrate(
        template_source,
        destination_source,
        dialect,
        review_state,
        applied_children,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    replay_bundle: &ReviewReplayBundle,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_substrate(
        template_source,
        destination_source,
        dialect,
        replay_bundle,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ReviewReplayBundleEnvelope,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_substrate(
        template_source,
        destination_source,
        dialect,
        envelope,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &ConformanceManifestReviewState,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_reviewed_nested_outputs_from_review_state_with_substrate(
        template_source,
        destination_source,
        dialect,
        review_state,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ConformanceManifestReviewStateEnvelope,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported Markdown backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_substrate(
        template_source,
        destination_source,
        dialect,
        envelope,
        markdown_merge::MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn markdown_embedded_families(
    analysis: &MarkdownAnalysis,
) -> Vec<markdown_merge::MarkdownEmbeddedFamilyCandidate> {
    markdown_merge::markdown_embedded_families(analysis)
}
