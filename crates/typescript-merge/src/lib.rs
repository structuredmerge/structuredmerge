use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, FamilyFeatureProfile, MergeResult,
    NamedOwnerKind, NamedOwnerProjectionPolicy, ParseResult, PolicyReference, PolicySurface,
    SourcePreservingOwnerDocument, ThreeWayMergeResult, merge_source_preserving_owners,
    normalized_parse_error_result, parse_error_result, project_named_top_level_owners,
    three_way_parse_error,
};
use tree_haver::{
    BackendReference, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "typescript-merge";

const TYPESCRIPT_DECLARATION_OWNER_KINDS: &[NamedOwnerKind<'static>] = &[
    NamedOwnerKind { node_kind: "class_declaration", path_kind: "class" },
    NamedOwnerKind { node_kind: "enum_declaration", path_kind: "enum" },
    NamedOwnerKind { node_kind: "function_declaration", path_kind: "function" },
    NamedOwnerKind { node_kind: "function_signature", path_kind: "function_signature" },
    NamedOwnerKind { node_kind: "interface_declaration", path_kind: "interface" },
    NamedOwnerKind { node_kind: "internal_module", path_kind: "internal_module" },
    NamedOwnerKind { node_kind: "type_alias_declaration", path_kind: "type_alias" },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeScriptDialect {
    TypeScript,
    Tsx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeScriptBackend {
    TreeSitter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeScriptOwnerKind {
    Import,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptOwner {
    pub path: String,
    pub owner_kind: TypeScriptOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptOwnerMatchResult {
    pub matched: Vec<TypeScriptOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleImport {
    pub path: String,
    pub match_key: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDeclaration {
    pub path: String,
    pub match_key: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptAnalysis {
    pub dialect: TypeScriptDialect,
    pub source: String,
    pub owners: Vec<TypeScriptOwner>,
    pub imports: Vec<ModuleImport>,
    pub declarations: Vec<ModuleDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<TypeScriptDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptBackendFeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
}

fn parse_request(source: &str, dialect: TypeScriptDialect) -> ParserRequest {
    let language = match dialect {
        TypeScriptDialect::TypeScript => "typescript",
        TypeScriptDialect::Tsx => "tsx",
    };
    ParserRequest {
        source: source.to_string(),
        language: language.to_string(),
        dialect: Some(language.to_string()),
    }
}

fn slice_span(source: &str, start: usize, end: usize) -> String {
    source[start..end].trim().to_string()
}

fn line_anchored_span(source: &str, start: usize, end: usize) -> String {
    let line_start = source[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[line_start..end].trim().to_string()
}

pub fn typescript_feature_profile() -> TypeScriptFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "typescript".to_string(),
        supported_dialects: vec!["typescript".to_string(), "tsx".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    TypeScriptFeatureProfile {
        family: "typescript",
        supported_dialects: vec![TypeScriptDialect::TypeScript, TypeScriptDialect::Tsx],
        supported_policies: shared.supported_policies,
    }
}

pub fn typescript_backend_feature_profile(
    _backend: TypeScriptBackend,
) -> TypeScriptBackendFeatureProfile {
    TypeScriptBackendFeatureProfile {
        backend: language_pack_adapter_info().backend,
        backend_ref: Some(kreuzberg_language_pack_backend()),
        supports_dialects: true,
        supported_policies: vec![destination_wins_array_policy()],
    }
}

pub fn typescript_plan_context(backend: TypeScriptBackend) -> ConformanceFamilyPlanContext {
    let feature_profile = typescript_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: typescript_feature_profile().family.to_string(),
            supported_dialects: vec!["typescript".to_string(), "tsx".to_string()],
            supported_policies: typescript_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: feature_profile.backend,
            supports_dialects: feature_profile.supports_dialects,
            supported_policies: feature_profile.supported_policies,
        }),
        merge_engine: None,
    }
}

pub fn typescript_backends() -> Vec<TypeScriptBackend> {
    vec![TypeScriptBackend::TreeSitter]
}

pub fn parse_typescript(
    source: &str,
    dialect: TypeScriptDialect,
) -> ParseResult<TypeScriptAnalysis> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source, dialect));
    if !parsed.ok {
        return normalized_parse_error_result(parsed.diagnostics);
    }
    let index = match NormalizedTreeIndex::new(&parsed.nodes) {
        Ok(index) => index,
        Err(message) => return parse_error(message),
    };
    let Ok(root) = index.root(&parsed.root_id) else {
        return parse_error("normalized TypeScript parse has no valid root node");
    };
    let top_level = index.children(root);
    let mut imports = Vec::new();
    let mut declarations = Vec::new();
    for node in top_level {
        if node.kind == "comment" {
            continue;
        }
        if node.kind == "import_statement" {
            let Some(module) = index.find_descendant(node, |child| child.kind == "string") else {
                return parse_error("TypeScript import has no literal module source");
            };
            let match_key = unquote(module.source_fragment.trim());
            imports.push(ModuleImport {
                path: format!("/imports/{}", imports.len()),
                match_key,
                text: format!(
                    "{}\n",
                    slice_span(source, node.span.range.start_byte, node.span.range.end_byte)
                ),
            });
            continue;
        }
        let declaration = declaration_node(node, &index);
        let Some(name) = declaration.and_then(|value| declaration_name(value, &index)) else {
            return parse_error(format!("unsupported top-level TypeScript node {:?}", node.kind));
        };
        declarations.push(ModuleDeclaration {
            path: format!("/declarations/{name}"),
            match_key: name,
            text: format!(
                "{}\n",
                line_anchored_span(source, node.span.range.start_byte, node.span.range.end_byte)
            ),
        });
    }
    declarations.sort_by(|left, right| left.path.cmp(&right.path));

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(TypeScriptAnalysis {
            dialect,
            source: source.to_string(),
            owners: [
                imports
                    .iter()
                    .map(|item| TypeScriptOwner {
                        path: item.path.clone(),
                        owner_kind: TypeScriptOwnerKind::Import,
                        match_key: Some(item.match_key.clone()),
                    })
                    .collect::<Vec<_>>(),
                declarations
                    .iter()
                    .map(|item| TypeScriptOwner {
                        path: item.path.clone(),
                        owner_kind: TypeScriptOwnerKind::Declaration,
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

pub fn merge_typescript_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    dialect: TypeScriptDialect,
) -> ThreeWayMergeResult<String> {
    let base = match parse_source_preserving_typescript(base_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("base", message),
    };
    let ours = match parse_source_preserving_typescript(ours_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("ours", message),
    };
    let theirs = match parse_source_preserving_typescript(theirs_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("theirs", message),
    };

    merge_source_preserving_owners(base, ours, theirs, |output| {
        parse_source_preserving_typescript(output, dialect)
    })
}

fn parse_source_preserving_typescript(
    source: &str,
    dialect: TypeScriptDialect,
) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source, dialect));
    if !parsed.ok {
        return Err(parsed.diagnostics.join("; "));
    }
    if !parsed.source_fragments_available {
        return Err("TypeScript parser did not retain source fragments".to_string());
    }
    project_named_top_level_owners(
        source,
        &parsed.root_id,
        &parsed.nodes,
        NamedOwnerProjectionPolicy {
            family: "TypeScript",
            owner_kinds: TYPESCRIPT_DECLARATION_OWNER_KINDS,
            ignored_kinds: &[],
            wrapper_kinds: &["export_statement", "ambient_declaration"],
            name_fields: &["name"],
            fallback_name_kinds: &["identifier", "type_identifier"],
            accept_any_named_kind: false,
        },
    )
}

fn declaration_node<'a>(
    node: &'a NormalizedTreeNode,
    index: &NormalizedTreeIndex<'a>,
) -> Option<&'a NormalizedTreeNode> {
    if supported_declaration_kind(&node.kind) {
        return Some(node);
    }
    if node.kind == "export_statement" || node.kind == "ambient_declaration" {
        return index
            .children(node)
            .into_iter()
            .find(|child| supported_declaration_kind(&child.kind));
    }
    None
}

fn supported_declaration_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "enum_declaration"
            | "function_declaration"
            | "function_signature"
            | "interface_declaration"
            | "internal_module"
            | "type_alias_declaration"
    )
}

fn declaration_name(node: &NormalizedTreeNode, index: &NormalizedTreeIndex<'_>) -> Option<String> {
    index
        .children(node)
        .into_iter()
        .find(|child| child.field_name.as_deref() == Some("name"))
        .or_else(|| {
            index
                .children(node)
                .into_iter()
                .find(|child| matches!(child.kind.as_str(), "identifier" | "type_identifier"))
        })
        .map(|child| child.source_fragment.clone())
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if matches!(bytes[0], b'\'' | b'"') && bytes[0] == bytes[value.len() - 1] {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn parse_error<T>(message: impl Into<String>) -> ParseResult<T> {
    parse_error_result(message)
}

pub fn match_typescript_owners(
    template: &TypeScriptAnalysis,
    destination: &TypeScriptAnalysis,
) -> TypeScriptOwnerMatchResult {
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

    TypeScriptOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| TypeScriptOwnerMatch {
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

pub fn merge_typescript(
    template_source: &str,
    destination_source: &str,
    dialect: TypeScriptDialect,
) -> MergeResult<String> {
    let template = parse_typescript(template_source, dialect);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_typescript(destination_source, dialect);
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

    let output = [import_lines.trim().to_string(), merged_declarations]
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
