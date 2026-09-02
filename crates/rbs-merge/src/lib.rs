use std::collections::HashMap;

use ast_merge::{
    CommentAttachment, CommentRegion, Diagnostic, DiagnosticCategory, DiagnosticSeverity,
    LayoutGap, MergeResult, ParseResult, SourceEdit, apply_source_edits,
    augment_normalized_comments_with_owners, match_owner_paths, normalized_layout_owners_for_kinds,
};
use tree_haver::{
    AnalysisHandle, NormalizedTreeNode, ParserRequest, parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "rbs-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RbsDialect {
    Rbs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RbsOwnerKind {
    Class,
    Module,
    Interface,
    TypeAlias,
    Constant,
    Global,
    Alias,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RbsOwner {
    pub path: String,
    pub owner_kind: RbsOwnerKind,
    pub match_key: String,
    pub source_fragment: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RbsOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RbsOwnerMatchResult {
    pub matched: Vec<RbsOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RbsAnalysis {
    pub dialect: RbsDialect,
    pub source: String,
    pub owners: Vec<RbsOwner>,
    pub comment_regions: Vec<CommentRegion>,
    pub layout_gaps: Vec<LayoutGap>,
    pub comment_attachments: Vec<CommentAttachment>,
}

impl AnalysisHandle for RbsAnalysis {
    fn kind(&self) -> &'static str {
        "rbs"
    }
}

fn diagnostic(category: DiagnosticCategory, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category,
        message: message.into(),
        path: None,
        review: None,
    }
}

fn normalize_comment(text: &str) -> String {
    text.trim().strip_prefix('#').unwrap_or(text.trim()).trim().to_string()
}

fn declaration_kind(
    declaration: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> RbsOwnerKind {
    let kind = declaration
        .child_ids
        .iter()
        .filter_map(|id| nodes.get(id.as_str()))
        .map(|node| node.kind.as_str())
        .find(|kind| kind.ends_with("_decl"));
    match kind {
        Some("class_decl") => RbsOwnerKind::Class,
        Some("module_decl") => RbsOwnerKind::Module,
        Some("interface_decl") => RbsOwnerKind::Interface,
        Some("type_alias_decl") => RbsOwnerKind::TypeAlias,
        Some("const_decl") => RbsOwnerKind::Constant,
        Some("global_decl") => RbsOwnerKind::Global,
        Some("class_alias_decl" | "module_alias_decl") => RbsOwnerKind::Alias,
        _ => RbsOwnerKind::Declaration,
    }
}

fn declaration_key(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_owners(nodes: &[NormalizedTreeNode]) -> Vec<RbsOwner> {
    let nodes_by_id = nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let mut declarations = nodes.iter().filter(|node| node.kind == "decl").collect::<Vec<_>>();
    declarations.sort_by_key(|node| node.span.range.start_byte);
    declarations
        .into_iter()
        .filter_map(|node| {
            let match_key = declaration_key(&node.source_fragment);
            if match_key.is_empty() {
                return None;
            }
            Some(RbsOwner {
                path: format!("/declarations/{match_key}"),
                owner_kind: declaration_kind(node, &nodes_by_id),
                match_key,
                source_fragment: node.source_fragment.clone(),
            })
        })
        .collect()
}

pub fn parse_rbs(source: &str, dialect: RbsDialect) -> ParseResult<RbsAnalysis> {
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: "rbs".to_string(),
        dialect: Some("rbs".to_string()),
    });
    if !parsed.ok {
        return ParseResult {
            ok: false,
            diagnostics: parsed
                .diagnostics
                .into_iter()
                .map(|message| diagnostic(DiagnosticCategory::ParseError, message))
                .collect(),
            analysis: None,
            policies: vec![],
        };
    }

    let layout_owners =
        match normalized_layout_owners_for_kinds(source, &parsed.root_id, &parsed.nodes, &["decl"])
        {
            Ok(owners) => owners,
            Err(error) => {
                return ParseResult {
                    ok: false,
                    diagnostics: vec![diagnostic(DiagnosticCategory::ConfigurationError, error)],
                    analysis: None,
                    policies: vec![],
                };
            }
        };
    let augmentation = match augment_normalized_comments_with_owners(
        source,
        &layout_owners,
        &parsed.nodes,
        "hash_comment",
        normalize_comment,
    ) {
        Ok(augmentation) => augmentation,
        Err(error) => {
            return ParseResult {
                ok: false,
                diagnostics: vec![diagnostic(DiagnosticCategory::ConfigurationError, error)],
                analysis: None,
                policies: vec![],
            };
        }
    };

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(RbsAnalysis {
            dialect,
            source: source.to_string(),
            owners: collect_owners(&parsed.nodes),
            comment_regions: augmentation.regions,
            layout_gaps: augmentation.gaps,
            comment_attachments: augmentation.attachments,
        }),
        policies: vec![],
    }
}

pub fn match_rbs_owners(template: &RbsAnalysis, destination: &RbsAnalysis) -> RbsOwnerMatchResult {
    let result = match_owner_paths(
        &template.owners,
        &destination.owners,
        |owner| owner.path.as_str(),
        |owner| owner.path.as_str(),
    );
    RbsOwnerMatchResult {
        matched: result
            .matched
            .iter()
            .map(|entry| RbsOwnerMatch {
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

pub fn merge_rbs(
    template_source: &str,
    destination_source: &str,
    dialect: RbsDialect,
) -> MergeResult<String> {
    let template = parse_rbs(template_source, dialect);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }
    let destination = parse_rbs(destination_source, dialect);
    if !destination.ok {
        return MergeResult {
            ok: false,
            diagnostics: destination
                .diagnostics
                .into_iter()
                .map(|entry| Diagnostic {
                    category: if entry.category == DiagnosticCategory::ParseError {
                        DiagnosticCategory::DestinationParseError
                    } else {
                        entry.category
                    },
                    ..entry
                })
                .collect(),
            output: None,
            policies: vec![],
        };
    }

    let template = template.analysis.expect("template analysis");
    let destination = destination.analysis.expect("destination analysis");
    let matches = match_owner_paths(
        &template.owners,
        &destination.owners,
        |owner| owner.path.as_str(),
        |owner| owner.path.as_str(),
    );
    let additions = matches
        .unmatched_template
        .iter()
        .map(|index| template.owners[*index].source_fragment.trim().to_string())
        .filter(|source| !source.is_empty())
        .collect::<Vec<_>>();
    if additions.is_empty() {
        return MergeResult {
            ok: true,
            diagnostics: vec![],
            output: Some(destination_source.to_string()),
            policies: vec![],
        };
    }

    let mut insertion = String::new();
    if !destination_source.is_empty() && !destination_source.ends_with('\n') {
        insertion.push('\n');
    }
    if !destination_source.is_empty() && !destination_source.ends_with("\n\n") {
        insertion.push('\n');
    }
    insertion.push_str(&additions.join("\n\n"));
    insertion.push('\n');
    match apply_source_edits(
        destination_source,
        &[SourceEdit::insert(destination_source.len(), insertion)],
    ) {
        Ok(output) => {
            MergeResult { ok: true, diagnostics: vec![], output: Some(output), policies: vec![] }
        }
        Err(error) => MergeResult {
            ok: false,
            diagnostics: vec![diagnostic(
                DiagnosticCategory::ConfigurationError,
                error.to_string(),
            )],
            output: None,
            policies: vec![],
        },
    }
}
