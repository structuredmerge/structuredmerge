use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, FamilyFeatureProfile, MergeResult,
    NamedOwnerKind, NamedOwnerProjectionPolicy, ParseResult, PolicyReference, PolicySurface,
    SourcePreservingOwnerDocument, ThreeWayMergeOutcome, ThreeWayMergeResult, error_diagnostic,
    merge_source_preserving_owners, normalized_parse_error_result, parse_error_result,
    project_named_top_level_owners, three_way_parse_error,
};
use syn::File;
use tree_haver::{
    BackendReference, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "rust-merge";

const RUST_SOURCE_PRESERVING_OWNER_KINDS: &[NamedOwnerKind<'static>] = &[
    NamedOwnerKind { node_kind: "const_item", path_kind: "const" },
    NamedOwnerKind { node_kind: "enum_item", path_kind: "enum" },
    NamedOwnerKind { node_kind: "function_item", path_kind: "function" },
    NamedOwnerKind { node_kind: "mod_item", path_kind: "mod" },
    NamedOwnerKind { node_kind: "static_item", path_kind: "static" },
    NamedOwnerKind { node_kind: "struct_item", path_kind: "struct" },
    NamedOwnerKind { node_kind: "trait_item", path_kind: "trait" },
    NamedOwnerKind { node_kind: "type_item", path_kind: "type" },
    NamedOwnerKind { node_kind: "union_item", path_kind: "union" },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RustDialect {
    Rust,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RustBackend {
    TreeSitter,
    Native,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RustOwnerKind {
    Import,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RustOwner {
    pub path: String,
    pub owner_kind: RustOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RustOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RustOwnerMatchResult {
    pub matched: Vec<RustOwnerMatch>,
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
    pub declaration_kind: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RustAnalysis {
    pub dialect: RustDialect,
    pub source: String,
    pub owners: Vec<RustOwner>,
    pub imports: Vec<ModuleImport>,
    pub declarations: Vec<ModuleDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<RustDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustBackendFeatureProfile {
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
        language: "rust".to_string(),
        dialect: Some("rust".to_string()),
    }
}

fn slice_span(source: &str, start: usize, end: usize) -> String {
    source[start..end].trim().to_string()
}

fn line_anchored_span(source: &str, start: usize, end: usize) -> String {
    let line_start = source[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[line_start..end].trim().to_string()
}

pub fn rust_feature_profile() -> RustFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "rust".to_string(),
        supported_dialects: vec!["rust".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    RustFeatureProfile {
        family: "rust",
        supported_dialects: shared.supported_dialects.iter().map(|_| RustDialect::Rust).collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn rust_backend_feature_profile(backend: RustBackend) -> RustBackendFeatureProfile {
    match backend {
        RustBackend::Native => RustBackendFeatureProfile {
            backend: "syn".to_string(),
            backend_ref: None,
            supports_dialects: true,
            supported_policies: vec![destination_wins_array_policy()],
        },
        RustBackend::TreeSitter => RustBackendFeatureProfile {
            backend: language_pack_adapter_info().backend,
            backend_ref: Some(kreuzberg_language_pack_backend()),
            supports_dialects: true,
            supported_policies: vec![destination_wins_array_policy()],
        },
    }
}

pub fn rust_plan_context(backend: RustBackend) -> ConformanceFamilyPlanContext {
    let feature_profile = rust_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: rust_feature_profile().family.to_string(),
            supported_dialects: vec!["rust".to_string()],
            supported_policies: rust_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: feature_profile.backend,
            supports_dialects: feature_profile.supports_dialects,
            supported_policies: feature_profile.supported_policies,
        }),
        merge_engine: None,
    }
}

pub fn rust_backends() -> Vec<RustBackend> {
    vec![RustBackend::TreeSitter, RustBackend::Native]
}

pub fn parse_rust(source: &str, _dialect: RustDialect) -> ParseResult<RustAnalysis> {
    parse_rust_with_backend(source, RustDialect::Rust, RustBackend::TreeSitter)
}

pub fn parse_rust_with_backend(
    source: &str,
    _dialect: RustDialect,
    backend: RustBackend,
) -> ParseResult<RustAnalysis> {
    if backend == RustBackend::Native {
        return parse_rust_native(source);
    }

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
            "line_comment" | "block_comment" => {}
            "use_declaration" => imports.push(ModuleImport {
                path: format!("/imports/{}", imports.len()),
                match_key: rust_use_key(&node.source_fragment),
                text: format!(
                    "{}\n",
                    slice_span(source, node.span.range.start_byte, node.span.range.end_byte)
                ),
            }),
            kind if supported_analysis_declaration(kind) => {
                let Some(name) = declaration_name(node, &index) else {
                    return parse_error_result(format!(
                        "Rust declaration {kind:?} has no stable name"
                    ));
                };
                declarations.push(ModuleDeclaration {
                    path: format!("/declarations/{name}"),
                    match_key: name,
                    declaration_kind: rust_declaration_kind(kind).to_string(),
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
            kind => return parse_error_result(format!("unsupported top-level Rust node {kind:?}")),
        }
    }
    declarations.sort_by(|left, right| left.path.cmp(&right.path));

    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(RustAnalysis {
            dialect: RustDialect::Rust,
            source: source.to_string(),
            owners: [
                imports
                    .iter()
                    .map(|item| RustOwner {
                        path: item.path.clone(),
                        owner_kind: RustOwnerKind::Import,
                        match_key: Some(item.match_key.clone()),
                    })
                    .collect::<Vec<_>>(),
                declarations
                    .iter()
                    .map(|item| RustOwner {
                        path: item.path.clone(),
                        owner_kind: RustOwnerKind::Declaration,
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

pub fn merge_rust_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    dialect: RustDialect,
) -> ThreeWayMergeResult<String> {
    merge_rust_three_way_with_backend(
        base_source,
        ours_source,
        theirs_source,
        dialect,
        RustBackend::TreeSitter,
    )
}

pub fn merge_rust_three_way_with_backend(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    _dialect: RustDialect,
    backend: RustBackend,
) -> ThreeWayMergeResult<String> {
    if backend == RustBackend::Native {
        return ThreeWayMergeResult {
            outcome: ThreeWayMergeOutcome::Error,
            diagnostics: vec![error_diagnostic(
                ast_merge::DiagnosticCategory::UnsupportedFeature,
                "syn does not expose source spans required for source-preserving Rust merge3",
            )],
            conflicts: vec![],
            output: None,
            policies: vec![],
        };
    }

    let base = match parse_source_preserving_rust(base_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("base", message),
    };
    let ours = match parse_source_preserving_rust(ours_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("ours", message),
    };
    let theirs = match parse_source_preserving_rust(theirs_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("theirs", message),
    };

    merge_source_preserving_owners(base, ours, theirs, parse_source_preserving_rust)
}

fn parse_source_preserving_rust(source: &str) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return Err(parsed.diagnostics.join("; "));
    }
    if !parsed.source_fragments_available {
        return Err("Rust parser did not retain source fragments".to_string());
    }
    project_named_top_level_owners(
        source,
        &parsed.root_id,
        &parsed.nodes,
        NamedOwnerProjectionPolicy {
            family: "Rust",
            owner_kinds: RUST_SOURCE_PRESERVING_OWNER_KINDS,
            ignored_kinds: &["use_declaration"],
            wrapper_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &["identifier", "type_identifier"],
            accept_any_named_kind: false,
        },
    )
}

fn supported_analysis_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "const_item"
            | "enum_item"
            | "function_item"
            | "mod_item"
            | "static_item"
            | "struct_item"
            | "trait_item"
            | "type_item"
            | "union_item"
    )
}

fn rust_declaration_kind(node_kind: &str) -> &'static str {
    match node_kind {
        "const_item" => "const",
        "enum_item" => "enum",
        "function_item" => "function",
        "mod_item" => "module",
        "static_item" => "static",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "type_item" => "type",
        "union_item" => "union",
        _ => "declaration",
    }
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

fn rust_use_key(source: &str) -> String {
    source
        .trim()
        .strip_prefix("pub ")
        .unwrap_or(source.trim())
        .strip_prefix("use ")
        .unwrap_or(source.trim())
        .strip_suffix(';')
        .unwrap_or(source.trim())
        .trim()
        .to_string()
}

pub fn match_rust_owners(
    template: &RustAnalysis,
    destination: &RustAnalysis,
) -> RustOwnerMatchResult {
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

    RustOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| RustOwnerMatch {
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

pub fn merge_rust(
    template_source: &str,
    destination_source: &str,
    dialect: RustDialect,
) -> MergeResult<String> {
    merge_rust_with_backend(template_source, destination_source, dialect, RustBackend::TreeSitter)
}

pub fn merge_rust_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: RustDialect,
    backend: RustBackend,
) -> MergeResult<String> {
    let template = parse_rust_with_backend(template_source, dialect, backend);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_rust_with_backend(destination_source, dialect, backend);
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
    let destination_declaration_keys = destination_analysis
        .declarations
        .iter()
        .map(|item| item.match_key.clone())
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
            .filter(|item| !destination_declaration_keys.contains(&item.match_key))
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

fn parse_rust_native(source: &str) -> ParseResult<RustAnalysis> {
    let _file: File = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => {
            return ParseResult {
                ok: false,
                diagnostics: vec![ast_merge::Diagnostic {
                    severity: ast_merge::DiagnosticSeverity::Error,
                    category: ast_merge::DiagnosticCategory::ParseError,
                    message: error.to_string(),
                    path: None,
                    review: None,
                }],
                analysis: None,
                policies: vec![],
            };
        }
    };

    ParseResult {
        ok: false,
        diagnostics: vec![ast_merge::Diagnostic {
            severity: ast_merge::DiagnosticSeverity::Error,
            category: ast_merge::DiagnosticCategory::UnsupportedFeature,
            message: "syn backend parsed the file but does not expose source spans required for source-preserving Rust merges; enable a span-capable native backend before using RustBackend::Native.".to_string(),
            path: None,
            review: None,
        }],
        analysis: None,
        policies: vec![],
    }
}
