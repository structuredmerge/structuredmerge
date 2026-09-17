//! Introspection for explicit native merge entry points, not registry negotiation.
use serde::{Deserialize, Serialize};

pub(crate) const YAML_MAPPING: &str = "kernel.yaml.native_mapping.v1";
pub(crate) const PYTHON_DECLARATIONS: &str = "kernel.python.native_declarations.v1";
pub(crate) const JSON_NESTED: &str = "kernel.json.nested.v1";

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
