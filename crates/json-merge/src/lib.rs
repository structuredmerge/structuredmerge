use ast_merge::{Diagnostic, DiagnosticCategory, DiagnosticSeverity, MergeResult, ParseResult};
use serde_json::Value;
use tree_haver::{AnalysisHandle, ParserAdapter, ParserRequest};

pub const PACKAGE_NAME: &str = "json-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonDialect {
    Json,
    Jsonc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonRootKind {
    Object,
    Array,
    Scalar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonOwnerKind {
    Member,
    Element,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonOwner {
    pub path: String,
    pub owner_kind: JsonOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonOwnerMatchResult {
    pub matched: Vec<JsonOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonAnalysis {
    pub dialect: JsonDialect,
    pub allows_comments: bool,
    pub normalized_source: String,
    pub root_kind: JsonRootKind,
    pub owners: Vec<JsonOwner>,
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

pub trait JsonStructureAnalyzer {
    fn analyze(&self, source: &str, dialect: JsonDialect) -> ParseResult<JsonAnalysis>;
}

pub trait JsonOwnerMatcher {
    fn match_owners(&self, template: &JsonAnalysis, destination: &JsonAnalysis) -> JsonOwnerMatchResult;
}

pub fn json_parse_request(source: &str, dialect: JsonDialect) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: "json".to_string(),
        dialect: Some(match dialect {
            JsonDialect::Json => "json".to_string(),
            JsonDialect::Jsonc => "jsonc".to_string(),
        }),
    }
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

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn analyze_value(value: &Value, path: &str) -> (JsonRootKind, Vec<JsonOwner>) {
    match value {
        Value::Object(map) => {
            let mut owners = Vec::new();
            for (key, child) in map {
                let child_path = format!("{}/{}", path, escape_pointer_segment(key));
                owners.push(JsonOwner {
                    path: child_path.clone(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some(key.clone()),
                });
                owners.extend(analyze_value(child, &child_path).1);
            }
            (JsonRootKind::Object, owners)
        }
        Value::Array(items) => {
            let mut owners = Vec::new();
            for (index, child) in items.iter().enumerate() {
                let child_path = format!("{}/{}", path, index);
                owners.push(JsonOwner {
                    path: child_path.clone(),
                    owner_kind: JsonOwnerKind::Element,
                    match_key: None,
                });
                owners.extend(analyze_value(child, &child_path).1);
            }
            (JsonRootKind::Array, owners)
        }
        _ => (JsonRootKind::Scalar, Vec::new()),
    }
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

    let decoded = match serde_json::from_str::<Value>(&normalized_source) {
        Ok(value) => value,
        Err(_) => {
            return ParseResult {
                ok: false,
                diagnostics: vec![parse_error("JSON parse failed.")],
                analysis: None,
            };
        }
    };
    let (root_kind, owners) = analyze_value(&decoded, "");

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(JsonAnalysis {
            dialect,
            allows_comments: matches!(dialect, JsonDialect::Jsonc),
            normalized_source,
            root_kind,
            owners,
        }),
    }
}

pub fn match_json_owners(template: &JsonAnalysis, destination: &JsonAnalysis) -> JsonOwnerMatchResult {
    let destination_paths: std::collections::BTreeSet<&str> =
        destination.owners.iter().map(|owner| owner.path.as_str()).collect();
    let template_paths: std::collections::BTreeSet<&str> =
        template.owners.iter().map(|owner| owner.path.as_str()).collect();

    let matched = template
        .owners
        .iter()
        .map(|owner| owner.path.as_str())
        .filter(|path| destination_paths.contains(path))
        .map(|path| JsonOwnerMatch {
            template_path: path.to_string(),
            destination_path: path.to_string(),
        })
        .collect();

    let unmatched_template = template
        .owners
        .iter()
        .map(|owner| owner.path.as_str())
        .filter(|path| !destination_paths.contains(path))
        .map(str::to_string)
        .collect();

    let unmatched_destination = destination
        .owners
        .iter()
        .map(|owner| owner.path.as_str())
        .filter(|path| !template_paths.contains(path))
        .map(str::to_string)
        .collect();

    JsonOwnerMatchResult {
        matched,
        unmatched_template,
        unmatched_destination,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        match_json_owners, parse_json, JsonDialect, JsonOwner, JsonOwnerKind, JsonOwnerMatch,
        JsonRootKind,
    };
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

    #[test]
    fn analyzes_json_structure() {
        let source = "{\n  \"name\": \"structuredmerge\",\n  \"tags\": [\"merge\", \"ast\"],\n  \"meta\": {\"enabled\": true}\n}\n";
        let result = parse_json(source, JsonDialect::Json);
        let analysis = result.analysis.unwrap();

        assert_eq!(analysis.root_kind, JsonRootKind::Object);
        assert_eq!(
            analysis.owners,
            vec![
                JsonOwner {
                    path: "/meta".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("meta".to_string()),
                },
                JsonOwner {
                    path: "/meta/enabled".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("enabled".to_string()),
                },
                JsonOwner {
                    path: "/name".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("name".to_string()),
                },
                JsonOwner {
                    path: "/tags".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("tags".to_string()),
                },
                JsonOwner {
                    path: "/tags/0".to_string(),
                    owner_kind: JsonOwnerKind::Element,
                    match_key: None,
                },
                JsonOwner {
                    path: "/tags/1".to_string(),
                    owner_kind: JsonOwnerKind::Element,
                    match_key: None,
                }
            ]
        );
    }

    #[test]
    fn matches_json_owners_by_path() {
        let template = parse_json(
            "{\n  \"name\": \"structuredmerge\",\n  \"tags\": [\"merge\", \"ast\"],\n  \"meta\": {\"enabled\": true}\n}\n",
            JsonDialect::Json,
        )
        .analysis
        .unwrap();
        let destination = parse_json(
            "{\n  \"name\": \"structuredmerge\",\n  \"tags\": [\"merge\"],\n  \"meta\": {\"enabled\": true},\n  \"extra\": 1\n}\n",
            JsonDialect::Json,
        )
        .analysis
        .unwrap();

        let result = match_json_owners(&template, &destination);

        assert_eq!(
            result.matched,
            vec![
                JsonOwnerMatch {
                    template_path: "/meta".to_string(),
                    destination_path: "/meta".to_string(),
                },
                JsonOwnerMatch {
                    template_path: "/meta/enabled".to_string(),
                    destination_path: "/meta/enabled".to_string(),
                },
                JsonOwnerMatch {
                    template_path: "/name".to_string(),
                    destination_path: "/name".to_string(),
                },
                JsonOwnerMatch {
                    template_path: "/tags".to_string(),
                    destination_path: "/tags".to_string(),
                },
                JsonOwnerMatch {
                    template_path: "/tags/0".to_string(),
                    destination_path: "/tags/0".to_string(),
                },
            ]
        );
        assert_eq!(result.unmatched_template, vec!["/tags/1".to_string()]);
        assert_eq!(result.unmatched_destination, vec!["/extra".to_string()]);
    }
}
