use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, ParseResult,
};
use pulldown_cmark::Parser;
use tree_haver::{ParserRequest, current_backend_id, parse_with_language_pack};

pub const PACKAGE_NAME: &str = "markdown-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownDialect {
    Markdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownBackend {
    PulldownCmark,
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MarkdownEmbeddedFamilyCandidate {
    pub path: String,
    pub language: String,
    pub family: String,
    pub dialect: String,
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

fn validate_native_markdown(source: &str) {
    let parser = Parser::new(source);
    for _ in parser {}
}

fn resolve_backend(backend: Option<MarkdownBackend>) -> MarkdownBackend {
    if let Some(backend) = backend {
        return backend;
    }

    match current_backend_id().as_deref() {
        Some("kreuzberg-language-pack") => MarkdownBackend::KreuzbergLanguagePack,
        _ => MarkdownBackend::PulldownCmark,
    }
}

pub fn markdown_feature_profile() -> MarkdownFeatureProfile {
    MarkdownFeatureProfile {
        family: "markdown",
        supported_dialects: vec![MarkdownDialect::Markdown],
    }
}

pub fn available_markdown_backends() -> Vec<MarkdownBackend> {
    vec![MarkdownBackend::PulldownCmark, MarkdownBackend::KreuzbergLanguagePack]
}

pub fn markdown_backend_feature_profile(backend: MarkdownBackend) -> MarkdownBackendFeatureProfile {
    MarkdownBackendFeatureProfile {
        family: "markdown",
        supported_dialects: vec![MarkdownDialect::Markdown],
        backend: match backend {
            MarkdownBackend::PulldownCmark => "pulldown-cmark".to_string(),
            MarkdownBackend::KreuzbergLanguagePack => "kreuzberg-language-pack".to_string(),
        },
    }
}

pub fn markdown_plan_context() -> ConformanceFamilyPlanContext {
    markdown_plan_context_with_backend(MarkdownBackend::PulldownCmark)
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
            supports_dialects: backend != MarkdownBackend::KreuzbergLanguagePack,
            supported_policies: vec![],
        }),
    }
}

pub fn parse_markdown(source: &str, dialect: MarkdownDialect) -> ParseResult<MarkdownAnalysis> {
    parse_markdown_with_backend(source, dialect, resolve_backend(None))
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
        MarkdownBackend::PulldownCmark => validate_native_markdown(source),
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
