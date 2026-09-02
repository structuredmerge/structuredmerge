use std::collections::{BTreeMap, HashMap, HashSet};

use ast_merge::{
    CommentAugmentation, LayoutOwner, SourceEdit, apply_source_edits,
    augment_normalized_comments_with_owners,
};
use tree_haver::{NormalizedTreeNode, ParserRequest, parse_normalized_with_language_pack};

use crate::{TomlOwner, TomlOwnerKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlProjectionScopeKind {
    Root,
    Table,
    TableArray,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlProjectionEntry {
    pub key_components: Vec<String>,
    pub node_id: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub value_signature: String,
    pub array_item_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomlProjectionScope {
    pub kind: TomlProjectionScopeKind,
    pub path_components: Vec<String>,
    pub instance: usize,
    pub node_id: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub entries: Vec<TomlProjectionEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TomlSyntaxScopeKind {
    Root,
    Table,
    TableArray,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TomlSyntaxEntry {
    key: String,
    path: String,
    node_id: String,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    owned_start_line: usize,
    value_signature: String,
    array_item_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TomlSyntaxScope {
    kind: TomlSyntaxScopeKind,
    path: String,
    instance: usize,
    node_id: String,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    owned_start_line: usize,
    entries: Vec<TomlSyntaxEntry>,
}

impl TomlSyntaxScope {
    fn identity(&self) -> String {
        match self.kind {
            TomlSyntaxScopeKind::Root => "root".to_string(),
            TomlSyntaxScopeKind::Table => format!("table:{}", self.path),
            TomlSyntaxScopeKind::TableArray => {
                format!("table_array:{}#{}", self.path, self.instance)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TomlSyntaxDocument {
    source: String,
    scopes: Vec<TomlSyntaxScope>,
    pub(crate) owners: Vec<TomlOwner>,
    pub(crate) comment_augmentation: CommentAugmentation,
    pub(crate) semantic: BTreeMap<String, String>,
}

impl TomlSyntaxDocument {
    pub(crate) fn expected_merge_semantic(
        &self,
        destination: &TomlSyntaxDocument,
    ) -> BTreeMap<String, String> {
        let mut expected = destination.semantic.clone();
        for (identity, signature) in &self.semantic {
            expected.entry(identity.clone()).or_insert_with(|| signature.clone());
        }
        expected
    }
}

pub(crate) fn analyze_toml_document(source: &str) -> Result<TomlSyntaxDocument, String> {
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: "toml".to_string(),
        dialect: Some("toml".to_string()),
    });
    if !parsed.ok {
        return Err(parsed
            .diagnostics
            .first()
            .cloned()
            .unwrap_or_else(|| "TreeHaver could not parse TOML source.".to_string()));
    }

    let nodes = parsed.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let root = nodes
        .get(parsed.root_id.as_str())
        .copied()
        .ok_or_else(|| "TreeHaver normalized TOML parse omitted its root node.".to_string())?;
    if root.kind != "document" {
        return Err(format!(
            "TreeHaver normalized TOML parse returned unsupported root kind {}.",
            root.kind
        ));
    }

    let mut table_array_instances = HashMap::<String, usize>::new();
    let root_entries = direct_children(root, &nodes)
        .filter(|node| node.kind == "pair")
        .map(|node| build_entry(node, "", &nodes))
        .collect::<Result<Vec<_>, _>>()?;
    let mut scopes = vec![TomlSyntaxScope {
        kind: TomlSyntaxScopeKind::Root,
        path: String::new(),
        instance: 0,
        node_id: root.id.clone(),
        start_byte: root.span.range.start_byte,
        end_byte: root.span.range.end_byte,
        start_line: node_start_line(root),
        end_line: node_end_line(root),
        owned_start_line: node_start_line(root),
        entries: root_entries,
    }];

    for node in direct_children(root, &nodes) {
        let kind = match node.kind.as_str() {
            "table" => TomlSyntaxScopeKind::Table,
            "table_array_element" => TomlSyntaxScopeKind::TableArray,
            _ => continue,
        };
        let path = path_string(&table_key_components(node, &nodes)?);
        let instance = if kind == TomlSyntaxScopeKind::TableArray {
            let next = table_array_instances.entry(path.clone()).or_default();
            let instance = *next;
            *next += 1;
            instance
        } else {
            0
        };
        let entries = direct_children(node, &nodes)
            .filter(|child| child.kind == "pair")
            .map(|child| build_entry(child, &path, &nodes))
            .collect::<Result<Vec<_>, _>>()?;
        scopes.push(TomlSyntaxScope {
            kind,
            path,
            instance,
            node_id: node.id.clone(),
            start_byte: node.span.range.start_byte,
            end_byte: node.span.range.end_byte,
            start_line: node_start_line(node),
            end_line: node_end_line(node),
            owned_start_line: node_start_line(node),
            entries,
        });
    }

    validate_scopes(&scopes)?;
    let layout_owners = syntax_layout_owners(&scopes);
    let comment_augmentation = augment_normalized_comments_with_owners(
        source,
        &layout_owners,
        &parsed.nodes,
        "hash_comment",
        normalize_toml_comment,
    )?;
    apply_owned_starts(&mut scopes, &comment_augmentation);
    let owners = collect_owners(&scopes);
    let semantic = collect_semantic(&scopes);

    Ok(TomlSyntaxDocument {
        source: source.to_string(),
        scopes,
        owners,
        comment_augmentation,
        semantic,
    })
}

pub(crate) fn analyze_toml_projection_document(
    source: &str,
    projection: Vec<TomlProjectionScope>,
    comments: Vec<ast_merge::TrackedComment>,
) -> Result<TomlSyntaxDocument, String> {
    let mut scopes = projection
        .into_iter()
        .map(|scope| {
            let kind = match scope.kind {
                TomlProjectionScopeKind::Root => TomlSyntaxScopeKind::Root,
                TomlProjectionScopeKind::Table => TomlSyntaxScopeKind::Table,
                TomlProjectionScopeKind::TableArray => TomlSyntaxScopeKind::TableArray,
            };
            let path = if kind == TomlSyntaxScopeKind::Root {
                String::new()
            } else {
                path_string(&scope.path_components)
            };
            TomlSyntaxScope {
                kind,
                path: path.clone(),
                instance: scope.instance,
                node_id: scope.node_id,
                start_byte: scope.start_byte,
                end_byte: scope.end_byte,
                start_line: scope.start_line,
                end_line: scope.end_line,
                owned_start_line: scope.start_line,
                entries: scope
                    .entries
                    .into_iter()
                    .map(|entry| {
                        let key = path_string(&entry.key_components);
                        TomlSyntaxEntry {
                            path: if path.is_empty() {
                                key.clone()
                            } else {
                                format!("{path}{key}")
                            },
                            key,
                            node_id: entry.node_id,
                            start_byte: entry.start_byte,
                            end_byte: entry.end_byte,
                            start_line: entry.start_line,
                            end_line: entry.end_line,
                            owned_start_line: entry.start_line,
                            value_signature: entry.value_signature,
                            array_item_count: entry.array_item_count,
                        }
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    validate_projection(source, &scopes)?;
    validate_scopes(&scopes)?;
    let layout_owners = syntax_layout_owners(&scopes);
    let lines = source_lines(source);
    let comment_augmentation =
        ast_merge::augment_comments(&lines, &layout_owners, &comments, "hash_comment")?;
    apply_owned_starts(&mut scopes, &comment_augmentation);
    let owners = collect_owners(&scopes);
    let semantic = collect_semantic(&scopes);
    Ok(TomlSyntaxDocument {
        source: source.to_string(),
        scopes,
        owners,
        comment_augmentation,
        semantic,
    })
}

pub(crate) fn merge_toml_documents(
    template: &TomlSyntaxDocument,
    destination: &TomlSyntaxDocument,
) -> Result<String, String> {
    if template.source == destination.source {
        return Ok(destination.source.clone());
    }
    validate_merge_scope_compatibility(template, destination)?;

    let mut insertions = BTreeMap::<usize, Vec<String>>::new();
    let destination_scopes =
        destination.scopes.iter().map(|scope| (scope.identity(), scope)).collect::<HashMap<_, _>>();

    for template_scope in &template.scopes {
        if let Some(destination_scope) = destination_scopes.get(&template_scope.identity()) {
            plan_entry_insertions(
                template,
                destination,
                template_scope,
                destination_scope,
                &mut insertions,
            )?;
        }
    }
    plan_scope_insertions(template, destination, &destination_scopes, &mut insertions)?;

    let edits = insertions
        .into_iter()
        .map(|(byte, fragments)| SourceEdit::insert(byte, fragments.concat()))
        .collect::<Vec<_>>();
    apply_source_edits(&destination.source, &edits).map_err(|error| error.to_string())
}

fn direct_children<'a>(
    node: &'a NormalizedTreeNode,
    nodes: &'a HashMap<&str, &'a NormalizedTreeNode>,
) -> impl Iterator<Item = &'a NormalizedTreeNode> + 'a {
    node.child_ids.iter().filter_map(|id| nodes.get(id.as_str()).copied())
}

fn named_children<'a>(
    node: &'a NormalizedTreeNode,
    nodes: &'a HashMap<&str, &'a NormalizedTreeNode>,
) -> impl Iterator<Item = &'a NormalizedTreeNode> + 'a {
    direct_children(node, nodes).filter(|child| child.named && child.kind != "comment")
}

fn build_entry(
    pair: &NormalizedTreeNode,
    scope_path: &str,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<TomlSyntaxEntry, String> {
    let children = named_children(pair, nodes).collect::<Vec<_>>();
    let key_node = children
        .first()
        .copied()
        .ok_or_else(|| "TreeHaver normalized TOML pair omitted its key.".to_string())?;
    let value_node = children
        .get(1)
        .copied()
        .ok_or_else(|| "TreeHaver normalized TOML pair omitted its value.".to_string())?;
    let key_components = key_components(key_node, nodes)?;
    let key = path_string(&key_components);
    let path = if scope_path.is_empty() { key.clone() } else { format!("{scope_path}{key}") };
    let array_item_count =
        if value_node.kind == "array" { named_children(value_node, nodes).count() } else { 0 };

    Ok(TomlSyntaxEntry {
        key,
        path,
        node_id: pair.id.clone(),
        start_byte: pair.span.range.start_byte,
        end_byte: pair.span.range.end_byte,
        start_line: node_start_line(pair),
        end_line: node_end_line(pair),
        owned_start_line: node_start_line(pair),
        value_signature: normalized_node_signature(value_node, nodes),
        array_item_count,
    })
}

fn table_key_components(
    table: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<Vec<String>, String> {
    let key = named_children(table, nodes)
        .find(|child| child.kind != "pair")
        .ok_or_else(|| format!("TreeHaver normalized {} omitted its key.", table.kind))?;
    key_components(key, nodes)
}

fn key_components(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<Vec<String>, String> {
    match node.kind.as_str() {
        "bare_key" => Ok(vec![node.source_fragment.clone()]),
        "dotted_key" => {
            let mut components = Vec::new();
            for child in named_children(node, nodes) {
                components.extend(key_components(child, nodes)?);
            }
            Ok(components)
        }
        kind => Err(format!(
            "Unsupported TOML key node {kind}. Quoted keys are not yet ownership-safe."
        )),
    }
}

fn path_string(components: &[String]) -> String {
    format!("/{}", components.join("/"))
}

fn normalized_node_signature(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> String {
    let children = named_children(node, nodes).collect::<Vec<_>>();
    if children.is_empty() {
        return format!("{}:{:?}", node.kind, node.source_fragment.trim());
    }
    format!(
        "{}:[{}]",
        node.kind,
        children
            .iter()
            .map(|child| normalized_node_signature(child, nodes))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn syntax_layout_owners(scopes: &[TomlSyntaxScope]) -> Vec<LayoutOwner> {
    let mut owners = Vec::new();
    for scope in scopes {
        if scope.kind != TomlSyntaxScopeKind::Root {
            owners.push(LayoutOwner {
                owner_id: scope.node_id.clone(),
                start_line: scope.start_line,
                end_line: scope.start_line,
            });
        }
        owners.extend(scope.entries.iter().map(|entry| LayoutOwner {
            owner_id: entry.node_id.clone(),
            start_line: entry.start_line,
            end_line: entry.end_line,
        }));
    }
    owners.sort_by_key(|owner| (owner.start_line, owner.end_line, owner.owner_id.clone()));
    owners
}

fn apply_owned_starts(scopes: &mut [TomlSyntaxScope], augmentation: &CommentAugmentation) {
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

    for scope in scopes {
        if scope.kind != TomlSyntaxScopeKind::Root {
            scope.owned_start_line =
                attachments.get(scope.node_id.as_str()).map_or(scope.start_line, |attachment| {
                    attachment.owned_start_line(scope.start_line, &regions, &gaps)
                });
        }
        for entry in &mut scope.entries {
            entry.owned_start_line =
                attachments.get(entry.node_id.as_str()).map_or(entry.start_line, |attachment| {
                    attachment.owned_start_line(entry.start_line, &regions, &gaps)
                });
        }
    }
}

fn validate_scopes(scopes: &[TomlSyntaxScope]) -> Result<(), String> {
    let mut scope_kinds = HashMap::<&str, TomlSyntaxScopeKind>::new();
    let mut table_array_counts = HashMap::<&str, usize>::new();
    let mut entries = HashSet::<String>::new();

    for scope in scopes {
        if scope.kind != TomlSyntaxScopeKind::Root {
            if let Some(existing) = scope_kinds.insert(&scope.path, scope.kind) {
                if existing != TomlSyntaxScopeKind::TableArray
                    || scope.kind != TomlSyntaxScopeKind::TableArray
                {
                    return Err(format!("Duplicate or conflicting TOML table {}.", scope.path));
                }
            }
        }
        if scope.kind == TomlSyntaxScopeKind::TableArray {
            *table_array_counts.entry(&scope.path).or_default() += 1;
        }
        for entry in &scope.entries {
            let identity = format!("{}:{}", scope.identity(), entry.key);
            if !entries.insert(identity) {
                return Err(format!("Duplicate TOML key {}.", entry.path));
            }
        }
    }
    if let Some((path, _)) = table_array_counts.iter().find(|(_, count)| **count > 1) {
        return Err(format!(
            "TOML array of tables {path} has multiple elements; identity matching is not yet supported."
        ));
    }
    Ok(())
}

fn validate_projection(source: &str, scopes: &[TomlSyntaxScope]) -> Result<(), String> {
    if scopes.len()
        != 1 + scopes.iter().filter(|scope| scope.kind != TomlSyntaxScopeKind::Root).count()
        || scopes.first().is_none_or(|scope| scope.kind != TomlSyntaxScopeKind::Root)
        || scopes.iter().filter(|scope| scope.kind == TomlSyntaxScopeKind::Root).count() != 1
    {
        return Err(
            "TOML parser projection must contain exactly one leading root scope.".to_string()
        );
    }
    let line_count = source_lines(source).len();
    for scope in scopes {
        validate_projected_range(
            source,
            &scope.node_id,
            scope.start_byte,
            scope.end_byte,
            scope.start_line,
            scope.end_line,
            line_count,
        )?;
        if scope.kind != TomlSyntaxScopeKind::Root && scope.path.is_empty() {
            return Err(format!("TOML scope {} has an empty path.", scope.node_id));
        }
        for entry in &scope.entries {
            validate_projected_range(
                source,
                &entry.node_id,
                entry.start_byte,
                entry.end_byte,
                entry.start_line,
                entry.end_line,
                line_count,
            )?;
            if entry.key == "/" || entry.value_signature.is_empty() {
                return Err(format!("TOML entry {} has an incomplete projection.", entry.node_id));
            }
            if entry.start_byte < scope.start_byte || entry.end_byte > scope.end_byte {
                return Err(format!(
                    "TOML entry {} falls outside projected scope {}.",
                    entry.node_id, scope.node_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_projected_range(
    source: &str,
    node_id: &str,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    line_count: usize,
) -> Result<(), String> {
    if node_id.is_empty()
        || start_byte > end_byte
        || end_byte > source.len()
        || !source.is_char_boundary(start_byte)
        || !source.is_char_boundary(end_byte)
        || start_line == 0
        || end_line < start_line
        || end_line > line_count
    {
        return Err(format!("TOML parser projection has an invalid range for {node_id:?}."));
    }
    Ok(())
}

fn collect_owners(scopes: &[TomlSyntaxScope]) -> Vec<TomlOwner> {
    let mut owners = BTreeMap::<String, TomlOwner>::new();
    for scope in scopes {
        if scope.kind != TomlSyntaxScopeKind::Root {
            let components = scope.path.trim_start_matches('/').split('/').collect::<Vec<_>>();
            for index in 0..components.len() {
                let path = format!("/{}", components[..=index].join("/"));
                let owner_kind = if index + 1 == components.len()
                    && scope.kind == TomlSyntaxScopeKind::TableArray
                {
                    TomlOwnerKind::TableArray
                } else {
                    TomlOwnerKind::Table
                };
                owners.entry(path.clone()).or_insert(TomlOwner {
                    path,
                    owner_kind,
                    match_key: Some(components[index].to_string()),
                });
            }
        }
        for entry in &scope.entries {
            owners.entry(entry.path.clone()).or_insert(TomlOwner {
                path: entry.path.clone(),
                owner_kind: TomlOwnerKind::KeyValue,
                match_key: Some(
                    entry.key.trim_start_matches('/').rsplit('/').next().unwrap().to_string(),
                ),
            });
            for index in 0..entry.array_item_count {
                let path = format!("{}/{index}", entry.path);
                owners.insert(
                    path.clone(),
                    TomlOwner { path, owner_kind: TomlOwnerKind::ArrayItem, match_key: None },
                );
            }
        }
    }
    owners.into_values().collect()
}

fn collect_semantic(scopes: &[TomlSyntaxScope]) -> BTreeMap<String, String> {
    let mut semantic = BTreeMap::new();
    for scope in scopes {
        semantic.insert(format!("scope:{}", scope.identity()), format!("{:?}", scope.kind));
        for entry in &scope.entries {
            semantic.insert(
                format!("entry:{}:{}", scope.identity(), entry.key),
                entry.value_signature.clone(),
            );
        }
    }
    semantic
}

fn validate_merge_scope_compatibility(
    template: &TomlSyntaxDocument,
    destination: &TomlSyntaxDocument,
) -> Result<(), String> {
    let template_by_path = template
        .scopes
        .iter()
        .filter(|scope| scope.kind != TomlSyntaxScopeKind::Root)
        .map(|scope| (&scope.path, scope.kind))
        .collect::<HashMap<_, _>>();
    for scope in destination.scopes.iter().filter(|scope| scope.kind != TomlSyntaxScopeKind::Root) {
        if template_by_path.get(&scope.path).is_some_and(|kind| *kind != scope.kind) {
            return Err(format!(
                "TOML scope {} changes between table and array-of-tables.",
                scope.path
            ));
        }
    }
    Ok(())
}

fn plan_entry_insertions(
    template: &TomlSyntaxDocument,
    destination: &TomlSyntaxDocument,
    template_scope: &TomlSyntaxScope,
    destination_scope: &TomlSyntaxScope,
    insertions: &mut BTreeMap<usize, Vec<String>>,
) -> Result<(), String> {
    let destination_entries = destination_scope
        .entries
        .iter()
        .map(|entry| (entry.key.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let missing = template_scope
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !destination_entries.contains_key(entry.key.as_str()))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let has_match = template_scope
        .entries
        .iter()
        .any(|entry| destination_entries.contains_key(entry.key.as_str()));
    for (index, entry) in missing {
        let insertion_byte = if destination_scope.entries.is_empty() {
            empty_scope_insertion_byte(destination, destination_scope)
        } else if has_match {
            let next_match = template_scope.entries[(index + 1)..]
                .iter()
                .find_map(|candidate| destination_entries.get(candidate.key.as_str()).copied());
            if let Some(next) = next_match {
                line_start_byte(&destination.source, next.owned_start_line)
            } else {
                let previous_match = template_scope.entries[..index]
                    .iter()
                    .rev()
                    .find_map(|candidate| destination_entries.get(candidate.key.as_str()).copied());
                previous_match.map_or_else(
                    || {
                        line_start_byte(
                            &destination.source,
                            destination_scope.entries[0].owned_start_line,
                        )
                    },
                    |previous| line_after_byte(&destination.source, previous.end_line),
                )
            }
        } else {
            destination_scope
                .entries
                .iter()
                .find(|candidate| candidate.key > entry.key)
                .map_or_else(
                    || scope_entries_end_byte(destination, destination_scope),
                    |candidate| line_start_byte(&destination.source, candidate.owned_start_line),
                )
        };
        let mut fragment = ensure_line_terminated(line_fragment(
            &template.source,
            entry.owned_start_line,
            entry.end_line,
        ));
        if template_scope.kind == TomlSyntaxScopeKind::Root
            && template_scope.entries[(index + 1)..].is_empty()
            && template
                .scopes
                .get(1)
                .is_some_and(|next_scope| next_scope.start_line > entry.end_line + 1)
            && !fragment.ends_with("\n\n")
        {
            fragment.push('\n');
        }
        insertions.entry(insertion_byte).or_default().push(fragment);
    }
    Ok(())
}

fn plan_scope_insertions(
    template: &TomlSyntaxDocument,
    destination: &TomlSyntaxDocument,
    destination_scopes: &HashMap<String, &TomlSyntaxScope>,
    insertions: &mut BTreeMap<usize, Vec<String>>,
) -> Result<(), String> {
    let template_scopes = template
        .scopes
        .iter()
        .filter(|scope| scope.kind != TomlSyntaxScopeKind::Root)
        .collect::<Vec<_>>();
    let missing = template_scopes
        .iter()
        .enumerate()
        .filter(|(_, scope)| !destination_scopes.contains_key(&scope.identity()))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    let has_match =
        template_scopes.iter().any(|scope| destination_scopes.contains_key(&scope.identity()));

    for (index, scope) in missing {
        let insertion_byte = if has_match {
            let next_match = template_scopes[(index + 1)..]
                .iter()
                .find_map(|candidate| destination_scopes.get(&candidate.identity()).copied());
            if let Some(next) = next_match {
                line_start_byte(&destination.source, next.owned_start_line)
            } else {
                let previous_match = template_scopes[..index]
                    .iter()
                    .rev()
                    .find_map(|candidate| destination_scopes.get(&candidate.identity()).copied());
                previous_match.map_or(destination.source.len(), |previous| previous.end_byte)
            }
        } else {
            destination.source.len()
        };
        let start = line_start_byte(&template.source, scope.owned_start_line);
        let fragment = template.source[start..scope.end_byte].to_string();
        let fragment = section_fragment(&destination.source, insertion_byte, fragment);
        insertions.entry(insertion_byte).or_default().push(fragment);
    }
    Ok(())
}

fn empty_scope_insertion_byte(document: &TomlSyntaxDocument, scope: &TomlSyntaxScope) -> usize {
    if scope.kind == TomlSyntaxScopeKind::Root {
        return document
            .scopes
            .iter()
            .find(|candidate| candidate.kind != TomlSyntaxScopeKind::Root)
            .map_or(document.source.len(), |candidate| {
                line_start_byte(&document.source, candidate.owned_start_line)
            });
    }
    line_after_byte(&document.source, scope.start_line)
}

fn scope_entries_end_byte(document: &TomlSyntaxDocument, scope: &TomlSyntaxScope) -> usize {
    scope.entries.last().map_or_else(
        || empty_scope_insertion_byte(document, scope),
        |entry| line_after_byte(&document.source, entry.end_line),
    )
}

fn line_start_byte(source: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    source.match_indices('\n').nth(line - 2).map_or(source.len(), |(byte, _)| byte + 1)
}

fn source_lines(source: &str) -> Vec<String> {
    let mut lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
    if source.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn line_after_byte(source: &str, line: usize) -> usize {
    let start = line_start_byte(source, line);
    source[start..].find('\n').map_or(source.len(), |offset| start + offset + 1)
}

fn line_fragment(source: &str, start_line: usize, end_line: usize) -> String {
    source[line_start_byte(source, start_line)..line_after_byte(source, end_line)].to_string()
}

fn ensure_line_terminated(mut fragment: String) -> String {
    if !fragment.ends_with('\n') {
        fragment.push('\n');
    }
    fragment
}

fn section_fragment(destination: &str, insertion_byte: usize, mut fragment: String) -> String {
    if insertion_byte > 0
        && !destination[..insertion_byte].ends_with("\n\n")
        && !fragment.starts_with('\n')
    {
        fragment.insert(0, '\n');
    }
    ensure_line_terminated(fragment)
}

fn node_start_line(node: &NormalizedTreeNode) -> usize {
    node.span.start_point.row + 1
}

fn node_end_line(node: &NormalizedTreeNode) -> usize {
    if node.span.end_point.column == 0 && node.span.end_point.row > node.span.start_point.row {
        node.span.end_point.row
    } else {
        node.span.end_point.row + 1
    }
}

fn normalize_toml_comment(text: &str) -> String {
    text.trim().strip_prefix('#').unwrap_or(text.trim()).trim().to_string()
}
