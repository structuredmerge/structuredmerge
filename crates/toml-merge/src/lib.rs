use ast_merge::{
    CommentAttachment, CommentRegion, ConformanceFamilyPlanContext, ConformanceFeatureProfileView,
    Diagnostic, DiagnosticCategory, DiagnosticSeverity, FamilyFeatureProfile, LayoutGap,
    MergeResult, ParseResult, PolicyReference, PolicySurface, match_owner_paths,
};
use tree_haver::{BackendReference, kreuzberg_language_pack_backend};

mod source_preserving;

pub use source_preserving::{TomlProjectionEntry, TomlProjectionScope, TomlProjectionScopeKind};
use source_preserving::{
    TomlSyntaxDocument, analyze_toml_document, analyze_toml_projection_document,
    merge_toml_documents,
};

pub const PACKAGE_NAME: &str = "structuredmerge-toml-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlDialect {
    Toml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlRootKind {
    Table,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlOwnerKind {
    Table,
    TableArray,
    KeyValue,
    ArrayItem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlOwner {
    pub path: String,
    pub owner_kind: TomlOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlOwnerMatchResult {
    pub matched: Vec<TomlOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlAnalysis {
    pub dialect: TomlDialect,
    pub normalized_source: String,
    pub root_kind: TomlRootKind,
    pub owners: Vec<TomlOwner>,
    pub comment_regions: Vec<CommentRegion>,
    pub layout_gaps: Vec<LayoutGap>,
    pub comment_attachments: Vec<CommentAttachment>,
    pub(crate) document: TomlSyntaxDocument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlMergeResolution {
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<TomlDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlBackend {
    TreeSitter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlBackendFeatureProfile {
    pub family: &'static str,
    pub backend: String,
    pub backend_ref: BackendReference,
    pub supported_dialects: Vec<TomlDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

fn destination_wins_array_policy() -> PolicyReference {
    PolicyReference { surface: PolicySurface::Array, name: "destination_wins_array".to_string() }
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

fn unsupported_feature(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::UnsupportedFeature,
        message: message.to_string(),
        path: None,
        review: None,
    }
}

pub fn toml_feature_profile() -> TomlFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "toml".to_string(),
        supported_dialects: vec!["toml".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    TomlFeatureProfile {
        family: "toml",
        supported_dialects: shared.supported_dialects.iter().map(|_| TomlDialect::Toml).collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn available_toml_backends() -> Vec<TomlBackend> {
    vec![TomlBackend::TreeSitter]
}

pub fn toml_backend_feature_profile(backend: Option<TomlBackend>) -> TomlBackendFeatureProfile {
    let _ = resolve_backend(backend);
    let backend_ref = kreuzberg_language_pack_backend();

    TomlBackendFeatureProfile {
        family: "toml",
        backend: backend_ref.id.clone(),
        backend_ref,
        supported_dialects: vec![TomlDialect::Toml],
        supported_policies: vec![destination_wins_array_policy()],
    }
}

pub fn toml_plan_context(backend: Option<TomlBackend>) -> ConformanceFamilyPlanContext {
    let backend_profile = toml_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: toml_feature_profile().family.to_string(),
            supported_dialects: vec!["toml".to_string()],
            supported_policies: toml_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: backend_profile.backend,
            supports_dialects: false,
            supported_policies: backend_profile.supported_policies,
        }),
        merge_engine: None,
    }
}

fn resolve_backend(backend: Option<TomlBackend>) -> TomlBackend {
    backend.unwrap_or(TomlBackend::TreeSitter)
}

pub fn analyze_toml_source(source: &str, dialect: TomlDialect) -> ParseResult<TomlAnalysis> {
    if dialect != TomlDialect::Toml {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature("Unsupported TOML dialect.")],
            analysis: None,
            policies: vec![],
        };
    }

    match analyze_toml_document(source) {
        Ok(document) => ParseResult {
            ok: true,
            diagnostics: vec![],
            analysis: Some(TomlAnalysis {
                dialect: TomlDialect::Toml,
                normalized_source: source.to_string(),
                root_kind: TomlRootKind::Table,
                owners: document.owners.clone(),
                comment_regions: document.comment_augmentation.regions.clone(),
                layout_gaps: document.comment_augmentation.gaps.clone(),
                comment_attachments: document.comment_augmentation.attachments.clone(),
                document,
            }),
            policies: vec![],
        },
        Err(message) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&message)],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn analyze_toml_projection(
    source: &str,
    dialect: TomlDialect,
    scopes: Vec<TomlProjectionScope>,
    comments: Vec<ast_merge::TrackedComment>,
) -> ParseResult<TomlAnalysis> {
    if dialect != TomlDialect::Toml {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature("Unsupported TOML dialect.")],
            analysis: None,
            policies: vec![],
        };
    }
    match analyze_toml_projection_document(source, scopes, comments) {
        Ok(document) => ParseResult {
            ok: true,
            diagnostics: vec![],
            analysis: Some(TomlAnalysis {
                dialect,
                normalized_source: source.to_string(),
                root_kind: TomlRootKind::Table,
                owners: document.owners.clone(),
                comment_regions: document.comment_augmentation.regions.clone(),
                layout_gaps: document.comment_augmentation.gaps.clone(),
                comment_attachments: document.comment_augmentation.attachments.clone(),
                document,
            }),
            policies: vec![],
        },
        Err(message) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&message)],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn parse_toml(
    source: &str,
    dialect: TomlDialect,
    backend: Option<TomlBackend>,
) -> ParseResult<TomlAnalysis> {
    let resolved = resolve_backend(backend);
    if resolved != TomlBackend::TreeSitter {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {resolved:?}."
            ))],
            analysis: None,
            policies: vec![],
        };
    }

    analyze_toml_source(source, dialect)
}

pub fn match_toml_owners(
    template: &TomlAnalysis,
    destination: &TomlAnalysis,
) -> TomlOwnerMatchResult {
    let result = match_owner_paths(
        &template.owners,
        &destination.owners,
        |owner| owner.path.as_str(),
        |owner| owner.path.as_str(),
    );
    TomlOwnerMatchResult {
        matched: result
            .matched
            .iter()
            .map(|entry| TomlOwnerMatch {
                template_path: template.owners[entry.template_index].path.clone(),
                destination_path: destination.owners[entry.destination_index].path.clone(),
            })
            .collect(),
        unmatched_template: result
            .unmatched_template
            .iter()
            .map(|index| template.owners[*index].path.clone())
            .collect(),
        unmatched_destination: result
            .unmatched_destination
            .iter()
            .map(|index| destination.owners[*index].path.clone())
            .collect(),
    }
}

pub fn merge_toml_with_parser(
    template_source: &str,
    destination_source: &str,
    dialect: TomlDialect,
    parser: impl Fn(&str, TomlDialect) -> ParseResult<TomlAnalysis>,
) -> MergeResult<String> {
    let template = parser(template_source, dialect);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parser(destination_source, dialect);
    if !destination.ok {
        return MergeResult {
            ok: false,
            diagnostics: destination
                .diagnostics
                .into_iter()
                .map(|diagnostic| Diagnostic {
                    category: if diagnostic.category == DiagnosticCategory::ParseError {
                        DiagnosticCategory::DestinationParseError
                    } else {
                        diagnostic.category
                    },
                    ..diagnostic
                })
                .collect(),
            output: None,
            policies: vec![],
        };
    }

    let template = template.analysis.expect("successful TOML parse should include analysis");
    let destination = destination.analysis.expect("successful TOML parse should include analysis");
    match merge_toml_documents(&template.document, &destination.document) {
        Ok(output) => {
            let verification = parser(&output, dialect);
            match verification.analysis {
                Some(analysis)
                    if verification.ok
                        && analysis.document.semantic
                            == template.document.expected_merge_semantic(&destination.document) =>
                {
                    MergeResult {
                        ok: true,
                        diagnostics: vec![],
                        output: Some(output),
                        policies: vec![destination_wins_array_policy()],
                    }
                }
                Some(_) => MergeResult {
                    ok: false,
                    diagnostics: vec![unsupported_feature(
                        "Source-preserving TOML render changed the planned structure.",
                    )],
                    output: None,
                    policies: vec![],
                },
                None => MergeResult {
                    ok: false,
                    diagnostics: verification.diagnostics,
                    output: None,
                    policies: vec![],
                },
            }
        }
        Err(message) => MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&message)],
            output: None,
            policies: vec![],
        },
    }
}

pub fn merge_toml(
    template_source: &str,
    destination_source: &str,
    dialect: TomlDialect,
    backend: Option<TomlBackend>,
) -> MergeResult<String> {
    let resolved = resolve_backend(backend);
    if resolved != TomlBackend::TreeSitter {
        return MergeResult {
            ok: false,
            diagnostics: vec![unsupported_feature(&format!(
                "Unsupported TOML backend {resolved:?}."
            ))],
            output: None,
            policies: vec![],
        };
    }

    merge_toml_with_parser(template_source, destination_source, dialect, |source, parse_dialect| {
        parse_toml(source, parse_dialect, Some(resolved))
    })
}
