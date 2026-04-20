use ast_merge::{FamilyFeatureProfile, MergeResult, ParseResult, PolicyReference, PolicySurface};
use tree_haver::{
    ParserRequest, ProcessRequest, parse_with_language_pack, process_with_language_pack,
};

pub const PACKAGE_NAME: &str = "typescript-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeScriptDialect {
    TypeScript,
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

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
}

fn parse_request(source: &str) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: "typescript".to_string(),
        dialect: Some("typescript".to_string()),
    }
}

fn process_request(source: &str) -> ProcessRequest {
    ProcessRequest { source: source.to_string(), language: "typescript".to_string() }
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
        supported_dialects: vec!["typescript".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    TypeScriptFeatureProfile {
        family: "typescript",
        supported_dialects: shared
            .supported_dialects
            .iter()
            .map(|_| TypeScriptDialect::TypeScript)
            .collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn parse_typescript(
    source: &str,
    _dialect: TypeScriptDialect,
) -> ParseResult<TypeScriptAnalysis> {
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
            match_key: item.source.clone(),
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
        analysis: Some(TypeScriptAnalysis {
            dialect: TypeScriptDialect::TypeScript,
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
