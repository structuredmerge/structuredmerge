use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use ast_merge::{Diagnostic, ParseResult, PolicyReference};
use tree_sitter_language_pack::{
    PackConfig, ProcessConfig, init, parse_string, process, tree_has_error_nodes,
};

pub const PACKAGE_NAME: &str = "tree-haver";

pub trait AnalysisHandle {
    fn kind(&self) -> &'static str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserRequest {
    pub source: String,
    pub language: String,
    pub dialect: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReference {
    pub id: String,
    pub family: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterInfo {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

pub trait ParserAdapter<TAnalysis: AnalysisHandle> {
    fn info(&self) -> AdapterInfo;
    fn parse(&self, request: &ParserRequest) -> ParseResult<TAnalysis>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserDiagnostics {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRequest {
    pub source: String,
    pub language: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessStructureItem {
    pub kind: String,
    pub name: Option<String>,
    pub span: ProcessSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessImportInfo {
    pub source: String,
    pub items: Vec<String>,
    pub span: ProcessSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessDiagnostic {
    pub message: String,
    pub severity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePackAnalysis {
    pub language: String,
    pub dialect: Option<String>,
    pub root_type: String,
    pub has_error: bool,
    pub backend_ref: BackendReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePackProcessAnalysis {
    pub language: String,
    pub structure: Vec<ProcessStructureItem>,
    pub imports: Vec<ProcessImportInfo>,
    pub diagnostics: Vec<ProcessDiagnostic>,
    pub backend_ref: BackendReference,
}

impl AnalysisHandle for LanguagePackAnalysis {
    fn kind(&self) -> &'static str {
        "tree-sitter"
    }
}

impl AnalysisHandle for LanguagePackProcessAnalysis {
    fn kind(&self) -> &'static str {
        "tree-sitter-process"
    }
}

pub fn kreuzberg_language_pack_backend() -> BackendReference {
    BackendReference {
        id: "kreuzberg-language-pack".to_string(),
        family: "tree-sitter".to_string(),
    }
}

pub fn language_pack_adapter_info() -> AdapterInfo {
    AdapterInfo {
        backend: "kreuzberg-language-pack".to_string(),
        backend_ref: Some(kreuzberg_language_pack_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

fn ensure_language_pack_language(language: &str) -> Result<(), String> {
    static INITIALIZED_LANGUAGES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let initialized_languages = INITIALIZED_LANGUAGES.get_or_init(|| Mutex::new(HashSet::new()));
    let mut initialized_languages = initialized_languages
        .lock()
        .map_err(|_| "language-pack initialization lock poisoned".to_string())?;

    if initialized_languages.contains(language) {
        return Ok(());
    }

    init(&PackConfig {
        cache_dir: None,
        languages: Some(vec![language.to_string()]),
        groups: None,
    })
    .map_err(|error| error.to_string())?;

    initialized_languages.insert(language.to_string());
    Ok(())
}

pub fn parse_with_language_pack(request: &ParserRequest) -> ParseResult<LanguagePackAnalysis> {
    if let Err(error) = ensure_language_pack_language(&request.language) {
        return ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::UnsupportedFeature,
                message: error,
                path: None,
                review: None,
            }],
            analysis: None,
            policies: vec![],
        };
    }

    match parse_string(&request.language, request.source.as_bytes()) {
        Ok(tree) => {
            let has_error = tree_has_error_nodes(&tree);
            if has_error {
                ParseResult {
                    ok: false,
                    diagnostics: vec![Diagnostic {
                        severity: ast_merge::DiagnosticSeverity::Error,
                        category: ast_merge::DiagnosticCategory::ParseError,
                        message: format!(
                            "tree-sitter-language-pack reported syntax errors for {}.",
                            request.language
                        ),
                        path: None,
                        review: None,
                    }],
                    analysis: None,
                    policies: vec![],
                }
            } else {
                ParseResult {
                    ok: true,
                    diagnostics: vec![],
                    analysis: Some(LanguagePackAnalysis {
                        language: request.language.clone(),
                        dialect: request.dialect.clone(),
                        root_type: tree.root_node().kind().to_string(),
                        has_error: false,
                        backend_ref: kreuzberg_language_pack_backend(),
                    }),
                    policies: vec![],
                }
            }
        }
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::UnsupportedFeature,
                message: error.to_string(),
                path: None,
                review: None,
            }],
            analysis: None,
            policies: vec![],
        },
    }
}

fn structure_kind_name(kind: &tree_sitter_language_pack::StructureKind) -> String {
    match kind {
        tree_sitter_language_pack::StructureKind::Function => "function".to_string(),
        tree_sitter_language_pack::StructureKind::Method => "method".to_string(),
        tree_sitter_language_pack::StructureKind::Class => "class".to_string(),
        tree_sitter_language_pack::StructureKind::Struct => "struct".to_string(),
        tree_sitter_language_pack::StructureKind::Interface => "interface".to_string(),
        tree_sitter_language_pack::StructureKind::Enum => "enum".to_string(),
        tree_sitter_language_pack::StructureKind::Module => "module".to_string(),
        tree_sitter_language_pack::StructureKind::Trait => "trait".to_string(),
        tree_sitter_language_pack::StructureKind::Impl => "impl".to_string(),
        tree_sitter_language_pack::StructureKind::Namespace => "namespace".to_string(),
        tree_sitter_language_pack::StructureKind::Other(other) => other.clone(),
    }
}

fn normalize_typescript_import(item: &tree_sitter_language_pack::ImportInfo) -> ProcessImportInfo {
    let source_match = item
        .source
        .split("from")
        .nth(1)
        .and_then(|tail| tail.split(['"', '\'']).nth(1))
        .or_else(|| item.source.split(['"', '\'']).nth(1))
        .unwrap_or(item.source.as_str())
        .to_string();
    let items = item
        .source
        .split('{')
        .nth(1)
        .and_then(|tail| tail.split('}').next())
        .map(|raw| {
            raw.split(',')
                .map(|part| part.replace("type", "").trim().to_string())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ProcessImportInfo {
        source: source_match,
        items,
        span: ProcessSpan {
            start_byte: item.span.start_byte,
            end_byte: item.span.end_byte,
            start_row: item.span.start_line,
            start_col: item.span.start_column,
            end_row: item.span.end_line,
            end_col: item.span.end_column,
        },
    }
}

fn diagnostic_severity_name(severity: &tree_sitter_language_pack::DiagnosticSeverity) -> String {
    match severity {
        tree_sitter_language_pack::DiagnosticSeverity::Error => "error".to_string(),
        tree_sitter_language_pack::DiagnosticSeverity::Warning => "warning".to_string(),
        tree_sitter_language_pack::DiagnosticSeverity::Info => "info".to_string(),
    }
}

pub fn process_with_language_pack(
    request: &ProcessRequest,
) -> ParseResult<LanguagePackProcessAnalysis> {
    if let Err(error) = ensure_language_pack_language(&request.language) {
        return ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::UnsupportedFeature,
                message: error,
                path: None,
                review: None,
            }],
            analysis: None,
            policies: vec![],
        };
    }

    match process(&request.source, &ProcessConfig::new(&request.language).all()) {
        Ok(result) => ParseResult {
            ok: true,
            diagnostics: vec![],
            analysis: Some(LanguagePackProcessAnalysis {
                language: result.language,
                structure: result
                    .structure
                    .iter()
                    .map(|item| ProcessStructureItem {
                        kind: structure_kind_name(&item.kind),
                        name: item.name.clone(),
                        span: ProcessSpan {
                            start_byte: item.span.start_byte,
                            end_byte: item.span.end_byte,
                            start_row: item.span.start_line,
                            start_col: item.span.start_column,
                            end_row: item.span.end_line,
                            end_col: item.span.end_column,
                        },
                    })
                    .collect(),
                imports: result
                    .imports
                    .iter()
                    .map(|item| {
                        if request.language == "typescript" {
                            normalize_typescript_import(item)
                        } else {
                            ProcessImportInfo {
                                source: item.source.clone(),
                                items: item.items.clone(),
                                span: ProcessSpan {
                                    start_byte: item.span.start_byte,
                                    end_byte: item.span.end_byte,
                                    start_row: item.span.start_line,
                                    start_col: item.span.start_column,
                                    end_row: item.span.end_line,
                                    end_col: item.span.end_column,
                                },
                            }
                        }
                    })
                    .collect(),
                diagnostics: result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| ProcessDiagnostic {
                        message: diagnostic.message.clone(),
                        severity: diagnostic_severity_name(&diagnostic.severity),
                    })
                    .collect(),
                backend_ref: kreuzberg_language_pack_backend(),
            }),
            policies: vec![],
        },
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: ast_merge::DiagnosticSeverity::Error,
                category: ast_merge::DiagnosticCategory::UnsupportedFeature,
                message: error.to_string(),
                path: None,
                review: None,
            }],
            analysis: None,
            policies: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json_with_language_pack() {
        let result = parse_with_language_pack(&ParserRequest {
            source: "{\"alpha\":1}".to_string(),
            language: "json".to_string(),
            dialect: None,
        });

        assert!(result.ok);
        assert_eq!(
            result.analysis,
            Some(LanguagePackAnalysis {
                language: "json".to_string(),
                dialect: None,
                root_type: "document".to_string(),
                has_error: false,
                backend_ref: kreuzberg_language_pack_backend(),
            })
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn reports_invalid_json_with_language_pack() {
        let result = parse_with_language_pack(&ParserRequest {
            source: "{\"alpha\":1,}".to_string(),
            language: "json".to_string(),
            dialect: None,
        });

        assert!(!result.ok);
        assert!(result.analysis.is_none());
        assert_eq!(
            result.diagnostics[0].message,
            "tree-sitter-language-pack reported syntax errors for json."
        );
    }
}
