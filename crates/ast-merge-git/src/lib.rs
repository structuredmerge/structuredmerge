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
    pub reparse_after_render: Option<bool>,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FormattingPreservation {
    pub line_diff_score: f64,
    pub character_diff_score: f64,
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
        render_report: Merge3RenderReport {
            strategy: render_strategy
                .or_else(|| request.render_policy.clone())
                .unwrap_or_else(|| "canonical".to_string()),
        },
        formatting_preservation: formatting_preservation
            .unwrap_or(FormattingPreservation { line_diff_score: 0.0, character_diff_score: 0.0 }),
        reparse_after_render,
    }
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
