use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeResult, ParseResult, PolicyReference,
    PolicySurface,
};
use toml::Value;
use tree_haver::{BackendReference, kreuzberg_language_pack_backend, parse_with_language_pack};

pub const PACKAGE_NAME: &str = "toml-merge";

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
    }
}

fn validate_scalar_array(items: &[Value], path: &str) -> Result<(), Diagnostic> {
    if items.iter().all(|item| {
        matches!(item, Value::String(_) | Value::Integer(_) | Value::Float(_) | Value::Boolean(_))
    }) {
        return Ok(());
    }

    Err(unsupported_feature(&format!(
        "Unsupported TOML array value at {}. Only scalar arrays are supported.",
        display_path(path)
    )))
}

fn display_path(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn resolve_backend(backend: Option<TomlBackend>) -> TomlBackend {
    backend.unwrap_or(TomlBackend::TreeSitter)
}

fn validate_toml_node(value: &Value, path: &str) -> Result<(), Diagnostic> {
    match value {
        Value::String(_) | Value::Integer(_) | Value::Float(_) | Value::Boolean(_) => Ok(()),
        Value::Array(items) => validate_scalar_array(items, path),
        Value::Table(table) => {
            let mut keys = table.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                let next_path = format!("{path}/{key}");
                if let Some(next_value) = table.get(&key) {
                    validate_toml_node(next_value, &next_path)?;
                }
            }
            Ok(())
        }
        _ => Err(unsupported_feature(&format!(
            "Unsupported TOML value at {}. Only tables, scalar values, and scalar arrays are supported.",
            display_path(path)
        ))),
    }
}

fn render_toml_scalar(value: &Value) -> String {
    match value {
        Value::String(text) => format!("{text:?}"),
        Value::Boolean(boolean) => boolean.to_string(),
        Value::Integer(integer) => integer.to_string(),
        Value::Float(float) => {
            let rendered = float.to_string();
            if rendered.contains('.') { rendered } else { format!("{rendered}.0") }
        }
        _ => unreachable!("render_toml_scalar only supports TOML scalars"),
    }
}

fn render_toml_value(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let rendered = items.iter().map(render_toml_scalar).collect::<Vec<_>>().join(", ");
            format!("[{rendered}]")
        }
        Value::String(_) | Value::Boolean(_) | Value::Integer(_) | Value::Float(_) => {
            render_toml_scalar(value)
        }
        _ => unreachable!("render_toml_value does not render tables"),
    }
}

fn render_toml_table(table: &toml::map::Map<String, Value>, path: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut keys = table.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    let value_keys = keys
        .iter()
        .filter(|key| !matches!(table.get(*key), Some(Value::Table(_))))
        .cloned()
        .collect::<Vec<_>>();
    let table_keys = keys
        .iter()
        .filter(|key| matches!(table.get(*key), Some(Value::Table(_))))
        .cloned()
        .collect::<Vec<_>>();

    if !path.is_empty() {
        lines.push(format!("[{}]", path.join(".")));
    }

    for key in value_keys {
        if let Some(value) = table.get(&key) {
            lines.push(format!("{key} = {}", render_toml_value(value)));
        }
    }

    for key in table_keys {
        if !lines.is_empty() {
            lines.push(String::new());
        }

        if let Some(Value::Table(nested)) = table.get(&key) {
            let mut nested_path = path.to_vec();
            nested_path.push(key);
            lines.extend(render_toml_table(nested, &nested_path));
        }
    }

    lines
}

fn canonical_toml(table: &toml::map::Map<String, Value>) -> String {
    format!("{}\n", render_toml_table(table, &[]).join("\n"))
}

fn collect_toml_owners(table: &toml::map::Map<String, Value>, prefix: &str) -> Vec<TomlOwner> {
    let mut owners = Vec::new();
    let mut keys = table.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    for key in keys {
        let path = format!("{prefix}/{key}");
        match table.get(&key) {
            Some(Value::Array(items)) => {
                owners.push(TomlOwner {
                    path: path.clone(),
                    owner_kind: TomlOwnerKind::KeyValue,
                    match_key: Some(key.clone()),
                });
                owners.extend(items.iter().enumerate().map(|(index, _)| TomlOwner {
                    path: format!("{path}/{index}"),
                    owner_kind: TomlOwnerKind::ArrayItem,
                    match_key: None,
                }));
            }
            Some(Value::Table(nested)) => {
                owners.push(TomlOwner {
                    path: path.clone(),
                    owner_kind: TomlOwnerKind::Table,
                    match_key: Some(key.clone()),
                });
                owners.extend(collect_toml_owners(nested, &path));
            }
            Some(_) => owners.push(TomlOwner {
                path,
                owner_kind: TomlOwnerKind::KeyValue,
                match_key: Some(key),
            }),
            None => {}
        }
    }

    owners
}

fn merge_toml_tables(
    template: &toml::map::Map<String, Value>,
    destination: &toml::map::Map<String, Value>,
) -> toml::map::Map<String, Value> {
    let mut merged = toml::map::Map::new();
    let mut keys = template.keys().chain(destination.keys()).cloned().collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        match (template.get(&key), destination.get(&key)) {
            (None, Some(destination_value)) => {
                merged.insert(key, destination_value.clone());
            }
            (Some(template_value), None) => {
                merged.insert(key, template_value.clone());
            }
            (Some(Value::Table(template_table)), Some(Value::Table(destination_table))) => {
                merged.insert(
                    key,
                    Value::Table(merge_toml_tables(template_table, destination_table)),
                );
            }
            (_, Some(destination_value)) => {
                merged.insert(key, destination_value.clone());
            }
            _ => {}
        }
    }

    merged
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

    match toml::from_str::<toml::map::Map<String, Value>>(source) {
        Ok(table) => {
            if let Err(diagnostic) = validate_toml_node(&Value::Table(table.clone()), "") {
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
                analysis: Some(TomlAnalysis {
                    dialect: TomlDialect::Toml,
                    normalized_source: canonical_toml(&table),
                    root_kind: TomlRootKind::Table,
                    owners: collect_toml_owners(&table, ""),
                }),
                policies: vec![],
            }
        }
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![parse_error(&error.to_string())],
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

    let syntax = parse_with_language_pack(&tree_haver::ParserRequest {
        source: source.to_string(),
        language: "toml".to_string(),
        dialect: Some("toml".to_string()),
    });
    if !syntax.ok {
        return ParseResult {
            ok: false,
            diagnostics: syntax.diagnostics,
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

    TomlOwnerMatchResult {
        matched: template
            .owners
            .iter()
            .filter(|owner| destination_owners.contains(&owner.path))
            .map(|owner| TomlOwnerMatch {
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

    let template_value = match template.analysis.as_ref().and_then(|analysis| {
        toml::from_str::<toml::map::Map<String, Value>>(&analysis.normalized_source).ok()
    }) {
        Some(table) => table,
        _ => {
            return MergeResult {
                ok: false,
                diagnostics: vec![parse_error("TOML merge requires a table-root template.")],
                output: None,
                policies: vec![],
            };
        }
    };
    let destination_value = match destination.analysis.as_ref().and_then(|analysis| {
        toml::from_str::<toml::map::Map<String, Value>>(&analysis.normalized_source).ok()
    }) {
        Some(table) => table,
        _ => {
            return MergeResult {
                ok: false,
                diagnostics: vec![destination_parse_error(
                    "TOML merge requires a table-root destination.",
                )],
                output: None,
                policies: vec![],
            };
        }
    };

    let merged = merge_toml_tables(&template_value, &destination_value);
    MergeResult {
        ok: true,
        diagnostics: vec![],
        output: Some(canonical_toml(&merged)),
        policies: vec![destination_wins_array_policy()],
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
