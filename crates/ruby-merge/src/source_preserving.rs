use std::collections::{BTreeMap, HashMap, HashSet};

use ast_merge::{
    CommentAugmentation, LayoutOwner, SourceEdit, apply_source_edits,
    augment_normalized_comments_with_owners,
};
use tree_haver::{NormalizedTreeNode, ParserRequest, parse_normalized_with_language_pack};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RubyOwnerKind {
    Class,
    Module,
    Method,
    SingletonMethod,
}

impl RubyOwnerKind {
    fn from_node(node: &NormalizedTreeNode) -> Option<Self> {
        match node.kind.as_str() {
            "class" => Some(Self::Class),
            "module" => Some(Self::Module),
            "method" => Some(Self::Method),
            "singleton_method" => Some(Self::SingletonMethod),
            _ => None,
        }
    }

    fn identity_prefix(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Module => "module",
            Self::Method => "method",
            Self::SingletonMethod => "singleton_method",
        }
    }

    fn container(self) -> bool {
        matches!(self, Self::Class | Self::Module)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RubySyntaxOwner {
    identity: String,
    node_id: String,
    start_line: usize,
    end_line: usize,
    owned_start_line: usize,
    insertion_byte: Option<usize>,
    children: Vec<RubySyntaxOwner>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RubySyntaxDocument {
    source: String,
    owners: Vec<RubySyntaxOwner>,
}

pub(crate) fn merge_ruby_source_preserving(
    template_source: &str,
    destination_source: &str,
) -> Result<String, String> {
    if template_source == destination_source {
        return Ok(destination_source.to_string());
    }

    let template = analyze_document(template_source)?;
    let destination = analyze_document(destination_source)?;
    let mut insertions = BTreeMap::<usize, Vec<String>>::new();
    plan_scope_insertions(
        &template,
        &destination,
        &template.owners,
        &destination.owners,
        destination.source.len(),
        &mut insertions,
    )?;

    let edits = insertions
        .into_iter()
        .map(|(byte, fragments)| SourceEdit::insert(byte, fragments.concat()))
        .collect::<Vec<_>>();
    let output =
        apply_source_edits(&destination.source, &edits).map_err(|error| error.to_string())?;
    verify_owner_union(&template, &destination, &output)?;
    Ok(output)
}

fn analyze_document(source: &str) -> Result<RubySyntaxDocument, String> {
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: "ruby".to_string(),
        dialect: Some("ruby".to_string()),
    });
    if !parsed.ok {
        return Err(parsed
            .diagnostics
            .first()
            .cloned()
            .unwrap_or_else(|| "TreeHaver could not parse Ruby source.".to_string()));
    }

    let nodes = parsed.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let root = nodes
        .get(parsed.root_id.as_str())
        .copied()
        .ok_or_else(|| "TreeHaver normalized Ruby parse omitted its root node.".to_string())?;
    if root.kind != "program" {
        return Err(format!(
            "TreeHaver normalized Ruby parse returned unsupported root kind {}.",
            root.kind
        ));
    }

    let mut owners = build_scope_owners(source, root, &nodes)?;
    apply_owned_starts(source, &parsed.nodes, &mut owners)?;
    validate_owner_scope(&owners, "program")?;

    Ok(RubySyntaxDocument { source: source.to_string(), owners })
}

fn build_scope_owners(
    source: &str,
    scope: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<Vec<RubySyntaxOwner>, String> {
    direct_children(scope, nodes)
        .filter_map(|node| RubyOwnerKind::from_node(node).map(|kind| (node, kind)))
        .map(|(node, kind)| build_owner(source, node, kind, nodes))
        .collect()
}

fn build_owner(
    source: &str,
    node: &NormalizedTreeNode,
    kind: RubyOwnerKind,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<RubySyntaxOwner, String> {
    let name = direct_children(node, nodes)
        .find(|child| child.field_name.as_deref() == Some("name"))
        .map(|child| child.source_fragment.trim().to_string())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("TreeHaver normalized Ruby {} omitted its name.", node.kind))?;
    let body =
        direct_children(node, nodes).find(|child| child.field_name.as_deref() == Some("body"));
    let children =
        body.map_or_else(|| Ok(Vec::new()), |body| build_scope_owners(source, body, nodes))?;
    let insertion_byte = if kind.container() {
        Some(
            direct_children(node, nodes)
                .find(|child| child.kind == "end")
                .map(|child| line_start_byte(source, child.span.start_point.row + 1))
                .ok_or_else(|| {
                    format!(
                        "TreeHaver normalized Ruby {} omitted its closing end token.",
                        node.kind
                    )
                })?,
        )
    } else {
        None
    };
    let start_line = node.span.start_point.row + 1;
    let end_line = node.span.end_point.row + 1;

    Ok(RubySyntaxOwner {
        identity: format!("{}:{name}", kind.identity_prefix()),
        node_id: node.id.clone(),
        start_line,
        end_line,
        owned_start_line: start_line,
        insertion_byte,
        children,
    })
}

fn direct_children<'a>(
    node: &'a NormalizedTreeNode,
    nodes: &'a HashMap<&str, &'a NormalizedTreeNode>,
) -> impl Iterator<Item = &'a NormalizedTreeNode> + 'a {
    node.child_ids.iter().filter_map(|id| nodes.get(id.as_str()).copied())
}

fn apply_owned_starts(
    source: &str,
    nodes: &[NormalizedTreeNode],
    owners: &mut [RubySyntaxOwner],
) -> Result<(), String> {
    let layout_owners = owners
        .iter()
        .map(|owner| LayoutOwner {
            owner_id: owner.node_id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        })
        .collect::<Vec<_>>();
    let augmentation = augment_normalized_comments_with_owners(
        source,
        &layout_owners,
        nodes,
        "hash_comment",
        normalize_ruby_comment,
    )?;
    apply_scope_owned_starts(owners, &augmentation);
    for owner in owners {
        apply_owned_starts(source, nodes, &mut owner.children)?;
    }
    Ok(())
}

fn apply_scope_owned_starts(owners: &mut [RubySyntaxOwner], augmentation: &CommentAugmentation) {
    let regions = augmentation
        .regions
        .iter()
        .map(|region| (region.id.clone(), region.clone()))
        .collect::<HashMap<_, _>>();
    let gaps = augmentation
        .gaps
        .iter()
        .map(|gap| (gap.id.clone(), gap.clone()))
        .collect::<HashMap<_, _>>();
    let attachments = augmentation
        .attachments
        .iter()
        .map(|attachment| (attachment.owner_id.as_str(), attachment))
        .collect::<HashMap<_, _>>();

    for owner in owners {
        owner.owned_start_line =
            attachments.get(owner.node_id.as_str()).map_or(owner.start_line, |attachment| {
                attachment.owned_start_line(owner.start_line, &regions, &gaps)
            });
    }
}

fn normalize_ruby_comment(text: &str) -> String {
    text.trim_start()
        .strip_prefix('#')
        .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
        .unwrap_or(text)
        .trim()
        .to_string()
}

fn validate_owner_scope(owners: &[RubySyntaxOwner], scope: &str) -> Result<(), String> {
    let mut identities = HashSet::new();
    for owner in owners {
        if !identities.insert(owner.identity.as_str()) {
            return Err(format!("Duplicate Ruby owner {} in {scope}.", owner.identity));
        }
        validate_owner_scope(&owner.children, &format!("{scope}/{}", owner.identity))?;
    }
    Ok(())
}

fn plan_scope_insertions(
    template_document: &RubySyntaxDocument,
    destination_document: &RubySyntaxDocument,
    template_owners: &[RubySyntaxOwner],
    destination_owners: &[RubySyntaxOwner],
    insertion_byte: usize,
    insertions: &mut BTreeMap<usize, Vec<String>>,
) -> Result<(), String> {
    let destination_by_identity = destination_owners
        .iter()
        .map(|owner| (owner.identity.as_str(), owner))
        .collect::<HashMap<_, _>>();
    let mut missing = Vec::new();

    for template_owner in template_owners {
        if let Some(destination_owner) =
            destination_by_identity.get(template_owner.identity.as_str())
        {
            if let Some(child_insertion_byte) = destination_owner.insertion_byte {
                plan_scope_insertions(
                    template_document,
                    destination_document,
                    &template_owner.children,
                    &destination_owner.children,
                    child_insertion_byte,
                    insertions,
                )?;
            }
        } else {
            missing.push(template_owner);
        }
    }

    if !missing.is_empty() {
        let fragments = missing
            .into_iter()
            .map(|owner| {
                line_fragment(&template_document.source, owner.owned_start_line, owner.end_line)
            })
            .collect::<Vec<_>>();
        insertions.entry(insertion_byte).or_default().push(section_fragment(
            &destination_document.source,
            insertion_byte,
            !destination_owners.is_empty(),
            fragments,
        ));
    }
    Ok(())
}

fn line_start_byte(source: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    source.match_indices('\n').nth(line - 2).map_or(source.len(), |(byte, _)| byte + 1)
}

fn line_after_byte(source: &str, line: usize) -> usize {
    let start = line_start_byte(source, line);
    source[start..].find('\n').map_or(source.len(), |offset| start + offset + 1)
}

fn line_fragment(source: &str, start_line: usize, end_line: usize) -> String {
    source[line_start_byte(source, start_line)..line_after_byte(source, end_line)].to_string()
}

fn section_fragment(
    destination: &str,
    insertion_byte: usize,
    has_existing_owners: bool,
    fragments: Vec<String>,
) -> String {
    let mut fragment = fragments.join("\n");
    if !fragment.ends_with('\n') {
        fragment.push('\n');
    }
    if insertion_byte > 0 && !destination[..insertion_byte].ends_with('\n') {
        fragment.insert(0, '\n');
    }
    if has_existing_owners
        && !destination[..insertion_byte].ends_with("\n\n")
        && !fragment.starts_with('\n')
    {
        fragment.insert(0, '\n');
    }
    fragment
}

fn verify_owner_union(
    template: &RubySyntaxDocument,
    destination: &RubySyntaxDocument,
    output: &str,
) -> Result<(), String> {
    let rendered = analyze_document(output)
        .map_err(|error| format!("Ruby source-preserving output failed to reparse: {error}"))?;
    let mut expected = HashSet::new();
    collect_owner_paths(&template.owners, "", &mut expected);
    collect_owner_paths(&destination.owners, "", &mut expected);
    let mut actual = HashSet::new();
    collect_owner_paths(&rendered.owners, "", &mut actual);
    if expected != actual {
        return Err(
            "Ruby source-preserving output did not retain the expected owner set.".to_string()
        );
    }
    Ok(())
}

fn collect_owner_paths(owners: &[RubySyntaxOwner], parent: &str, paths: &mut HashSet<String>) {
    for owner in owners {
        let path = format!("{parent}/{}", owner.identity);
        paths.insert(path.clone());
        collect_owner_paths(&owner.children, &path, paths);
    }
}
