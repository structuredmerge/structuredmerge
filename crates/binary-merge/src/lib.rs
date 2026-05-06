use ast_merge::PolicyReference;
use tree_haver::{
    BinaryDiagnostic, BinaryMergeReport, BinaryNestedDispatch, BinaryRenderPolicy, ByteRange,
};

pub const PACKAGE_NAME: &str = "binary-merge";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<String>,
    pub supported_policies: Vec<PolicyReference>,
}

pub fn binary_feature_profile() -> BinaryFeatureProfile {
    BinaryFeatureProfile {
        family: "binary",
        supported_dialects: vec![],
        supported_policies: vec![],
    }
}

pub fn render_policy(
    schema_path: &str,
    byte_range: ByteRange,
    operation: &str,
    disposition: &str,
    reason: &str,
) -> BinaryRenderPolicy {
    BinaryRenderPolicy {
        schema_path: schema_path.to_string(),
        byte_range: Some(byte_range),
        operation: operation.to_string(),
        disposition: disposition.to_string(),
        reason: reason.to_string(),
    }
}

pub fn unsafe_diagnostic(
    schema_path: &str,
    byte_range: ByteRange,
    message: &str,
) -> BinaryDiagnostic {
    BinaryDiagnostic {
        severity: "error".to_string(),
        category: "unsafe_binary_mutation".to_string(),
        message: message.to_string(),
        schema_path: schema_path.to_string(),
        byte_range: Some(byte_range),
    }
}

pub fn preservation_report(
    format: &str,
    schema: &str,
    matched_schema_paths: Vec<String>,
    preserved_ranges: Vec<ByteRange>,
) -> BinaryMergeReport {
    BinaryMergeReport {
        format: format.to_string(),
        schema: schema.to_string(),
        matched_schema_paths,
        preserved_ranges,
        rewritten_nodes: vec![],
        checksum_updates: vec![],
        nested_dispatches: Vec::<BinaryNestedDispatch>::new(),
        diagnostics: vec![],
    }
}
