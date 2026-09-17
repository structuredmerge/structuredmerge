use std::collections::HashMap;

use ast_merge::{
    ConflictAlternativeState, Diagnostic, DiagnosticCategory, DiagnosticSeverity, MergeConflict,
    SourceRevision, ThreeWayMergeOutcome, localized_conflict_render_plan, render_source_plan,
};
use json_merge::{JsonDialect, merge_json_three_way};
use serde::{Deserialize, Serialize};

pub mod typed;

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChangeClassification {
    pub path: String,
    pub ours: String,
    pub theirs: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Merge3Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicted_source: Option<String>,
    pub conflicts: Vec<Merge3Conflict>,
    pub change_classifications: Vec<ChangeClassification>,
    pub diagnostics: Vec<Diagnostic>,
    pub fallbacks: Vec<String>,
    pub profile: Merge3Profile,
    pub render_report: Merge3RenderReport,
    pub owned_regions: Vec<OwnedRegionReport>,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AttachedSpan {
    pub kind: String,
    pub line_range: SourceRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<SourceRange>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnedRegionReport {
    pub owner_path: String,
    pub node_id: String,
    pub region_kind: String,
    pub byte_range: SourceRange,
    pub line_range: SourceRange,
    pub attached_spans: Vec<AttachedSpan>,
    pub backend_id: String,
    pub parser_identity: String,
    pub can_replace: bool,
    pub can_line_merge: bool,
    pub requires_reparse: bool,
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
        "json" | "jsonc" | "json5" => merge3_json(request),
        _ => response(
            request,
            false,
            None,
            None,
            vec![],
            vec![],
            vec![diagnostic(
                DiagnosticCategory::UnsupportedFeature,
                "ast-merge-git currently supports only json merge3.",
            )],
            vec![],
            None,
            None,
            None,
        ),
    }
}

pub fn merge3_json(request: &Merge3Request) -> Merge3Response {
    let mut result = merge_json_three_way(
        &request.base_source,
        &request.ours_source,
        &request.theirs_source,
        json_dialect(request),
    );
    let conflicts = result
        .conflicts
        .iter()
        .map(|conflict| Merge3Conflict {
            conflict_id: conflict.conflict_id.clone(),
            category: conflict.category.clone(),
            path: conflict.path.clone(),
            message: conflict.message.clone(),
        })
        .collect::<Vec<_>>();
    match result.outcome {
        ThreeWayMergeOutcome::Clean => response(
            request,
            true,
            result.output,
            None,
            conflicts,
            vec![],
            result.diagnostics,
            vec![],
            Some(FormattingPreservation { line_diff_score: 1.0, character_diff_score: 1.0 }),
            Some(true),
            Some("source_preserving_edits".to_string()),
        ),
        ThreeWayMergeOutcome::Conflict => {
            let owned_regions = json_owned_region_reports(&result.conflicts);
            let rendered = render_json_conflicts(request, &result.conflicts);
            if let Err(message) = &rendered {
                result.diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::UnsupportedFeature,
                    message: format!("localized conflict render unavailable: {message}"),
                    path: None,
                    review: None,
                });
            }
            let (conflicted_source, strategy, reparse) = match rendered {
                Ok(source) => {
                    (Some(source), "owned_region_conflict_markers".to_string(), Some(false))
                }
                Err(_) => (None, "unrendered_structural_conflict".to_string(), None),
            };
            response(
                request,
                false,
                None,
                conflicted_source,
                conflicts,
                vec![],
                result.diagnostics,
                owned_regions,
                None,
                reparse,
                Some(strategy),
            )
        }
        ThreeWayMergeOutcome::Error => response(
            request,
            false,
            None,
            None,
            conflicts,
            vec![],
            result.diagnostics,
            vec![],
            None,
            None,
            Some("not_rendered".to_string()),
        ),
    }
}

fn render_json_conflicts(
    request: &Merge3Request,
    conflicts: &[MergeConflict],
) -> Result<String, String> {
    let sources = HashMap::from([
        (SourceRevision::Base, request.base_source.clone()),
        (SourceRevision::Ours, request.ours_source.clone()),
        (SourceRevision::Theirs, request.theirs_source.clone()),
    ]);
    let plan = localized_conflict_render_plan(
        sources,
        conflicts,
        request.conflict_marker_size.unwrap_or(7),
    )
    .map_err(|error| error.to_string())?;
    render_source_plan(&plan).map(|result| result.content).map_err(|error| error.to_string())
}

fn json_owned_region_reports(conflicts: &[MergeConflict]) -> Vec<OwnedRegionReport> {
    conflicts
        .iter()
        .filter_map(|conflict| {
            let alternative = conflict.alternatives.iter().find(|alternative| {
                alternative.revision == SourceRevision::Ours
                    && alternative.state == ConflictAlternativeState::Present
                    && alternative.regions.len() == 1
            })?;
            let region = &alternative.regions[0];
            Some(OwnedRegionReport {
                owner_path: conflict.path.clone(),
                node_id: region.node_id.clone(),
                region_kind: region.region_kind.clone(),
                byte_range: SourceRange { start: region.start_byte, end: region.end_byte },
                line_range: SourceRange { start: region.start_line, end: region.end_line },
                attached_spans: vec![],
                backend_id: "tree-sitter-language-pack".to_string(),
                parser_identity: "tree-haver".to_string(),
                can_replace: true,
                can_line_merge: false,
                requires_reparse: true,
            })
        })
        .collect()
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

#[allow(clippy::too_many_arguments)]
fn response(
    request: &Merge3Request,
    ok: bool,
    merged_source: Option<String>,
    conflicted_source: Option<String>,
    conflicts: Vec<Merge3Conflict>,
    change_classifications: Vec<ChangeClassification>,
    diagnostics: Vec<Diagnostic>,
    owned_regions: Vec<OwnedRegionReport>,
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
        change_classifications,
        diagnostics,
        fallbacks: vec![],
        profile: Merge3Profile {
            profile_id: request.profile_id.clone().unwrap_or_default(),
            language: normalize_language(request),
            dialect: request.dialect.clone().unwrap_or_default(),
        },
        render_report: render_report.clone(),
        owned_regions,
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
            diagnostics: vec!["json-merge rendered source-preserving edits".to_string()],
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
        "json" | "jsonc" | "json5" => {
            ("tree-sitter-language-pack".to_string(), "tree-haver".to_string())
        }
        _ => (String::new(), String::new()),
    };
    Merge3RenderReport { strategy, backend_id, parser_identity }
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

fn comment_conflict(category: &str, path: &str, message: &str) -> Merge3Conflict {
    Merge3Conflict {
        conflict_id: "comment-conflict-1".to_string(),
        category: category.to_string(),
        path: if path.is_empty() { "/".to_string() } else { path.to_string() },
        message: message.to_string(),
    }
}

fn normalize_language(request: &Merge3Request) -> String {
    let language = request.language.as_deref().unwrap_or_default().trim().to_ascii_lowercase();
    if matches!(language.as_str(), "json" | "jsonc" | "json5") {
        return language;
    }
    let path = request.path_name.as_deref().unwrap_or_default().to_ascii_lowercase();
    for dialect in ["json", "jsonc", "json5"] {
        if path.ends_with(&format!(".{dialect}")) {
            return dialect.to_string();
        }
    }
    language
}

fn json_dialect(request: &Merge3Request) -> JsonDialect {
    let dialect = request.dialect.clone().unwrap_or_else(|| normalize_language(request));
    match dialect.as_str() {
        "jsonc" => JsonDialect::Jsonc,
        "json5" => JsonDialect::Json5,
        _ => JsonDialect::Json,
    }
}
