//! Static operation profile and native entry-point scope, not registry negotiation.
use crate::OperationKind;
use serde::{Deserialize, Serialize};

pub(crate) const YAML_MAPPING: &str = "kernel.yaml.native_mapping.v1";
pub(crate) const PYTHON_DECLARATIONS: &str = "kernel.python.native_declarations.v1";
pub(crate) const JSON_NESTED: &str = "kernel.json.nested.v1";
pub(crate) const GIT_JSON: &str = "kernel.git.json.v1";
pub(crate) const BASH_OWNERS: &str = "kernel.bash.owners.v1";
pub(crate) const GO_OWNERS: &str = "kernel.go.owners.v1";
pub(crate) const RUST_OWNERS: &str = "kernel.rust.owners.v1";
pub(crate) const TYPESCRIPT_OWNERS: &str = "kernel.typescript.owners.v1";

const ALL_OPERATIONS: &[OperationKind] =
    &[OperationKind::Analyze, OperationKind::Diff2, OperationKind::Merge2, OperationKind::Merge3];
const YAML_OPERATIONS: &[OperationKind] =
    &[OperationKind::Analyze, OperationKind::Diff2, OperationKind::Merge3];
const GIT_OPERATIONS: &[OperationKind] = &[OperationKind::Merge3];

pub(crate) fn operation_parse_options(id: &str, operation: OperationKind) -> crate::ParseOptions {
    if matches!(id, JSON_NESTED | GIT_JSON) {
        if operation == OperationKind::Analyze {
            crate::ParseOptions {
                comments: true,
                tokens: false,
                diagnostics: true,
                native_extensions: true,
            }
        } else {
            crate::ParseOptions::default()
        }
    } else {
        // Native owner profiles retain layout as bytes; only the extension
        // channel is required, not optional comment/token/diagnostic channels.
        crate::ParseOptions {
            comments: false,
            tokens: false,
            diagnostics: false,
            native_extensions: true,
        }
    }
}

pub(crate) fn profile_operations(id: &str) -> &'static [OperationKind] {
    match id {
        YAML_MAPPING => YAML_OPERATIONS,
        GIT_JSON => GIT_OPERATIONS,
        JSON_NESTED | PYTHON_DECLARATIONS | BASH_OWNERS | GO_OWNERS | RUST_OWNERS
        | TYPESCRIPT_OWNERS => ALL_OPERATIONS,
        _ => &[],
    }
}

/// Static implemented profile scope, not a successful parser/merge observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationProfileDeclaration {
    pub id: String,
    pub provider_id: String,
    pub family: String,
    /// Explicit dialect selectors accepted in addition to an absent selector.
    pub explicit_dialects: Vec<String>,
    pub operations: Vec<OperationKind>,
    pub semantic_runtime: String,
    pub parser_contract: String,
    pub required_native_extension: Option<String>,
    /// None means not probed; listing a declaration never loads a parser.
    pub parser_available: Option<bool>,
    pub syntax_scope: Vec<String>,
    pub limitations: Vec<String>,
    pub experimental: bool,
    pub approved_as_default: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationProfileCatalog {
    pub schema: String,
    pub profiles: Vec<OperationProfileDeclaration>,
}

/// Declarations for execute_operation, distinct from the two existing explicit
/// native entry points. All requests still undergo syntax/policy/parse checks.
pub fn operation_profile_catalog() -> OperationProfileCatalog {
    let native = native_merge_profiles();
    let mut profiles = vec![];
    for (id, provider, family, dialects, scope, limits) in [
        (
            BASH_OWNERS,
            "kernel.bash",
            "bash",
            vec!["bash"],
            "top-level functions, variable assignments and literal test_expect_success calls",
            "whole owners only; dynamic titles, duplicate identities and unsupported top-level constructs fail closed",
        ),
        (
            GIT_JSON,
            "kernel.git.json",
            "json",
            vec!["json", "jsonc", "json5"],
            "JSON-family three-way merges with Git review framing",
            "merge3 only; conflict review output is not a partially resolved merge",
        ),
        (
            GO_OWNERS,
            "kernel.go",
            "go",
            vec!["go"],
            "top-level function declarations as whole owners",
            "no import reconciliation or nested-body merge; membership plus existing-owner edits conflict",
        ),
        (
            JSON_NESTED,
            "kernel.json",
            "json",
            vec!["json", "jsonc", "json5"],
            "nested object members; arrays and scalar values as whole owners",
            "duplicate identities and unsupported syntax fail closed; exact source/layout constraints remain operation-specific",
        ),
        (PYTHON_DECLARATIONS, "kernel.python", "python", vec![], "", ""),
        (
            RUST_OWNERS,
            "kernel.rust",
            "rust",
            vec!["rust"],
            "top-level const, enum, function, module, static, struct, trait, type and union declarations",
            "no impl blocks, macro semantics or nested-body merge; membership plus existing-owner edits conflict",
        ),
        (
            TYPESCRIPT_OWNERS,
            "kernel.typescript",
            "typescript",
            vec!["typescript", "tsx"],
            "named top-level declarations and supported single-owner wrappers; TSX uses its own grammar",
            "no nested-body merge or compiler semantics; duplicate, multi-owner and unsupported nested wrappers fail closed",
        ),
        (YAML_MAPPING, "kernel.yaml", "yaml", vec![], "", ""),
    ] {
        let native_profile = native.iter().find(|profile| profile.id == id);
        let mut limitations = native_profile
            .map_or_else(|| vec![limits.into()], |profile| profile.limitations.clone());
        limitations.push("declared operations still require supported policies, valid source-bound parser facts and operation-specific syntax".into());
        if profile_operations(id).contains(&OperationKind::Merge2) {
            limitations.push("merge2 is current-preferred template-into-current; unsupported ownership/layout plans fail closed".into());
        }
        profiles.push(OperationProfileDeclaration {
            id: id.into(),
            provider_id: provider.into(),
            family: family.into(),
            explicit_dialects: dialects.into_iter().map(str::to_string).collect(),
            operations: profile_operations(id).to_vec(),
            semantic_runtime: "rust".into(),
            parser_contract: crate::service::PARSE_RESULT_SCHEMA.into(),
            required_native_extension: native_profile
                .map(|profile| profile.native_extension.clone()),
            parser_available: None,
            syntax_scope: native_profile
                .map_or_else(|| vec![scope.into()], |profile| profile.syntax_scope.clone()),
            limitations,
            experimental: true,
            approved_as_default: false,
        });
    }
    profiles.sort_by(|left, right| left.id.cmp(&right.id));
    OperationProfileCatalog {
        schema: "structuredmerge.operation-profile-catalog/v1".into(),
        profiles,
    }
}

pub(crate) fn native_language<'a>(family: &'a str, dialect: Option<&str>) -> Option<&'a str> {
    match (family, dialect) {
        ("typescript", None | Some("typescript")) => Some("typescript"),
        ("typescript", Some("tsx")) => Some("tsx"),
        (_, None) => Some(family),
        ("bash" | "go" | "rust", Some(dialect)) if dialect == family => Some(family),
        _ => None,
    }
}

pub(crate) fn profile_parser_language<'a>(
    family: &'a str,
    dialect: Option<&str>,
) -> Option<&'a str> {
    if family == "json" {
        match dialect {
            None | Some("json") => Some("json"),
            Some("jsonc" | "json5") => Some("json5"),
            _ => None,
        }
    } else {
        native_language(family, dialect)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeMergeProfile {
    pub id: String,
    pub family: String,
    pub operation: String,
    pub entry_point: String,
    pub analysis_crate: String,
    pub merge_crate: String,
    pub semantic_runtime: String,
    pub parse_contract: String,
    pub native_extension: String,
    pub syntax_scope: Vec<String>,
    pub limitations: Vec<String>,
    pub experimental: bool,
    pub approved_as_default: bool,
}

/// Does not load or probe parsers. A declared profile may be unavailable in a
/// host until a compatible parser is registered and successfully negotiated.
pub fn native_merge_profiles() -> Vec<NativeMergeProfile> {
    vec![
        NativeMergeProfile {
            id: PYTHON_DECLARATIONS.into(),
            family: "python".into(),
            operation: "merge3".into(),
            entry_point: "merge_python_declarations".into(),
            analysis_crate: "python-merge".into(),
            merge_crate: "ast-merge".into(),
            semantic_runtime: "rust".into(),
            parse_contract: crate::service::PARSE_RESULT_SCHEMA.into(),
            native_extension: "structuredmerge.extension/python-libcst/v1".into(),
            syntax_scope: vec![
                "single-name top-level assignments".into(),
                "undecorated top-level functions and classes as whole owners".into(),
            ],
            limitations: vec![
                "no imports, decorated declarations, chained or destructuring assignments".into(),
                "no nested-body merge or full Python semantic equivalence claim".into(),
                "changed unowned layout fails closed".into(),
            ],
            experimental: true,
            approved_as_default: false,
        },
        NativeMergeProfile {
            id: YAML_MAPPING.into(),
            family: "yaml".into(),
            operation: "merge3".into(),
            entry_point: "merge_yaml_mapping".into(),
            analysis_crate: "yaml-merge".into(),
            merge_crate: "ast-merge".into(),
            semantic_runtime: "rust".into(),
            parse_contract: crate::service::PARSE_RESULT_SCHEMA.into(),
            native_extension: yaml_merge::typed::PSYCH_EXTENSION.into(),
            syntax_scope: vec![
                "top-level block mapping with unique conservative string keys".into(),
                "nested values as whole owners".into(),
            ],
            limitations: vec![
                "no aliases, anchors, tags, complex or ambiguous keys".into(),
                "no nested-value merge".into(),
                "changed unowned layout fails closed".into(),
            ],
            experimental: true,
            approved_as_default: false,
        },
    ]
}
