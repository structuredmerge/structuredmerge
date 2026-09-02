use std::collections::{HashMap, HashSet};

use ast_merge::{
    AppliedDelegatedChildOutput, CommentAttachment, CommentRegion, ConformanceFamilyPlanContext,
    ConformanceFeatureProfileView, ConformanceManifestReviewState,
    ConformanceManifestReviewStateEnvelope, DelegatedChildGroupReviewState,
    DelegatedChildOperation, Diagnostic, DiagnosticCategory, DiagnosticSeverity, DiscoveredSurface,
    FamilyFeatureProfile, LayoutGap, MergeResult, ParseResult, RenderFragment, ReviewReplayBundle,
    ReviewReplayBundleEnvelope, SourceFragment, SourceRenderPlan, SourceRevision, SurfaceOwnerKind,
    SurfaceOwnerRef, augment_normalized_tree_comments, execute_reviewed_nested_merge,
    import_conformance_manifest_review_state_envelope, import_review_replay_bundle_envelope,
    match_owner_paths, merge_sequence_order_constraints, render_source_plan,
};
use tree_haver::{NormalizedTreeNode, ParserRequest, parse_normalized_with_language_pack};

pub const PACKAGE_NAME: &str = "markdown-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownDialect {
    Markdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownBackend {
    KreuzbergLanguagePack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownRootKind {
    Document,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownOwnerKind {
    Heading,
    CodeFence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownHeadingStyle {
    Atx,
    Setext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownOwner {
    pub path: String,
    pub owner_kind: MarkdownOwnerKind,
    pub match_key: String,
    pub level: Option<usize>,
    pub heading_text: Option<String>,
    pub heading_style: Option<MarkdownHeadingStyle>,
    pub info_string: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub content_start_byte: Option<usize>,
    pub content_end_byte: Option<usize>,
}

impl MarkdownOwner {
    pub fn heading(
        index: usize,
        level: usize,
        title: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        Self {
            path: format!("/heading/{index}"),
            owner_kind: MarkdownOwnerKind::Heading,
            match_key: format!("h{level}:{}", slugify(title)),
            level: Some(level),
            heading_text: Some(title.to_string()),
            heading_style: Some(MarkdownHeadingStyle::Atx),
            info_string: None,
            start_byte,
            end_byte,
            content_start_byte: None,
            content_end_byte: None,
        }
    }

    pub fn setext_heading(
        index: usize,
        level: usize,
        title: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        let mut owner = Self::heading(index, level, title, start_byte, end_byte);
        owner.heading_style = Some(MarkdownHeadingStyle::Setext);
        owner
    }

    pub fn code_fence(
        index: usize,
        info_string: Option<&str>,
        start_byte: usize,
        end_byte: usize,
        content_range: Option<(usize, usize)>,
    ) -> Self {
        let info_string = info_string.filter(|value| !value.is_empty()).map(str::to_string);
        Self {
            path: format!("/code_fence/{index}"),
            owner_kind: MarkdownOwnerKind::CodeFence,
            match_key: format!("fence:{}", info_string.as_deref().unwrap_or("plain")),
            level: None,
            heading_text: None,
            heading_style: None,
            info_string,
            start_byte,
            end_byte,
            content_start_byte: content_range.map(|range| range.0),
            content_end_byte: content_range.map(|range| range.1),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownOwnerMatchResult {
    pub matched: Vec<MarkdownOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownAnalysis {
    pub dialect: MarkdownDialect,
    pub normalized_source: String,
    pub root_kind: MarkdownRootKind,
    pub owners: Vec<MarkdownOwner>,
    pub comment_regions: Vec<CommentRegion>,
    pub layout_gaps: Vec<LayoutGap>,
    pub comment_attachments: Vec<CommentAttachment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MarkdownSection {
    path: String,
    text: String,
    insertion_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourcePreservingMarkdownSection {
    id: String,
    start_line: usize,
    end_line: usize,
    source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourcePreservingMarkdownDocument {
    source: String,
    sections: Vec<SourcePreservingMarkdownSection>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MarkdownEmbeddedFamilyCandidate {
    pub path: String,
    pub language: String,
    pub family: String,
    pub dialect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AppliedChildOutput {
    pub operation_id: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NestedChildOutput {
    pub surface_address: String,
    pub output: String,
}

impl tree_haver::AnalysisHandle for MarkdownAnalysis {
    fn kind(&self) -> &'static str {
        "markdown"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<MarkdownDialect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownBackendFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<MarkdownDialect>,
    pub backend: String,
    pub backend_ref: tree_haver::BackendReference,
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

fn configuration_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ConfigurationError,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn normalize_markdown_source(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn include_trailing_line_ending(source: &str, end_byte: usize) -> usize {
    match source.as_bytes().get(end_byte..) {
        Some([b'\r', b'\n', ..]) => end_byte + 2,
        Some([b'\n' | b'\r', ..]) => end_byte + 1,
        _ => end_byte,
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in value.trim().to_lowercase().chars() {
        let mapped = if character.is_ascii_alphanumeric() { Some(character) } else { None };

        if let Some(character) = mapped {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "section".to_string() } else { slug }
}

fn collect_tree_sitter_markdown_owners(nodes: &[NormalizedTreeNode]) -> Vec<MarkdownOwner> {
    let nodes_by_id = nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let mut owners = Vec::new();
    let mut heading_index = 0usize;
    let mut code_fence_index = 0usize;

    for node in nodes {
        if node.kind == "atx_heading" && document_section_node(node, &nodes_by_id) {
            let Some(marker) = child_of_kind(node, &nodes_by_id, |kind| {
                kind.starts_with("atx_h") && kind.ends_with("_marker")
            }) else {
                continue;
            };
            let Some(level) = atx_heading_level(&marker.kind) else {
                continue;
            };
            let Some(inline) = child_of_kind(node, &nodes_by_id, |kind| kind == "inline") else {
                continue;
            };
            let title = inline.source_fragment.trim();
            if title.is_empty() {
                continue;
            }
            owners.push(MarkdownOwner::heading(
                heading_index,
                level,
                title,
                node.span.range.start_byte,
                node.span.range.end_byte,
            ));
            heading_index += 1;
            continue;
        }

        if setext_heading_level(&node.kind).is_some() && document_section_node(node, &nodes_by_id) {
            let Some(level) = setext_heading_level(&node.kind) else {
                continue;
            };
            let Some(inline) = child_of_kind(node, &nodes_by_id, |kind| kind == "inline") else {
                continue;
            };
            let title = inline.source_fragment.trim();
            if title.is_empty() {
                continue;
            }
            owners.push(MarkdownOwner::setext_heading(
                heading_index,
                level,
                title,
                node.span.range.start_byte,
                node.span.range.end_byte,
            ));
            heading_index += 1;
            continue;
        }

        if node.kind == "fenced_code_block" {
            let info_string = child_of_kind(node, &nodes_by_id, |kind| kind == "info_string")
                .and_then(|child| child.source_fragment.split_whitespace().next())
                .unwrap_or_default()
                .to_string();
            let content = child_of_kind(node, &nodes_by_id, |kind| kind == "code_fence_content");
            owners.push(MarkdownOwner::code_fence(
                code_fence_index,
                Some(&info_string),
                node.span.range.start_byte,
                node.span.range.end_byte,
                content.map(|child| (child.span.range.start_byte, child.span.range.end_byte)),
            ));
            code_fence_index += 1;
        }
    }

    owners.sort_by_key(|owner| owner.start_byte);
    owners
}

fn child_of_kind<'a>(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &'a NormalizedTreeNode>,
    predicate: impl Fn(&str) -> bool,
) -> Option<&'a NormalizedTreeNode> {
    node.child_ids
        .iter()
        .filter_map(|id| nodes.get(id.as_str()).copied())
        .find(|child| predicate(&child.kind))
}

fn atx_heading_level(marker_kind: &str) -> Option<usize> {
    marker_kind.strip_prefix("atx_h")?.strip_suffix("_marker")?.parse().ok()
}

fn setext_heading_level(node_kind: &str) -> Option<usize> {
    if !node_kind.starts_with("setext_") || !node_kind.ends_with("_heading") {
        return None;
    }
    if node_kind.contains("h1") {
        Some(1)
    } else if node_kind.contains("h2") {
        Some(2)
    } else {
        None
    }
}

fn document_section_node(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> bool {
    let mut parent_id = node.parent_id.as_deref();
    while let Some(id) = parent_id {
        let Some(parent) = nodes.get(id).copied() else {
            return false;
        };
        if parent.kind == "document" {
            return true;
        }
        if parent.kind != "section" {
            return false;
        }
        parent_id = parent.parent_id.as_deref();
    }
    false
}

pub fn markdown_feature_profile() -> MarkdownFeatureProfile {
    MarkdownFeatureProfile {
        family: "markdown",
        supported_dialects: vec![MarkdownDialect::Markdown],
    }
}

pub fn available_markdown_backends() -> Vec<MarkdownBackend> {
    vec![MarkdownBackend::KreuzbergLanguagePack]
}

pub fn markdown_backend_feature_profile(backend: MarkdownBackend) -> MarkdownBackendFeatureProfile {
    let backend_ref = tree_haver::kreuzberg_language_pack_backend();
    MarkdownBackendFeatureProfile {
        family: "markdown",
        supported_dialects: vec![MarkdownDialect::Markdown],
        backend: match backend {
            MarkdownBackend::KreuzbergLanguagePack => backend_ref.id.clone(),
        },
        backend_ref,
    }
}

pub fn markdown_plan_context() -> ConformanceFamilyPlanContext {
    markdown_plan_context_with_backend(MarkdownBackend::KreuzbergLanguagePack)
}

pub fn markdown_plan_context_with_backend(
    backend: MarkdownBackend,
) -> ConformanceFamilyPlanContext {
    let backend_profile = markdown_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: "markdown".to_string(),
            supported_dialects: vec!["markdown".to_string()],
            supported_policies: vec![],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: backend_profile.backend,
            supports_dialects: false,
            supported_policies: vec![],
        }),
        merge_engine: None,
    }
}

pub fn parse_markdown(source: &str, dialect: MarkdownDialect) -> ParseResult<MarkdownAnalysis> {
    parse_markdown_with_backend(source, dialect, MarkdownBackend::KreuzbergLanguagePack)
}

pub fn parse_markdown_with_backend(
    source: &str,
    dialect: MarkdownDialect,
    backend: MarkdownBackend,
) -> ParseResult<MarkdownAnalysis> {
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

    match backend {
        MarkdownBackend::KreuzbergLanguagePack => {
            let syntax = parse_normalized_with_language_pack(&ParserRequest {
                source: source.to_string(),
                language: "markdown".to_string(),
                dialect: Some("markdown".to_string()),
            });
            if !syntax.ok {
                return ParseResult {
                    ok: false,
                    diagnostics: syntax
                        .diagnostics
                        .iter()
                        .map(|message| Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            category: DiagnosticCategory::ParseError,
                            message: message.clone(),
                            path: None,
                            review: None,
                        })
                        .collect(),
                    analysis: None,
                    policies: vec![],
                };
            }

            let augmentation = match augment_normalized_tree_comments(
                source,
                &syntax.root_id,
                &syntax.nodes,
                "html_comment",
                normalize_markdown_comment,
            ) {
                Ok(augmentation) => augmentation,
                Err(error) => {
                    return ParseResult {
                        ok: false,
                        diagnostics: vec![configuration_error(&error)],
                        analysis: None,
                        policies: vec![],
                    };
                }
            };
            let owners = collect_tree_sitter_markdown_owners(&syntax.nodes);
            ParseResult {
                ok: true,
                diagnostics: vec![],
                analysis: Some(MarkdownAnalysis {
                    dialect,
                    normalized_source: source.to_string(),
                    root_kind: MarkdownRootKind::Document,
                    owners,
                    comment_regions: augmentation.regions,
                    layout_gaps: augmentation.gaps,
                    comment_attachments: augmentation.attachments,
                }),
                policies: vec![],
            }
        }
    }
}

fn normalize_markdown_comment(text: &str) -> String {
    text.trim()
        .strip_prefix("<!--")
        .unwrap_or(text.trim())
        .trim_end_matches("-->")
        .trim()
        .to_string()
}

pub fn match_markdown_owners(
    template: MarkdownAnalysis,
    destination: MarkdownAnalysis,
) -> MarkdownOwnerMatchResult {
    let result = match_owner_paths(
        &template.owners,
        &destination.owners,
        |owner| owner.path.as_str(),
        |owner| owner.path.as_str(),
    );
    MarkdownOwnerMatchResult {
        matched: result
            .matched
            .iter()
            .map(|entry| MarkdownOwnerMatch {
                template_path: template.owners[entry.template_index].path.clone(),
                destination_path: destination.owners[entry.destination_index].path.clone(),
            })
            .collect(),
        unmatched_template: result
            .unmatched_template
            .iter()
            .map(|index| template.owners[*index].path.clone())
            .collect(),
        unmatched_destination: result
            .unmatched_destination
            .iter()
            .map(|index| destination.owners[*index].path.clone())
            .collect(),
    }
}

fn collect_markdown_sections(source: &str, owners: &[MarkdownOwner]) -> Vec<MarkdownSection> {
    let mut ordered = owners.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|owner| owner.start_byte);

    ordered
        .iter()
        .enumerate()
        .filter_map(|(index, owner)| {
            let reconstruction_end =
                ordered.get(index + 1).map(|next| next.start_byte).unwrap_or(source.len());
            let insertion_end = if owner.owner_kind == MarkdownOwnerKind::CodeFence {
                owner.end_byte
            } else {
                reconstruction_end
            };
            Some(MarkdownSection {
                path: owner.path.clone(),
                text: source.get(owner.start_byte..reconstruction_end)?.to_string(),
                insertion_text: source.get(owner.start_byte..insertion_end)?.to_string(),
            })
        })
        .collect()
}

pub fn apply_markdown_delegated_child_outputs(
    source: &str,
    operations: &[DelegatedChildOperation],
    apply_plan: &ast_merge::DelegatedChildApplyPlan,
    applied_children: &[AppliedChildOutput],
) -> MergeResult<String> {
    let parsed = parse_markdown(source, MarkdownDialect::Markdown);
    let Some(analysis) = parsed.analysis else {
        return MergeResult {
            ok: false,
            diagnostics: parsed.diagnostics,
            output: None,
            policies: vec![],
        };
    };
    apply_markdown_delegated_child_outputs_with_analysis(
        source,
        &analysis,
        operations,
        apply_plan,
        applied_children,
    )
}

fn apply_markdown_delegated_child_outputs_with_analysis(
    source: &str,
    analysis: &MarkdownAnalysis,
    operations: &[DelegatedChildOperation],
    apply_plan: &ast_merge::DelegatedChildApplyPlan,
    applied_children: &[AppliedChildOutput],
) -> MergeResult<String> {
    let ranges = analysis
        .owners
        .iter()
        .filter(|owner| owner.owner_kind == MarkdownOwnerKind::CodeFence)
        .filter_map(|owner| {
            Some((owner.path.as_str(), (owner.content_start_byte?, owner.content_end_byte?)))
        })
        .collect::<HashMap<_, _>>();
    let operations_by_id = operations
        .iter()
        .map(|operation| (operation.operation_id.clone(), operation))
        .collect::<HashMap<_, _>>();
    let outputs_by_id = applied_children
        .iter()
        .map(|entry| (entry.operation_id.clone(), entry.output.clone()))
        .collect::<HashMap<_, _>>();

    let mut replacements = Vec::new();
    for entry in &apply_plan.entries {
        let Some(operation) = operations_by_id.get(&entry.delegated_group.child_operation_id)
        else {
            continue;
        };
        let Some(output) = outputs_by_id.get(&entry.delegated_group.child_operation_id) else {
            continue;
        };
        let Some((start, end)) = ranges.get(operation.surface.owner.address.as_str()).copied()
        else {
            return MergeResult {
                ok: false,
                diagnostics: vec![configuration_error(&format!(
                    "missing fenced-code range for {}",
                    operation.surface.owner.address
                ))],
                output: None,
                policies: vec![],
            };
        };
        replacements.push((start, end, output.clone()));
    }

    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0));
    let mut output_source = source.to_string();
    for (start, end, output) in replacements {
        let replacement = fenced_body_replacement(source, start, end, &output);
        output_source.replace_range(start..end, &replacement);
    }

    MergeResult { ok: true, diagnostics: vec![], output: Some(output_source), policies: vec![] }
}

fn fenced_body_replacement(source: &str, start: usize, end: usize, output: &str) -> String {
    if output.is_empty() || output.ends_with('\n') || output.ends_with('\r') {
        return output.to_string();
    }

    let line_ending =
        source.get(start..end).filter(|body| body.contains("\r\n")).map_or("\n", |_| "\r\n");
    format!("{output}{line_ending}")
}

pub fn merge_markdown_with_nested_outputs(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    nested_outputs: &[NestedChildOutput],
) -> MergeResult<String> {
    merge_markdown_with_nested_outputs_with_backend(
        template_source,
        destination_source,
        dialect,
        nested_outputs,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_nested_outputs_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    nested_outputs: &[NestedChildOutput],
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_nested_outputs_with_parser(
        template_source,
        destination_source,
        dialect,
        nested_outputs,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_nested_outputs_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    nested_outputs: &[NestedChildOutput],
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    ast_merge::execute_nested_merge(
        &nested_outputs
            .iter()
            .map(|nested_output| ast_merge::DelegatedChildSurfaceOutput {
                surface_address: nested_output.surface_address.clone(),
                output: nested_output.output.clone(),
            })
            .collect::<Vec<_>>(),
        &ast_merge::DelegatedChildOutputResolutionOptions {
            default_family: "markdown".to_string(),
            request_id_prefix: "nested_markdown_child".to_string(),
        },
        ast_merge::NestedMergeExecutionCallbacks {
            merge_parent: || {
                merge_markdown_using_parser(template_source, destination_source, dialect, &parser)
            },
            discover_operations: |merged_output| {
                let analysis = parser(merged_output, dialect);
                if !analysis.ok || analysis.analysis.is_none() {
                    return ast_merge::NestedMergeDiscoveryResult {
                        ok: false,
                        diagnostics: analysis.diagnostics,
                        operations: None,
                    };
                }

                ast_merge::NestedMergeDiscoveryResult {
                    ok: true,
                    diagnostics: vec![],
                    operations: Some(markdown_delegated_child_operations(
                        analysis.analysis.as_ref().expect("analysis"),
                        "markdown-document-0",
                    )),
                }
            },
            apply_resolved_outputs: |merged_output, operations, apply_plan, applied_children| {
                let translated = applied_children
                    .iter()
                    .map(|entry| AppliedChildOutput {
                        operation_id: entry.operation_id.clone(),
                        output: entry.output.clone(),
                    })
                    .collect::<Vec<_>>();

                let parsed = parser(merged_output, dialect);
                let Some(analysis) = parsed.analysis else {
                    return MergeResult {
                        ok: false,
                        diagnostics: parsed.diagnostics,
                        output: None,
                        policies: vec![],
                    };
                };
                apply_markdown_delegated_child_outputs_with_analysis(
                    merged_output,
                    &analysis,
                    operations,
                    apply_plan,
                    &translated,
                )
            },
        },
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &DelegatedChildGroupReviewState,
    applied_children: &[AppliedChildOutput],
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_with_backend(
        template_source,
        destination_source,
        dialect,
        review_state,
        applied_children,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &DelegatedChildGroupReviewState,
    applied_children: &[AppliedChildOutput],
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_with_parser(
        template_source,
        destination_source,
        dialect,
        review_state,
        applied_children,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &DelegatedChildGroupReviewState,
    applied_children: &[AppliedChildOutput],
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    let resolved_children = applied_children
        .iter()
        .map(|child| AppliedDelegatedChildOutput {
            operation_id: child.operation_id.clone(),
            output: child.output.clone(),
        })
        .collect::<Vec<_>>();

    execute_reviewed_nested_merge(
        review_state,
        "markdown",
        &resolved_children,
        ast_merge::NestedMergeExecutionCallbacks {
            merge_parent: || {
                merge_markdown_using_parser(template_source, destination_source, dialect, &parser)
            },
            discover_operations: |merged_output| {
                let analysis = parser(merged_output, dialect);
                if !analysis.ok || analysis.analysis.is_none() {
                    return ast_merge::NestedMergeDiscoveryResult {
                        ok: false,
                        diagnostics: analysis.diagnostics,
                        operations: None,
                    };
                }

                ast_merge::NestedMergeDiscoveryResult {
                    ok: true,
                    diagnostics: vec![],
                    operations: Some(markdown_delegated_child_operations(
                        analysis.analysis.as_ref().expect("analysis"),
                        "markdown-document-0",
                    )),
                }
            },
            apply_resolved_outputs: |merged_output, operations, apply_plan, applied_children| {
                let translated = applied_children
                    .iter()
                    .map(|entry| AppliedChildOutput {
                        operation_id: entry.operation_id.clone(),
                        output: entry.output.clone(),
                    })
                    .collect::<Vec<_>>();

                let parsed = parser(merged_output, dialect);
                let Some(analysis) = parsed.analysis else {
                    return MergeResult {
                        ok: false,
                        diagnostics: parsed.diagnostics,
                        output: None,
                        policies: vec![],
                    };
                };
                apply_markdown_delegated_child_outputs_with_analysis(
                    merged_output,
                    &analysis,
                    operations,
                    apply_plan,
                    &translated,
                )
            },
        },
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    replay_bundle: &ReviewReplayBundle,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_backend(
        template_source,
        destination_source,
        dialect,
        replay_bundle,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    replay_bundle: &ReviewReplayBundle,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_parser(
        template_source,
        destination_source,
        dialect,
        replay_bundle,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    replay_bundle: &ReviewReplayBundle,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    if let Some(execution) = replay_bundle
        .reviewed_nested_executions
        .iter()
        .find(|execution| execution.family == "markdown")
    {
        let applied_children = execution
            .applied_children
            .iter()
            .map(|child| AppliedChildOutput {
                operation_id: child.operation_id.clone(),
                output: child.output.clone(),
            })
            .collect::<Vec<_>>();
        return merge_markdown_with_reviewed_nested_outputs_with_parser(
            template_source,
            destination_source,
            dialect,
            &execution.review_state,
            &applied_children,
            parser,
        );
    }

    MergeResult {
        ok: false,
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ConfigurationError,
            message:
                "review replay bundle does not include a reviewed nested execution for markdown."
                    .to_string(),
            path: None,
            review: None,
        }],
        output: None,
        policies: vec![],
    }
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &ConformanceManifestReviewState,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_review_state_with_backend(
        template_source,
        destination_source,
        dialect,
        review_state,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &ConformanceManifestReviewState,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_review_state_with_parser(
        template_source,
        destination_source,
        dialect,
        review_state,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    review_state: &ConformanceManifestReviewState,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    if let Some(execution) = review_state
        .reviewed_nested_executions
        .iter()
        .find(|execution| execution.family == "markdown")
    {
        let applied_children = execution
            .applied_children
            .iter()
            .map(|child| AppliedChildOutput {
                operation_id: child.operation_id.clone(),
                output: child.output.clone(),
            })
            .collect::<Vec<_>>();
        return merge_markdown_with_reviewed_nested_outputs_with_parser(
            template_source,
            destination_source,
            dialect,
            &execution.review_state,
            &applied_children,
            parser,
        );
    }

    MergeResult {
        ok: false,
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ConfigurationError,
            message: "review state does not include a reviewed nested execution for markdown."
                .to_string(),
            path: None,
            review: None,
        }],
        output: None,
        policies: vec![],
    }
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ReviewReplayBundleEnvelope,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_backend(
        template_source,
        destination_source,
        dialect,
        envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ReviewReplayBundleEnvelope,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_parser(
        template_source,
        destination_source,
        dialect,
        envelope,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_envelope_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ReviewReplayBundleEnvelope,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    match import_review_replay_bundle_envelope(envelope) {
        Ok(bundle) => merge_markdown_with_reviewed_nested_outputs_from_replay_bundle_with_parser(
            template_source,
            destination_source,
            dialect,
            &bundle,
            parser,
        ),
        Err(error) => MergeResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: match error.category {
                    ast_merge::ReviewTransportImportErrorCategory::KindMismatch => {
                        DiagnosticCategory::KindMismatch
                    }
                    ast_merge::ReviewTransportImportErrorCategory::UnsupportedVersion => {
                        DiagnosticCategory::UnsupportedVersion
                    }
                },
                message: error.message,
                path: None,
                review: None,
            }],
            output: None,
            policies: vec![],
        },
    }
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ConformanceManifestReviewStateEnvelope,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_backend(
        template_source,
        destination_source,
        dialect,
        envelope,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ConformanceManifestReviewStateEnvelope,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_parser(
        template_source,
        destination_source,
        dialect,
        envelope,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_reviewed_nested_outputs_from_review_state_envelope_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    envelope: &ConformanceManifestReviewStateEnvelope,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    match import_conformance_manifest_review_state_envelope(envelope) {
        Ok(state) => merge_markdown_with_reviewed_nested_outputs_from_review_state_with_parser(
            template_source,
            destination_source,
            dialect,
            &state,
            parser,
        ),
        Err(error) => MergeResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: match error.category {
                    ast_merge::ReviewTransportImportErrorCategory::KindMismatch => {
                        DiagnosticCategory::KindMismatch
                    }
                    ast_merge::ReviewTransportImportErrorCategory::UnsupportedVersion => {
                        DiagnosticCategory::UnsupportedVersion
                    }
                },
                message: error.message,
                path: None,
                review: None,
            }],
            output: None,
            policies: vec![],
        },
    }
}

pub fn merge_markdown(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
) -> MergeResult<String> {
    merge_markdown_with_backend(
        template_source,
        destination_source,
        dialect,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_with_parser(
        template_source,
        destination_source,
        dialect,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    merge_markdown_using_parser(template_source, destination_source, dialect, &parser)
}

pub fn merge_markdown_source_preserving(
    current_source: &str,
    incoming_source: &str,
    dialect: MarkdownDialect,
) -> MergeResult<String> {
    merge_markdown_source_preserving_with_backend(
        current_source,
        incoming_source,
        dialect,
        MarkdownBackend::KreuzbergLanguagePack,
    )
}

pub fn merge_markdown_source_preserving_with_backend(
    current_source: &str,
    incoming_source: &str,
    dialect: MarkdownDialect,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    merge_markdown_source_preserving_with_parser(
        current_source,
        incoming_source,
        dialect,
        |source, parse_dialect| parse_markdown_with_backend(source, parse_dialect, backend),
    )
}

pub fn merge_markdown_source_preserving_with_parser(
    current_source: &str,
    incoming_source: &str,
    dialect: MarkdownDialect,
    parser: impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    let current =
        match parse_source_preserving_document(current_source, dialect, "current", &parser) {
            Ok(document) => document,
            Err(diagnostic) => return failed_markdown_merge(diagnostic),
        };
    let incoming =
        match parse_source_preserving_document(incoming_source, dialect, "incoming", &parser) {
            Ok(document) => document,
            Err(diagnostic) => return failed_markdown_merge(diagnostic),
        };

    let current_ids = current.sections.iter().map(|section| section.id.clone()).collect::<Vec<_>>();
    let incoming_ids =
        incoming.sections.iter().map(|section| section.id.clone()).collect::<Vec<_>>();
    let mut selected = current_ids.clone();
    let mut selected_set = selected.iter().cloned().collect::<HashSet<_>>();
    for node_id in &incoming_ids {
        if selected_set.insert(node_id.clone()) {
            selected.push(node_id.clone());
        }
    }
    let ordered = match merge_sequence_order_constraints(
        &[current_ids.clone(), incoming_ids.clone()],
        &selected,
    ) {
        Ok(ordered) => ordered,
        Err(error) => {
            return failed_markdown_merge(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::MergeConflict,
                message: format!("Markdown section order cannot be proven: {error:?}"),
                path: Some("<document>".to_string()),
                review: None,
            });
        }
    };

    let current_by_id = current
        .sections
        .iter()
        .map(|section| (section.id.as_str(), section))
        .collect::<HashMap<_, _>>();
    let incoming_by_id = incoming
        .sections
        .iter()
        .map(|section| (section.id.as_str(), section))
        .collect::<HashMap<_, _>>();
    let fragments = ordered
        .iter()
        .map(|node_id| {
            let (revision, section) = current_by_id
                .get(node_id.as_str())
                .map(|section| (SourceRevision::Ours, *section))
                .or_else(|| {
                    incoming_by_id
                        .get(node_id.as_str())
                        .map(|section| (SourceRevision::Theirs, *section))
                })
                .expect("selected Markdown section");
            RenderFragment::Source(SourceFragment {
                revision,
                start_line: section.start_line,
                end_line: section.end_line,
                metadata: HashMap::from([
                    ("section_id".to_string(), serde_json::json!(section.id)),
                    ("source_preserved".to_string(), serde_json::json!(true)),
                ]),
            })
        })
        .collect::<Vec<_>>();
    let plan = match SourceRenderPlan::new(
        HashMap::from([
            (SourceRevision::Ours, current.source.clone()),
            (SourceRevision::Theirs, incoming.source.clone()),
        ]),
        fragments,
    ) {
        Ok(plan) => plan,
        Err(error) => return failed_render_merge(error.to_string()),
    };
    let rendered = match render_source_plan(&plan) {
        Ok(rendered) => rendered,
        Err(error) => return failed_render_merge(error.to_string()),
    };

    let verified =
        match parse_source_preserving_document(&rendered.content, dialect, "output", &parser) {
            Ok(document) => document,
            Err(diagnostic) => return failed_markdown_merge(diagnostic),
        };
    let expected_sections = ordered
        .iter()
        .map(|node_id| {
            current_by_id
                .get(node_id.as_str())
                .copied()
                .or_else(|| incoming_by_id.get(node_id.as_str()).copied())
                .expect("selected Markdown section")
        })
        .collect::<Vec<_>>();
    if verified.sections.len() != expected_sections.len()
        || verified
            .sections
            .iter()
            .zip(expected_sections)
            .any(|(actual, expected)| actual.id != expected.id || actual.source != expected.source)
    {
        return failed_render_merge(
            "rendered Markdown sections did not retain their selected source bytes".to_string(),
        );
    }

    MergeResult { ok: true, diagnostics: vec![], output: Some(rendered.content), policies: vec![] }
}

fn parse_source_preserving_document(
    source: &str,
    dialect: MarkdownDialect,
    role: &str,
    parser: &impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> Result<SourcePreservingMarkdownDocument, Diagnostic> {
    if source.contains('\0') {
        return Err(unsupported_markdown_document(role, "source contains a NUL byte"));
    }
    let parsed = parser(source, dialect);
    let Some(analysis) = parsed.analysis.filter(|_| parsed.ok) else {
        return Err(parsed.diagnostics.into_iter().next().unwrap_or_else(|| Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::ParseError,
            message: format!("Markdown {role} source could not be parsed"),
            path: None,
            review: None,
        }));
    };
    if analysis.normalized_source != source {
        return Err(unsupported_markdown_document(
            role,
            "parser did not retain the exact source bytes",
        ));
    }
    let headings = analysis
        .owners
        .iter()
        .filter(|owner| owner.owner_kind == MarkdownOwnerKind::Heading)
        .collect::<Vec<_>>();
    if headings.is_empty() {
        return Err(unsupported_markdown_document(role, "document has no heading sections"));
    }
    if headings.iter().any(|owner| owner.heading_style != Some(MarkdownHeadingStyle::Atx)) {
        return Err(unsupported_markdown_document(role, "document contains a setext heading"));
    }
    let levels = headings.iter().filter_map(|owner| owner.level).collect::<HashSet<_>>();
    if levels.len() != 1 {
        return Err(unsupported_markdown_document(
            role,
            "document contains a nested heading hierarchy",
        ));
    }
    if headings.first().is_none_or(|owner| owner.start_byte != 0) {
        return Err(unsupported_markdown_document(
            role,
            "source exists before the first heading section",
        ));
    }
    if headings.windows(2).any(|pair| pair[0].start_byte >= pair[1].start_byte) {
        return Err(unsupported_markdown_document(role, "heading ranges overlap"));
    }

    let line_count = source.split_inclusive('\n').count();
    let mut seen = HashSet::new();
    let mut sections = Vec::with_capacity(headings.len());
    for (index, owner) in headings.iter().enumerate() {
        let level = owner.level.expect("heading level");
        let title = owner.heading_text.as_deref().unwrap_or_default();
        let id = serde_json::json!([level, title]).to_string();
        if !seen.insert(id.clone()) {
            return Err(Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::Ambiguity,
                message: format!("Markdown {role} source contains duplicate heading {title:?}"),
                path: None,
                review: None,
            });
        }
        let end_byte = headings.get(index + 1).map_or(source.len(), |next| next.start_byte);
        let Some(section_source) = source.get(owner.start_byte..end_byte) else {
            return Err(unsupported_markdown_document(role, "heading byte range is invalid"));
        };
        let Some(start_line) = source_line_at_byte(source, owner.start_byte) else {
            return Err(unsupported_markdown_document(
                role,
                "heading does not start at a source line boundary",
            ));
        };
        let end_line = headings
            .get(index + 1)
            .and_then(|next| source_line_at_byte(source, next.start_byte))
            .map_or(line_count, |next_line| next_line - 1);
        sections.push(SourcePreservingMarkdownSection {
            id,
            start_line,
            end_line,
            source: section_source.to_string(),
        });
    }

    Ok(SourcePreservingMarkdownDocument { source: source.to_string(), sections })
}

fn source_line_at_byte(source: &str, offset: usize) -> Option<usize> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }
    if offset > 0 && source.as_bytes()[offset - 1] != b'\n' {
        return None;
    }
    Some(source.as_bytes()[..offset].iter().filter(|byte| **byte == b'\n').count() + 1)
}

fn unsupported_markdown_document(role: &str, message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::UnsupportedFeature,
        message: format!("Markdown {role} {message}"),
        path: None,
        review: None,
    }
}

fn failed_render_merge(message: String) -> MergeResult<String> {
    failed_markdown_merge(Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ConfigurationError,
        message,
        path: None,
        review: None,
    })
}

fn failed_markdown_merge(diagnostic: Diagnostic) -> MergeResult<String> {
    MergeResult { ok: false, diagnostics: vec![diagnostic], output: None, policies: vec![] }
}

fn merge_markdown_using_parser(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    parser: &impl Fn(&str, MarkdownDialect) -> ParseResult<MarkdownAnalysis>,
) -> MergeResult<String> {
    let template = parser(template_source, dialect);
    if !template.ok || template.analysis.is_none() {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parser(destination_source, dialect);
    if !destination.ok || destination.analysis.is_none() {
        return MergeResult {
            ok: false,
            diagnostics: destination.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let template_analysis = template.analysis.expect("template analysis");
    let destination_analysis = destination.analysis.expect("destination analysis");
    let destination_sections = collect_markdown_sections(
        &destination_analysis.normalized_source,
        &destination_analysis.owners,
    );
    let template_sections =
        collect_markdown_sections(&template_analysis.normalized_source, &template_analysis.owners);
    let destination_paths = destination_sections
        .iter()
        .map(|section| section.path.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut merged_sections = destination_sections
        .iter()
        .map(|section| section.text.clone())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    merged_sections.extend(
        template_sections
            .iter()
            .filter(|section| {
                !destination_paths.contains(&section.path) && !section.text.is_empty()
            })
            .map(|section| section.insertion_text.clone()),
    );

    let output = join_markdown_sections(&merged_sections);
    MergeResult { ok: true, diagnostics: vec![], output: Some(output), policies: vec![] }
}

fn join_markdown_sections(sections: &[String]) -> String {
    let mut output = String::new();
    for section in sections {
        if !output.is_empty() && !ends_with_blank_line(&output) && !starts_with_line_ending(section)
        {
            let line_ending = preferred_line_ending(&output);
            if !ends_with_line_ending(&output) {
                output.push_str(line_ending);
            }
            output.push_str(line_ending);
        }
        output.push_str(section);
    }
    output
}

fn starts_with_line_ending(value: &str) -> bool {
    value.starts_with(['\n', '\r'])
}

fn ends_with_line_ending(value: &str) -> bool {
    value.ends_with(['\n', '\r'])
}

fn ends_with_blank_line(value: &str) -> bool {
    value.ends_with("\n\n") || value.ends_with("\r\n\r\n") || value.ends_with("\r\r")
}

fn preferred_line_ending(value: &str) -> &'static str {
    if value.contains("\r\n") {
        "\r\n"
    } else if value.contains('\r') {
        "\r"
    } else {
        "\n"
    }
}

fn code_fence_family(info_string: Option<&str>) -> Option<&'static str> {
    match info_string.unwrap_or_default().to_lowercase().as_str() {
        "ts" | "typescript" => Some("typescript"),
        "rust" | "rs" => Some("rust"),
        "go" => Some("go"),
        "json" | "jsonc" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    }
}

fn code_fence_dialect(info_string: Option<&str>, family: Option<&str>) -> Option<String> {
    let language = info_string.unwrap_or_default().to_lowercase();
    match family {
        Some("typescript") => Some("typescript".to_string()),
        Some("rust") => Some("rust".to_string()),
        Some("go") => Some("go".to_string()),
        Some("json") => Some(if language == "jsonc" { "jsonc" } else { "json" }.to_string()),
        Some("yaml") => Some("yaml".to_string()),
        Some("toml") => Some("toml".to_string()),
        _ => None,
    }
}

pub fn markdown_embedded_families(
    analysis: &MarkdownAnalysis,
) -> Vec<MarkdownEmbeddedFamilyCandidate> {
    analysis
        .owners
        .iter()
        .filter_map(|owner| {
            if owner.owner_kind != MarkdownOwnerKind::CodeFence {
                return None;
            }

            let language = owner.info_string.clone()?;
            let family = code_fence_family(Some(&language))?;
            let dialect = code_fence_dialect(Some(&language), Some(family))?;
            Some(MarkdownEmbeddedFamilyCandidate {
                path: owner.path.clone(),
                language,
                family: family.to_string(),
                dialect,
            })
        })
        .collect()
}

pub fn markdown_discovered_surfaces(analysis: &MarkdownAnalysis) -> Vec<DiscoveredSurface> {
    markdown_embedded_families(analysis)
        .into_iter()
        .map(|candidate| DiscoveredSurface {
            surface_kind: "markdown_fenced_code_block".to_string(),
            declared_language: Some(candidate.language.clone()),
            effective_language: candidate.dialect.clone(),
            address: format!("document[0] > fenced_code_block[{}]", candidate.path),
            parent_address: Some("document[0]".to_string()),
            span: None,
            owner: SurfaceOwnerRef {
                kind: SurfaceOwnerKind::StructuralOwner,
                address: candidate.path.clone(),
            },
            reconstruction_strategy: "portable_write".to_string(),
            metadata: std::collections::HashMap::from([
                ("family".to_string(), serde_json::Value::String(candidate.family)),
                ("dialect".to_string(), serde_json::Value::String(candidate.dialect)),
                ("path".to_string(), serde_json::Value::String(candidate.path)),
            ]),
        })
        .collect()
}

pub fn markdown_delegated_child_operations(
    analysis: &MarkdownAnalysis,
    parent_operation_id: &str,
) -> Vec<DelegatedChildOperation> {
    markdown_discovered_surfaces(analysis)
        .into_iter()
        .enumerate()
        .map(|(index, surface)| DelegatedChildOperation {
            operation_id: format!("markdown-fence-{index}"),
            parent_operation_id: parent_operation_id.to_string(),
            requested_strategy: "delegate_child_surface".to_string(),
            language_chain: vec!["markdown".to_string(), surface.effective_language.clone()],
            surface,
        })
        .collect()
}
