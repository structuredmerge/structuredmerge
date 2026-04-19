use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use ast_merge::{Diagnostic, ParseResult, PolicyReference};
use tree_sitter_language_pack::{PackConfig, init, parse_string, tree_has_error_nodes};

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
pub struct LanguagePackAnalysis {
    pub language: String,
    pub dialect: Option<String>,
    pub root_type: String,
    pub has_error: bool,
    pub backend_ref: BackendReference,
}

impl AnalysisHandle for LanguagePackAnalysis {
    fn kind(&self) -> &'static str {
        "tree-sitter"
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
