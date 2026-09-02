use std::sync::Once;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, ConformanceManifestReviewState,
    ConformanceManifestReviewStateEnvelope, DelegatedChildGroupReviewState, Diagnostic,
    DiagnosticCategory, DiagnosticSeverity, MergeResult, ParseResult, ReviewReplayBundle,
    ReviewReplayBundleEnvelope,
};
use markdown_merge::{
    AppliedChildOutput, MarkdownAnalysis, MarkdownDialect, MarkdownHeadingStyle, MarkdownOwner,
    include_trailing_line_ending, markdown_feature_profile,
    match_markdown_owners as match_markdown_owners_with_substrate,
    merge_markdown_source_preserving_with_parser as merge_markdown_source_preserving_with_substrate,
    merge_markdown_with_parser as merge_markdown_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_parser as merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_parser as merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_parser as merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_from_review_state_with_parser as merge_markdown_with_reviewed_nested_outputs_from_review_state_with_substrate,
    merge_markdown_with_reviewed_nested_outputs_with_parser as merge_markdown_with_reviewed_nested_outputs_with_substrate,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
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
        merge_engine: None,
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

    let owners = collect_pulldown_owners(source);
    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(MarkdownAnalysis {
            dialect,
            normalized_source: source.to_string(),
            root_kind: markdown_merge::MarkdownRootKind::Document,
            owners,
            comment_regions: vec![],
            layout_gaps: vec![],
            comment_attachments: vec![],
        }),
        policies: vec![],
    }
}

struct PendingHeading {
    level: usize,
    style: MarkdownHeadingStyle,
    start_byte: usize,
    end_byte: usize,
    text: String,
}

struct PendingFence {
    info_string: Option<String>,
    start_byte: usize,
    end_byte: usize,
    content_start_byte: Option<usize>,
    content_end_byte: Option<usize>,
}

enum ProjectedOwner {
    Heading(PendingHeading),
    Fence(PendingFence),
}

impl ProjectedOwner {
    fn start_byte(&self) -> usize {
        match self {
            Self::Heading(owner) => owner.start_byte,
            Self::Fence(owner) => owner.start_byte,
        }
    }
}

fn collect_pulldown_owners(source: &str) -> Vec<MarkdownOwner> {
    let mut projected = Vec::new();
    let mut heading = None;
    let mut fence = None;
    let mut container_depth = 0usize;

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) if container_depth == 0 => {
                heading = Some(PendingHeading {
                    level: heading_level(level),
                    style: pulldown_heading_style(source, range.start),
                    start_byte: range.start,
                    end_byte: range.end,
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(owner) = heading.take() {
                    projected.push(ProjectedOwner::Heading(owner));
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let info = info.split_whitespace().next().filter(|value| !value.is_empty());
                fence = Some(PendingFence {
                    info_string: info.map(str::to_string),
                    start_byte: range.start,
                    end_byte: include_trailing_line_ending(source, range.end),
                    content_start_byte: None,
                    content_end_byte: None,
                });
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(owner) = fence.take() {
                    projected.push(ProjectedOwner::Fence(owner));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(owner) = heading.as_mut() {
                    owner.text.push_str(&text);
                }
                if let Some(owner) = fence.as_mut() {
                    owner.content_start_byte.get_or_insert(range.start);
                    owner.content_end_byte = Some(range.end);
                }
            }
            Event::Start(tag) if owner_container_start(&tag) => {
                container_depth += 1;
            }
            Event::End(tag) if owner_container_end(tag) => {
                container_depth = container_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    projected.sort_by_key(ProjectedOwner::start_byte);
    let mut heading_index = 0usize;
    let mut fence_index = 0usize;
    projected
        .into_iter()
        .map(|owner| match owner {
            ProjectedOwner::Heading(owner) => {
                let result = match owner.style {
                    MarkdownHeadingStyle::Atx => MarkdownOwner::heading(
                        heading_index,
                        owner.level,
                        &owner.text,
                        owner.start_byte,
                        owner.end_byte,
                    ),
                    MarkdownHeadingStyle::Setext => MarkdownOwner::setext_heading(
                        heading_index,
                        owner.level,
                        &owner.text,
                        owner.start_byte,
                        owner.end_byte,
                    ),
                };
                heading_index += 1;
                result
            }
            ProjectedOwner::Fence(owner) => {
                let content_range = owner.content_start_byte.zip(owner.content_end_byte);
                let result = MarkdownOwner::code_fence(
                    fence_index,
                    owner.info_string.as_deref(),
                    owner.start_byte,
                    owner.end_byte,
                    content_range,
                );
                fence_index += 1;
                result
            }
        })
        .collect()
}

fn pulldown_heading_style(source: &str, start_byte: usize) -> MarkdownHeadingStyle {
    let line = source
        .get(start_byte..)
        .and_then(|remaining| remaining.split(['\r', '\n']).next())
        .unwrap_or_default();
    if line.trim_start().starts_with('#') {
        MarkdownHeadingStyle::Atx
    } else {
        MarkdownHeadingStyle::Setext
    }
}

fn heading_level(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn owner_container_start(tag: &Tag<'_>) -> bool {
    matches!(tag, Tag::BlockQuote(_) | Tag::List(_) | Tag::Item)
}

fn owner_container_end(tag: TagEnd) -> bool {
    matches!(tag, TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item)
}

pub fn match_markdown_owners(
    template: MarkdownAnalysis,
    destination: MarkdownAnalysis,
) -> markdown_merge::MarkdownOwnerMatchResult {
    match_markdown_owners_with_substrate(template, destination)
}

fn parse_markdown_for_merge(
    source: &str,
    dialect: MarkdownDialect,
) -> ParseResult<MarkdownAnalysis> {
    parse_markdown(source, dialect, None)
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
        parse_markdown_for_merge,
    )
}

pub fn merge_markdown_source_preserving(
    current_source: &str,
    incoming_source: &str,
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

    merge_markdown_source_preserving_with_substrate(
        current_source,
        incoming_source,
        dialect,
        parse_markdown_for_merge,
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
        parse_markdown_for_merge,
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
        parse_markdown_for_merge,
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
        parse_markdown_for_merge,
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
        parse_markdown_for_merge,
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
        parse_markdown_for_merge,
    )
}

pub fn markdown_embedded_families(
    analysis: &MarkdownAnalysis,
) -> Vec<markdown_merge::MarkdownEmbeddedFamilyCandidate> {
    markdown_merge::markdown_embedded_families(analysis)
}
