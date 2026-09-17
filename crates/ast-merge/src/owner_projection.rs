use std::collections::HashSet;

use tree_haver::{NodeRole, NormalizedTreeIndex, NormalizedTreeNode};

use crate::{SourcePreservingOwner, SourcePreservingOwnerDocument};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedOwnerKind<'a> {
    pub node_kind: &'a str,
    pub path_kind: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedOwnerProjectionPolicy<'a> {
    pub family: &'a str,
    pub owner_kinds: &'a [NamedOwnerKind<'a>],
    pub ignored_kinds: &'a [&'a str],
    pub wrapper_kinds: &'a [&'a str],
    pub name_fields: &'a [&'a str],
    pub fallback_name_kinds: &'a [&'a str],
    pub accept_any_named_kind: bool,
}

pub fn project_named_top_level_owners(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Result<SourcePreservingOwnerDocument, String> {
    project_named_top_level_analysis(source, root_id, nodes, policy)
        .map(|analysis| analysis.document)
}

/// Preserve native references at the moment the family establishes ownership.
/// The original document-only API delegates here without changing its policy.
pub fn project_named_top_level_analysis(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Result<crate::native_analysis::NativeOwnerAnalysis, String> {
    let index = NormalizedTreeIndex::new(nodes)?;
    let root = index.root(root_id)?;
    let mut owners = Vec::new();
    let mut owner_ids = HashSet::new();
    let mut owner_node_ids = std::collections::BTreeMap::new();

    for top_level in index.children(root) {
        if top_level.role == NodeRole::Comment
            || policy.ignored_kinds.contains(&top_level.kind.as_str())
        {
            continue;
        }

        let owner_node = resolve_owner_node(top_level, &index, policy)?;
        let path_kind = owner_path_kind(owner_node, policy).ok_or_else(|| {
            format!("unsupported top-level {} node {:?}", policy.family, top_level.kind)
        })?;
        let name = owner_name(owner_node, &index, policy)?;
        let path = format!("/{path_kind}:{name}");
        if !owner_ids.insert(path.clone()) {
            return Err(format!(
                "{} document has duplicate top-level owner identity {path:?}",
                policy.family
            ));
        }
        if top_level.source_fragment.is_empty() {
            return Err(format!("{} owner {path:?} has no source fragment", policy.family));
        }

        owner_node_ids.insert(path.clone(), vec![top_level.id.clone()]);
        owners.push(SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint: top_level.source_fragment.clone(),
            start_byte: top_level.span.range.start_byte,
            end_byte: top_level.span.range.end_byte,
            start_line: top_level.span.start_point.row + 1,
            end_line: top_level.span.end_point.row + 1,
        });
    }

    if owners.is_empty() {
        return Err(format!("{} document has no supported top-level named owners", policy.family));
    }

    Ok(crate::native_analysis::NativeOwnerAnalysis {
        document: SourcePreservingOwnerDocument { source: source.to_string(), owners },
        owner_node_ids,
    })
}

fn resolve_owner_node<'a>(
    top_level: &'a NormalizedTreeNode,
    index: &NormalizedTreeIndex<'a>,
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Result<&'a NormalizedTreeNode, String> {
    if owner_path_kind(top_level, policy).is_some() {
        return Ok(top_level);
    }
    if !policy.wrapper_kinds.contains(&top_level.kind.as_str()) {
        return Err(format!("unsupported top-level {} node {:?}", policy.family, top_level.kind));
    }

    let candidates = index
        .children(top_level)
        .into_iter()
        .filter(|child| owner_path_kind(child, policy).is_some())
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [owner] => Ok(owner),
        [] => Err(format!("{} wrapper {:?} has no supported owner", policy.family, top_level.kind)),
        _ => Err(format!("{} wrapper {:?} has ambiguous owners", policy.family, top_level.kind)),
    }
}

fn owner_path_kind(
    node: &NormalizedTreeNode,
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Option<String> {
    policy
        .owner_kinds
        .iter()
        .find(|candidate| candidate.node_kind == node.kind)
        .map(|candidate| candidate.path_kind.to_string())
        .or_else(|| policy.accept_any_named_kind.then(|| node.kind.clone()))
}

fn owner_name(
    node: &NormalizedTreeNode,
    index: &NormalizedTreeIndex<'_>,
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Result<String, String> {
    let children = index.children(node);
    let field_candidates = children
        .iter()
        .filter(|child| {
            child.field_name.as_deref().is_some_and(|field| policy.name_fields.contains(&field))
        })
        .copied()
        .collect::<Vec<_>>();
    let candidates = if field_candidates.is_empty() {
        children
            .into_iter()
            .filter(|child| policy.fallback_name_kinds.contains(&child.kind.as_str()))
            .collect::<Vec<_>>()
    } else {
        field_candidates
    };

    let candidate = match candidates.as_slice() {
        [candidate] => candidate,
        [] => {
            return Err(format!("{} owner {:?} has no stable name", policy.family, node.kind));
        }
        _ => {
            return Err(format!("{} owner {:?} has an ambiguous name", policy.family, node.kind));
        }
    };
    let name = candidate.source_fragment.trim();
    if name.is_empty() {
        return Err(format!("{} owner {:?} has an empty name", policy.family, node.kind));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tree_haver::{ByteRange, NodeRole, SourcePoint, SourceSpan};

    use super::*;

    fn node(
        id: &str,
        kind: &str,
        parent_id: Option<&str>,
        child_ids: &[&str],
        field_name: Option<&str>,
        source_fragment: &str,
        byte_range: [usize; 2],
    ) -> NormalizedTreeNode {
        let [start_byte, end_byte] = byte_range;
        NormalizedTreeNode {
            id: id.to_string(),
            kind: kind.to_string(),
            role: NodeRole::Structural,
            parent_id: parent_id.map(str::to_string),
            child_ids: child_ids.iter().map(|value| (*value).to_string()).collect(),
            span: SourceSpan {
                range: ByteRange { start_byte, end_byte },
                start_point: SourcePoint { row: 0, column: start_byte },
                end_point: SourcePoint { row: 0, column: end_byte },
            },
            field_name: field_name.map(str::to_string),
            named: true,
            anonymous: false,
            has_source_text: true,
            source_fragment: source_fragment.to_string(),
            backend_kind: Some(kind.to_string()),
            semantic_roles: vec![],
            backend_roles: vec!["test".to_string()],
            unsupported_features: vec![],
            metadata: BTreeMap::new(),
        }
    }

    fn policy<'a>() -> NamedOwnerProjectionPolicy<'a> {
        static OWNER_KINDS: &[NamedOwnerKind<'static>] =
            &[NamedOwnerKind { node_kind: "function_definition", path_kind: "function" }];
        NamedOwnerProjectionPolicy {
            family: "test",
            owner_kinds: OWNER_KINDS,
            ignored_kinds: &["import_statement"],
            wrapper_kinds: &["export_statement"],
            name_fields: &["name"],
            fallback_name_kinds: &[],
            accept_any_named_kind: false,
        }
    }

    #[test]
    fn projects_exact_top_level_owner_ranges_and_ignores_layout_nodes() {
        let source = "import x\n\nfn left() {}\n\nfn right() {}\n";
        let nodes = vec![
            node(
                "root",
                "module",
                None,
                &["import", "left", "right"],
                None,
                source,
                [0, source.len()],
            ),
            node("import", "import_statement", Some("root"), &[], None, "import x", [0, 8]),
            node(
                "left",
                "function_definition",
                Some("root"),
                &["left-name"],
                None,
                "fn left() {}",
                [10, 22],
            ),
            node("left-name", "identifier", Some("left"), &[], Some("name"), "left", [13, 17]),
            node(
                "right",
                "function_definition",
                Some("root"),
                &["right-name"],
                None,
                "fn right() {}",
                [24, 37],
            ),
            node("right-name", "identifier", Some("right"), &[], Some("name"), "right", [27, 32]),
        ];

        let document = project_named_top_level_owners(source, "root", &nodes, policy()).unwrap();

        assert_eq!(document.owners.len(), 2);
        assert_eq!(document.owners[0].id, "/function:left");
        assert_eq!(document.owners[0].start_byte, 10);
        assert_eq!(document.owners[1].id, "/function:right");
        assert_eq!(document.owners[1].fingerprint, "fn right() {}");
        let analysis = project_named_top_level_analysis(source, "root", &nodes, policy()).unwrap();
        assert_eq!(analysis.document, document);
        assert_eq!(analysis.owner_node_ids["/function:left"], ["left"]);
        assert_eq!(analysis.owner_node_ids["/function:right"], ["right"]);
    }

    #[test]
    fn rejects_unowned_ambiguous_and_duplicate_top_level_structure() {
        let source = "value = 1";
        let unsupported = vec![
            node("root", "module", None, &["value"], None, source, [0, source.len()]),
            node("value", "assignment", Some("root"), &[], None, source, [0, source.len()]),
        ];
        assert!(
            project_named_top_level_owners(source, "root", &unsupported, policy())
                .unwrap_err()
                .contains("unsupported top-level")
        );

        let duplicate_source = "fn same() {}\nfn same() {}";
        let duplicate = vec![
            node(
                "root",
                "module",
                None,
                &["one", "two"],
                None,
                duplicate_source,
                [0, duplicate_source.len()],
            ),
            node(
                "one",
                "function_definition",
                Some("root"),
                &["one-name"],
                None,
                "fn same() {}",
                [0, 12],
            ),
            node("one-name", "identifier", Some("one"), &[], Some("name"), "same", [3, 7]),
            node(
                "two",
                "function_definition",
                Some("root"),
                &["two-name"],
                None,
                "fn same() {}",
                [13, 25],
            ),
            node("two-name", "identifier", Some("two"), &[], Some("name"), "same", [16, 20]),
        ];
        assert!(
            project_named_top_level_owners(duplicate_source, "root", &duplicate, policy())
                .unwrap_err()
                .contains("duplicate top-level owner identity")
        );
    }

    #[test]
    fn generic_mode_requires_an_unambiguous_name_field() {
        let source = "def answer(): pass";
        let nodes = vec![
            node("root", "module", None, &["function"], None, source, [0, source.len()]),
            node(
                "function",
                "function_definition",
                Some("root"),
                &["name"],
                None,
                source,
                [0, source.len()],
            ),
            node("name", "identifier", Some("function"), &[], Some("name"), "answer", [4, 10]),
        ];
        let generic = NamedOwnerProjectionPolicy {
            family: "python",
            owner_kinds: &[],
            ignored_kinds: &[],
            wrapper_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &[],
            accept_any_named_kind: true,
        };

        let document = project_named_top_level_owners(source, "root", &nodes, generic).unwrap();

        assert_eq!(document.owners[0].id, "/function_definition:answer");
    }
}
