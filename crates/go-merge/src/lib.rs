use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeConflict, MergeResult, NamedOwnerKind,
    NamedOwnerProjectionPolicy, ParseResult, PolicyReference, PolicySurface,
    SourcePreservingOwnerDocument, ThreeWayMergeOutcome, ThreeWayMergeResult,
    merge_source_preserving_owners, normalized_parse_error_result, parse_error_result,
    project_named_top_level_owners, three_way_parse_error,
};
use tree_haver::{
    BackendReference, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "go-merge";

const GO_FUNCTION_OWNER_KINDS: &[NamedOwnerKind<'static>] =
    &[NamedOwnerKind { node_kind: "function_declaration", path_kind: "function" }];

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
    membership_changed
        && base.owners.iter().any(|base_owner| {
            [&ours.owners, &theirs.owners].into_iter().any(|owners| {
                owners
                    .iter()
                    .find(|owner| owner.id == base_owner.id)
                    .is_some_and(|owner| owner.fingerprint != base_owner.fingerprint)
            })
        })
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
    project_named_top_level_owners(
        source,
        &parsed.root_id,
        &parsed.nodes,
        NamedOwnerProjectionPolicy {
            family: "Go",
            owner_kinds: GO_FUNCTION_OWNER_KINDS,
            ignored_kinds: &["package_clause", "import_declaration"],
            wrapper_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &["identifier"],
            accept_any_named_kind: false,
        },
    )
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
