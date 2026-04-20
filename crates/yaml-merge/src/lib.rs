use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeResult, ParseResult, PolicyReference,
    PolicySurface,
};
use serde_json::{Map, Value};
use tree_haver::{ParserRequest, current_backend_id, parse_with_language_pack};

pub const PACKAGE_NAME: &str = "yaml-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YamlDialect {
    Yaml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YamlBackend {
    SerdeYaml,
    YamlSerde,
    KreuzbergLanguagePack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YamlRootKind {
    Mapping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YamlOwnerKind {
    Mapping,
    KeyValue,
    SequenceItem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlOwner {
    pub path: String,
    pub owner_kind: YamlOwnerKind,
    pub match_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlOwnerMatch {
    pub template_path: String,
    pub destination_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlOwnerMatchResult {
    pub matched: Vec<YamlOwnerMatch>,
    pub unmatched_template: Vec<String>,
    pub unmatched_destination: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlAnalysis {
    pub dialect: YamlDialect,
    pub normalized_source: String,
    pub root_kind: YamlRootKind,
    pub owners: Vec<YamlOwner>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<YamlDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YamlBackendFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<YamlDialect>,
    pub supported_policies: Vec<PolicyReference>,
    pub backend: String,
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

fn destination_parse_error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category: DiagnosticCategory::DestinationParseError,
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

pub fn yaml_feature_profile() -> YamlFeatureProfile {
    let shared = FamilyFeatureProfile {
        family: "yaml".to_string(),
        supported_dialects: vec!["yaml".to_string()],
        supported_policies: vec![destination_wins_array_policy()],
    };

    YamlFeatureProfile {
        family: "yaml",
        supported_dialects: shared.supported_dialects.iter().map(|_| YamlDialect::Yaml).collect(),
        supported_policies: shared.supported_policies,
    }
}

pub fn available_yaml_backends() -> Vec<YamlBackend> {
    vec![YamlBackend::SerdeYaml, YamlBackend::YamlSerde, YamlBackend::KreuzbergLanguagePack]
}

pub fn yaml_backend_feature_profile(backend: YamlBackend) -> YamlBackendFeatureProfile {
    YamlBackendFeatureProfile {
        family: "yaml",
        supported_dialects: vec![YamlDialect::Yaml],
        supported_policies: vec![destination_wins_array_policy()],
        backend: match backend {
            YamlBackend::SerdeYaml => "serde_yaml".to_string(),
            YamlBackend::YamlSerde => "yaml_serde".to_string(),
            YamlBackend::KreuzbergLanguagePack => "kreuzberg-language-pack".to_string(),
        },
    }
}

pub fn yaml_plan_context() -> ConformanceFamilyPlanContext {
    yaml_plan_context_with_backend(YamlBackend::SerdeYaml)
}

pub fn yaml_plan_context_with_backend(backend: YamlBackend) -> ConformanceFamilyPlanContext {
    let backend_profile = yaml_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: backend_profile.family.to_string(),
            supported_dialects: vec!["yaml".to_string()],
            supported_policies: backend_profile.supported_policies.clone(),
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: backend_profile.backend,
            supports_dialects: backend != YamlBackend::KreuzbergLanguagePack,
            supported_policies: backend_profile.supported_policies,
        }),
    }
}

fn resolve_backend(backend: Option<YamlBackend>) -> YamlBackend {
    if let Some(backend) = backend {
        return backend;
    }

    match current_backend_id().as_deref() {
        Some("kreuzberg-language-pack") => YamlBackend::KreuzbergLanguagePack,
        _ => YamlBackend::SerdeYaml,
    }
}

fn display_path(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn render_yaml_scalar(value: &Value) -> String {
    match value {
        Value::String(text) => {
            if text
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '.' | '-'))
            {
                text.clone()
            } else {
                format!("{text:?}")
            }
        }
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        _ => unreachable!("render_yaml_scalar only supports YAML scalars"),
    }
}

fn validate_yaml_node(value: &Value, path: &str) -> Result<(), Diagnostic> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => Ok(()),
        Value::Array(items) => {
            if items
                .iter()
                .all(|item| matches!(item, Value::String(_) | Value::Bool(_) | Value::Number(_)))
            {
                Ok(())
            } else {
                Err(unsupported_feature(&format!(
                    "Unsupported YAML sequence value at {}. Only scalar sequences are supported.",
                    display_path(path)
                )))
            }
        }
        Value::Object(mapping) => {
            let mut keys = mapping.keys().cloned().collect::<Vec<_>>();
            keys.sort();

            for key in keys {
                let next_path = format!("{path}/{key}");
                if let Some(value) = mapping.get(&key) {
                    validate_yaml_node(value, &next_path)?;
                }
            }
            Ok(())
        }
        _ => Err(unsupported_feature(&format!(
            "Unsupported YAML value at {}. Only mappings, scalar values, and scalar sequences are supported.",
            display_path(path)
        ))),
    }
}

fn render_yaml_node(key: &str, value: &Value, indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);

    match value {
        Value::Array(items) => {
            let mut lines = vec![format!("{prefix}{key}:")];
            lines.extend(
                items.iter().map(|item| {
                    format!("{}- {}", " ".repeat(indent + 2), render_yaml_scalar(item))
                }),
            );
            lines
        }
        Value::Object(mapping) => {
            let mut lines = vec![format!("{prefix}{key}:")];
            lines.extend(render_yaml_mapping(mapping, indent + 2));
            lines
        }
        _ => vec![format!("{prefix}{key}: {}", render_yaml_scalar(value))],
    }
}

fn render_yaml_mapping(mapping: &Map<String, Value>, indent: usize) -> Vec<String> {
    let mut keys = mapping.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    let mut lines = Vec::new();
    for key in keys {
        if let Some(value) = mapping.get(&key) {
            lines.extend(render_yaml_node(&key, value, indent));
        }
    }

    lines
}

fn canonical_yaml(mapping: &Map<String, Value>) -> String {
    format!("{}\n", render_yaml_mapping(mapping, 0).join("\n"))
}

fn collect_yaml_owners(mapping: &Map<String, Value>, prefix: &str) -> Vec<YamlOwner> {
    let mut keys = mapping.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    let mut owners = Vec::new();
    for key in keys {
        let path = format!("{prefix}/{key}");
        match mapping.get(&key) {
            Some(Value::Array(items)) => {
                owners.push(YamlOwner {
                    path: path.clone(),
                    owner_kind: YamlOwnerKind::KeyValue,
                    match_key: Some(key.clone()),
                });
                for index in 0..items.len() {
                    owners.push(YamlOwner {
                        path: format!("{path}/{index}"),
                        owner_kind: YamlOwnerKind::SequenceItem,
                        match_key: None,
                    });
                }
            }
            Some(Value::Object(nested)) => {
                owners.push(YamlOwner {
                    path: path.clone(),
                    owner_kind: YamlOwnerKind::Mapping,
                    match_key: Some(key.clone()),
                });
                owners.extend(collect_yaml_owners(nested, &path));
            }
            Some(_) => {
                owners.push(YamlOwner {
                    path,
                    owner_kind: YamlOwnerKind::KeyValue,
                    match_key: Some(key),
                });
            }
            None => {}
        }
    }

    owners
}

fn parse_yaml_value(source: &str, backend: YamlBackend) -> Result<Value, Diagnostic> {
    match backend {
        YamlBackend::SerdeYaml => {
            serde_yaml::from_str::<Value>(source).map_err(|error| parse_error(&error.to_string()))
        }
        YamlBackend::YamlSerde => {
            yaml_serde::from_str::<Value>(source).map_err(|error| parse_error(&error.to_string()))
        }
        YamlBackend::KreuzbergLanguagePack => {
            serde_yaml::from_str::<Value>(source).map_err(|error| parse_error(&error.to_string()))
        }
    }
}

pub fn parse_yaml(source: &str, dialect: YamlDialect) -> ParseResult<YamlAnalysis> {
    parse_yaml_with_backend(source, dialect, resolve_backend(None))
}

pub fn parse_yaml_with_backend(
    source: &str,
    dialect: YamlDialect,
    backend: YamlBackend,
) -> ParseResult<YamlAnalysis> {
    if dialect != YamlDialect::Yaml {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature("Unsupported YAML dialect.")],
            analysis: None,
            policies: vec![],
        };
    }

    if backend == YamlBackend::KreuzbergLanguagePack {
        let backend_result = parse_with_language_pack(&ParserRequest {
            source: source.to_string(),
            language: "yaml".to_string(),
            dialect: Some("yaml".to_string()),
        });
        if !backend_result.ok {
            return ParseResult {
                ok: false,
                diagnostics: backend_result.diagnostics,
                analysis: None,
                policies: vec![],
            };
        }
    }

    match parse_yaml_value(source, backend) {
        Ok(Value::Object(mapping)) => {
            if let Err(diagnostic) = validate_yaml_node(&Value::Object(mapping.clone()), "") {
                return ParseResult {
                    ok: false,
                    diagnostics: vec![diagnostic],
                    analysis: None,
                    policies: vec![],
                };
            }

            ParseResult {
                ok: true,
                diagnostics: vec![],
                analysis: Some(YamlAnalysis {
                    dialect: YamlDialect::Yaml,
                    normalized_source: canonical_yaml(&mapping),
                    root_kind: YamlRootKind::Mapping,
                    owners: collect_yaml_owners(&mapping, ""),
                }),
                policies: vec![],
            }
        }
        Ok(_) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error("YAML documents must parse to a mapping root.")],
            analysis: None,
            policies: vec![],
        },
        Err(diagnostic) => ParseResult {
            ok: false,
            diagnostics: vec![diagnostic],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn match_yaml_owners(
    template: &YamlAnalysis,
    destination: &YamlAnalysis,
) -> YamlOwnerMatchResult {
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

    YamlOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| YamlOwnerMatch {
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

fn merge_yaml_mappings(
    template: &Map<String, Value>,
    destination: &Map<String, Value>,
) -> Map<String, Value> {
    let mut merged = Map::new();
    let mut keys = template.keys().chain(destination.keys()).cloned().collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let template_value = template.get(&key);
        let destination_value = destination.get(&key);

        match (template_value, destination_value) {
            (None, Some(destination)) => {
                merged.insert(key, destination.clone());
            }
            (Some(template), None) => {
                merged.insert(key, template.clone());
            }
            (Some(Value::Object(template_mapping)), Some(Value::Object(destination_mapping))) => {
                merged.insert(
                    key,
                    Value::Object(merge_yaml_mappings(template_mapping, destination_mapping)),
                );
            }
            (_, Some(destination)) => {
                merged.insert(key, destination.clone());
            }
            _ => {}
        }
    }

    merged
}

pub fn merge_yaml(
    template_source: &str,
    destination_source: &str,
    dialect: YamlDialect,
) -> MergeResult<String> {
    merge_yaml_with_backend(template_source, destination_source, dialect, YamlBackend::SerdeYaml)
}

pub fn merge_yaml_with_backend(
    template_source: &str,
    destination_source: &str,
    dialect: YamlDialect,
    backend: YamlBackend,
) -> MergeResult<String> {
    let template = parse_yaml_with_backend(template_source, dialect, backend);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_yaml_with_backend(destination_source, dialect, backend);
    if !destination.ok {
        return MergeResult {
            ok: false,
            diagnostics: destination
                .diagnostics
                .into_iter()
                .map(|diagnostic| {
                    if diagnostic.category == DiagnosticCategory::ParseError {
                        destination_parse_error(&diagnostic.message)
                    } else {
                        diagnostic
                    }
                })
                .collect(),
            output: None,
            policies: vec![],
        };
    }

    let template_mapping = parse_yaml_value(
        &template.analysis.as_ref().expect("template analysis").normalized_source,
        backend,
    );
    let destination_mapping = parse_yaml_value(
        &destination.analysis.as_ref().expect("destination analysis").normalized_source,
        backend,
    );

    match (template_mapping, destination_mapping) {
        (Ok(Value::Object(template_mapping)), Ok(Value::Object(destination_mapping))) => {
            let merged = merge_yaml_mappings(&template_mapping, &destination_mapping);
            MergeResult {
                ok: true,
                diagnostics: vec![],
                output: Some(canonical_yaml(&merged)),
                policies: vec![destination_wins_array_policy()],
            }
        }
        (_, Err(error)) => MergeResult {
            ok: false,
            diagnostics: vec![destination_parse_error(&error.message)],
            output: None,
            policies: vec![],
        },
        (Err(error), _) => MergeResult {
            ok: false,
            diagnostics: vec![parse_error(&error.message)],
            output: None,
            policies: vec![],
        },
        _ => MergeResult {
            ok: false,
            diagnostics: vec![parse_error("YAML documents must parse to a mapping root.")],
            output: None,
            policies: vec![],
        },
    }
}
