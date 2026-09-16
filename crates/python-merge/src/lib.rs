//! Conservative top-level declaration profile. LibCST supplies syntax only.
use ast_merge::{SourcePreservingOwner, SourcePreservingOwnerDocument};
use std::collections::BTreeSet;
use tree_haver::{parsed::ParseNode, service::ParsedResult};
use unicode_normalization::UnicodeNormalization;

/// Derive identities and whole-statement ownership in Rust. Nested bodies are
/// opaque exact source, never recursively merged by this initial profile.
pub fn declaration_owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    let document = &parsed.document;
    if !document.output().ok || parsed.source.descriptor() != &document.output().source {
        return Err("invalid native parse".into());
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|_| "Python requires UTF-8")?;
    let root = document
        .node(document.output().root_id.as_deref().ok_or("missing module")?)
        .ok_or("missing module")?;
    if root.kind != "Module" || root.native_type != "libcst.Module" {
        return Err("expected native LibCST module".into());
    }
    let mut names = BTreeSet::new();
    let mut owners = Vec::new();
    if document.output().nodes.iter().any(|node| !node.unsupported_features.is_empty()) {
        return Err("unsupported native syntax".into());
    }
    for edge in &root.children {
        if edge.field_name.as_deref() != Some("body") {
            return Err("unsupported module edge".into());
        }
        let statement = document.node(&edge.node_id).ok_or("missing statement")?;
        let field = |node: &ParseNode, name: &str| -> Result<&ParseNode, String> {
            let edges: Vec<_> = node
                .children
                .iter()
                .filter(|edge| edge.field_name.as_deref() == Some(name))
                .collect();
            if edges.len() != 1 {
                return Err(format!("expected one {name}"));
            }
            document.node(&edges[0].node_id).ok_or_else(|| "missing child".into())
        };
        let name = match statement.kind.as_str() {
            "FunctionDef" | "ClassDef" => {
                let decorated = statement
                    .extensions
                    .iter()
                    .find(|extension| {
                        extension.schema == "structuredmerge.extension/python-libcst/v1"
                            && extension.namespace == "python-libcst"
                    })
                    .and_then(|extension| extension.payload.get("decorators_count"))
                    .and_then(|value| value.as_u64())
                    .ok_or("missing native decorator count")?;
                if decorated != 0 {
                    return Err("decorated declarations require a richer ownership profile".into());
                }
                field(statement, "name")?
            }
            "SimpleStatementLine" => {
                let assign = field(statement, "body")?;
                if assign.kind != "Assign" {
                    return Err("only simple assignments are supported".into());
                }
                let target = field(assign, "targets")?;
                if target.kind != "AssignTarget" {
                    return Err("invalid native assignment target".into());
                }
                field(target, "target")?
            }
            _ => return Err(format!("unsupported top-level statement: {}", statement.kind)),
        };
        if name.kind != "Name" {
            return Err("only name bindings are supported".into());
        }
        let value = name
            .extensions
            .iter()
            .find(|extension| {
                extension.schema == "structuredmerge.extension/python-libcst/v1"
                    && extension.namespace == "python-libcst"
            })
            .and_then(|extension| extension.payload.get("value"))
            .and_then(|value| value.as_str())
            .ok_or("missing native name value")?;
        // Python compares identifiers after NFKC normalization. Use that for
        // identity only; the exact lexical bytes remain in the source owner.
        let identity: String = value.nfkc().collect();
        if identity.is_empty() || !names.insert(identity.clone()) {
            return Err("empty or duplicate top-level name".into());
        }
        let range = &statement.span.range;
        let bytes = parsed.source.slice(range.clone()).map_err(|_| "invalid statement range")?;
        let path = format!("/{}", identity.replace('~', "~0").replace('/', "~1"));
        owners.push(SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint: std::str::from_utf8(bytes).map_err(|_| "statement cuts UTF-8")?.to_owned(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            start_line: statement.span.start_point.row + 1,
            end_line: statement.span.end_point.row + 1,
        });
    }
    Ok(SourcePreservingOwnerDocument { source: source.into(), owners })
}
