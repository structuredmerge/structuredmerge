//! Owned parser facts for Slice 1024. This layer does not assign merge ownership.
//! Native extensions and unknown compatible fields survive serde forwarding.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    NodeRole, SourceSpan,
    source::{SourceDescriptor, SourceDocument, SourceRole},
};

pub type Metadata = BTreeMap<String, Value>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeExtension {
    pub schema: String,
    pub namespace: String,
    pub capabilities: Vec<String>,
    pub payload: Value,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildEdge {
    pub node_id: String,
    pub index: u64,
    pub field_name: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseNode {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub native_type: String,
    pub role: NodeRole,
    pub named: bool,
    pub missing: bool,
    pub has_error: bool,
    pub span: SourceSpan,
    pub parent_id: Option<String>,
    pub children: Vec<ChildEdge>,
    pub semantic_roles: Vec<String>,
    pub unsupported_features: Vec<String>,
    pub extensions: Vec<NativeExtension>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseDiagnostic {
    pub id: String,
    pub severity: ParseSeverity,
    pub category: String,
    pub code: Option<String>,
    pub message: String,
    pub source_role: SourceRole,
    pub span: Option<SourceSpan>,
    pub node_id: Option<String>,
    pub blocking: bool,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentHint {
    Leading,
    Inline,
    Trailing,
    Preamble,
    Postlude,
    Orphan,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseComment {
    pub node_id: String,
    pub native_kind: String,
    pub attachment_hint: AttachmentHint,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

/// Parser payload. TreeHaver's service adds validated backend and selection evidence.
/// It is intentionally not a second whole-operation or host-workflow envelope.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParseOutput {
    pub request_id: String,
    pub source: SourceDescriptor,
    pub ok: bool,
    pub root_id: Option<String>,
    pub nodes: Vec<ParseNode>,
    pub comments: Vec<ParseComment>,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub extensions: Vec<NativeExtension>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseValidationLimits {
    pub max_nodes: usize,
    pub max_diagnostics: usize,
    pub partial_tree_allowed: bool,
    pub comments_supported: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseValidationError {
    IdentityMismatch,
    LimitExceeded,
    InvalidOutcome,
    InvalidNode,
    InvalidSpan,
    InvalidEdge,
    DisconnectedTree,
    InvalidComment,
    InvalidDiagnostic,
    InvalidExtension,
}

/// Owns validated records; callers cannot mutate them after validation.
#[derive(Clone, Debug)]
pub struct ParsedDocument {
    output: ParseOutput,
    index: BTreeMap<String, usize>,
}

impl ParsedDocument {
    pub fn validate(
        output: ParseOutput,
        request_id: &str,
        source: &SourceDocument,
        limits: ParseValidationLimits,
    ) -> Result<Self, ParseValidationError> {
        use ParseValidationError as E;
        if request_id.is_empty()
            || output.request_id != request_id
            || &output.source != source.descriptor()
        {
            return Err(E::IdentityMismatch);
        }
        if output.nodes.len() > limits.max_nodes
            || output.diagnostics.len() > limits.max_diagnostics
        {
            return Err(E::LimitExceeded);
        }
        let blocking = output.diagnostics.iter().any(|diagnostic| diagnostic.blocking);
        if output.ok && (blocking || output.root_id.is_none())
            || !output.ok && (!blocking || !limits.partial_tree_allowed && !output.nodes.is_empty())
            || output.root_id.is_none() != output.nodes.is_empty()
        {
            return Err(E::InvalidOutcome);
        }
        let mut index = BTreeMap::new();
        for (position, node) in output.nodes.iter().enumerate() {
            if node.id.is_empty()
                || node.kind.is_empty()
                || node.native_type.is_empty()
                || index.insert(node.id.clone(), position).is_some()
                || !sorted_unique(&node.semantic_roles)
                || !sorted_unique(&node.unsupported_features)
                || node.role == NodeRole::Virtual
                    && (!node.span.range.is_empty() || node.metadata.is_empty())
                || output.ok && (node.has_error || node.missing || node.role == NodeRole::Error)
            {
                return Err(E::InvalidNode);
            }
            validate_span(&node.span, source)?;
            validate_extensions(&node.extensions)?;
        }
        if let Some(root_id) = &output.root_id {
            let root = index.get(root_id).ok_or(E::InvalidNode)?;
            if output.nodes[*root].parent_id.is_some() {
                return Err(E::InvalidEdge);
            }
            let mut incoming = BTreeSet::new();
            for node in &output.nodes {
                for (edge_index, edge) in node.children.iter().enumerate() {
                    let child = &output.nodes[*index.get(&edge.node_id).ok_or(E::InvalidEdge)?];
                    if edge.index != edge_index as u64
                        || !incoming.insert(edge.node_id.as_str())
                        || child.parent_id.as_deref() != Some(node.id.as_str())
                        || !node.span.range.contains_range(&child.span.range)
                        || edge.field_name.as_ref().is_some_and(String::is_empty)
                    {
                        return Err(E::InvalidEdge);
                    }
                }
            }
            let mut seen = BTreeSet::new();
            let mut pending = vec![root_id.as_str()];
            while let Some(id) = pending.pop() {
                if !seen.insert(id) {
                    return Err(E::InvalidEdge);
                }
                let node = &output.nodes[index[id]];
                pending.extend(node.children.iter().map(|edge| edge.node_id.as_str()));
            }
            if seen.len() != output.nodes.len() {
                return Err(E::DisconnectedTree);
            }
        }
        let comment_nodes: BTreeSet<_> = output
            .nodes
            .iter()
            .filter(|node| node.role == NodeRole::Comment)
            .map(|node| node.id.as_str())
            .collect();
        let mut comments = BTreeSet::new();
        for comment in &output.comments {
            if !limits.comments_supported
                || !comment_nodes.contains(comment.node_id.as_str())
                || !comments.insert(comment.node_id.as_str())
            {
                return Err(E::InvalidComment);
            }
        }
        if comments != comment_nodes {
            return Err(E::InvalidComment);
        }
        let mut diagnostic_ids = BTreeSet::new();
        for diagnostic in &output.diagnostics {
            if diagnostic.id.is_empty()
                || diagnostic.category.is_empty()
                || diagnostic.source_role != output.source.role
                || !diagnostic_ids.insert(&diagnostic.id)
                || diagnostic.node_id.as_ref().is_some_and(|id| !index.contains_key(id))
                || diagnostic.blocking && diagnostic.severity != ParseSeverity::Error
            {
                return Err(E::InvalidDiagnostic);
            }
            if let Some(span) = &diagnostic.span {
                validate_span(span, source)?;
            }
        }
        validate_extensions(&output.extensions)?;
        Ok(Self { output, index })
    }

    pub fn output(&self) -> &ParseOutput {
        &self.output
    }

    pub fn node(&self, id: &str) -> Option<&ParseNode> {
        self.index.get(id).map(|&index| &self.output.nodes[index])
    }
}

fn sorted_unique(values: &[String]) -> bool {
    values.iter().all(|value| !value.is_empty()) && values.windows(2).all(|pair| pair[0] < pair[1])
}

pub(crate) fn validate_extensions(
    extensions: &[NativeExtension],
) -> Result<(), ParseValidationError> {
    let mut schemas = BTreeSet::new();
    for extension in extensions {
        if extension.schema.is_empty()
            || extension.namespace.is_empty()
            || !sorted_unique(&extension.capabilities)
            || !schemas.insert(&extension.schema)
        {
            return Err(ParseValidationError::InvalidExtension);
        }
    }
    Ok(())
}

fn validate_span(span: &SourceSpan, source: &SourceDocument) -> Result<(), ParseValidationError> {
    let error = ParseValidationError::InvalidSpan;
    source.slice(span.range.clone()).map_err(|_| error)?;
    if source.point(span.range.start_byte).map_err(|_| error)? != span.start_point
        || source.point(span.range.end_byte).map_err(|_| error)? != span.end_point
    {
        return Err(error);
    }
    Ok(())
}
