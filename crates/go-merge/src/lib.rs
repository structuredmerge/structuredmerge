use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeConflict, MergeResult, ParseResult,
    PolicyReference, PolicySurface, SourcePreservingOwnerDocument, ThreeWayMergeOutcome,
    ThreeWayMergeResult, merge_source_preserving_owners, normalized_parse_error_result,
    parse_error_result, three_way_parse_error,
};
use tree_haver::{
    BackendReference, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "go-merge";

pub mod directional;
pub mod typed;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GoDialect {
    Go,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GoBackend {
    TreeSitter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GoOwnerKind {
    Import,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GoOwner {
    pub path: String,
    pub owner_kind: GoOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GoOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GoOwnerMatchResult {
    pub matched: Vec<GoOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModuleImport {
    pub path: String,
    pub match_key: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModuleDeclaration {
    pub path: String,
    pub match_key: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GoAnalysis {
    pub dialect: GoDialect,
    pub source: String,
    pub owners: Vec<GoOwner>,
    pub imports: Vec<ModuleImport>,
    pub declarations: Vec<ModuleDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<GoDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoBackendFeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
}

fn parse_request(source: &str) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: "go".to_string(),
        dialect: Some("go".to_string()),
    }
}

fn slice_span(source: &str, start: usize, end: usize) -> String {
    source[start..end].trim().to_string()
}

fn line_anchored_span(source: &str, start: usize, end: usize) -> String {
    let line_start = source[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[line_start..end].trim().to_string()
}

pub fn go_feature_profile() -> GoFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "go".to_string(),
        supported_dialects: vec!["go".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    GoFeatureProfile {
        family: "go",
        supported_dialects: shared.supported_dialects.iter().map(|_| GoDialect::Go).collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn go_backend_feature_profile(_backend: GoBackend) -> GoBackendFeatureProfile {
    GoBackendFeatureProfile {
        backend: language_pack_adapter_info().backend,
        backend_ref: Some(kreuzberg_language_pack_backend()),
        supports_dialects: true,
        supported_policies: vec![destination_wins_array_policy()],
    }
}

pub fn go_plan_context(backend: GoBackend) -> ConformanceFamilyPlanContext {
    let feature_profile = go_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: go_feature_profile().family.to_string(),
            supported_dialects: vec!["go".to_string()],
            supported_policies: go_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: feature_profile.backend,
            supports_dialects: feature_profile.supports_dialects,
            supported_policies: feature_profile.supported_policies,
        }),
        merge_engine: None,
    }
}

pub fn go_backends() -> Vec<GoBackend> {
    vec![GoBackend::TreeSitter]
}

pub fn parse_go(source: &str, _dialect: GoDialect) -> ParseResult<GoAnalysis> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return normalized_parse_error_result(parsed.diagnostics);
    }
    let index = match NormalizedTreeIndex::new(&parsed.nodes) {
        Ok(index) => index,
        Err(message) => return parse_error_result(message),
    };
    let root = match index.root(&parsed.root_id) {
        Ok(root) => root,
        Err(message) => return parse_error_result(message),
    };
    let mut imports = Vec::new();
    let mut declarations = Vec::new();
    for node in index.children(root) {
        match node.kind.as_str() {
            "comment" | "package_clause" => {}
            "import_declaration" => {
                let paths = go_import_paths(node, &index);
                if paths.len() != 1 {
                    return parse_error_result(
                        "Go grouped or source-less imports are unsupported by merge2",
                    );
                }
                imports.push(ModuleImport {
                    path: format!("/imports/{}", imports.len()),
                    match_key: paths[0].clone(),
                    text: format!(
                        "{}\n",
                        slice_span(source, node.span.range.start_byte, node.span.range.end_byte)
                    ),
                });
            }
            "function_declaration" => {
                let Some(name) = declaration_name(node, &index) else {
                    return parse_error_result("Go function declaration has no stable name");
                };
                declarations.push(ModuleDeclaration {
                    path: format!("/declarations/{name}"),
                    match_key: name,
                    text: format!(
                        "{}\n",
                        line_anchored_span(
                            source,
                            node.span.range.start_byte,
                            node.span.range.end_byte
                        )
                    ),
                });
            }
            kind => return parse_error_result(format!("unsupported top-level Go node {kind:?}")),
        }
    }
    declarations.sort_by(|left, right| left.path.cmp(&right.path));

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(GoAnalysis {
            dialect: GoDialect::Go,
            source: source.to_string(),
            owners: [
                imports
                    .iter()
                    .map(|item| GoOwner {
                        path: item.path.clone(),
                        owner_kind: GoOwnerKind::Import,
                        match_key: Some(item.match_key.clone()),
                    })
                    .collect::<Vec<_>>(),
                declarations
                    .iter()
                    .map(|item| GoOwner {
                        path: item.path.clone(),
                        owner_kind: GoOwnerKind::Declaration,
                        match_key: Some(item.match_key.clone()),
                    })
                    .collect::<Vec<_>>(),
            ]
            .concat(),
            imports,
            declarations,
        }),
        policies: vec![],
    }
}

pub fn merge_go_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    _dialect: GoDialect,
) -> ThreeWayMergeResult<String> {
    let base = match parse_source_preserving_go(base_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("base", message),
    };
    let ours = match parse_source_preserving_go(ours_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("ours", message),
    };
    let theirs = match parse_source_preserving_go(theirs_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("theirs", message),
    };

    if go_membership_change_with_owner_edit(&base, &ours, &theirs) {
        return conservative_membership_conflict();
    }

    merge_source_preserving_owners(base, ours, theirs, parse_source_preserving_go)
}

fn go_membership_change_with_owner_edit(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> bool {
    let grouped_ids = |document: &SourcePreservingOwnerDocument| {
        document
            .owners
            .iter()
            .filter(|owner| owner.id.starts_with("/var_group:"))
            .map(|owner| owner.id.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let base_groups = grouped_ids(base);
    if grouped_ids(ours) != base_groups || grouped_ids(theirs) != base_groups {
        return true;
    }

    let base_ids =
        base.owners.iter().map(|owner| owner.id.as_str()).collect::<std::collections::HashSet<_>>();
    let ours_ids =
        ours.owners.iter().map(|owner| owner.id.as_str()).collect::<std::collections::HashSet<_>>();
    let theirs_ids = theirs
        .owners
        .iter()
        .map(|owner| owner.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let membership_changed = base_ids != ours_ids || base_ids != theirs_ids;
    if !membership_changed {
        return false;
    }
    unmanaged_fingerprint(ours) != unmanaged_fingerprint(theirs)
}

fn unmanaged_fingerprint(document: &SourcePreservingOwnerDocument) -> String {
    use std::collections::HashSet;

    let mut owned_lines = HashSet::new();
    for owner in &document.owners {
        for line in owner.start_line..=owner.end_line {
            owned_lines.insert(line);
        }
    }
    document
        .source
        .split_inclusive('\n')
        .enumerate()
        .filter_map(|(index, line)| (!owned_lines.contains(&(index + 1))).then_some(line))
        .collect::<String>()
}

fn conservative_membership_conflict() -> ThreeWayMergeResult<String> {
    let conflict = MergeConflict {
        conflict_id: "go-unmanaged-source".to_string(),
        category: "unmanaged_source_change".to_string(),
        path: "<unmanaged-source>".to_string(),
        fallback_scope: "full_file".to_string(),
        message: "Go owner membership changed alongside an existing owner edit; source ownership is unproven"
            .to_string(),
        alternatives: vec![],
    };
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Conflict,
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::MergeConflict,
            message: conflict.message.clone(),
            path: Some(conflict.path.clone()),
            review: None,
        }],
        conflicts: vec![conflict],
        output: None,
        policies: vec![],
    }
}

fn parse_source_preserving_go(source: &str) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return Err(parsed.diagnostics.join("; "));
    }
    if !parsed.source_fragments_available {
        return Err("Go parser did not retain source fragments".to_string());
    }
    project_go_owners(source, &parsed.root_id, &parsed.nodes).map(|analysis| analysis.document)
}

/// Project Go functions and variable declarations as whole source owners.
/// Grouped declarations use their ordered member identities, matching the
/// Ruby provider's whole-declaration ownership boundary.
pub(crate) fn project_go_owners(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
) -> Result<ast_merge::native_analysis::NativeOwnerAnalysis, String> {
    use std::collections::{BTreeMap, HashSet};
    use tree_haver::NodeRole;

    let index = NormalizedTreeIndex::new(nodes)?;
    let root = index.root(root_id)?;
    let mut owners = Vec::new();
    let mut owner_node_ids = BTreeMap::new();
    let mut owner_ids = HashSet::new();
    let mut pending_comments = Vec::new();
    let mut last_owner: Option<String> = None;

    for node in index.children(root) {
        if node.role == NodeRole::Comment || node.kind == "comment" {
            if let Some(owner_id) = &last_owner {
                if let Some(owner) = owners
                    .iter_mut()
                    .find(|owner: &&mut ast_merge::SourcePreservingOwner| owner.id == *owner_id)
                {
                    let gap = source.get(owner.end_byte..node.span.range.start_byte).unwrap_or("");
                    if gap.bytes().all(|byte| matches!(byte, b'\t' | b'\r' | b' ')) {
                        owner.end_byte = node.span.range.end_byte;
                        owner.end_line = node.span.end_point.row + 1;
                        owner.fingerprint = source[owner.start_byte..owner.end_byte].to_string();
                        owner_node_ids
                            .entry(owner_id.clone())
                            .or_insert_with(Vec::new)
                            .push(node.id.clone());
                        continue;
                    }
                }
            }
            pending_comments.push(node);
            continue;
        }
        if matches!(node.kind.as_str(), "package_clause" | "import_declaration") {
            pending_comments.clear();
            last_owner = None;
            continue;
        }
        let id = match node.kind.as_str() {
            "function_declaration" => {
                let name = declaration_name(node, &index)
                    .ok_or("Go function declaration has no stable name")?;
                format!("/function:{name}")
            }
            "var_declaration" => {
                let descendants = index.descendants(node);
                let mut members = descendants
                    .iter()
                    .filter(|child| child.kind == "var_spec")
                    .map(|spec| {
                        let children = index.children(spec);
                        let names = children
                            .iter()
                            .filter(|child| child.field_name.as_deref() == Some("name"))
                            .map(|child| child.source_fragment.trim().to_string())
                            .collect::<Vec<_>>();
                        if names.is_empty() {
                            children
                                .iter()
                                .filter(|child| child.kind == "identifier")
                                .take(1)
                                .map(|child| child.source_fragment.trim().to_string())
                                .collect()
                        } else {
                            names
                        }
                    })
                    .collect::<Vec<_>>();
                if members.is_empty() || members.iter().any(Vec::is_empty) {
                    return Err("Go variable declaration has no stable member names".into());
                }
                if descendants.iter().any(|child| child.kind == "var_spec_list") {
                    let identity =
                        members.iter().map(|member| member.join(",")).collect::<Vec<_>>().join(";");
                    format!("/var_group:{identity}")
                } else {
                    let names = members.remove(0);
                    if names.len() == 1 {
                        format!("/var:{}", names[0])
                    } else {
                        format!("/var:{}", names.join(","))
                    }
                }
            }
            kind => {
                pending_comments.clear();
                return Err(format!("unsupported top-level Go node {kind:?}"));
            }
        };
        if !owner_ids.insert(id.clone()) {
            return Err(format!("Go document has duplicate top-level owner identity {id:?}"));
        }
        if node.source_fragment.is_empty() {
            return Err(format!("Go owner {id:?} has no source fragment"));
        }
        let mut start_byte = node.span.range.start_byte;
        let mut start_line = node.span.start_point.row + 1;
        let mut attached_ids = vec![node.id.clone()];
        for comment in pending_comments.iter().rev() {
            let gap = source.get(comment.span.range.end_byte..start_byte).unwrap_or("");
            if !gap.bytes().all(|byte| matches!(byte, b'\t' | b'\r' | b'\n' | b' '))
                || gap.bytes().filter(|byte| *byte == b'\n').count() > 1
            {
                break;
            }
            start_byte = comment.span.range.start_byte;
            start_line = comment.span.start_point.row + 1;
            attached_ids.push(comment.id.clone());
        }
        attached_ids.reverse();
        let fingerprint = source
            .get(start_byte..node.span.range.end_byte)
            .ok_or("Go owner source range is invalid")?
            .to_string();
        owner_node_ids.insert(id.clone(), attached_ids);
        owners.push(ast_merge::SourcePreservingOwner {
            id: id.clone(),
            path: id,
            fingerprint,
            start_byte,
            end_byte: node.span.range.end_byte,
            start_line,
            end_line: node.span.end_point.row + 1,
        });
        last_owner = owners.last().map(|owner| owner.id.clone());
        pending_comments.clear();
    }
    if owners.is_empty() {
        return Err("Go document has no supported top-level owners".into());
    }
    Ok(ast_merge::native_analysis::NativeOwnerAnalysis {
        document: SourcePreservingOwnerDocument { source: source.to_string(), owners },
        owner_node_ids,
    })
}

fn go_import_paths(node: &NormalizedTreeNode, index: &NormalizedTreeIndex<'_>) -> Vec<String> {
    index
        .descendants(node)
        .into_iter()
        .filter(|child| {
            matches!(child.kind.as_str(), "interpreted_string_literal" | "raw_string_literal")
        })
        .map(|child| unquote(child.source_fragment.trim()))
        .collect()
}

fn declaration_name(node: &NormalizedTreeNode, index: &NormalizedTreeIndex<'_>) -> Option<String> {
    index
        .children(node)
        .into_iter()
        .find(|child| child.field_name.as_deref() == Some("name"))
        .or_else(|| index.children(node).into_iter().find(|child| child.kind == "identifier"))
        .map(|child| child.source_fragment.clone())
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if matches!(bytes[0], b'`' | b'"') && bytes[0] == bytes[value.len() - 1] {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

pub fn match_go_owners(template: &GoAnalysis, destination: &GoAnalysis) -> GoOwnerMatchResult {
    let destination_owners = destination
        .owners
        .iter()
        .map(|owner| owner.path.clone())
        .collect::<std::collections::HashSet<_>>();
    let template_owners = template
        .owners
        .iter()
        .map(|owner| owner.path.clone())
        .collect::<std::collections::HashSet<_>>();

    GoOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| GoOwnerMatch {
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

pub fn merge_go(
    template_source: &str,
    destination_source: &str,
    dialect: GoDialect,
) -> MergeResult<String> {
    let template = parse_go(template_source, dialect);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_go(destination_source, dialect);
    if !destination.ok {
        return MergeResult {
            ok: false,
            diagnostics: destination
                .diagnostics
                .into_iter()
                .map(|mut diagnostic| {
                    if diagnostic.category == ast_merge::DiagnosticCategory::ParseError {
                        diagnostic.category = ast_merge::DiagnosticCategory::DestinationParseError;
                    }
                    diagnostic
                })
                .collect(),
            output: None,
            policies: vec![],
        };
    }

    let template_analysis = template.analysis.expect("successful parse should include analysis");
    let destination_analysis =
        destination.analysis.expect("successful parse should include analysis");
    let destination_declarations = destination_analysis
        .declarations
        .iter()
        .map(|item| item.path.clone())
        .collect::<std::collections::HashSet<_>>();
    let import_lines = destination_analysis
        .imports
        .iter()
        .map(|item| item.text.clone())
        .collect::<Vec<_>>()
        .join("");
    let merged_declarations = [
        destination_analysis.declarations.iter().map(|item| item.text.clone()).collect::<Vec<_>>(),
        template_analysis
            .declarations
            .iter()
            .filter(|item| !destination_declarations.contains(&item.path))
            .map(|item| item.text.clone())
            .collect::<Vec<_>>(),
    ]
    .concat()
    .join("\n")
    .trim()
    .to_string();

    let package_clause = package_clause_text(destination_source).unwrap_or_default();
    let output = [package_clause, import_lines.trim().to_string(), merged_declarations]
        .into_iter()
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(format!("{output}\n")),
        policies: vec![destination_wins_array_policy()],
    }
}

fn package_clause_text(source: &str) -> Option<String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return None;
    }
    let index = NormalizedTreeIndex::new(&parsed.nodes).ok()?;
    let root = index.root(&parsed.root_id).ok()?;
    index
        .children(root)
        .into_iter()
        .find(|node| node.kind == "package_clause")
        .map(|node| node.source_fragment.trim().to_string())
}
