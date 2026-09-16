//! Rust-owned block-mapping analysis over TreeHaver facts. This initial profile
//! is not a claim of full YAML, comment-ownership, or operation-envelope parity.
use std::collections::BTreeSet;

use ast_merge::{SourcePreservingOwner, SourcePreservingOwnerDocument, ThreeWayMergeResult};
use tree_haver::{
    ByteRange,
    parsed::ParseNode,
    service::{ExecutionContext, ParseRequest, ParseService, ParsedResult, ParserRegistrySnapshot},
};

pub const PSYCH_EXTENSION: &str = "structuredmerge.extension/ruby-psych/v1";

pub use ast_merge::typed_merge::NativeMergeError as MappingMergeError;

/// Compare two explicit before/after revisions using the same native syntax
/// ownership profile as merge3. TreeHaver selects/parses; Rust classifies diff.
pub fn diff_mapping_sources(
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
) -> Result<ast_merge::typed_diff::NativeDiffExecution, MappingMergeError> {
    ast_merge::typed_diff::diff_native_sources_with_evidence(
        "yaml",
        requests,
        service,
        snapshot,
        context,
        mapping_owners,
    )
}

/// Analysis is derived here, never accepted as a parser-supplied owner list.
/// Entire top-level entries are owners; nested entries are not independently
/// merged in this first profile. Changed unowned layout fails closed.
pub fn mapping_owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    let document = &parsed.document;
    if parsed.source.descriptor() != &document.output().source {
        return Err("parsed tree and source identity differ".into());
    }
    if !document.output().ok {
        return Err("native parser rejected source".into());
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|_| "YAML requires UTF-8")?;
    let mut root = document
        .node(document.output().root_id.as_deref().ok_or("missing root")?)
        .ok_or("missing root")?;
    for kind in ["stream", "document"] {
        if root.kind != kind || root.children.len() != 1 {
            return Err("expected one YAML stream document".into());
        }
        root = document.node(&root.children[0].node_id).ok_or("missing document child")?;
    }
    if root.kind != "mapping"
        || native(root)?.get("style").and_then(|value| value.as_u64()) != Some(1)
    {
        return Err("profile requires a block mapping".into());
    }
    for node in &document.output().nodes {
        let facts = native(node)?;
        if node.kind == "alias"
            || facts.get("anchor").is_some_and(|value| !value.is_null())
            || facts.get("tag").is_some_and(|value| !value.is_null())
            || !node.unsupported_features.is_empty()
        {
            return Err("aliases, anchors, explicit tags, or unsupported native syntax require another profile".into());
        }
    }
    if root.children.is_empty() || root.children.len() % 2 != 0 {
        return Err("expected mapping key/value pairs".into());
    }
    let mut owners = Vec::new();
    let mut names = BTreeSet::new();
    for pair in root.children.chunks_exact(2) {
        if pair[0].field_name.as_deref() != Some("key")
            || pair[1].field_name.as_deref() != Some("value")
        {
            return Err("mapping edges must identify native keys and values".into());
        }
        let key = document.node(&pair[0].node_id).ok_or("missing key")?;
        let value = document.node(&pair[1].node_id).ok_or("missing value")?;
        if key.kind != "scalar" {
            return Err("complex mapping keys are not supported by this profile".into());
        }
        let key_facts = native(key)?;
        let name = key_facts
            .get("value")
            .and_then(|value| value.as_str())
            .ok_or("missing scalar value")?;
        // Plain scalar type resolution and merge keys require a richer YAML
        // profile. This profile admits only quoted string keys or conservative
        // identifier-like plain keys; classification uses native scalar facts.
        let plain = key_facts
            .get("plain")
            .and_then(|value| value.as_bool())
            .ok_or("missing scalar style")?;
        if name.is_empty() || name == "<<" || plain && !plain_string_key(name) {
            return Err("mapping key requires YAML scalar-resolution policy".into());
        }
        if !names.insert(name.to_owned()) {
            return Err("duplicate mapping key".into());
        }
        let range = ByteRange {
            start_byte: key.span.range.start_byte,
            end_byte: value.span.range.end_byte,
        };
        if key.span.range.end_byte > value.span.range.start_byte || range.is_empty() {
            return Err("invalid mapping pair span".into());
        }
        let bytes = parsed.source.slice(range.clone()).map_err(|_| "invalid owner range")?;
        let fingerprint =
            std::str::from_utf8(bytes).map_err(|_| "owner cuts a UTF-8 code point")?.to_owned();
        // JSON Pointer escaping prevents native names from aliasing owner paths.
        let path = format!("/{}", name.replace('~', "~0").replace('/', "~1"));
        owners.push(SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint,
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            start_line: key.span.start_point.row + 1,
            end_line: value.span.end_point.row + 1,
        });
    }
    Ok(SourcePreservingOwnerDocument { source: source.to_owned(), owners })
}

fn native(node: &ParseNode) -> Result<&serde_json::Value, String> {
    node.extensions
        .iter()
        .find(|extension| {
            extension.schema == PSYCH_EXTENSION && extension.namespace == "ruby-psych"
        })
        .map(|extension| &extension.payload)
        .ok_or_else(|| "missing supported native Psych facts".into())
}

fn plain_string_key(name: &str) -> bool {
    // No source scanning: this is conservative validation of Psych's decoded
    // scalar value until the full YAML scalar-resolution policy is integrated.
    let mut chars = name.chars();
    chars.next().is_some_and(|character| character.is_alphabetic() || character == '_')
        && chars
            .all(|character| character.is_alphanumeric() || character == '_' || character == '-')
        && !["null", "true", "false", "yes", "no", "on", "off", "y", "n"]
            .contains(&name.to_ascii_lowercase().as_str())
}

/// All input and verification parsing goes through the same TreeHaver snapshot.
/// This is the internal family operation, not yet the generated facade envelope.
pub fn merge_mapping_sources(
    requests: Vec<ParseRequest>,
    service: &dyn ParseService,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
) -> Result<ThreeWayMergeResult<String>, MappingMergeError> {
    ast_merge::typed_merge::merge_native_sources(
        "yaml",
        requests,
        service,
        snapshot,
        context,
        mapping_owners,
    )
}
