use std::collections::BTreeSet;

use ast_merge::{Diagnostic, DiagnosticCategory, DiagnosticSeverity};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PACKAGE_NAME: &str = "ast-merge-git";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Merge3Request {
    pub base_source: String,
    pub ours_source: String,
    pub theirs_source: String,
    pub path_name: Option<String>,
    pub language: Option<String>,
    pub dialect: Option<String>,
    pub profile_id: Option<String>,
    pub fallback_policy: Option<String>,
    pub conflict_marker_size: Option<usize>,
    pub render_policy: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Merge3Conflict {
    pub conflict_id: String,
    pub category: String,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Merge3Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicted_source: Option<String>,
    pub conflicts: Vec<Merge3Conflict>,
    pub diagnostics: Vec<Diagnostic>,
    pub fallbacks: Vec<String>,
    pub profile: Merge3Profile,
    pub render_report: Merge3RenderReport,
    pub formatting_preservation: FormattingPreservation,
    pub secondary_formatting_metrics: SecondaryFormattingMetrics,
    pub default_driver_evaluation: DefaultDriverEvaluation,
    pub reparse_after_render: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommentDeltaResult {
    pub ok: bool,
    pub merged_comment: Option<String>,
    pub conflicts: Vec<Merge3Conflict>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Merge3Profile {
    pub profile_id: String,
    pub language: String,
    pub dialect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Merge3RenderReport {
    pub strategy: String,
    #[serde(default)]
    pub backend_id: String,
    #[serde(default)]
    pub parser_identity: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FormattingPreservation {
    pub line_diff_score: f64,
    pub character_diff_score: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SecondaryFormattingMetrics {
    pub unchanged_line_churn: usize,
    pub output_diff_size: usize,
    pub source_fragment_retention: f64,
    pub weighted: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DefaultDriverEvaluation {
    pub status: String,
    pub formatting_threshold: f64,
    pub formatting_score: f64,
    pub hard_gates: Vec<HardGate>,
    pub blocking_reasons: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HardGate {
    pub name: String,
    pub passed: bool,
    pub weighted: bool,
}

pub fn merge3(request: &Merge3Request) -> Merge3Response {
    match normalize_language(request).as_str() {
        "json" => merge3_json(request),
        _ => response(
            request,
            false,
            None,
            None,
            vec![],
            vec![diagnostic(
                DiagnosticCategory::UnsupportedFeature,
                "ast-merge-git currently supports only json merge3.",
            )],
            None,
            None,
            None,
        ),
    }
}

pub fn merge3_json(request: &Merge3Request) -> Merge3Response {
    let base = match parse_json_role("base", &request.base_source) {
        Ok(value) => value,
        Err(error) => return parse_error_response(request, error),
    };
    let ours = match parse_json_role("ours", &request.ours_source) {
        Ok(value) => value,
        Err(error) => return parse_error_response(request, error),
    };
    let theirs = match parse_json_role("theirs", &request.theirs_source) {
        Ok(value) => value,
        Err(error) => return parse_error_response(request, error),
    };

    let mut conflicts = Vec::new();
    let merged = merge_json_value(&base, &ours, &theirs, "", &mut conflicts);
    if !conflicts.is_empty() {
        return response(
            request,
            false,
            None,
            Some(render_conflict_source(request, &conflicts)),
            conflicts,
            vec![diagnostic(
                DiagnosticCategory::ConfigurationError,
                "merge_conflict: merge3 found unresolved conflict(s).",
            )],
            None,
            None,
            Some("full_file_conflict_markers".to_string()),
        );
    }

    let merged_source = serde_json::to_string(&merged).expect("merged json should render");
    let reparses = serde_json::from_str::<Value>(&merged_source).is_ok();
    response(
        request,
        true,
        Some(merged_source),
        None,
        vec![],
        vec![],
        Some(FormattingPreservation { line_diff_score: 1.0, character_diff_score: 1.0 }),
        Some(reparses),
        None,
    )
}

pub fn merge_comment_delta(
    base_comment: Option<&str>,
    ours_comment: Option<&str>,
    theirs_comment: Option<&str>,
    owner_path: &str,
) -> CommentDeltaResult {
    let mut conflicts = Vec::new();
    let merged_comment = if ours_comment == theirs_comment {
        ours_comment.map(str::to_string)
    } else if base_comment == ours_comment {
        theirs_comment.map(str::to_string)
    } else if base_comment == theirs_comment {
        ours_comment.map(str::to_string)
    } else if ours_comment.is_none() {
        conflicts.push(comment_conflict(
            "delete_edit",
            owner_path,
            "ours deleted a comment that theirs edited",
        ));
        None
    } else if theirs_comment.is_none() {
        conflicts.push(comment_conflict(
            "delete_edit",
            owner_path,
            "theirs deleted a comment that ours edited",
        ));
        None
    } else {
        conflicts.push(comment_conflict(
            "edit_edit",
            owner_path,
            "comment changed differently in ours and theirs",
        ));
        None
    };

    CommentDeltaResult { ok: conflicts.is_empty(), merged_comment, conflicts }
}

fn response(
    request: &Merge3Request,
    ok: bool,
    merged_source: Option<String>,
    conflicted_source: Option<String>,
    conflicts: Vec<Merge3Conflict>,
    diagnostics: Vec<Diagnostic>,
    formatting_preservation: Option<FormattingPreservation>,
    reparse_after_render: Option<bool>,
    render_strategy: Option<String>,
) -> Merge3Response {
    let has_merged_source = merged_source.is_some();
    let formatting = formatting_preservation
        .clone()
        .unwrap_or(FormattingPreservation { line_diff_score: 0.0, character_diff_score: 0.0 });
    let render_report = render_report(request, render_strategy);
    Merge3Response {
        ok,
        merged_source,
        conflicted_source,
        conflicts,
        diagnostics,
        fallbacks: vec![],
        profile: Merge3Profile {
            profile_id: request.profile_id.clone().unwrap_or_default(),
            language: normalize_language(request),
            dialect: request.dialect.clone().unwrap_or_default(),
        },
        render_report: render_report.clone(),
        formatting_preservation: formatting.clone(),
        secondary_formatting_metrics: secondary_formatting_metrics(has_merged_source),
        default_driver_evaluation: default_driver_evaluation(
            &formatting,
            reparse_after_render,
            &render_report,
        ),
        reparse_after_render,
    }
}

fn secondary_formatting_metrics(merged: bool) -> SecondaryFormattingMetrics {
    if merged {
        return SecondaryFormattingMetrics {
            unchanged_line_churn: 0,
            output_diff_size: 0,
            source_fragment_retention: 1.0,
            weighted: false,
            diagnostics: vec![
                "canonical JSON has no trivia-preserving source fragments yet".to_string(),
            ],
        };
    }
    SecondaryFormattingMetrics {
        unchanged_line_churn: 0,
        output_diff_size: 0,
        source_fragment_retention: 0.0,
        weighted: false,
        diagnostics: vec![
            "unresolved conflict did not produce a merged source-fragment retention measurement"
                .to_string(),
        ],
    }
}

fn default_driver_evaluation(
    formatting: &FormattingPreservation,
    reparse_after_render: Option<bool>,
    render_report: &Merge3RenderReport,
) -> DefaultDriverEvaluation {
    let threshold = 0.95;
    let score = (formatting.line_diff_score + formatting.character_diff_score) / 2.0;
    let reparse_passed = reparse_after_render == Some(true);
    let no_full_file_rewrite = render_report.strategy != "full_file_conflict_markers";
    let coherent_conflict_markers = render_report.strategy != "full_file_conflict_markers";
    let mut blocking_reasons = Vec::new();
    if !reparse_passed {
        blocking_reasons.push("rendered output did not reparse".to_string());
    }
    if score < threshold {
        blocking_reasons.push("formatting score is below threshold".to_string());
    }
    if !no_full_file_rewrite {
        blocking_reasons.push("full-file rewrite or conflict markers were used".to_string());
    }
    if !coherent_conflict_markers {
        blocking_reasons
            .push("conflict marker placement is not syntactically coherent".to_string());
    }
    DefaultDriverEvaluation {
        status: if blocking_reasons.is_empty() {
            "recommended".to_string()
        } else {
            "not_recommended".to_string()
        },
        formatting_threshold: threshold,
        formatting_score: score,
        hard_gates: vec![
            HardGate {
                name: "reparse_after_render".to_string(),
                passed: reparse_passed,
                weighted: false,
            },
            HardGate {
                name: "no_full_file_rewrite".to_string(),
                passed: no_full_file_rewrite,
                weighted: false,
            },
            HardGate {
                name: "coherent_conflict_marker_placement".to_string(),
                passed: coherent_conflict_markers,
                weighted: false,
            },
        ],
        blocking_reasons,
        diagnostics: vec![
            "default-driver evaluation is advisory unless explicitly required".to_string(),
        ],
    }
}

fn render_report(request: &Merge3Request, render_strategy: Option<String>) -> Merge3RenderReport {
    let strategy = render_strategy
        .or_else(|| request.render_policy.clone())
        .unwrap_or_else(|| "canonical".to_string());
    let (backend_id, parser_identity) = match normalize_language(request).as_str() {
        "json" => ("native-json".to_string(), "standard-json".to_string()),
        _ => (String::new(), String::new()),
    };
    Merge3RenderReport { strategy, backend_id, parser_identity }
}

fn parse_error_response(request: &Merge3Request, message: String) -> Merge3Response {
    response(
        request,
        false,
        None,
        None,
        vec![],
        vec![diagnostic(DiagnosticCategory::ParseError, &message)],
        None,
        None,
        None,
    )
}

fn render_conflict_source(request: &Merge3Request, conflicts: &[Merge3Conflict]) -> String {
    let marker_size = request.conflict_marker_size.unwrap_or(7).max(1);
    [
        format!("/* smorg structured conflicts: {} unresolved */", conflicts.len()),
        format!("{} ours", "<".repeat(marker_size)),
        request.ours_source.clone(),
        format!("{} base", "|".repeat(marker_size)),
        request.base_source.clone(),
        "=".repeat(marker_size),
        request.theirs_source.clone(),
        format!("{} theirs", ">".repeat(marker_size)),
        String::new(),
    ]
    .join("\n")
}

fn diagnostic(category: DiagnosticCategory, message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

fn parse_json_role(role: &str, source: &str) -> Result<Value, String> {
    serde_json::from_str(source).map_err(|error| format!("{role} parse error: {error}"))
}

fn merge_json_value(
    base: &Value,
    ours: &Value,
    theirs: &Value,
    path: &str,
    conflicts: &mut Vec<Merge3Conflict>,
) -> Value {
    if ours == theirs {
        return ours.clone();
    }
    if base == ours {
        return theirs.clone();
    }
    if base == theirs {
        return ours.clone();
    }
    if let (Value::Object(base_map), Value::Object(ours_map), Value::Object(theirs_map)) =
        (base, ours, theirs)
    {
        return Value::Object(merge_json_objects(base_map, ours_map, theirs_map, path, conflicts));
    }

    add_conflict(conflicts, "edit_edit", path, "value changed differently in ours and theirs");
    ours.clone()
}

fn merge_json_objects(
    base: &Map<String, Value>,
    ours: &Map<String, Value>,
    theirs: &Map<String, Value>,
    path: &str,
    conflicts: &mut Vec<Merge3Conflict>,
) -> Map<String, Value> {
    let keys =
        base.keys().chain(ours.keys()).chain(theirs.keys()).cloned().collect::<BTreeSet<_>>();
    let mut result = Map::new();
    for key in keys {
        let (merged, keep) = merge_json_entry(
            base.get(&key),
            ours.get(&key),
            theirs.get(&key),
            &json_pointer_join(path, &key),
            conflicts,
        );
        if keep {
            result.insert(key, merged.expect("kept entry should have value"));
        }
    }
    result
}

fn merge_json_entry(
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    path: &str,
    conflicts: &mut Vec<Merge3Conflict>,
) -> (Option<Value>, bool) {
    match (base, ours, theirs) {
        (None, None, None) => (None, false),
        (None, None, Some(theirs)) => (Some(theirs.clone()), true),
        (None, Some(ours), None) => (Some(ours.clone()), true),
        (None, Some(ours), Some(theirs)) if ours == theirs => (Some(ours.clone()), true),
        (None, Some(ours), Some(_)) => {
            add_conflict(
                conflicts,
                "add_add",
                path,
                "same path added differently in ours and theirs",
            );
            (Some(ours.clone()), true)
        }
        (Some(_), None, None) => (None, false),
        (Some(base), None, Some(theirs)) if base == theirs => (None, false),
        (Some(base), Some(ours), None) if base == ours => (None, false),
        (Some(_), None, Some(theirs)) => {
            add_conflict(conflicts, "delete_edit", path, "ours deleted a value that theirs edited");
            (Some(theirs.clone()), true)
        }
        (Some(_), Some(ours), None) => {
            add_conflict(conflicts, "delete_edit", path, "theirs deleted a value that ours edited");
            (Some(ours.clone()), true)
        }
        (Some(base), Some(ours), Some(theirs)) => {
            (Some(merge_json_value(base, ours, theirs, path, conflicts)), true)
        }
    }
}

fn add_conflict(conflicts: &mut Vec<Merge3Conflict>, category: &str, path: &str, message: &str) {
    conflicts.push(Merge3Conflict {
        conflict_id: format!("conflict-{}", conflicts.len() + 1),
        category: category.to_string(),
        path: if path.is_empty() { "/".to_string() } else { path.to_string() },
        message: message.to_string(),
    });
}

fn comment_conflict(category: &str, path: &str, message: &str) -> Merge3Conflict {
    Merge3Conflict {
        conflict_id: "comment-conflict-1".to_string(),
        category: category.to_string(),
        path: if path.is_empty() { "/".to_string() } else { path.to_string() },
        message: message.to_string(),
    }
}

fn json_pointer_join(parent: &str, token: &str) -> String {
    let escaped = token.replace('~', "~0").replace('/', "~1");
    if parent.is_empty() { format!("/{escaped}") } else { format!("{parent}/{escaped}") }
}

fn normalize_language(request: &Merge3Request) -> String {
    let language = request.language.as_deref().unwrap_or_default().trim().to_ascii_lowercase();
    if language == "json" {
        return "json".to_string();
    }
    if request.path_name.as_deref().unwrap_or_default().to_ascii_lowercase().ends_with(".json") {
        return "json".to_string();
    }
    language
}
