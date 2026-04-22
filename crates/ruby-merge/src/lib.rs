use std::collections::{HashMap, HashSet};

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, DelegatedChildOperation,
    DiagnosticCategory, DiscoveredSurface, FamilyFeatureProfile, MergeResult, ParseResult,
    PolicyReference, PolicySurface, SurfaceOwnerKind, SurfaceOwnerRef, SurfaceSpan,
};
use tree_haver::{ParserRequest, parse_with_language_pack};

pub const PACKAGE_NAME: &str = "ruby-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RubyDialect {
    Ruby,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RubyOwnerKind {
    Require,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyOwner {
    pub path: String,
    pub owner_kind: RubyOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyOwnerMatchResult {
    pub matched: Vec<RubyOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyAnalysis {
    pub dialect: RubyDialect,
    pub source: String,
    pub owners: Vec<RubyOwner>,
    pub discovered_surfaces: Vec<DiscoveredSurface>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedChildOutput {
    pub operation_id: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NestedChildOutput {
    pub surface_address: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<RubyDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RubyBackendFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<RubyDialect>,
    pub supported_policies: Vec<PolicyReference>,
    pub backend: String,
    pub supports_dialects: bool,
}

#[derive(Clone, Debug)]
struct CommentEntry {
    line: usize,
    raw: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RubyRequireEntry {
    text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RubyDeclarationEntry {
    path: String,
    text: String,
}

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
}

fn configuration_error(message: &str) -> ast_merge::Diagnostic {
    ast_merge::Diagnostic {
        severity: ast_merge::DiagnosticSeverity::Error,
        category: DiagnosticCategory::ConfigurationError,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

fn parse_request(source: &str) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: "ruby".to_string(),
        dialect: Some("ruby".to_string()),
    }
}

fn normalize_source(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn comment_line(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn normalize_comment_content(raw: &str) -> String {
    raw.trim_start()
        .strip_prefix('#')
        .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
        .unwrap_or(raw)
        .trim()
        .to_string()
}

fn doc_comment_content(raw: &str) -> bool {
    let content = normalize_comment_content(raw);
    if content.is_empty() {
        return false;
    }
    if regex_lite::Regex::new(r"^(?::nocov:|[\w-]+:(?:freeze|unfreeze))$")
        .expect("directive regex")
        .is_match(&content)
    {
        return false;
    }
    ![
        "coding",
        "encoding",
        "frozen_string_literal",
        "shareable_constant_value",
        "typed",
        "warn_indent",
    ]
    .iter()
    .any(|prefix| content.starts_with(&format!("{prefix}:")))
}

fn comment_prefix(raw: &str) -> String {
    regex_lite::Regex::new(r"^\s*#\s?")
        .expect("comment prefix regex")
        .find(raw)
        .map(|match_| match_.as_str().to_string())
        .unwrap_or_else(|| "# ".to_string())
}

fn declared_example_language(rest: &str) -> Option<String> {
    regex_lite::Regex::new(r"^\[(?P<language>[^\]]+)\]")
        .expect("example language regex")
        .captures(rest.trim())
        .and_then(|captures| captures.name("language"))
        .map(|language| language.as_str().trim().to_lowercase().replace('-', "_"))
}

fn surfaces_for_owner(
    owner_name: &str,
    comment_entries: &[CommentEntry],
) -> Vec<DiscoveredSurface> {
    let filtered_entries = comment_entries
        .iter()
        .filter(|entry| doc_comment_content(&entry.raw))
        .cloned()
        .collect::<Vec<_>>();
    if filtered_entries.is_empty() {
        return Vec::new();
    }

    let mut doc_metadata = HashMap::new();
    doc_metadata
        .insert("owner_signature".to_string(), serde_json::Value::String(owner_name.to_string()));
    doc_metadata.insert(
        "comment_prefix".to_string(),
        serde_json::Value::String(comment_prefix(&filtered_entries[0].raw)),
    );
    doc_metadata.insert(
        "entries".to_string(),
        serde_json::Value::Array(
            filtered_entries
                .iter()
                .map(|entry| serde_json::json!({ "line": entry.line, "raw": entry.raw }))
                .collect(),
        ),
    );

    let doc_surface = DiscoveredSurface {
        surface_kind: "ruby_doc_comment".to_string(),
        declared_language: Some("yard".to_string()),
        effective_language: "yard".to_string(),
        address: format!("document[0] > ruby_doc_comment[{owner_name}]"),
        parent_address: Some("document[0]".to_string()),
        span: Some(SurfaceSpan {
            start_line: filtered_entries[0].line,
            end_line: filtered_entries[filtered_entries.len() - 1].line,
        }),
        owner: SurfaceOwnerRef {
            kind: SurfaceOwnerKind::OwnedRegion,
            address: format!("/declarations/{owner_name}"),
        },
        reconstruction_strategy: "rewrite_with_prefix_preservation".to_string(),
        metadata: doc_metadata,
    };

    let normalized = filtered_entries
        .iter()
        .map(|entry| normalize_comment_content(&entry.raw))
        .collect::<Vec<_>>();
    let mut surfaces = vec![doc_surface.clone()];

    for (tag_index, content) in normalized.iter().enumerate() {
        let example_match = regex_lite::Regex::new(r"^@example\b(?P<rest>.*)$")
            .expect("example tag regex")
            .captures(content);
        let Some(example_match) = example_match else {
            continue;
        };

        let body_start = tag_index + 1;
        let mut body_end = normalized.len();
        for (index, candidate) in normalized.iter().enumerate().skip(body_start) {
            if regex_lite::Regex::new(r"^@[a-z_]+\b").expect("tag prefix regex").is_match(candidate)
            {
                body_end = index;
                break;
            }
        }
        if body_start >= body_end {
            continue;
        }
        let body_entries = &filtered_entries[body_start..body_end];
        if body_entries.is_empty() {
            continue;
        }

        let language = declared_example_language(
            example_match.name("rest").map(|match_| match_.as_str()).unwrap_or(""),
        )
        .unwrap_or_else(|| "ruby".to_string());
        let mut example_metadata = HashMap::new();
        example_metadata
            .insert("tag_kind".to_string(), serde_json::Value::String("example".to_string()));
        example_metadata
            .insert("tag_index".to_string(), serde_json::Value::Number(tag_index.into()));
        example_metadata.insert(
            "tag_text".to_string(),
            serde_json::Value::String(normalized[tag_index].clone()),
        );
        example_metadata.insert(
            "comment_prefix".to_string(),
            serde_json::Value::String(
                doc_surface
                    .metadata
                    .get("comment_prefix")
                    .and_then(|value| value.as_str())
                    .unwrap_or("# ")
                    .to_string(),
            ),
        );

        surfaces.push(DiscoveredSurface {
            surface_kind: "yard_example_block".to_string(),
            declared_language: Some(language.clone()),
            effective_language: language,
            address: format!("{} > yard_example[{tag_index}]", doc_surface.address),
            parent_address: Some(doc_surface.address.clone()),
            span: Some(SurfaceSpan {
                start_line: body_entries[0].line,
                end_line: body_entries[body_entries.len() - 1].line,
            }),
            owner: SurfaceOwnerRef {
                kind: SurfaceOwnerKind::OwnedRegion,
                address: doc_surface.address.clone(),
            },
            reconstruction_strategy: "rewrite_with_prefix_preservation".to_string(),
            metadata: example_metadata,
        });
    }

    surfaces
}

fn analyze_ruby_document(source: &str) -> RubyAnalysis {
    let normalized = normalize_source(source);
    let mut requires = Vec::new();
    let mut declarations = Vec::new();
    let mut surfaces = Vec::new();
    let mut pending_comments = Vec::new();

    for (index, line) in normalized.split('\n').enumerate() {
        let line_number = index + 1;
        let stripped = line.trim();

        if comment_line(line) {
            pending_comments.push(CommentEntry { line: line_number, raw: line.to_string() });
            continue;
        }
        if stripped.is_empty() {
            pending_comments.clear();
            continue;
        }

        if let Some(captures) =
            regex_lite::Regex::new(r#"^\s*require(?:_relative)?\s+["']([^"']+)["']"#)
                .expect("require regex")
                .captures(line)
        {
            requires.push(RubyOwner {
                path: format!("/requires/{}", requires.len()),
                owner_kind: RubyOwnerKind::Require,
                match_key: captures.get(1).map(|value| value.as_str().to_string()),
            });
            pending_comments.clear();
            continue;
        }

        let declaration = regex_lite::Regex::new(r"^\s*class\s+([A-Z]\w*(?:::\w+)*)")
            .expect("class regex")
            .captures(line)
            .or_else(|| {
                regex_lite::Regex::new(r"^\s*module\s+([A-Z]\w*(?:::\w+)*)")
                    .expect("module regex")
                    .captures(line)
            })
            .or_else(|| {
                regex_lite::Regex::new(r"^\s*def\s+(?:self\.)?([a-zA-Z_]\w*[!?=]?)")
                    .expect("def regex")
                    .captures(line)
            });
        if let Some(declaration) = declaration {
            let name = declaration.get(1).expect("name capture").as_str().to_string();
            declarations.push(RubyOwner {
                path: format!("/declarations/{name}"),
                owner_kind: RubyOwnerKind::Declaration,
                match_key: Some(name.clone()),
            });
            surfaces.extend(surfaces_for_owner(&name, &pending_comments));
            pending_comments.clear();
            continue;
        }

        pending_comments.clear();
    }

    let mut owners = [requires, declarations].concat();
    owners.sort_by(|left, right| left.path.cmp(&right.path));
    RubyAnalysis {
        dialect: RubyDialect::Ruby,
        source: normalized,
        owners,
        discovered_surfaces: surfaces,
    }
}

pub fn ruby_feature_profile() -> RubyFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "ruby".to_string(),
        supported_dialects: vec!["ruby".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };
    RubyFeatureProfile {
        family: "ruby",
        supported_dialects: shared.supported_dialects.iter().map(|_| RubyDialect::Ruby).collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn available_ruby_backends() -> Vec<String> {
    vec!["kreuzberg-language-pack".to_string()]
}

pub fn ruby_backend_feature_profile() -> RubyBackendFeatureProfile {
    RubyBackendFeatureProfile {
        family: "ruby",
        supported_dialects: vec![RubyDialect::Ruby],
        supported_policies: vec![destination_wins_array_policy()],
        backend: "kreuzberg-language-pack".to_string(),
        supports_dialects: true,
    }
}

pub fn ruby_plan_context() -> ConformanceFamilyPlanContext {
    let backend_profile = ruby_backend_feature_profile();
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: "ruby".to_string(),
            supported_dialects: vec!["ruby".to_string()],
            supported_policies: vec![destination_wins_array_policy()],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: backend_profile.backend,
            supports_dialects: backend_profile.supports_dialects,
            supported_policies: backend_profile.supported_policies,
        }),
    }
}

pub fn parse_ruby(source: &str, _dialect: RubyDialect) -> ParseResult<RubyAnalysis> {
    let parsed = parse_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return ParseResult {
            ok: false,
            diagnostics: parsed.diagnostics,
            analysis: None,
            policies: vec![],
        };
    }

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(analyze_ruby_document(source)),
        policies: vec![],
    }
}

pub fn match_ruby_owners(
    template: &RubyAnalysis,
    destination: &RubyAnalysis,
) -> RubyOwnerMatchResult {
    let destination_owners =
        destination.owners.iter().map(|owner| owner.path.clone()).collect::<HashSet<_>>();
    let template_owners =
        template.owners.iter().map(|owner| owner.path.clone()).collect::<HashSet<_>>();

    RubyOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| RubyOwnerMatch {
                template_path: owner.path.clone(),
                destination_path: owner.path.clone(),
            })
            .collect(),
        unmatched_template: template
            .owners
            .iter()
            .map(|owner| owner.path.clone())
            .filter(|path| !destination_owners.contains(path))
            .collect(),
        unmatched_destination: destination
            .owners
            .iter()
            .map(|owner| owner.path.clone())
            .filter(|path| !template_owners.contains(path))
            .collect(),
    }
}

fn collect_ruby_require_entries(source: &str) -> Vec<RubyRequireEntry> {
    let require_regex = regex_lite::Regex::new(r#"^\s*require(?:_relative)?\s+["']([^"']+)["']"#)
        .expect("require regex");
    normalize_source(source)
        .split('\n')
        .filter(|line| require_regex.is_match(line))
        .map(|line| RubyRequireEntry { text: line.trim_end().to_string() })
        .collect()
}

fn collect_ruby_declaration_entries(source: &str) -> Vec<RubyDeclarationEntry> {
    let normalized = normalize_source(source);
    let lines = normalized.split('\n').collect::<Vec<_>>();
    let require_regex = regex_lite::Regex::new(r#"^\s*require(?:_relative)?\s+["']([^"']+)["']"#)
        .expect("require regex");
    let class_regex =
        regex_lite::Regex::new(r"^\s*class\s+([A-Z]\w*(?:::\w+)*)").expect("class regex");
    let module_regex =
        regex_lite::Regex::new(r"^\s*module\s+([A-Z]\w*(?:::\w+)*)").expect("module regex");
    let def_regex =
        regex_lite::Regex::new(r"^\s*def\s+(?:self\.)?([a-zA-Z_]\w*[!?=]?)").expect("def regex");
    let mut entries = Vec::new();
    let mut pending_comments = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let stripped = line.trim();
        if comment_line(line) {
            pending_comments.push(index);
            index += 1;
            continue;
        }
        if stripped.is_empty() {
            pending_comments.clear();
            index += 1;
            continue;
        }
        if require_regex.is_match(line) {
            pending_comments.clear();
            index += 1;
            continue;
        }

        let declaration = class_regex
            .captures(line)
            .or_else(|| module_regex.captures(line))
            .or_else(|| def_regex.captures(line));
        let Some(declaration) = declaration else {
            pending_comments.clear();
            index += 1;
            continue;
        };

        let name = declaration.get(1).expect("name capture").as_str().to_string();
        let start = pending_comments.first().copied().unwrap_or(index);
        let mut depth = 1usize;
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let candidate = lines[cursor].trim();
            if class_regex.is_match(candidate)
                || module_regex.is_match(candidate)
                || def_regex.is_match(candidate)
            {
                depth += 1;
            }
            if candidate == "end" {
                depth -= 1;
                if depth == 0 {
                    cursor += 1;
                    break;
                }
            }
            cursor += 1;
        }

        entries.push(RubyDeclarationEntry {
            path: format!("/declarations/{name}"),
            text: lines[start..cursor].join("\n").trim().to_string(),
        });
        pending_comments.clear();
        index = cursor;
    }

    entries
}

pub fn merge_ruby(
    template_source: &str,
    destination_source: &str,
    dialect: RubyDialect,
) -> MergeResult<String> {
    let template = parse_ruby(template_source, dialect);
    if !template.ok || template.analysis.is_none() {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_ruby(destination_source, dialect);
    if !destination.ok || destination.analysis.is_none() {
        let diagnostics = destination
            .diagnostics
            .into_iter()
            .map(|mut diagnostic| {
                if diagnostic.category == DiagnosticCategory::ParseError {
                    diagnostic.category = DiagnosticCategory::DestinationParseError;
                }
                diagnostic
            })
            .collect();
        return MergeResult { ok: false, diagnostics, output: None, policies: vec![] };
    }

    let template_analysis = template.analysis.expect("template analysis");
    let destination_analysis = destination.analysis.expect("destination analysis");
    let requires = collect_ruby_require_entries(&destination_analysis.source);
    let destination_declarations = collect_ruby_declaration_entries(&destination_analysis.source);
    let template_declarations = collect_ruby_declaration_entries(&template_analysis.source);
    let destination_paths =
        destination_declarations.iter().map(|entry| entry.path.clone()).collect::<HashSet<_>>();
    let mut sections = Vec::new();
    if !requires.is_empty() {
        sections.push(
            requires
                .iter()
                .map(|entry| entry.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string(),
        );
    }
    sections.extend(destination_declarations.iter().map(|entry| entry.text.clone()));
    sections.extend(
        template_declarations
            .iter()
            .filter(|entry| !destination_paths.contains(&entry.path))
            .map(|entry| entry.text.clone()),
    );

    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(format!("{}\n", sections.join("\n\n").trim())),
        policies: vec![destination_wins_array_policy()],
    }
}

fn ruby_example_line_prefix(line: &str) -> String {
    regex_lite::Regex::new(r"^(\s*#\s*)")
        .expect("ruby example prefix regex")
        .captures(line)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .unwrap_or_else(|| "# ".to_string())
}

pub fn apply_ruby_delegated_child_outputs(
    source: &str,
    operations: &[DelegatedChildOperation],
    apply_plan: &ast_merge::DelegatedChildApplyPlan,
    applied_children: &[AppliedChildOutput],
) -> MergeResult<String> {
    let mut lines = normalize_source(source).split('\n').map(str::to_string).collect::<Vec<_>>();
    let operations_by_id = operations
        .iter()
        .map(|operation| (operation.operation_id.clone(), operation))
        .collect::<HashMap<_, _>>();
    let outputs_by_id = applied_children
        .iter()
        .map(|entry| (entry.operation_id.clone(), entry.output.clone()))
        .collect::<HashMap<_, _>>();

    let mut replacements = Vec::new();
    for entry in &apply_plan.entries {
        let Some(operation) = operations_by_id.get(&entry.delegated_group.child_operation_id)
        else {
            continue;
        };
        let Some(span) = operation.surface.span.as_ref() else {
            continue;
        };
        let Some(output) = outputs_by_id.get(&entry.delegated_group.child_operation_id) else {
            continue;
        };
        replacements.push((span.start_line - 1, span.end_line - 1, output.clone()));
    }

    replacements.sort_by(|left, right| right.0.cmp(&left.0));
    for (start, end, output) in replacements {
        if start >= lines.len() {
            return MergeResult {
                ok: false,
                diagnostics: vec![configuration_error("invalid delegated child span.")],
                output: None,
                policies: vec![destination_wins_array_policy()],
            };
        }
        let prefix = ruby_example_line_prefix(&lines[start]);
        let replacement_lines = if output.is_empty() {
            Vec::new()
        } else {
            output
                .trim_end_matches('\n')
                .split('\n')
                .map(|line| format!("{prefix}{line}"))
                .collect::<Vec<_>>()
        };
        lines.splice(start..=end, replacement_lines);
    }

    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(format!("{}\n", lines.join("\n").trim_end_matches('\n'))),
        policies: vec![destination_wins_array_policy()],
    }
}

pub fn merge_ruby_with_nested_outputs(
    template_source: &str,
    destination_source: &str,
    dialect: RubyDialect,
    nested_outputs: &[NestedChildOutput],
) -> MergeResult<String> {
    ast_merge::execute_nested_merge(
        &nested_outputs
            .iter()
            .map(|nested_output| ast_merge::DelegatedChildSurfaceOutput {
                surface_address: nested_output.surface_address.clone(),
                output: nested_output.output.clone(),
            })
            .collect::<Vec<_>>(),
        &ast_merge::DelegatedChildOutputResolutionOptions {
            default_family: "ruby".to_string(),
            request_id_prefix: "nested_ruby_child".to_string(),
        },
        ast_merge::NestedMergeExecutionCallbacks {
            merge_parent: || merge_ruby(template_source, destination_source, dialect),
            discover_operations: |merged_output| {
                let analysis = parse_ruby(merged_output, dialect);
                if !analysis.ok || analysis.analysis.is_none() {
                    return ast_merge::NestedMergeDiscoveryResult {
                        ok: false,
                        diagnostics: analysis.diagnostics,
                        operations: None,
                    };
                }

                ast_merge::NestedMergeDiscoveryResult {
                    ok: true,
                    diagnostics: vec![],
                    operations: Some(ruby_delegated_child_operations(
                        analysis.analysis.as_ref().expect("analysis"),
                        "ruby-document-0",
                    )),
                }
            },
            apply_resolved_outputs: |merged_output, operations, apply_plan, applied_children| {
                let translated = applied_children
                    .iter()
                    .map(|entry| AppliedChildOutput {
                        operation_id: entry.operation_id.clone(),
                        output: entry.output.clone(),
                    })
                    .collect::<Vec<_>>();

                apply_ruby_delegated_child_outputs(
                    merged_output,
                    operations,
                    apply_plan,
                    &translated,
                )
            },
        },
    )
}

pub fn ruby_discovered_surfaces(analysis: &RubyAnalysis) -> Vec<DiscoveredSurface> {
    analysis.discovered_surfaces.clone()
}

pub fn ruby_delegated_child_operations(
    analysis: &RubyAnalysis,
    parent_operation_id: &str,
) -> Vec<DelegatedChildOperation> {
    let mut operations = Vec::new();
    let mut doc_operation_ids = HashMap::new();
    let mut doc_index = 0usize;
    let mut example_index = 0usize;

    for surface in &analysis.discovered_surfaces {
        if surface.surface_kind != "ruby_doc_comment" {
            continue;
        }
        let operation_id = format!("ruby-doc-comment-{doc_index}");
        doc_operation_ids.insert(surface.address.clone(), operation_id.clone());
        operations.push(DelegatedChildOperation {
            operation_id,
            parent_operation_id: parent_operation_id.to_string(),
            requested_strategy: "delegate_child_surface".to_string(),
            language_chain: vec!["ruby".to_string(), surface.effective_language.clone()],
            surface: surface.clone(),
        });
        doc_index += 1;
    }

    for surface in &analysis.discovered_surfaces {
        if surface.surface_kind != "yard_example_block" {
            continue;
        }
        operations.push(DelegatedChildOperation {
            operation_id: format!("yard-example-{example_index}"),
            parent_operation_id: surface
                .parent_address
                .as_ref()
                .and_then(|address| doc_operation_ids.get(address))
                .cloned()
                .unwrap_or_else(|| parent_operation_id.to_string()),
            requested_strategy: "delegate_child_surface".to_string(),
            language_chain: vec![
                "ruby".to_string(),
                "yard".to_string(),
                surface.effective_language.clone(),
            ],
            surface: surface.clone(),
        });
        example_index += 1;
    }

    operations
}
