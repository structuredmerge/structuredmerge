use std::sync::Once;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeResult, ParseResult, PolicyReference, PolicySurface, TrackedComment,
};
use pest::{Parser, iterators::Pair};
use pest_grammars::toml::{Rule as PestTomlRule, TomlParser as PestTomlParser};
use toml_merge::{
    TomlAnalysis, TomlDialect, TomlFeatureProfile, TomlOwnerMatchResult, TomlProjectionEntry,
    TomlProjectionScope, TomlProjectionScopeKind, analyze_toml_projection,
    match_toml_owners as match_toml_owners_with_substrate, merge_toml_with_parser,
    toml_feature_profile,
};
use tree_haver::{BackendReference, register_backend};

pub const PACKAGE_NAME: &str = "pest-toml-merge";
pub const BACKEND_ID: &str = "pest";

fn ensure_backend_registered() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_backend(BackendReference {
            id: BACKEND_ID.to_string(),
            family: "peg".to_string(),
        });
    });
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

fn parse_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::ParseError,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn available_toml_backends() -> Vec<String> {
    ensure_backend_registered();
    vec![BACKEND_ID.to_string()]
}

pub fn toml_backend_feature_profile() -> std::collections::BTreeMap<String, serde_json::Value> {
    ensure_backend_registered();
    let mut profile = serde_json::Map::new();
    profile.insert("family".to_string(), serde_json::Value::String("toml".to_string()));
    profile.insert(
        "supported_dialects".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("toml".to_string())]),
    );
    profile.insert(
        "supported_policies".to_string(),
        serde_json::Value::Array(vec![serde_json::json!({
            "surface": "array",
            "name": "destination_wins_array",
        })]),
    );
    profile.insert("backend".to_string(), serde_json::Value::String(BACKEND_ID.to_string()));
    profile.insert(
        "backend_ref".to_string(),
        serde_json::json!({
            "id": BACKEND_ID,
            "family": "peg",
        }),
    );
    profile.into_iter().collect()
}

pub fn toml_plan_context() -> ConformanceFamilyPlanContext {
    ensure_backend_registered();
    ConformanceFamilyPlanContext {
        family_profile: ast_merge::FamilyFeatureProfile {
            family: "toml".to_string(),
            supported_dialects: vec!["toml".to_string()],
            supported_policies: vec![PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            }],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: BACKEND_ID.to_string(),
            supports_dialects: false,
            supported_policies: vec![PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            }],
        }),
        merge_engine: None,
    }
}

pub fn provider_toml_feature_profile() -> TomlFeatureProfile {
    toml_feature_profile()
}

pub fn parse_toml(
    source: &str,
    dialect: TomlDialect,
    backend: Option<&str>,
) -> ParseResult<TomlAnalysis> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {requested}."
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    let mut parsed = match PestTomlParser::parse(PestTomlRule::toml, source) {
        Ok(parsed) => parsed,
        Err(error) => {
            return ParseResult {
                ok: false,
                diagnostics: vec![parse_error(&error.to_string())],
                analysis: None,
                policies: vec![],
            };
        }
    };
    let root = match parsed.next() {
        Some(root) => root,
        None => {
            return ParseResult {
                ok: false,
                diagnostics: vec![parse_error("Pest TOML parser returned no document.")],
                analysis: None,
                policies: vec![],
            };
        }
    };
    match project_pest_document(source, root) {
        Ok((scopes, comments)) => analyze_toml_projection(source, dialect, scopes, comments),
        Err(message) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&message)],
            analysis: None,
            policies: vec![],
        },
    }
}

fn project_pest_document(
    source: &str,
    root: Pair<'_, PestTomlRule>,
) -> Result<(Vec<TomlProjectionScope>, Vec<TrackedComment>), String> {
    let comments = projected_comments(source, &root);
    let root_span = root.as_span();
    let children = root.clone().into_inner().collect::<Vec<_>>();
    let mut next_id = 0;
    let root_entries = children
        .iter()
        .filter(|pair| pair.as_rule() == PestTomlRule::pair)
        .map(|pair| project_entry(pair.clone(), &mut next_id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut scopes = vec![TomlProjectionScope {
        kind: TomlProjectionScopeKind::Root,
        path_components: vec![],
        instance: 0,
        node_id: projected_id(&mut next_id),
        start_byte: root_span.start(),
        end_byte: root_span.end(),
        start_line: 1,
        end_line: source_line_count(source),
        entries: root_entries,
    }];
    let mut table_array_instances = std::collections::HashMap::<Vec<String>, usize>::new();

    for pair in children
        .into_iter()
        .filter(|pair| matches!(pair.as_rule(), PestTomlRule::table | PestTomlRule::array_table))
    {
        let kind = if pair.as_rule() == PestTomlRule::table {
            TomlProjectionScopeKind::Table
        } else {
            TomlProjectionScopeKind::TableArray
        };
        let span = pair.as_span();
        let children = pair.clone().into_inner().collect::<Vec<_>>();
        let path_components = children
            .iter()
            .filter(|child| child.as_rule() == PestTomlRule::key)
            .map(|key| projected_key(key.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let instance = if kind == TomlProjectionScopeKind::TableArray {
            let next = table_array_instances.entry(path_components.clone()).or_default();
            let instance = *next;
            *next += 1;
            instance
        } else {
            0
        };
        let entries = children
            .into_iter()
            .filter(|child| child.as_rule() == PestTomlRule::pair)
            .map(|entry| project_entry(entry, &mut next_id))
            .collect::<Result<Vec<_>, _>>()?;
        let (start_line, _) = span.start_pos().line_col();
        let (end_line, end_column) = span.end_pos().line_col();
        scopes.push(TomlProjectionScope {
            kind,
            path_components,
            instance,
            node_id: projected_id(&mut next_id),
            start_byte: span.start(),
            end_byte: span.end(),
            start_line,
            end_line: effective_end_line(start_line, end_line, end_column),
            entries,
        });
    }
    Ok((scopes, comments))
}

fn project_entry(
    pair: Pair<'_, PestTomlRule>,
    next_id: &mut usize,
) -> Result<TomlProjectionEntry, String> {
    let span = pair.as_span();
    let mut children = pair.into_inner();
    let key = children.next().ok_or_else(|| "Pest TOML pair omitted its key.".to_string())?;
    let value = children.next().ok_or_else(|| "Pest TOML pair omitted its value.".to_string())?;
    let (start_line, _) = span.start_pos().line_col();
    let (end_line, end_column) = span.end_pos().line_col();
    Ok(TomlProjectionEntry {
        key_components: vec![projected_key(key.as_str())?],
        node_id: projected_id(next_id),
        start_byte: span.start(),
        end_byte: span.end(),
        start_line,
        end_line: effective_end_line(start_line, end_line, end_column),
        value_signature: pest_value_signature(&value),
        array_item_count: if value.as_rule() == PestTomlRule::array {
            value.clone().into_inner().count()
        } else {
            0
        },
    })
}

fn projected_key(source: &str) -> Result<String, String> {
    if source
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        Ok(source.to_string())
    } else {
        Err("Pest TOML provider does not yet normalize quoted key identity.".to_string())
    }
}

fn pest_value_signature(pair: &Pair<'_, PestTomlRule>) -> String {
    let children = pair.clone().into_inner().collect::<Vec<_>>();
    if children.is_empty() {
        return format!("{:?}:{:?}", pair.as_rule(), pair.as_str().trim());
    }
    format!(
        "{:?}:[{}]",
        pair.as_rule(),
        children.iter().map(pest_value_signature).collect::<Vec<_>>().join(",")
    )
}

fn projected_comments(source: &str, root: &Pair<'_, PestTomlRule>) -> Vec<TrackedComment> {
    let mut protected = Vec::new();
    collect_comment_protected_ranges(root.clone(), &mut protected);
    let mut comments = Vec::new();
    let mut line_start = 0;
    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let line_without_newline = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(offset) = line_without_newline.match_indices('#').find_map(|(offset, _)| {
            let byte = line_start + offset;
            (!protected.iter().any(|(start, end)| (*start..*end).contains(&byte))).then_some(offset)
        }) {
            let text = &line_without_newline[offset..];
            comments.push(TrackedComment {
                line: line_index + 1,
                text: text.to_string(),
                normalized_content: text.trim_start_matches('#').trim().to_string(),
                full_line: line_without_newline[..offset].trim().is_empty(),
                indent: Some(offset),
            });
        }
        line_start += line.len();
    }
    comments
}

fn collect_comment_protected_ranges(
    pair: Pair<'_, PestTomlRule>,
    protected: &mut Vec<(usize, usize)>,
) {
    if matches!(
        pair.as_rule(),
        PestTomlRule::string
            | PestTomlRule::literal
            | PestTomlRule::multi_line_string
            | PestTomlRule::multi_line_literal
            | PestTomlRule::key
    ) {
        let span = pair.as_span();
        protected.push((span.start(), span.end()));
        return;
    }
    for child in pair.into_inner() {
        collect_comment_protected_ranges(child, protected);
    }
}

fn projected_id(next_id: &mut usize) -> String {
    let id = format!("pest:toml:{}", *next_id);
    *next_id += 1;
    id
}

fn effective_end_line(start_line: usize, end_line: usize, end_column: usize) -> usize {
    if end_column == 1 && end_line > start_line { end_line - 1 } else { end_line }
}

fn source_line_count(source: &str) -> usize {
    source.lines().count().max(1)
}

pub fn match_toml_owners(
    template: &TomlAnalysis,
    destination: &TomlAnalysis,
) -> TomlOwnerMatchResult {
    match_toml_owners_with_substrate(template, destination)
}

pub fn merge_toml(
    template_source: &str,
    destination_source: &str,
    dialect: TomlDialect,
    backend: Option<&str>,
) -> MergeResult<String> {
    ensure_backend_registered();
    let requested = backend.unwrap_or(BACKEND_ID);
    if requested != BACKEND_ID {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {requested}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_toml_with_parser(template_source, destination_source, dialect, |source, parse_dialect| {
        parse_toml(source, parse_dialect, None)
    })
}
