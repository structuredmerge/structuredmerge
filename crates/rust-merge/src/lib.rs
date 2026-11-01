use ast_merge::{
    CommentAttachmentPolicy, ConformanceFamilyPlanContext, ConformanceFeatureProfileView,
    Diagnostic, DiagnosticCategory, DiagnosticSeverity, FamilyFeatureProfile, MergeConflict,
    MergeResult, NamedOwnerKind, NamedOwnerProjectionPolicy, ParseResult, PolicyReference,
    PolicySurface, SourcePreservingOwnerDocument, ThreeWayMergeOutcome, ThreeWayMergeResult,
    merge_source_preserving_owners, normalized_parse_error_result, parse_error_result,
    project_named_top_level_owners, three_way_parse_error,
};
use syn::{File, Item, spanned::Spanned};
use tree_haver::{
    BackendReference, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "rust-merge";

pub mod directional;
pub mod syn_parser;
pub mod typed;
pub use syn_parser::RustSynParserProvider;

const RUST_SOURCE_PRESERVING_OWNER_KINDS: &[NamedOwnerKind<'static>] = &[
    NamedOwnerKind { node_kind: "const_item", path_kind: "const" },
    NamedOwnerKind { node_kind: "enum_item", path_kind: "enum" },
    NamedOwnerKind { node_kind: "extern_crate_declaration", path_kind: "extern_crate" },
    NamedOwnerKind { node_kind: "function_item", path_kind: "function" },
    NamedOwnerKind { node_kind: "macro_definition", path_kind: "macro" },
    NamedOwnerKind { node_kind: "macro_invocation", path_kind: "macro_invocation" },
    NamedOwnerKind { node_kind: "mod_item", path_kind: "mod" },
    NamedOwnerKind { node_kind: "static_item", path_kind: "static" },
    NamedOwnerKind { node_kind: "struct_item", path_kind: "struct" },
    NamedOwnerKind { node_kind: "trait_item", path_kind: "trait" },
    NamedOwnerKind { node_kind: "trait_alias_item", path_kind: "trait_alias" },
    NamedOwnerKind { node_kind: "type_item", path_kind: "type" },
    NamedOwnerKind { node_kind: "union_item", path_kind: "union" },
    NamedOwnerKind { node_kind: "use_declaration", path_kind: "use" },
];
const RUST_SYN_SOURCE_PRESERVING_OWNER_KINDS: &[NamedOwnerKind<'static>] = &[
    NamedOwnerKind { node_kind: "const_item", path_kind: "const" },
    NamedOwnerKind { node_kind: "enum_item", path_kind: "enum" },
    NamedOwnerKind { node_kind: "extern_crate_declaration", path_kind: "extern_crate" },
    NamedOwnerKind { node_kind: "function_item", path_kind: "function" },
    NamedOwnerKind { node_kind: "impl_item", path_kind: "impl" },
    NamedOwnerKind { node_kind: "macro_definition", path_kind: "macro" },
    NamedOwnerKind { node_kind: "macro_invocation", path_kind: "macro_invocation" },
    NamedOwnerKind { node_kind: "mod_item", path_kind: "mod" },
    NamedOwnerKind { node_kind: "static_item", path_kind: "static" },
    NamedOwnerKind { node_kind: "struct_item", path_kind: "struct" },
    NamedOwnerKind { node_kind: "trait_item", path_kind: "trait" },
    NamedOwnerKind { node_kind: "trait_alias_item", path_kind: "trait_alias" },
    NamedOwnerKind { node_kind: "type_item", path_kind: "type" },
    NamedOwnerKind { node_kind: "union_item", path_kind: "union" },
    NamedOwnerKind { node_kind: "use_declaration", path_kind: "use" },
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

fn unsupported_feature_result<T>(message: impl Into<String>) -> ParseResult<T> {
    let mut result = parse_error_result(message);
    if let Some(diagnostic) = result.diagnostics.first_mut() {
        diagnostic.category = DiagnosticCategory::UnsupportedFeature;
    }
    result
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
            kind => {
                return unsupported_feature_result(format!(
                    "unsupported top-level Rust node {kind:?}"
                ));
            }
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
    let parse = match backend {
        RustBackend::Native => parse_source_preserving_rust_native,
        RustBackend::TreeSitter => parse_source_preserving_rust,
    };
    let base = match parse(base_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("base", message),
    };
    let ours = match parse(ours_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("ours", message),
    };
    let theirs = match parse(theirs_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("theirs", message),
    };

    if rust_membership_change_with_owner_edit(&base, &ours, &theirs) {
        return conservative_membership_conflict();
    }

    merge_source_preserving_owners(base, ours, theirs, parse)
}

fn rust_membership_change_with_owner_edit(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> bool {
    let non_use_ids = |document: &SourcePreservingOwnerDocument| {
        document
            .owners
            .iter()
            .filter(|owner| !owner.id.starts_with("/use:"))
            .map(|owner| owner.id.clone())
            .collect::<std::collections::HashSet<_>>()
    };
    let base_ids = non_use_ids(base);
    let ours_ids = non_use_ids(ours);
    let theirs_ids = non_use_ids(theirs);
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
        conflict_id: "rust-unmanaged-source".to_string(),
        category: "unmanaged_source_change".to_string(),
        path: "<unmanaged-source>".to_string(),
        fallback_scope: "full_file".to_string(),
        message: "Rust owner membership changed alongside an existing owner edit; source ownership is unproven"
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

fn parse_source_preserving_rust(source: &str) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return Err(parsed.diagnostics.join("; "));
    }
    if !parsed.source_fragments_available {
        return Err("Rust parser did not retain source fragments".to_string());
    }
    project_named_top_level_owners(source, &parsed.root_id, &parsed.nodes, owner_policy())
}

fn parse_source_preserving_rust_native(
    source: &str,
) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_rust_with_syn_provider(source)?;
    if !parsed.document.output().ok {
        let message = parsed
            .document
            .output()
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(if message.is_empty() {
            "native Rust parser rejected input".into()
        } else {
            message
        });
    }
    typed::analysis(&parsed).map(|analysis| analysis.document)
}

fn parse_source_preserving_rust_native_with_treesitter_comments(
    source: &str,
) -> Result<SourcePreservingOwnerDocument, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut owners = Vec::with_capacity(file.items.len());
    for item in file.items {
        let span = item.span().byte_range();
        let text = source
            .get(span.clone())
            .ok_or_else(|| "syn returned a source span outside the input buffer".to_string())?;
        let path = match &item {
            Item::Use(_) => format!("/use:{}", rust_use_key(text)),
            Item::Const(item) => format!("/const:{}", item.ident),
            Item::Enum(item) => format!("/enum:{}", item.ident),
            Item::ExternCrate(item) => format!("/extern_crate:{}", item.ident),
            Item::Fn(item) => format!("/function:{}", item.sig.ident),
            Item::Impl(item) => format!(
                "/impl:{}",
                RustSynParserProvider::impl_owner_identity(item, source)
                    .ok_or_else(|| "unsupported native Rust impl target type".to_string())?
            ),
            Item::Mod(item) => format!("/mod:{}", item.ident),
            Item::Static(item) => format!("/static:{}", item.ident),
            Item::Struct(item) => format!("/struct:{}", item.ident),
            Item::Trait(item) => format!("/trait:{}", item.ident),
            Item::TraitAlias(item) => format!("/trait_alias:{}", item.ident),
            Item::Type(item) => format!("/type:{}", item.ident),
            Item::Union(item) => format!("/union:{}", item.ident),
            _ => return Err("unsupported top-level native Rust item".to_string()),
        };
        if span.start >= span.end {
            return Err(format!("native Rust owner {path:?} has an empty source span"));
        }
        let start_line = source[..span.start].bytes().filter(|byte| *byte == b'\n').count() + 1;
        let end_line = source[..span.end].bytes().filter(|byte| *byte == b'\n').count() + 1;
        owners.push(ast_merge::SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint: text.to_string(),
            start_byte: span.start,
            end_byte: span.end,
            start_line,
            end_line,
        });
    }
    let comment_projection = parse_source_preserving_rust(source)?;
    for owner in &mut owners {
        let comment_owner = comment_projection
            .owners
            .iter()
            .find(|candidate| candidate.path == owner.path)
            .ok_or_else(|| {
                format!("shared comment projection could not match native owner {:?}", owner.path)
            })?;
        if comment_owner.start_byte > owner.start_byte || comment_owner.end_byte < owner.end_byte {
            return Err(format!(
                "shared comment projection does not contain native owner {:?}",
                owner.path
            ));
        }
        owner.start_byte = comment_owner.start_byte;
        owner.start_line = comment_owner.start_line;
        owner.end_byte = comment_owner.end_byte;
        owner.end_line = comment_owner.end_line;
        owner.fingerprint = source[owner.start_byte..owner.end_byte].to_string();
    }
    Ok(SourcePreservingOwnerDocument { source: source.to_string(), owners })
}

fn parse_rust_with_syn_provider(source: &str) -> Result<tree_haver::service::ParsedResult, String> {
    use std::sync::{Arc, atomic::AtomicBool};
    use tree_haver::{
        service::{
            ExecutionContext, PARSE_REQUEST_SCHEMA, ParseOptions, ParseRequest, ParseService,
            ParserRegistry, ParserSelection, TreeHaverParseService,
        },
        source::{SourceEncoding, SourceRole, source_input},
    };

    let provider_id = "rust.syn.direct";
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(RustSynParserProvider::new(provider_id)))
        .map_err(|error| format!("could not register native Rust parser: {error:?}"))?;
    let snapshot = registry
        .snapshot()
        .map_err(|error| format!("could not snapshot native Rust parser: {error:?}"))?;
    let request = ParseRequest {
        schema: PARSE_REQUEST_SCHEMA.into(),
        request_id: "rust-native-parse".into(),
        source: source_input(
            "source".into(),
            SourceRole::Source,
            SourceEncoding::Utf8,
            source.as_bytes().to_vec(),
        )
        .map_err(|error| error.to_string())?,
        language: "rust".into(),
        dialect: Some("rust".into()),
        selection: ParserSelection {
            backend_id: Some(provider_id.into()),
            preference: vec![],
            required_capabilities: vec![],
        },
        options: ParseOptions {
            comments: true,
            tokens: false,
            diagnostics: true,
            native_extensions: true,
        },
        metadata: Default::default(),
        extra: Default::default(),
    };
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 1,
        max_input_bytes: u64::MAX,
        max_nodes: usize::MAX,
        max_diagnostics: usize::MAX,
    };
    TreeHaverParseService::default()
        .parse_batch(vec![request], &snapshot, &context)
        .map_err(|error| format!("native Rust parser service failed: {error:?}"))?
        .into_iter()
        .next()
        .ok_or_else(|| "native Rust parser service returned no result".into())
}

fn owner_policy() -> NamedOwnerProjectionPolicy<'static> {
    owner_policy_with_kinds(RUST_SOURCE_PRESERVING_OWNER_KINDS)
}

fn owner_policy_for_syn() -> NamedOwnerProjectionPolicy<'static> {
    owner_policy_with_kinds(RUST_SYN_SOURCE_PRESERVING_OWNER_KINDS)
}

fn owner_policy_with_kinds(
    owner_kinds: &'static [NamedOwnerKind<'static>],
) -> NamedOwnerProjectionPolicy<'static> {
    NamedOwnerProjectionPolicy {
        family: "Rust",
        owner_kinds,
        ignored_kinds: &["inner_attribute_item", "shebang"],
        wrapper_kinds: &[],
        prefix_node_kinds: &["attribute_item"],
        name_fields: &["name"],
        fallback_name_kinds: &[
            "identifier",
            "type_identifier",
            "scoped_identifier",
            "use_as_clause",
            "scoped_use_list",
            "use_wildcard",
        ],
        accept_any_named_kind: false,
        comment_attachment_policy: CommentAttachmentPolicy::FollowingOwner,
    }
}

fn supported_analysis_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "const_item"
            | "enum_item"
            | "extern_crate_declaration"
            | "function_item"
            | "impl_item"
            | "macro_definition"
            | "macro_invocation"
            | "mod_item"
            | "static_item"
            | "struct_item"
            | "trait_item"
            | "trait_alias_item"
            | "type_item"
            | "union_item"
    )
}

fn rust_declaration_kind(node_kind: &str) -> &'static str {
    match node_kind {
        "const_item" => "const",
        "enum_item" => "enum",
        "extern_crate_declaration" => "extern_crate",
        "function_item" => "function",
        "impl_item" => "impl",
        "macro_definition" => "macro",
        "macro_invocation" => "macro_invocation",
        "mod_item" => "module",
        "static_item" => "static",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "trait_alias_item" => "trait_alias",
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
    if backend == RustBackend::Native {
        return merge_rust_native_two_way(template_source, destination_source);
    }
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

    // Keep Tree-sitter as the selected backend acceptance gate. Rust-native syn
    // item spans and shared source-line dispositions drive exact owner emission.
    merge_rust_native_two_way(template_source, destination_source)
}

fn native_merge_error_result(message: impl Into<String>) -> MergeResult<String> {
    let parse = parse_error_result::<()>(message);
    MergeResult { ok: false, diagnostics: parse.diagnostics, output: None, policies: vec![] }
}

fn merge_rust_native_two_way(
    template_source: &str,
    destination_source: &str,
) -> MergeResult<String> {
    use std::collections::{HashMap, HashSet};

    let parse = |source: &str| -> Result<tree_haver::service::ParsedResult, String> {
        let parsed = parse_rust_with_syn_provider(source)?;
        if !parsed.document.output().ok {
            let message = parsed
                .document
                .output()
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(if message.is_empty() {
                "native Rust parser rejected input".into()
            } else {
                message
            });
        }
        Ok(parsed)
    };
    let incoming_parse = match parse(template_source) {
        Ok(parsed) => parsed,
        Err(message) => return native_merge_error_result(format!("template: {message}")),
    };
    let current_parse = match parse(destination_source) {
        Ok(parsed) => parsed,
        Err(message) => return native_merge_error_result(format!("destination: {message}")),
    };
    let incoming = match typed::source_line_analysis(&incoming_parse) {
        Ok(analysis) => analysis,
        Err(message) => return native_merge_error_result(format!("template ownership: {message}")),
    };
    let current = match typed::source_line_analysis(&current_parse) {
        Ok(analysis) => analysis,
        Err(message) => {
            return native_merge_error_result(format!("destination ownership: {message}"));
        }
    };

    let current_owner_ids = current
        .owners
        .document
        .owners
        .iter()
        .map(|owner| owner.id.as_str())
        .collect::<HashSet<_>>();
    let mut owner_revisions = current
        .owners
        .document
        .owners
        .iter()
        .map(|owner| (owner.id.clone(), ast_merge::SourceRevision::Ours))
        .collect::<HashMap<_, _>>();
    let additions = incoming
        .owners
        .document
        .owners
        .iter()
        .filter(|owner| {
            !owner.id.starts_with("/use:") && !current_owner_ids.contains(owner.id.as_str())
        })
        .collect::<Vec<_>>();
    let addition_ids = additions.iter().map(|owner| owner.id.as_str()).collect::<HashSet<_>>();
    for owner in &additions {
        owner_revisions.insert(owner.id.clone(), ast_merge::SourceRevision::Theirs);
    }

    let sources = HashMap::from([
        (ast_merge::SourceRevision::Ours, current.owners.document.source.clone()),
        (ast_merge::SourceRevision::Theirs, incoming.owners.document.source.clone()),
    ]);
    let dispositions = HashMap::from([
        (ast_merge::SourceRevision::Ours, current.dispositions.clone()),
        (ast_merge::SourceRevision::Theirs, incoming.dispositions.clone()),
    ]);
    let selections = match ast_merge::source_line_selections_for_owner_revisions(
        ast_merge::SourceRevision::Ours,
        &dispositions,
        &owner_revisions,
    ) {
        Ok(selections) => selections,
        Err(error) => return native_merge_error_result(error.to_string()),
    };
    let plan = match ast_merge::source_render_plan_from_line_selections(
        sources,
        dispositions,
        &selections,
    ) {
        Ok(plan) => plan,
        Err(error) => return native_merge_error_result(error.to_string()),
    };
    let rendered = match ast_merge::render_source_plan(&plan) {
        Ok(rendered) => rendered,
        Err(error) => return native_merge_error_result(error.to_string()),
    };
    let verified = match parse_source_preserving_rust_native(&rendered.content) {
        Ok(document) => document,
        Err(message) => {
            return native_merge_error_result(format!(
                "native Rust output verification failed: {message}"
            ));
        }
    };
    let expected_owner_ids = current
        .owners
        .document
        .owners
        .iter()
        .map(|owner| owner.id.as_str())
        .chain(
            incoming
                .owners
                .document
                .owners
                .iter()
                .filter(|owner| addition_ids.contains(owner.id.as_str()))
                .map(|owner| owner.id.as_str()),
        )
        .collect::<Vec<_>>();
    if verified.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>()
        != expected_owner_ids
    {
        return native_merge_error_result(
            "native Rust output changed destination-first owner selection or order",
        );
    }
    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(rendered.content),
        policies: vec![destination_wins_array_policy()],
    }
}

fn parse_rust_native(source: &str) -> ParseResult<RustAnalysis> {
    let file: File = match syn::parse_file(source) {
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

    let mut imports = Vec::new();
    let mut declarations = Vec::new();
    for item in file.items {
        let span = item.span().byte_range();
        let Some(text) = source.get(span).map(str::trim) else {
            return parse_error_result(
                "syn returned a source span outside the input buffer".to_string(),
            );
        };
        match item {
            Item::Use(_item) => imports.push(ModuleImport {
                path: format!("/imports/{}", imports.len()),
                match_key: rust_use_key(text),
                text: format!("{text}\n"),
            }),
            Item::Const(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "const",
            )),
            Item::Enum(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "enum",
            )),
            Item::ExternCrate(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "extern_crate",
            )),
            Item::Fn(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.sig.ident.to_string(),
                "function",
            )),
            Item::Impl(item) => match RustSynParserProvider::impl_owner_identity(&item, source) {
                Some(identity) => {
                    declarations.push(native_declaration(source, item.span(), identity, "impl"))
                }
                None => {
                    return unsupported_feature_result("unsupported native Rust impl target type");
                }
            },
            Item::Mod(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "module",
            )),
            Item::Static(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "static",
            )),
            Item::Struct(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "struct",
            )),
            Item::Trait(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "trait",
            )),
            Item::TraitAlias(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "trait_alias",
            )),
            Item::Type(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "type",
            )),
            Item::Union(item) => declarations.push(native_declaration(
                source,
                item.span(),
                item.ident.to_string(),
                "union",
            )),
            other => {
                return unsupported_feature_result(format!(
                    "unsupported top-level native Rust item {:?}",
                    std::mem::discriminant(&other)
                ));
            }
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

fn native_declaration(
    source: &str,
    span: proc_macro2::Span,
    name: String,
    kind: &str,
) -> ModuleDeclaration {
    let text = span.byte_range();
    let content = source.get(text).unwrap_or_default().trim();
    ModuleDeclaration {
        path: format!("/declarations/{name}"),
        match_key: name,
        declaration_kind: kind.to_string(),
        text: format!("{content}\n"),
    }
}
