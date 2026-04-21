use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, FamilyFeatureProfile, MergeResult,
    ParseResult, PolicyReference, PolicySurface,
};
use tree_haver::{
    BackendReference, ParserRequest, ProcessRequest, kreuzberg_language_pack_backend,
    language_pack_adapter_info, parse_with_language_pack, process_with_language_pack,
};

pub const PACKAGE_NAME: &str = "go-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoDialect {
    Go,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoBackend {
    TreeSitter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoOwnerKind {
    Import,
    Declaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoOwner {
    pub path: String,
    pub owner_kind: GoOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoOwnerMatchResult {
    pub matched: Vec<GoOwnerMatch>,
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

fn process_request(source: &str) -> ProcessRequest {
    ProcessRequest { source: source.to_string(), language: "go".to_string() }
}

fn slice_span(source: &str, start: usize, end: usize) -> String {
    source[start..end].trim().to_string()
}

fn line_anchored_span(source: &str, start: usize, end: usize) -> String {
    let line_start = source[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[line_start..end].trim().to_string()
}

fn normalize_go_import_path(import_source: &str) -> String {
    import_source
        .split('"')
        .nth(1)
        .unwrap_or(import_source.trim().trim_start_matches("import ").trim())
        .to_string()
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
    }
}

pub fn go_backends() -> Vec<GoBackend> {
    vec![GoBackend::TreeSitter]
}

pub fn parse_go(source: &str, _dialect: GoDialect) -> ParseResult<GoAnalysis> {
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
    let mut deduped_imports = std::collections::BTreeMap::<String, ModuleImport>::new();
    for item in &analysis.imports {
        let match_key = normalize_go_import_path(&item.source);
        let candidate = ModuleImport {
            path: String::new(),
            match_key: match_key.clone(),
            text: format!("{}\n", slice_span(source, item.span.start_byte, item.span.end_byte)),
        };
        match deduped_imports.get(&match_key) {
            Some(current) if current.text.len() >= candidate.text.len() => {}
            _ => {
                deduped_imports.insert(match_key, candidate);
            }
        }
    }
    let imports = deduped_imports
        .into_values()
        .enumerate()
        .map(|(index, mut item)| {
            item.path = format!("/imports/{index}");
            item
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
