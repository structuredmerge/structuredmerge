use std::collections::HashMap;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, DelegatedChildOperation,
    Diagnostic, DiagnosticCategory, DiagnosticSeverity, DiscoveredSurface, FamilyFeatureProfile,
    MergeResult, ParseResult, SurfaceOwnerKind, SurfaceOwnerRef,
};
use tree_haver::{ParserRequest, parse_with_language_pack};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownOwner {
    pub path: String,
    pub owner_kind: MarkdownOwnerKind,
    pub match_key: String,
    pub level: Option<usize>,
    pub info_string: Option<String>,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MarkdownSection {
    path: String,
    text: String,
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

pub fn collect_markdown_owners(source: &str) -> Vec<MarkdownOwner> {
    let normalized = normalize_markdown_source(source);
    let lines = normalized.split('\n').collect::<Vec<_>>();
    let mut owners = Vec::new();
    let mut heading_index = 0usize;
    let mut code_fence_index = 0usize;
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];

        if let Some((hashes, title)) = parse_heading(line) {
            let level = hashes.len();
            owners.push(MarkdownOwner {
                path: format!("/heading/{heading_index}"),
                owner_kind: MarkdownOwnerKind::Heading,
                match_key: format!("h{level}:{}", slugify(title)),
                level: Some(level),
                info_string: None,
            });
            heading_index += 1;
            index += 1;
            continue;
        }

        if let Some((marker_char, marker_length, info_string)) = parse_code_fence(line) {
            let info_string = info_string.unwrap_or_default();
            owners.push(MarkdownOwner {
                path: format!("/code_fence/{code_fence_index}"),
                owner_kind: MarkdownOwnerKind::CodeFence,
                match_key: format!(
                    "fence:{}",
                    if info_string.is_empty() { "plain" } else { info_string.as_str() }
                ),
                level: None,
                info_string: if info_string.is_empty() { None } else { Some(info_string) },
            });
            code_fence_index += 1;

            index += 1;
            while index < lines.len() {
                if is_code_fence_close(lines[index], marker_char, marker_length) {
                    break;
                }
                index += 1;
            }
            index += 1;
            continue;
        }

        index += 1;
    }

    owners
}

fn parse_heading(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_end();
    let hashes_len = trimmed.chars().take_while(|character| *character == '#').count();
    if !(1..=6).contains(&hashes_len) {
        return None;
    }

    let remainder = trimmed.get(hashes_len..)?.trim_start();
    if remainder.is_empty() {
        return None;
    }

    let title = remainder.trim_end_matches('#').trim();
    if title.is_empty() {
        return None;
    }

    Some((&trimmed[..hashes_len], title))
}

fn parse_code_fence(line: &str) -> Option<(char, usize, Option<String>)> {
    let trimmed = line.trim();
    let marker_char = if trimmed.starts_with("```") {
        '`'
    } else if trimmed.starts_with("~~~") {
        '~'
    } else {
        return None;
    };

    let marker_length = trimmed.chars().take_while(|character| *character == marker_char).count();
    let info = trimmed[marker_length..].trim();
    let info =
        info.split_whitespace().next().filter(|item| !item.is_empty()).map(|item| item.to_string());

    Some((marker_char, marker_length, info))
}

fn is_code_fence_close(line: &str, marker_char: char, marker_length: usize) -> bool {
    let trimmed = line.trim();
    let count = trimmed.chars().take_while(|character| *character == marker_char).count();
    count >= marker_length && trimmed.chars().all(|character| character == marker_char)
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
            let syntax = parse_with_language_pack(&ParserRequest {
                source: source.to_string(),
                language: "markdown".to_string(),
                dialect: Some("markdown".to_string()),
            });
            if !syntax.ok {
                return ParseResult {
                    ok: false,
                    diagnostics: syntax.diagnostics,
                    analysis: None,
                    policies: vec![],
                };
            }
        }
    }

    let normalized_source = normalize_markdown_source(source);
    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(MarkdownAnalysis {
            dialect,
            normalized_source: normalized_source.clone(),
            root_kind: MarkdownRootKind::Document,
            owners: collect_markdown_owners(&normalized_source),
        }),
        policies: vec![],
    }
}

pub fn match_markdown_owners(
    template: MarkdownAnalysis,
    destination: MarkdownAnalysis,
) -> MarkdownOwnerMatchResult {
    let destination_paths = destination
        .owners
        .iter()
        .map(|owner| owner.path.clone())
        .collect::<std::collections::HashSet<_>>();
    let template_paths = template
        .owners
        .iter()
        .map(|owner| owner.path.clone())
        .collect::<std::collections::HashSet<_>>();

    MarkdownOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_paths.contains(&owner.path))
            .map(|owner| MarkdownOwnerMatch {
                template_path: owner.path.clone(),
                destination_path: owner.path.clone(),
            })
            .collect(),
        unmatched_template: template
            .owners
            .iter()
            .map(|owner| owner.path.clone())
            .filter(|path| !destination_paths.contains(path))
            .collect(),
        unmatched_destination: destination
            .owners
            .iter()
            .map(|owner| owner.path.clone())
            .filter(|path| !template_paths.contains(path))
            .collect(),
    }
}

fn markdown_owner_start_indices(source: &str) -> std::collections::HashMap<String, usize> {
    let normalized = normalize_markdown_source(source);
    let lines = normalized.split('\n').collect::<Vec<_>>();
    let mut starts = std::collections::HashMap::new();
    let mut heading_index = 0usize;
    let mut code_fence_index = 0usize;
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];
        if parse_heading(line).is_some() {
            starts.insert(format!("/heading/{heading_index}"), index);
            heading_index += 1;
            index += 1;
            continue;
        }

        if let Some((marker_char, marker_length, _)) = parse_code_fence(line) {
            starts.insert(format!("/code_fence/{code_fence_index}"), index);
            code_fence_index += 1;
            index += 1;
            while index < lines.len() {
                if is_code_fence_close(lines[index], marker_char, marker_length) {
                    break;
                }
                index += 1;
            }
            index += 1;
            continue;
        }

        index += 1;
    }

    starts
}

fn collect_markdown_sections(source: &str, owners: &[MarkdownOwner]) -> Vec<MarkdownSection> {
    let normalized = normalize_markdown_source(source);
    let lines = normalized.split('\n').collect::<Vec<_>>();
    let starts = markdown_owner_start_indices(&normalized);
    let mut ordered = owners
        .iter()
        .filter_map(|owner| starts.get(&owner.path).map(|start| (owner, *start)))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(_, start)| *start);

    ordered
        .iter()
        .enumerate()
        .map(|(index, (owner, start))| {
            let end_exclusive =
                ordered.get(index + 1).map(|(_, start)| *start).unwrap_or(lines.len());
            MarkdownSection {
                path: owner.path.clone(),
                text: lines[*start..end_exclusive].join("\n").trim().to_string(),
            }
        })
        .collect()
}

fn markdown_fence_ranges(source: &str) -> HashMap<String, (usize, usize)> {
    let lines =
        normalize_markdown_source(source).split('\n').map(str::to_string).collect::<Vec<_>>();
    let mut ranges = HashMap::new();
    let mut code_fence_index = 0usize;
    let mut index = 0usize;

    while index < lines.len() {
        let line = &lines[index];
        if let Some((marker_char, marker_length, _)) = parse_code_fence(line) {
            let mut end = index;
            let mut cursor = index + 1;
            while cursor < lines.len() {
                if is_code_fence_close(&lines[cursor], marker_char, marker_length) {
                    end = cursor;
                    break;
                }
                if cursor == lines.len() - 1 {
                    end = cursor;
                }
                cursor += 1;
            }
            ranges.insert(format!("/code_fence/{code_fence_index}"), (index, end));
            code_fence_index += 1;
            index = end + 1;
            continue;
        }
        index += 1;
    }

    ranges
}

pub fn apply_markdown_delegated_child_outputs(
    source: &str,
    operations: &[DelegatedChildOperation],
    apply_plan: &ast_merge::DelegatedChildApplyPlan,
    applied_children: &[AppliedChildOutput],
) -> MergeResult<String> {
    let mut lines =
        normalize_markdown_source(source).split('\n').map(str::to_string).collect::<Vec<_>>();
    let ranges = markdown_fence_ranges(source);
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
        let Some((start, end)) = ranges.get(&operation.surface.owner.address).copied() else {
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

    replacements.sort_by(|left, right| right.0.cmp(&left.0));
    for (start, end, output) in replacements {
        let replacement_lines = if output.is_empty() {
            Vec::new()
        } else {
            output.trim_end_matches('\n').split('\n').map(str::to_string).collect::<Vec<_>>()
        };
        lines.splice(start + 1..end, replacement_lines);
    }

    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(format!("{}\n", lines.join("\n").trim_end_matches('\n'))),
        policies: vec![],
    }
}

pub fn merge_markdown_with_nested_outputs(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    nested_outputs: &[NestedChildOutput],
    backend: MarkdownBackend,
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
            merge_parent: || merge_markdown(template_source, destination_source, dialect, backend),
            discover_operations: |merged_output| {
                let analysis = parse_markdown_with_backend(merged_output, dialect, backend);
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

                apply_markdown_delegated_child_outputs(
                    merged_output,
                    operations,
                    apply_plan,
                    &translated,
                )
            },
        },
    )
}

pub fn merge_markdown(
    template_source: &str,
    destination_source: &str,
    dialect: MarkdownDialect,
    backend: MarkdownBackend,
) -> MergeResult<String> {
    let template = parse_markdown_with_backend(template_source, dialect, backend);
    if !template.ok || template.analysis.is_none() {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_markdown_with_backend(destination_source, dialect, backend);
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
            .map(|section| section.text.clone()),
    );

    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(format!("{}\n", merged_sections.join("\n\n").trim())),
        policies: vec![],
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
