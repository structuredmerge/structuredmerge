use std::collections::HashSet;

use tree_haver::{NodeRole, NormalizedTreeIndex, NormalizedTreeNode};

use crate::{
    CommentAttachmentPolicy, CommentScope, LayoutOwner, SourcePreservingOwner,
    SourcePreservingOwnerDocument, augment_normalized_comments_with_evidence_scope,
};

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
    /// Top-level syntax nodes that the family treats as declaration prefixes.
    pub prefix_node_kinds: &'a [&'a str],
    pub name_fields: &'a [&'a str],
    pub fallback_name_kinds: &'a [&'a str],
    pub accept_any_named_kind: bool,
    pub comment_attachment_policy: CommentAttachmentPolicy,
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
    project_named_top_level_analysis_with_comment_scope(source, root_id, nodes, policy, |_| {
        CommentScope::Owner
    })
}

/// Project syntax owners while allowing a parser adapter to identify comments
/// that belong to the document root. Attachment and emission remain shared.
pub fn project_named_top_level_analysis_with_comment_scope(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    policy: NamedOwnerProjectionPolicy<'_>,
    comment_scope: impl Fn(&NormalizedTreeNode) -> CommentScope,
) -> Result<crate::native_analysis::NativeOwnerAnalysis, String> {
    let index = NormalizedTreeIndex::new(nodes)?;
    let root = index.root(root_id)?;
    let mut owners = Vec::new();
    let mut owner_ids = HashSet::new();
    let mut owner_node_ids = std::collections::BTreeMap::new();
    let mut attached_comment_node_ids = std::collections::BTreeMap::new();
    let mut pending_prefix_nodes = Vec::new();

    for top_level in index.children(root) {
        if top_level.role == NodeRole::Comment {
            if comment_scope(top_level) == CommentScope::Root {
                pending_prefix_nodes.clear();
            } else if !policy.prefix_node_kinds.is_empty() {
                pending_prefix_nodes.push(top_level);
            }
            continue;
        }
        if policy.prefix_node_kinds.contains(&top_level.kind.as_str()) {
            pending_prefix_nodes.push(top_level);
            continue;
        }
        if policy.ignored_kinds.contains(&top_level.kind.as_str()) {
            pending_prefix_nodes.clear();
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

        let prefix_nodes =
            select_owner_prefix_nodes(source, &pending_prefix_nodes, top_level, &index, policy)?;
        pending_prefix_nodes.clear();
        let start_node = prefix_nodes.first().copied().unwrap_or(top_level);
        let mut owner_nodes = prefix_nodes
            .iter()
            .filter(|node| node.role != NodeRole::Comment)
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        owner_nodes.push(top_level.id.clone());
        owner_node_ids.insert(path.clone(), owner_nodes);
        let comment_ids = prefix_nodes
            .iter()
            .filter(|node| node.role == NodeRole::Comment)
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if !comment_ids.is_empty() {
            attached_comment_node_ids.insert(path.clone(), comment_ids);
        }
        let start_byte = start_node.span.range.start_byte;
        let end_byte = top_level.span.range.end_byte;
        owners.push(SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint: source[start_byte..end_byte].to_string(),
            start_byte,
            end_byte,
            start_line: start_node.span.start_point.row + 1,
            end_line: top_level.span.end_point.row + 1,
        });
    }

    if owners.is_empty() {
        return Err(format!("{} document has no supported top-level named owners", policy.family));
    }

    attached_comment_node_ids = attach_normalized_comment_ranges(
        source,
        &mut owners,
        &owner_node_ids,
        nodes,
        policy.comment_attachment_policy,
        &comment_scope,
        attached_comment_node_ids,
    )?;

    Ok(crate::native_analysis::NativeOwnerAnalysis {
        document: SourcePreservingOwnerDocument { source: source.to_string(), owners },
        owner_node_ids,
        attached_comment_node_ids,
    })
}

fn select_owner_prefix_nodes<'a>(
    source: &str,
    pending: &[&'a NormalizedTreeNode],
    owner: &NormalizedTreeNode,
    index: &NormalizedTreeIndex<'a>,
    policy: NamedOwnerProjectionPolicy<'_>,
) -> Result<Vec<&'a NormalizedTreeNode>, String> {
    let mut selected = Vec::new();
    let mut cursor = owner.span.range.start_byte;
    for node in pending.iter().rev() {
        if node.role == NodeRole::Comment
            && (is_inner_doc_comment(node, index)
                || !starts_on_standalone_line(source, node.span.range.start_byte))
        {
            break;
        }
        let start = node.span.range.start_byte;
        let end = node.span.range.end_byte;
        if start > end || end > cursor || cursor > source.len() {
            return Err("declaration prefix nodes have invalid source order".into());
        }
        let gap = &source[end..cursor];
        if !gap.bytes().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')) {
            break;
        }
        let blank_boundary = gap.bytes().filter(|byte| *byte == b'\n').count() > 1;
        let inner_doc = node.role == NodeRole::Comment && is_inner_doc_comment(node, index);
        let structural = !inner_doc
            && (policy.prefix_node_kinds.contains(&node.kind.as_str())
                || (node.role == NodeRole::Comment
                    && structural_doc_comment(node, index, policy.family)));
        if blank_boundary && !structural {
            break;
        }
        selected.push(*node);
        cursor = start;
    }
    selected.reverse();
    Ok(selected)
}

fn starts_on_standalone_line(source: &str, start_byte: usize) -> bool {
    if start_byte > source.len() || !source.is_char_boundary(start_byte) {
        return false;
    }
    let line_start = source[..start_byte].rfind('\n').map_or(0, |offset| offset + 1);
    source[line_start..start_byte].bytes().all(|byte| byte.is_ascii_whitespace())
}

fn is_inner_doc_comment(node: &NormalizedTreeNode, index: &NormalizedTreeIndex<'_>) -> bool {
    let fragment = node.source_fragment.trim_start();
    node.semantic_roles.iter().any(|role| role == "inner_doc_comment")
        || fragment.starts_with("//!")
        || fragment.starts_with("/*!")
        || index.descendants(node).iter().any(|child| child.kind == "inner_doc_comment_marker")
}

fn structural_doc_comment(
    node: &NormalizedTreeNode,
    index: &NormalizedTreeIndex<'_>,
    family: &str,
) -> bool {
    let fragment = node.source_fragment.trim_start();
    let rust_non_doc_prefix = family.eq_ignore_ascii_case("rust")
        && (fragment.starts_with("////") || fragment.starts_with("/***"));
    !rust_non_doc_prefix
        && !is_inner_doc_comment(node, index)
        && (node.semantic_roles.iter().any(|role| role == "doc_comment")
            || (fragment.starts_with("///") && !fragment.starts_with("////"))
            || (fragment.starts_with("/**") && !fragment.starts_with("/***"))
            || index.descendants(node).iter().any(|child| child.kind == "outer_doc_comment_marker"))
}

/// Extend structural owner source ranges with the comments assigned by the
/// shared source-preservation comment augmenter. Family projections still decide
/// which syntax nodes are owners; line ownership is resolved here once for all
/// source-preserving providers.
fn attach_normalized_comment_ranges(
    source: &str,
    owners: &mut [SourcePreservingOwner],
    owner_node_ids: &std::collections::BTreeMap<String, Vec<String>>,
    nodes: &[NormalizedTreeNode],
    policy: CommentAttachmentPolicy,
    comment_scope: &dyn Fn(&NormalizedTreeNode) -> CommentScope,
    mut attached_by_owner: std::collections::BTreeMap<String, Vec<String>>,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, String> {
    let nodes_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    for owner in owners.iter_mut() {
        if let Some(comment_ids) = attached_by_owner.get(&owner.id) {
            for id in comment_ids {
                let comment = nodes_by_id.get(id.as_str()).ok_or("unresolved prefix comment")?;
                if comment.role != NodeRole::Comment {
                    return Err("prefix comment reference has a non-comment role".into());
                }
                owner.start_byte = owner.start_byte.min(comment.span.range.start_byte);
                owner.start_line = owner.start_line.min(comment.span.start_point.row + 1);
            }
            owner.fingerprint = source[owner.start_byte..owner.end_byte].to_string();
        }
    }
    if !nodes.iter().any(|node| node.role == NodeRole::Comment) {
        return Ok(attached_by_owner);
    }
    let node_owner_ids = owner_node_ids
        .iter()
        .flat_map(|(owner_id, node_ids)| {
            node_ids.iter().map(move |node_id| (node_id.as_str(), owner_id.as_str()))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let layout_owners = owners
        .iter()
        .filter_map(|owner| {
            owner_node_ids.get(&owner.id)?.last().map(|node_id| LayoutOwner {
                owner_id: node_id.clone(),
                start_line: owner.start_line,
                end_line: owner.end_line,
            })
        })
        .collect::<Vec<_>>();
    if layout_owners.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }

    let prefix_comment_ranges = attached_by_owner
        .values()
        .flatten()
        .filter_map(|id| nodes_by_id.get(id.as_str()))
        .map(|node| node.span.range.clone())
        .collect::<Vec<_>>();
    let tree_index = NormalizedTreeIndex::new(nodes)?;
    let evidence = augment_normalized_comments_with_evidence_scope(
        source,
        &layout_owners,
        nodes,
        "source",
        policy,
        |node| {
            if is_inner_doc_comment(node, &tree_index)
                || prefix_comment_ranges.iter().any(|range| {
                    range.start_byte <= node.span.range.start_byte
                        && node.span.range.end_byte <= range.end_byte
                })
            {
                None
            } else {
                Some(comment_scope(node))
            }
        },
        str::to_string,
    )?;
    for attachment in &evidence.augmentation.attachments {
        let Some(owner_id) = node_owner_ids.get(attachment.owner_id.as_str()).copied() else {
            continue;
        };
        let Some(owner_index) = owners.iter().position(|owner| owner.id == owner_id) else {
            continue;
        };
        let owner = &mut owners[owner_index];
        for (kind, region_id) in [
            ("leading", attachment.leading_region_id.as_ref()),
            ("inline", attachment.inline_region_id.as_ref()),
            ("trailing", attachment.trailing_region_id.as_ref()),
        ] {
            let Some(region_id) = region_id else { continue };
            let comment_nodes = evidence
                .region_node_ids
                .get(region_id)
                .into_iter()
                .flatten()
                .filter_map(|node_id| nodes_by_id.get(node_id.as_str()).copied())
                .collect::<Vec<_>>();
            if comment_nodes.is_empty() {
                return Err(format!(
                    "attached comment region {region_id} has no native source node"
                ));
            }
            for node in &comment_nodes {
                let ids = attached_by_owner.entry(owner_id.to_string()).or_default();
                if !ids.contains(&node.id) {
                    ids.push(node.id.clone());
                }
            }
            if kind == "leading" {
                let first = comment_nodes
                    .iter()
                    .min_by_key(|node| node.span.range.start_byte)
                    .expect("comment node list is nonempty");
                owner.start_byte = owner.start_byte.min(first.span.range.start_byte);
                owner.start_line = owner.start_line.min(first.span.start_point.row + 1);
            } else {
                let last = comment_nodes
                    .iter()
                    .max_by_key(|node| node.span.range.end_byte)
                    .expect("comment node list is nonempty");
                owner.end_byte = owner.end_byte.max(last.span.range.end_byte);
                owner.end_line = owner.end_line.max(last.span.end_point.row + 1);
            }
        }
        owner.fingerprint = source[owner.start_byte..owner.end_byte].to_string();
    }
    for owner in owners {
        if let Some(ids) = attached_by_owner.get_mut(&owner.id) {
            ids.sort_by_key(|id| {
                nodes_by_id.get(id.as_str()).map_or(usize::MAX, |node| node.span.range.start_byte)
            });
        }
        owner.fingerprint = source[owner.start_byte..owner.end_byte].to_string();
    }
    Ok(attached_by_owner)
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
            prefix_node_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &[],
            accept_any_named_kind: false,
            comment_attachment_policy: CommentAttachmentPolicy::PrecedingOwner,
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
            prefix_node_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &[],
            accept_any_named_kind: true,
            comment_attachment_policy: CommentAttachmentPolicy::PrecedingOwner,
        };

        let document = project_named_top_level_owners(source, "root", &nodes, generic).unwrap();

        assert_eq!(document.owners[0].id, "/function_definition:answer");
    }
}
