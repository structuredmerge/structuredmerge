use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeResult, ParseResult, PolicyReference,
    PolicySurface,
};
use serde_yaml::{Mapping, Number, Value};

pub const PACKAGE_NAME: &str = "yaml-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YamlDialect {
    Yaml,
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

pub fn yaml_plan_context() -> ConformanceFamilyPlanContext {
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: yaml_feature_profile().family.to_string(),
            supported_dialects: vec!["yaml".to_string()],
            supported_policies: yaml_feature_profile().supported_policies,
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: "serde_yaml".to_string(),
            supports_dialects: true,
            supported_policies: vec![destination_wins_array_policy()],
        }),
    }
}

fn display_path(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn number_to_string(number: &Number) -> String {
    serde_yaml::to_string(number).unwrap_or_default().trim().to_string()
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
        Value::Number(number) => number_to_string(number),
        _ => unreachable!("render_yaml_scalar only supports YAML scalars"),
    }
}

fn validate_yaml_node(value: &Value, path: &str) -> Result<(), Diagnostic> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => Ok(()),
        Value::Sequence(items) => {
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
        Value::Mapping(mapping) => {
            let mut keys = mapping
                .keys()
                .map(|key| match key {
                    Value::String(text) => Ok(text.clone()),
                    _ => Err(unsupported_feature(&format!(
                        "Unsupported YAML mapping key at {}. Only string keys are supported.",
                        display_path(path)
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()?;
            keys.sort();

            for key in keys {
                let next_path = format!("{path}/{key}");
                if let Some(value) = mapping.get(Value::String(key)) {
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
        Value::Sequence(items) => {
            let mut lines = vec![format!("{prefix}{key}:")];
            lines.extend(
                items.iter().map(|item| {
                    format!("{}- {}", " ".repeat(indent + 2), render_yaml_scalar(item))
                }),
            );
            lines
        }
        Value::Mapping(mapping) => {
            let mut lines = vec![format!("{prefix}{key}:")];
            lines.extend(render_yaml_mapping(mapping, indent + 2));
            lines
        }
        _ => vec![format!("{prefix}{key}: {}", render_yaml_scalar(value))],
    }
}

fn render_yaml_mapping(mapping: &Mapping, indent: usize) -> Vec<String> {
    let mut keys = mapping
        .keys()
        .filter_map(|key| match key {
            Value::String(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    keys.sort();

    let mut lines = Vec::new();
    for key in keys {
        if let Some(value) = mapping.get(Value::String(key.clone())) {
            lines.extend(render_yaml_node(&key, value, indent));
        }
    }

    lines
}

fn canonical_yaml(mapping: &Mapping) -> String {
    format!("{}\n", render_yaml_mapping(mapping, 0).join("\n"))
}

fn collect_yaml_owners(mapping: &Mapping, prefix: &str) -> Vec<YamlOwner> {
    let mut keys = mapping
        .keys()
        .filter_map(|key| match key {
            Value::String(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    keys.sort();

    let mut owners = Vec::new();
    for key in keys {
        let path = format!("{prefix}/{key}");
        match mapping.get(Value::String(key.clone())) {
            Some(Value::Sequence(items)) => {
                owners.push(YamlOwner {
                    path: path.clone(),
                    owner_kind: YamlOwnerKind::KeyValue,
                    match_key: Some(key.clone()),
                });
                owners.extend(items.iter().enumerate().map(|(index, _)| YamlOwner {
                    path: format!("{path}/{index}"),
                    owner_kind: YamlOwnerKind::SequenceItem,
                    match_key: None,
                }));
            }
            Some(Value::Mapping(nested)) => {
                owners.push(YamlOwner {
                    path: path.clone(),
                    owner_kind: YamlOwnerKind::Mapping,
                    match_key: Some(key.clone()),
                });
                owners.extend(collect_yaml_owners(nested, &path));
            }
            Some(_) => owners.push(YamlOwner {
                path,
                owner_kind: YamlOwnerKind::KeyValue,
                match_key: Some(key),
            }),
            None => {}
        }
    }

    owners
}

fn merge_yaml_mappings(template: &Mapping, destination: &Mapping) -> Mapping {
    let mut merged = Mapping::new();
    let mut keys = template
        .keys()
        .chain(destination.keys())
        .filter_map(|key| match key {
            Value::String(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        match (
            template.get(Value::String(key.clone())),
            destination.get(Value::String(key.clone())),
        ) {
            (None, Some(destination_value)) => {
                merged.insert(Value::String(key), destination_value.clone());
            }
            (Some(template_value), None) => {
                merged.insert(Value::String(key), template_value.clone());
            }
            (Some(Value::Mapping(template_mapping)), Some(Value::Mapping(destination_mapping))) => {
                merged.insert(
                    Value::String(key),
                    Value::Mapping(merge_yaml_mappings(template_mapping, destination_mapping)),
                );
            }
            (_, Some(destination_value)) => {
                merged.insert(Value::String(key), destination_value.clone());
            }
            _ => {}
        }
    }

    merged
}

fn analyze_yaml_document(source: &str) -> ParseResult<YamlAnalysis> {
    match serde_yaml::from_str::<Value>(source) {
        Ok(Value::Mapping(mapping)) => {
            if let Err(diagnostic) = validate_yaml_node(&Value::Mapping(mapping.clone()), "") {
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
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&error.to_string())],
            analysis: None,
            policies: vec![],
        },
    }
}

pub fn parse_yaml(source: &str, dialect: YamlDialect) -> ParseResult<YamlAnalysis> {
    if dialect != YamlDialect::Yaml {
        return ParseResult {
            ok: false,
            diagnostics: vec![unsupported_feature("Unsupported YAML dialect.")],
            analysis: None,
            policies: vec![],
        };
    }

    analyze_yaml_document(source)
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

pub fn merge_yaml(
    template_source: &str,
    destination_source: &str,
    dialect: YamlDialect,
) -> MergeResult<String> {
    let template = parse_yaml(template_source, dialect);
    if !template.ok {
        return MergeResult {
            ok: false,
            diagnostics: template.diagnostics,
            output: None,
            policies: vec![],
        };
    }

    let destination = parse_yaml(destination_source, dialect);
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

    let template_mapping = match template
        .analysis
        .as_ref()
        .and_then(|analysis| serde_yaml::from_str::<Value>(&analysis.normalized_source).ok())
    {
        Some(Value::Mapping(mapping)) => mapping,
        _ => {
            return MergeResult {
                ok: false,
                diagnostics: vec![parse_error("YAML merge requires a mapping-root template.")],
                output: None,
                policies: vec![],
            };
        }
    };

    let destination_mapping = match destination
        .analysis
        .as_ref()
        .and_then(|analysis| serde_yaml::from_str::<Value>(&analysis.normalized_source).ok())
    {
        Some(Value::Mapping(mapping)) => mapping,
        _ => {
            return MergeResult {
                ok: false,
                diagnostics: vec![destination_parse_error(
                    "YAML merge requires a mapping-root destination.",
                )],
                output: None,
                policies: vec![],
            };
        }
    };

    let merged = merge_yaml_mappings(&template_mapping, &destination_mapping);
    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(canonical_yaml(&merged)),
        policies: vec![destination_wins_array_policy()],
    }
}
