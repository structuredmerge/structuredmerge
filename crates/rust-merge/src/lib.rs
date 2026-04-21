use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, FamilyFeatureProfile, MergeResult,
    ParseResult, PolicyReference, PolicySurface,
};
use syn::{File, Item};
use tree_haver::{
    BackendReference, ParserRequest, ProcessRequest, kreuzberg_language_pack_backend,
    language_pack_adapter_info, parse_with_language_pack, process_with_language_pack,
};

pub const PACKAGE_NAME: &str = "rust-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustDialect {
    Rust,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustBackend {
    TreeSitter,
    Native,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustOwnerKind {
    Import,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustOwner {
    pub path: String,
    pub owner_kind: RustOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustOwnerMatchResult {
    pub matched: Vec<RustOwnerMatch>,
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

fn process_request(source: &str) -> ProcessRequest {
    ProcessRequest { source: source.to_string(), language: "rust".to_string() }
}

fn slice_span(source: &str, start: usize, end: usize) -> String {
    source[start..end].trim().to_string()
}

fn line_anchored_span(source: &str, start: usize, end: usize) -> String {
    let line_start = source[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[line_start..end].trim().to_string()
}

fn normalize_rust_import_path(import_source: &str) -> String {
    import_source.trim().trim_start_matches("use ").trim_end_matches(';').trim().to_string()
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

    let parsed = parse_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return ParseResult {
            ok: false,
            diagnostics: parsed.diagnostics,
            analysis: None,
            policies: vec![],
        };
    }

    let processed = process_with_language_pack(&process_request(source));
    if !processed.ok {
        return ParseResult {
            ok: false,
            diagnostics: processed.diagnostics,
            analysis: None,
            policies: vec![],
        };
    }

    let analysis = processed.analysis.expect("successful process should include analysis");
    let imports = analysis
        .imports
        .iter()
        .enumerate()
        .map(|(index, item)| ModuleImport {
            path: format!("/imports/{index}"),
            match_key: normalize_rust_import_path(&item.source),
            text: format!("{}\n", slice_span(source, item.span.start_byte, item.span.end_byte)),
        })
        .collect::<Vec<_>>();
    let mut declarations = analysis
        .structure
        .iter()
        .filter_map(|item| {
            item.name.as_ref().map(|name| ModuleDeclaration {
                path: format!("/declarations/{name}"),
                match_key: name.clone(),
                text: format!(
                    "{}\n",
                    line_anchored_span(source, item.span.start_byte, item.span.end_byte)
                ),
            })
        })
        .collect::<Vec<_>>();
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

    let imports = extract_rust_imports(source)
        .into_iter()
        .enumerate()
        .map(|(index, (match_key, text))| ModuleImport {
            path: format!("/imports/{index}"),
            match_key,
            text,
        })
        .collect::<Vec<_>>();
    let mut declarations = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(item_fn) => Some(item_fn.sig.ident.to_string()),
            Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
            _ => None,
        })
        .filter_map(|name| {
            extract_named_rust_item(source, &name).map(|text| ModuleDeclaration {
                path: format!("/declarations/{name}"),
                match_key: name,
                text,
            })
        })
        .collect::<Vec<_>>();
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

fn extract_rust_imports(source: &str) -> Vec<(String, String)> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("use ") {
                return None;
            }

            Some((
                trimmed.trim_start_matches("use ").trim_end_matches(';').trim().to_string(),
                format!("{trimmed}\n"),
            ))
        })
        .collect()
}

fn extract_named_rust_item(source: &str, name: &str) -> Option<String> {
    for keyword in ["pub fn ", "fn ", "pub struct ", "struct "] {
        if let Some(index) = source.find(&format!("{keyword}{name}")) {
            return Some(extract_rust_block_or_line(source, index));
        }
    }
    None
}

fn extract_rust_block_or_line(source: &str, start: usize) -> String {
    let rest = &source[start..];
    let first_line = rest.lines().next().unwrap_or_default().trim();
    if first_line.ends_with(';') {
        return format!("{first_line}\n");
    }

    if let Some(open_brace) = rest.find('{') {
        let mut depth = 0usize;
        for (offset, ch) in rest[open_brace..].char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return format!("{}\n", rest[..open_brace + offset + 1].trim());
                }
            }
        }
    }

    format!("{first_line}\n")
}
