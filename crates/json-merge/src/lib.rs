use ast_merge::{
    CommentAttachment, CommentRegion, Diagnostic, DiagnosticCategory, DiagnosticSeverity,
    FamilyFeatureProfile, LayoutGap, MergeResult, ParseResult, PolicyReference, PolicySurface,
    match_owner_paths,
};
use tree_haver::{AnalysisHandle, ParserAdapter, ParserRequest};

mod source_preserving;
pub mod typed;
pub use source_preserving::{
    json_semantically_equivalent, merge_json_source_preserving, merge_json_three_way,
};

pub const PACKAGE_NAME: &str = "json-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonDialect {
    Json,
    Jsonc,
    Json5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonRootKind {
    Object,
    Array,
    Scalar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonOwnerKind {
    Member,
    Element,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct JsonOwner {
    pub path: String,
    pub owner_kind: JsonOwnerKind,
    pub match_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fragment: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct JsonOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct JsonOwnerMatchResult {
    pub matched: Vec<JsonOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct JsonAnalysis {
    pub dialect: JsonDialect,
    pub allows_comments: bool,
    pub normalized_source: String,
    pub root_kind: JsonRootKind,
    pub owners: Vec<JsonOwner>,
    pub comment_regions: Vec<CommentRegion>,
    pub layout_gaps: Vec<LayoutGap>,
    pub comment_attachments: Vec<CommentAttachment>,
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
    fn match_owners(
        &self,
        template: &JsonAnalysis,
        destination: &JsonAnalysis,
    ) -> JsonOwnerMatchResult;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonMergeResolution {
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<JsonDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

pub fn json_parse_request(source: &str, dialect: JsonDialect) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: source_preserving::parser_language(dialect).to_string(),
        dialect: Some(match dialect {
            JsonDialect::Json => "json".to_string(),
            JsonDialect::Jsonc => "jsonc".to_string(),
            JsonDialect::Json5 => "json5".to_string(),
        }),
    }
}

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
}

pub fn json_feature_profile() -> JsonFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "json".to_string(),
        supported_dialects: vec!["json".to_string(), "jsonc".to_string(), "json5".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    JsonFeatureProfile {
        family: "json",
        supported_dialects: shared
            .supported_dialects
            .into_iter()
            .map(|dialect| match dialect.as_str() {
                "jsonc" => JsonDialect::Jsonc,
                "json5" => JsonDialect::Json5,
                _ => JsonDialect::Json,
            })
            .collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn parse_json_with_language_pack(
    source: &str,
    dialect: JsonDialect,
) -> ParseResult<JsonAnalysis> {
    parse_json(source, dialect)
}

fn parse_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ParseError,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn parse_json(source: &str, dialect: JsonDialect) -> ParseResult<JsonAnalysis> {
    match source_preserving::analyze_document(source, dialect) {
        Ok(analysis) => ParseResult {
            ok: true,
            diagnostics: vec![],
            analysis: Some(analysis),
            policies: vec![],
        },
        Err(message) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&message)],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn match_json_owners(
    template: &JsonAnalysis,
    destination: &JsonAnalysis,
) -> JsonOwnerMatchResult {
    let result = match_owner_paths(
        &template.owners,
        &destination.owners,
        |owner| owner.path.as_str(),
        |owner| owner.path.as_str(),
    );
    JsonOwnerMatchResult {
        matched: result
            .matched
            .iter()
            .map(|entry| JsonOwnerMatch {
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

pub fn merge_json(
    template_source: &str,
    destination_source: &str,
    dialect: JsonDialect,
) -> MergeResult<String> {
    let mut result = merge_json_source_preserving(template_source, destination_source, dialect);
    if result.ok {
        result.policies.push(destination_wins_array_policy());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        JsonDialect, JsonOwner, JsonOwnerKind, JsonOwnerMatch, JsonRootKind,
        destination_wins_array_policy, match_json_owners, merge_json, parse_json,
    };
    use ast_merge::DiagnosticCategory;

    #[test]
    fn accepts_jsonc_comments() {
        let source = "{\n  // package status\n  \"enabled\": true,\n  /* package name */\n  \"name\": \"structuredmerge\"\n}\n";
        let result = parse_json(source, JsonDialect::Jsonc);

        assert!(result.ok);
        let analysis = result.analysis.unwrap();
        assert!(analysis.allows_comments);
        assert_eq!(analysis.comment_regions.len(), 2);
        assert_eq!(analysis.comment_attachments.len(), 2);
        assert_eq!(analysis.comment_regions[0].normalized_content(), "package status");
        assert_eq!(analysis.comment_regions[1].normalized_content(), "package name");
    }

    #[test]
    fn exposes_shared_layout_gap_ownership_without_changing_source() {
        let source = "{\n  \"first\": true,\n\n  \"second\": false\n}\n";
        let analysis = parse_json(source, JsonDialect::Json).analysis.unwrap();

        assert_eq!(analysis.normalized_source, source);
        assert_eq!(analysis.layout_gaps.len(), 1);
        assert_eq!(analysis.layout_gaps[0].kind, "interstitial");
        assert_eq!(analysis.layout_gaps[0].lines, [""]);
        let controller = analysis.layout_gaps[0].controller_owner_id().unwrap();
        assert_eq!(controller, analysis.comment_attachments[1].owner_id);
    }

    #[test]
    fn accepts_jsonc_trailing_commas_through_the_json5_grammar() {
        let source = "{\n  \"enabled\": true,\n  \"items\": [1, 2,],\n}\n";
        let result = parse_json(source, JsonDialect::Jsonc);

        assert!(result.ok);
        assert_eq!(result.analysis.unwrap().normalized_source, source);
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
                    source_fragment: Some("\"meta\": {\"enabled\": true}".to_string()),
                },
                JsonOwner {
                    path: "/meta/enabled".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("enabled".to_string()),
                    source_fragment: Some("\"enabled\": true".to_string()),
                },
                JsonOwner {
                    path: "/name".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("name".to_string()),
                    source_fragment: Some("\"name\": \"structuredmerge\"".to_string()),
                },
                JsonOwner {
                    path: "/tags".to_string(),
                    owner_kind: JsonOwnerKind::Member,
                    match_key: Some("tags".to_string()),
                    source_fragment: Some("\"tags\": [\"merge\", \"ast\"]".to_string()),
                },
                JsonOwner {
                    path: "/tags/0".to_string(),
                    owner_kind: JsonOwnerKind::Element,
                    match_key: None,
                    source_fragment: Some("\"merge\"".to_string()),
                },
                JsonOwner {
                    path: "/tags/1".to_string(),
                    owner_kind: JsonOwnerKind::Element,
                    match_key: None,
                    source_fragment: Some("\"ast\"".to_string()),
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

    #[test]
    fn resolves_json_merge() {
        let result = merge_json(
            "{\n  \"name\": \"structuredmerge\",\n  \"meta\": {\"enabled\": false, \"mode\": \"template\"},\n  \"tags\": [\"template\"],\n  \"template_only\": 1\n}\n",
            "{\n  \"meta\": {\"enabled\": true},\n  \"tags\": [\"destination\"],\n  \"destination_only\": 2\n}\n",
            JsonDialect::Json,
        );

        assert!(result.ok);
        assert_eq!(
            result.output,
            Some("{\n  \"name\": \"structuredmerge\",\n  \"meta\": {\n    \"enabled\": true,\n    \"mode\": \"template\"\n  },\n  \"tags\": [\"destination\"],\n  \"destination_only\": 2,\n  \"template_only\": 1\n}\n".to_string())
        );
    }

    #[test]
    fn reports_destination_parse_error_during_merge() {
        let result = merge_json("{\"alpha\":1}", "{\"alpha\":", JsonDialect::Json);

        assert!(!result.ok);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::DestinationParseError);
    }

    #[test]
    fn rejects_strict_json_trailing_commas_without_fallback() {
        let result = merge_json("{\"alpha\":1}", "{\"beta\":[1,2,],}", JsonDialect::Json);

        assert!(!result.ok);
        assert!(result.output.is_none());
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::DestinationParseError);
        assert!(result.policies.is_empty());
    }

    #[test]
    fn preserves_destination_array_as_baseline_array_policy() {
        let result = merge_json(
            "{\"items\":[1,2,3],\"meta\":{\"tags\":[\"template\"],\"mode\":\"template\"}}",
            "{\"items\":[9],\"meta\":{\"tags\":[\"destination\"]}}",
            JsonDialect::Json,
        );

        assert!(result.ok);
        assert_eq!(
            result.output.as_deref(),
            Some("{\"items\":[9],\"meta\":{\"tags\":[\"destination\"],\"mode\":\"template\"}}")
        );
        assert_eq!(result.policies, vec![destination_wins_array_policy()]);
    }

    #[test]
    fn does_not_apply_fallback_to_template_trailing_comma_input() {
        let result = merge_json("{\"alpha\":1,}", "{\"beta\":2}", JsonDialect::Json);

        assert!(!result.ok);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::ParseError);
    }

    #[test]
    fn does_not_apply_fallback_to_strict_json_comment_violations() {
        let result =
            merge_json("{\"alpha\":1}", "{\n  // note\n  \"beta\":2\n}", JsonDialect::Json);

        assert!(!result.ok);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::DestinationParseError);
    }
}
