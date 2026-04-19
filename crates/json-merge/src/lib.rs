use ast_merge::{Diagnostic, DiagnosticCategory, DiagnosticSeverity, MergeResult, ParseResult};
use serde_json::Value;
use tree_haver::{AnalysisHandle, ParserAdapter};

pub const PACKAGE_NAME: &str = "json-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonDialect {
    Json,
    Jsonc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonAnalysis {
    pub dialect: JsonDialect,
    pub allows_comments: bool,
    pub normalized_source: String,
}

impl AnalysisHandle for JsonAnalysis {
    fn kind(&self) -> &'static str {
        "json"
    }
}

pub trait JsonMerger {
    fn merge(&self, template: &JsonAnalysis, destination: &JsonAnalysis) -> MergeResult<String>;
}

pub trait JsonParserAdapter: ParserAdapter<JsonAnalysis> {}

pub trait JsonAnalyzer {
    fn parse(&self, source: &str, dialect: JsonDialect) -> ParseResult<JsonAnalysis>;
}

fn parse_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ParseError,
        message: message.to_string(),
        path: None,
    }
}

fn detect_trailing_comma(source: &str) -> bool {
    let chars: Vec<char> = source.chars().collect();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;
    let mut index = 0usize;

    while index < chars.len() {
        let char = chars[index];
        let next = chars.get(index + 1).copied();

        if in_line_comment {
            if char == '\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }

        if in_block_comment {
            if char == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if char == '\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if char == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if char == '"' {
            in_string = true;
            index += 1;
            continue;
        }

        if char == '/' && next == Some('/') {
            in_line_comment = true;
            index += 2;
            continue;
        }

        if char == '/' && next == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }

        if char == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if matches!(chars.get(lookahead), Some(']') | Some('}')) {
                return true;
            }
        }

        index += 1;
    }

    false
}

fn strip_json_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut result = String::new();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;
    let mut index = 0usize;

    while index < chars.len() {
        let char = chars[index];
        let next = chars.get(index + 1).copied();

        if in_line_comment {
            if char == '\n' {
                in_line_comment = false;
                result.push('\n');
            }
            index += 1;
            continue;
        }

        if in_block_comment {
            if char == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if in_string {
            result.push(char);
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if char == '\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if char == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if char == '"' {
            in_string = true;
            result.push(char);
            index += 1;
            continue;
        }

        if char == '/' && next == Some('/') {
            in_line_comment = true;
            index += 2;
            continue;
        }

        if char == '/' && next == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }

        result.push(char);
        index += 1;
    }

    result
}

pub fn parse_json(source: &str, dialect: JsonDialect) -> ParseResult<JsonAnalysis> {
    if detect_trailing_comma(source) {
        return ParseResult {
            ok: false,
            diagnostics: vec![parse_error("Trailing commas are not supported.")],
            analysis: None,
        };
    }

    let stripped = strip_json_comments(source);
    let normalized_source = match dialect {
        JsonDialect::Json => {
            if stripped != source {
                return ParseResult {
                    ok: false,
                    diagnostics: vec![parse_error("Comments are not supported in strict JSON.")],
                    analysis: None,
                };
            }
            source.to_string()
        }
        JsonDialect::Jsonc => stripped,
    };

    if serde_json::from_str::<Value>(&normalized_source).is_err() {
        return ParseResult {
            ok: false,
            diagnostics: vec![parse_error("JSON parse failed.")],
            analysis: None,
        };
    }

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(JsonAnalysis {
            dialect,
            allows_comments: matches!(dialect, JsonDialect::Jsonc),
            normalized_source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_json, JsonDialect};
    use ast_merge::DiagnosticCategory;

    #[test]
    fn accepts_jsonc_comments() {
        let source = "{\n  // package status\n  \"enabled\": true,\n  /* package name */\n  \"name\": \"structuredmerge\"\n}\n";
        let result = parse_json(source, JsonDialect::Jsonc);

        assert!(result.ok);
        assert!(result.analysis.is_some());
        assert!(result.analysis.unwrap().allows_comments);
    }

    #[test]
    fn rejects_trailing_commas() {
        let source = "{\n  \"enabled\": true,\n  \"items\": [1, 2,],\n}\n";
        let result = parse_json(source, JsonDialect::Jsonc);

        assert!(!result.ok);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::ParseError);
    }
}
