use std::collections::{BTreeMap, HashMap};

use ast_merge::{
    CommentAugmentation, ConflictAlternative, ConflictAlternativeState, Diagnostic,
    DiagnosticCategory, DiagnosticSeverity, LayoutOwner, MergeConflict, MergeResult, NodeIdentity,
    NodeSignature, OwnedSourceRegion, SequenceConflictCategory, SequenceMergeAnalysis,
    SequenceNode, SourceEdit, SourceRevision, ThreeWayMergeOutcome, ThreeWayMergeResult,
    analyze_three_way_sequence, apply_source_edits, augment_normalized_comments_with_owners,
};
use tree_haver::{
    ByteRange, NormalizedTreeNode, ParserRequest, parse_normalized_with_language_pack,
};

use crate::{JsonAnalysis, JsonDialect, JsonOwner, JsonOwnerKind, JsonRootKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum JsonSemanticValue {
    Object(BTreeMap<String, JsonSemanticValue>),
    Array(Vec<JsonSemanticValue>),
    Scalar { kind: String, value: String },
}

#[derive(Clone, Debug)]
pub(crate) struct JsonSyntaxMember {
    pub key: String,
    pub pair_range: ByteRange,
    pub owned_region: OwnedSourceRegion,
    pub pair_source: String,
    pub value: JsonSyntaxValue,
}

#[derive(Clone, Debug)]
pub(crate) struct JsonSyntaxValue {
    pub node_id: String,
    pub range: ByteRange,
    pub owned_region: OwnedSourceRegion,
    pub source: String,
    pub semantic: JsonSemanticValue,
    pub members: Vec<JsonSyntaxMember>,
    pub elements: Vec<JsonSyntaxValue>,
    pub closing_byte: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct JsonSyntaxDocument {
    pub source: String,
    pub root: JsonSyntaxValue,
    pub comment_augmentation: CommentAugmentation,
}

pub(crate) fn parser_language(dialect: JsonDialect) -> &'static str {
    match dialect {
        JsonDialect::Json => "json",
        JsonDialect::Jsonc | JsonDialect::Json5 => "json5",
    }
}

pub(crate) fn parse_document(
    source: &str,
    dialect: JsonDialect,
) -> Result<JsonSyntaxDocument, String> {
    let language = parser_language(dialect);
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: language.to_string(),
        dialect: Some(
            match dialect {
                JsonDialect::Json => "json",
                JsonDialect::Jsonc => "jsonc",
                JsonDialect::Json5 => "json5",
            }
            .to_string(),
        ),
    });
    if !parsed.ok {
        return Err(parsed
            .diagnostics
            .first()
            .cloned()
            .unwrap_or_else(|| format!("TreeHaver could not parse {language} source.")));
    }
    document_from_nodes(source, dialect, &parsed.root_id, &parsed.nodes)
}

pub(crate) fn document_from_nodes(
    source: &str,
    dialect: JsonDialect,
    root_id: &str,
    parsed_nodes: &[NormalizedTreeNode],
) -> Result<JsonSyntaxDocument, String> {
    if dialect == JsonDialect::Json && parsed_nodes.iter().any(|node| node.kind == "comment") {
        return Err("Comments are not supported in strict JSON.".to_string());
    }
    if dialect == JsonDialect::Jsonc {
        validate_jsonc_nodes(parsed_nodes)?;
    }

    let nodes = parsed_nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let root = nodes
        .get(root_id)
        .copied()
        .ok_or_else(|| "TreeHaver normalized parse omitted its root node.".to_string())?;
    let value_node = find_root_value(root, &nodes).ok_or_else(|| {
        "TreeHaver normalized parse did not contain a JSON root value.".to_string()
    })?;
    let root = build_value(value_node, &nodes)?;
    let comment_augmentation = json_comment_augmentation(source, &root, parsed_nodes)?;
    Ok(JsonSyntaxDocument { source: source.to_string(), root, comment_augmentation })
}

fn validate_jsonc_nodes(nodes: &[NormalizedTreeNode]) -> Result<(), String> {
    for node in nodes {
        let reason = match node.kind.as_str() {
            "identifier" => Some("unquoted object keys are JSON5-only"),
            "string" if serde_json::from_str::<String>(&node.source_fragment).is_err() => {
                Some("single-quoted strings and JSON5 escapes are not supported")
            }
            "number"
                if serde_json::from_str::<serde_json::Value>(&node.source_fragment).is_err() =>
            {
                Some("JSON5 numeric literals are not supported")
            }
            _ => None,
        };
        if let Some(reason) = reason {
            return Err(format!(
                "JSONC rejects {reason} at line {}, column {}.",
                node.span.start_point.row + 1,
                node.span.start_point.column + 1
            ));
        }
    }
    Ok(())
}

pub(crate) fn analyze_document(source: &str, dialect: JsonDialect) -> Result<JsonAnalysis, String> {
    let document = parse_document(source, dialect)?;
    Ok(analyze_syntax(document, dialect))
}

pub(crate) fn analyze_syntax(document: JsonSyntaxDocument, dialect: JsonDialect) -> JsonAnalysis {
    let root_kind = match document.root.semantic {
        JsonSemanticValue::Object(_) => JsonRootKind::Object,
        JsonSemanticValue::Array(_) => JsonRootKind::Array,
        JsonSemanticValue::Scalar { .. } => JsonRootKind::Scalar,
    };
    let mut owners = Vec::new();
    collect_owners(&document.root, "", &mut owners);
    owners.sort_by(|left, right| left.path.cmp(&right.path));

    JsonAnalysis {
        dialect,
        allows_comments: matches!(dialect, JsonDialect::Jsonc | JsonDialect::Json5),
        normalized_source: document.source,
        root_kind,
        owners,
        comment_regions: document.comment_augmentation.regions,
        layout_gaps: document.comment_augmentation.gaps,
        comment_attachments: document.comment_augmentation.attachments,
    }
}

fn json_comment_augmentation(
    source: &str,
    root: &JsonSyntaxValue,
    nodes: &[NormalizedTreeNode],
) -> Result<CommentAugmentation, String> {
    let owners = if root.members.is_empty() {
        root.elements
            .iter()
            .map(|element| layout_owner(&element.node_id, &element.owned_region))
            .collect::<Vec<_>>()
    } else {
        root.members
            .iter()
            .map(|member| layout_owner(&member.value.node_id, &member.owned_region))
            .collect::<Vec<_>>()
    };
    let owners = if owners.windows(2).any(|pair| pair[1].start_line <= pair[0].end_line) {
        vec![layout_owner(&root.node_id, &root.owned_region)]
    } else {
        owners
    };
    augment_normalized_comments_with_owners(source, &owners, nodes, "slash_comment", |text| {
        normalize_json_comment(text)
    })
}

fn layout_owner(node_id: &str, region: &OwnedSourceRegion) -> LayoutOwner {
    LayoutOwner {
        owner_id: node_id.to_string(),
        start_line: region.start_line,
        end_line: region.end_line,
    }
}

fn normalize_json_comment(text: &str) -> String {
    text.trim()
        .strip_prefix("//")
        .or_else(|| text.trim().strip_prefix("/*"))
        .unwrap_or_else(|| text.trim().trim_start_matches('*'))
        .trim_end_matches("*/")
        .trim()
        .to_string()
}

pub fn json_semantically_equivalent(
    left_source: &str,
    right_source: &str,
    dialect: JsonDialect,
) -> Result<bool, String> {
    let left = parse_document(left_source, dialect)?;
    let right = parse_document(right_source, dialect)?;
    Ok(left.root.semantic == right.root.semantic)
}

fn collect_owners(value: &JsonSyntaxValue, path: &str, owners: &mut Vec<JsonOwner>) {
    for member in &value.members {
        let child_path = join_path(path, &member.key);
        owners.push(JsonOwner {
            path: child_path.clone(),
            owner_kind: JsonOwnerKind::Member,
            match_key: Some(member.key.clone()),
            source_fragment: Some(member.pair_source.clone()),
        });
        collect_owners(&member.value, &child_path, owners);
    }
    for (index, element) in value.elements.iter().enumerate() {
        let child_path = format!("{path}/{index}");
        owners.push(JsonOwner {
            path: child_path.clone(),
            owner_kind: JsonOwnerKind::Element,
            match_key: None,
            source_fragment: Some(element.source.clone()),
        });
        collect_owners(element, &child_path, owners);
    }
}

fn find_root_value<'a>(
    node: &'a NormalizedTreeNode,
    nodes: &HashMap<&str, &'a NormalizedTreeNode>,
) -> Option<&'a NormalizedTreeNode> {
    if json_value_kind(&node.kind) {
        return Some(node);
    }
    node.child_ids
        .iter()
        .filter_map(|id| nodes.get(id.as_str()).copied())
        .find_map(|child| find_root_value(child, nodes))
}

fn json_value_kind(kind: &str) -> bool {
    matches!(
        kind,
        "object"
            | "array"
            | "string"
            | "number"
            | "true"
            | "false"
            | "null"
            | "identifier"
            | "unary_expression"
    )
}

fn build_value(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
) -> Result<JsonSyntaxValue, String> {
    let range = node.span.range.clone();
    let owned_region = owned_region(node);
    match node.kind.as_str() {
        "object" => build_object(node, nodes, range),
        "array" => {
            let elements = semantic_children(node, nodes)
                .filter(|child| json_value_kind(&child.kind))
                .map(|child| build_value(child, nodes))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(JsonSyntaxValue {
                node_id: node.id.clone(),
                range,
                owned_region,
                source: node.source_fragment.clone(),
                semantic: JsonSemanticValue::Array(
                    elements.iter().map(|element| element.semantic.clone()).collect(),
                ),
                members: vec![],
                elements,
                closing_byte: closing_delimiter_byte(node, nodes, "]"),
            })
        }
        _ => Ok(JsonSyntaxValue {
            node_id: node.id.clone(),
            range,
            owned_region,
            source: node.source_fragment.clone(),
            semantic: JsonSemanticValue::Scalar {
                kind: node.kind.clone(),
                value: normalized_scalar(node),
            },
            members: vec![],
            elements: vec![],
            closing_byte: None,
        }),
    }
}

fn build_object(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
    range: ByteRange,
) -> Result<JsonSyntaxValue, String> {
    let pair_nodes = semantic_children(node, nodes)
        .filter(|child| matches!(child.kind.as_str(), "pair" | "member"));
    let mut members = Vec::new();
    let mut semantic = BTreeMap::new();
    for pair in pair_nodes {
        let children = semantic_children(pair, nodes).collect::<Vec<_>>();
        let key_node = children
            .iter()
            .copied()
            .find(|child| child.field_name.as_deref() == Some("key"))
            .or_else(|| children.first().copied())
            .ok_or_else(|| format!("JSON {} node omitted its key.", pair.kind))?;
        let value_node = children
            .iter()
            .copied()
            .find(|child| child.field_name.as_deref() == Some("value"))
            .or_else(|| children.iter().copied().find(|child| child.id != key_node.id))
            .ok_or_else(|| format!("JSON {} node omitted its value.", pair.kind))?;
        let key = normalized_key(&key_node.source_fragment);
        let value = build_value(value_node, nodes)?;
        semantic.insert(key.clone(), value.semantic.clone());
        members.push(JsonSyntaxMember {
            key,
            pair_range: pair.span.range.clone(),
            owned_region: owned_region(pair),
            pair_source: pair.source_fragment.clone(),
            value,
        });
    }

    Ok(JsonSyntaxValue {
        node_id: node.id.clone(),
        range,
        owned_region: owned_region(node),
        source: node.source_fragment.clone(),
        semantic: JsonSemanticValue::Object(semantic),
        members,
        elements: vec![],
        closing_byte: closing_delimiter_byte(node, nodes, "}"),
    })
}

fn owned_region(node: &NormalizedTreeNode) -> OwnedSourceRegion {
    OwnedSourceRegion {
        node_id: node.id.clone(),
        region_kind: "node".to_string(),
        start_byte: node.span.range.start_byte,
        end_byte: node.span.range.end_byte,
        start_line: node.span.start_point.row + 1,
        end_line: node.span.end_point.row + 1,
    }
}

fn semantic_children<'a>(
    node: &'a NormalizedTreeNode,
    nodes: &'a HashMap<&str, &'a NormalizedTreeNode>,
) -> impl Iterator<Item = &'a NormalizedTreeNode> + 'a {
    node.child_ids
        .iter()
        .filter_map(|id| nodes.get(id.as_str()).copied())
        .filter(|child| child.named && child.kind != "comment")
}

fn closing_delimiter_byte(
    node: &NormalizedTreeNode,
    nodes: &HashMap<&str, &NormalizedTreeNode>,
    delimiter: &str,
) -> Option<usize> {
    node.child_ids
        .iter()
        .rev()
        .filter_map(|id| nodes.get(id.as_str()).copied())
        .find(|child| child.kind == delimiter)
        .map(|child| child.span.range.start_byte)
}

fn normalized_key(source: &str) -> String {
    let source = source.trim();
    if source.starts_with('"') {
        return serde_json::from_str::<String>(source).unwrap_or_else(|_| source.to_string());
    }
    if source.starts_with('\'') && source.ends_with('\'') && source.len() >= 2 {
        return source[1..source.len() - 1].replace("\\'", "'").replace("\\\\", "\\");
    }
    source.to_string()
}

fn normalized_scalar(node: &NormalizedTreeNode) -> String {
    let source = node.source_fragment.trim();
    if node.kind == "string" && source.starts_with('"') {
        return serde_json::from_str::<String>(source).unwrap_or_else(|_| source.to_string());
    }
    if node.kind == "string"
        && source.starts_with('\'')
        && source.ends_with('\'')
        && source.len() >= 2
    {
        return source[1..source.len() - 1].replace("\\'", "'").replace("\\\\", "\\");
    }
    source.to_string()
}

fn object_identity(value: &JsonSyntaxValue) -> Option<String> {
    value.members.iter().find(|member| member.key == "id").and_then(|member| {
        match &member.value.semantic {
            JsonSemanticValue::Scalar { value, .. } => Some(value.clone()),
            _ => None,
        }
    })
}

#[derive(Clone, Debug)]
struct ObjectAdditions {
    node_id: String,
    object_range: ByteRange,
    closing_byte: usize,
    first_member_byte: Option<usize>,
    last_member_end: Option<usize>,
    existing_members: Vec<(usize, String)>,
    additions: Vec<ObjectAddition>,
}

#[derive(Clone, Debug)]
struct ObjectAddition {
    before_byte: Option<usize>,
    pair_source: String,
}

#[derive(Default)]
struct MergePlan {
    edits: Vec<SourceEdit>,
    additions: BTreeMap<String, ObjectAdditions>,
    conflicts: Vec<MergeConflict>,
}

impl MergePlan {
    fn conflict(&mut self, path: &str, category: &str, message: impl Into<String>) {
        self.conflict_with_alternatives(path, category, message, vec![]);
    }

    fn conflict_with_regions(
        &mut self,
        path: &str,
        category: &str,
        message: impl Into<String>,
        regions: [&OwnedSourceRegion; 3],
    ) {
        self.conflict_with_alternatives(
            path,
            category,
            message,
            [SourceRevision::Base, SourceRevision::Ours, SourceRevision::Theirs]
                .into_iter()
                .zip(regions)
                .map(|(revision, region)| present_alternative(revision, region))
                .collect(),
        );
    }

    fn conflict_with_alternatives(
        &mut self,
        path: &str,
        category: &str,
        message: impl Into<String>,
        alternatives: Vec<ConflictAlternative>,
    ) {
        let message = message.into();
        self.conflicts.push(MergeConflict {
            conflict_id: format!("json:{}:{}", category, self.conflicts.len() + 1),
            category: category.to_string(),
            path: path.to_string(),
            fallback_scope: path.to_string(),
            message,
            alternatives,
        });
    }

    fn add_member(
        &mut self,
        target: &JsonSyntaxValue,
        donor: &JsonSyntaxMember,
        path: &str,
        before_byte: Option<usize>,
    ) {
        let Some(closing_byte) = target.closing_byte else {
            self.conflict(path, "missing_source_span", "target object has no closing delimiter");
            return;
        };
        self.additions
            .entry(target.node_id.clone())
            .or_insert_with(|| ObjectAdditions {
                node_id: target.node_id.clone(),
                object_range: target.range.clone(),
                closing_byte,
                first_member_byte: target
                    .members
                    .first()
                    .map(|member| member.pair_range.start_byte),
                last_member_end: target.members.last().map(|member| member.pair_range.end_byte),
                existing_members: target
                    .members
                    .iter()
                    .map(|member| (member.pair_range.start_byte, member.pair_source.clone()))
                    .collect(),
                additions: vec![],
            })
            .additions
            .push(ObjectAddition { before_byte, pair_source: donor.pair_source.clone() });
    }
}

fn present_alternative(
    revision: SourceRevision,
    region: &OwnedSourceRegion,
) -> ConflictAlternative {
    ConflictAlternative {
        revision,
        state: ConflictAlternativeState::Present,
        regions: vec![region.clone()],
    }
}

fn absent_alternative(revision: SourceRevision) -> ConflictAlternative {
    ConflictAlternative { revision, state: ConflictAlternativeState::Absent, regions: vec![] }
}

pub fn merge_json_source_preserving(
    template_source: &str,
    destination_source: &str,
    dialect: JsonDialect,
) -> MergeResult<String> {
    let template = match parse_document(template_source, dialect) {
        Ok(document) => document,
        Err(message) => return two_way_parse_failure(message, false),
    };
    let destination = match parse_document(destination_source, dialect) {
        Ok(document) => document,
        Err(message) => return two_way_parse_failure(message, true),
    };
    merge_documents_two_way(template, destination, |output, _, _| parse_document(output, dialect))
}

pub(crate) fn merge_documents_two_way(
    template: JsonSyntaxDocument,
    destination: JsonSyntaxDocument,
    mut verify: impl FnMut(
        &str,
        tree_haver::source::SourceRole,
        &[SourceEdit],
    ) -> Result<JsonSyntaxDocument, String>,
) -> MergeResult<String> {
    let mut plan = MergePlan::default();
    let expected = merge_template_value(&template.root, &destination.root, "", &mut plan);
    if !plan.conflicts.is_empty() {
        return MergeResult {
            ok: false,
            diagnostics: conflict_diagnostics(&plan.conflicts),
            output: None,
            policies: vec![],
        };
    }

    match render_plan(&destination.source, plan) {
        Ok((output, edits)) => {
            match verify(&output, tree_haver::source::SourceRole::Current, &edits) {
                Ok(rendered) if rendered.root.semantic == expected => MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some(output),
                    policies: vec![],
                },
                Ok(_) => render_failure("source-preserving JSON render changed the planned value"),
                Err(message) => {
                    render_failure(format!("source-preserving JSON render is invalid: {message}"))
                }
            }
        }
        Err(message) => render_failure(message),
    }
}

pub fn merge_json_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    dialect: JsonDialect,
) -> ThreeWayMergeResult<String> {
    let base = match parse_document(base_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_failure("base", message),
    };
    let ours = match parse_document(ours_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_failure("ours", message),
    };
    let theirs = match parse_document(theirs_source, dialect) {
        Ok(document) => document,
        Err(message) => return three_way_parse_failure("theirs", message),
    };
    merge_documents_three_way(base, ours, theirs, |output, _, _| parse_document(output, dialect))
}

pub(crate) fn merge_documents_three_way(
    base: JsonSyntaxDocument,
    ours: JsonSyntaxDocument,
    theirs: JsonSyntaxDocument,
    mut verify: impl FnMut(
        &str,
        tree_haver::source::SourceRole,
        &[SourceEdit],
    ) -> Result<JsonSyntaxDocument, String>,
) -> ThreeWayMergeResult<String> {
    // Even selected-input/no-op outcomes pass through output verification.
    let selected = if ours.root.semantic == theirs.root.semantic {
        Some((&ours, tree_haver::source::SourceRole::Ours))
    } else if ours.root.semantic == base.root.semantic {
        Some((&theirs, tree_haver::source::SourceRole::Theirs))
    } else if theirs.root.semantic == base.root.semantic {
        Some((&ours, tree_haver::source::SourceRole::Ours))
    } else {
        None
    };
    if let Some((selected, role)) = selected {
        return match verify(&selected.source, role, &[]) {
            Ok(rendered) if rendered.root.semantic == selected.root.semantic => {
                clean_three_way(selected.source.clone())
            }
            Ok(_) => three_way_render_failure("selected JSON output changed during verification"),
            Err(message) => three_way_render_failure(message),
        };
    }

    let mut plan = MergePlan::default();
    let expected = merge_three_values(
        &base.root,
        &ours.root,
        &theirs.root,
        "",
        [&base.root.owned_region, &ours.root.owned_region, &theirs.root.owned_region],
        &mut plan,
    );
    if !plan.conflicts.is_empty() {
        return conflicted_three_way(plan.conflicts);
    }
    let (output, edits) = match render_plan(&ours.source, plan) {
        Ok(output) => output,
        Err(message) => return three_way_render_failure(message),
    };
    match verify(&output, tree_haver::source::SourceRole::Ours, &edits) {
        Ok(rendered) if rendered.root.semantic == expected => clean_three_way(output),
        Ok(_) => {
            three_way_render_failure("source-preserving JSON render changed the planned value")
        }
        Err(message) => {
            three_way_render_failure(format!("source-preserving JSON render is invalid: {message}"))
        }
    }
}

fn merge_template_value(
    template: &JsonSyntaxValue,
    destination: &JsonSyntaxValue,
    path: &str,
    plan: &mut MergePlan,
) -> JsonSemanticValue {
    let (
        JsonSemanticValue::Object(_template_values),
        JsonSemanticValue::Object(destination_values),
    ) = (&template.semantic, &destination.semantic)
    else {
        return destination.semantic.clone();
    };
    let mut merged = destination_values.clone();
    for (index, template_member) in template.members.iter().enumerate() {
        let child_path = join_path(path, &template_member.key);
        if let Some(destination_member) = member(&destination.members, &template_member.key) {
            merged.insert(
                template_member.key.clone(),
                merge_template_value(
                    &template_member.value,
                    &destination_member.value,
                    &child_path,
                    plan,
                ),
            );
        } else {
            let before_byte = template.members.iter().skip(index + 1).find_map(|following| {
                member(&destination.members, &following.key)
                    .map(|member| member.pair_range.start_byte)
            });
            plan.add_member(destination, template_member, &child_path, before_byte);
            merged.insert(template_member.key.clone(), template_member.value.semantic.clone());
        }
    }
    JsonSemanticValue::Object(merged)
}

fn merge_three_values(
    base: &JsonSyntaxValue,
    ours: &JsonSyntaxValue,
    theirs: &JsonSyntaxValue,
    path: &str,
    owned_regions: [&OwnedSourceRegion; 3],
    plan: &mut MergePlan,
) -> JsonSemanticValue {
    if ours.semantic == theirs.semantic {
        return ours.semantic.clone();
    }
    if ours.semantic == base.semantic {
        plan.edits.push(SourceEdit::replace(
            ours.range.start_byte,
            ours.range.end_byte,
            theirs.source.clone(),
        ));
        return theirs.semantic.clone();
    }
    if theirs.semantic == base.semantic {
        return ours.semantic.clone();
    }

    if matches!(base.semantic, JsonSemanticValue::Array(_))
        && matches!(ours.semantic, JsonSemanticValue::Array(_))
        && matches!(theirs.semantic, JsonSemanticValue::Array(_))
    {
        merge_three_arrays(base, ours, theirs, path, owned_regions, plan);
        return ours.semantic.clone();
    }

    let (
        JsonSemanticValue::Object(base_values),
        JsonSemanticValue::Object(ours_values),
        JsonSemanticValue::Object(theirs_values),
    ) = (&base.semantic, &ours.semantic, &theirs.semantic)
    else {
        plan.conflict_with_regions(
            path,
            "modify_modify",
            "both sides changed the same JSON value",
            owned_regions,
        );
        return ours.semantic.clone();
    };

    let mut keys = ours.members.iter().map(|member| member.key.clone()).collect::<Vec<_>>();
    keys.extend(theirs.members.iter().map(|member| member.key.clone()));
    keys.sort();
    keys.dedup();
    let mut merged = BTreeMap::new();
    for key in keys {
        let child_path = join_path(path, &key);
        match (
            base_values.get(&key),
            ours_values.get(&key),
            theirs_values.get(&key),
            member(&base.members, &key),
            member(&ours.members, &key),
            member(&theirs.members, &key),
        ) {
            (None, Some(ours_value), None, _, _, _) => {
                merged.insert(key, ours_value.clone());
            }
            (None, None, Some(theirs_value), _, _, Some(theirs_member)) => {
                plan.add_member(ours, theirs_member, &child_path, None);
                merged.insert(key, theirs_value.clone());
            }
            (None, Some(ours_value), Some(theirs_value), _, _, _) if ours_value == theirs_value => {
                merged.insert(key, ours_value.clone());
            }
            (Some(_), None, None, _, _, _) => {}
            (Some(base_value), None, Some(theirs_value), _, _, _) if base_value == theirs_value => {
            }
            (
                Some(base_value),
                Some(ours_value),
                None,
                Some(base_member),
                Some(ours_member),
                None,
            ) if base_value == ours_value => {
                plan.conflict_with_alternatives(
                    &child_path,
                    "delete_requires_owner_edit",
                    "theirs deleted a JSON owner that is still present in ours",
                    vec![
                        present_alternative(SourceRevision::Base, &base_member.owned_region),
                        present_alternative(SourceRevision::Ours, &ours_member.owned_region),
                        absent_alternative(SourceRevision::Theirs),
                    ],
                );
            }
            (
                Some(_),
                Some(_),
                Some(_),
                Some(base_member),
                Some(ours_member),
                Some(theirs_member),
            ) => {
                merged.insert(
                    key,
                    merge_three_values(
                        &base_member.value,
                        &ours_member.value,
                        &theirs_member.value,
                        &child_path,
                        [
                            &base_member.owned_region,
                            &ours_member.owned_region,
                            &theirs_member.owned_region,
                        ],
                        plan,
                    ),
                );
            }
            (_, _, _, base_member, ours_member, theirs_member) => {
                plan.conflict_with_alternatives(
                    &child_path,
                    "ambiguous_owner_change",
                    "JSON owner addition, deletion, or modification is ambiguous",
                    vec![
                        alternative_for_member(SourceRevision::Base, base_member),
                        alternative_for_member(SourceRevision::Ours, ours_member),
                        alternative_for_member(SourceRevision::Theirs, theirs_member),
                    ],
                );
            }
        }
    }
    JsonSemanticValue::Object(merged)
}

fn merge_three_arrays(
    base: &JsonSyntaxValue,
    ours: &JsonSyntaxValue,
    theirs: &JsonSyntaxValue,
    path: &str,
    owned_regions: [&OwnedSourceRegion; 3],
    plan: &mut MergePlan,
) {
    let base_nodes = json_sequence_nodes(&base.elements);
    let ours_nodes = json_sequence_nodes(&ours.elements);
    let theirs_nodes = json_sequence_nodes(&theirs.elements);
    let analysis = analyze_three_way_sequence(&base_nodes, &ours_nodes, &theirs_nodes);

    for conflict in &analysis.conflicts {
        let conflict_path =
            conflict.base_index.map_or_else(|| path.to_string(), |index| format!("{path}/{index}"));
        plan.conflict_with_alternatives(
            &conflict_path,
            sequence_conflict_category(conflict.category),
            &conflict.message,
            sequence_conflict_alternatives(
                conflict.category,
                conflict.base_index,
                &analysis,
                base,
                ours,
                theirs,
                &conflict.ours_node_ids,
                &conflict.theirs_node_ids,
            ),
        );
    }

    if analysis.conflicts.is_empty() {
        plan.conflict_with_regions(
            path,
            "atomic_array_edit",
            "both sides changed an array governed by the atomic JSON array policy",
            owned_regions,
        );
    }
}

fn json_sequence_nodes(elements: &[JsonSyntaxValue]) -> Vec<SequenceNode> {
    elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let content = semantic_identity(&element.semantic);
            let primary = object_identity(element).map_or_else(
                || NodeSignature::content_identity(&content),
                |identity| Some(NodeSignature(vec!["json_object_id".to_string(), identity])),
            );
            SequenceNode {
                node_id: element.node_id.clone(),
                identity: NodeIdentity::new(primary)
                    .with_alias(NodeSignature::content_identity(&content))
                    .with_alias(Some(NodeSignature(vec![
                        "json_array_position".to_string(),
                        index.to_string(),
                    ]))),
                content_hash: content,
            }
        })
        .collect()
}

fn semantic_identity(value: &JsonSemanticValue) -> String {
    match value {
        JsonSemanticValue::Object(members) => {
            let body = members
                .iter()
                .map(|(key, value)| format!("{}:{key}:{}", key.len(), semantic_identity(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("object{{{body}}}")
        }
        JsonSemanticValue::Array(elements) => {
            let body = elements.iter().map(semantic_identity).collect::<Vec<_>>().join(",");
            format!("array[{body}]")
        }
        JsonSemanticValue::Scalar { kind, value } => {
            format!("scalar:{}:{kind}:{value}", value.len())
        }
    }
}

fn sequence_conflict_category(category: SequenceConflictCategory) -> &'static str {
    match category {
        SequenceConflictCategory::DeleteModify => "delete_modify",
        SequenceConflictCategory::ModifyModify => "modify_modify",
        SequenceConflictCategory::DuplicateInsertion => "duplicate_insertion",
        SequenceConflictCategory::Order => "order",
    }
}

#[allow(clippy::too_many_arguments)]
fn sequence_conflict_alternatives(
    category: SequenceConflictCategory,
    base_index: Option<usize>,
    analysis: &SequenceMergeAnalysis,
    base: &JsonSyntaxValue,
    ours: &JsonSyntaxValue,
    theirs: &JsonSyntaxValue,
    ours_node_ids: &[String],
    theirs_node_ids: &[String],
) -> Vec<ConflictAlternative> {
    if category == SequenceConflictCategory::Order {
        return vec![
            present_alternative(SourceRevision::Base, &base.owned_region),
            present_alternative(SourceRevision::Ours, &ours.owned_region),
            present_alternative(SourceRevision::Theirs, &theirs.owned_region),
        ];
    }

    if let Some(base_index) = base_index {
        let alignment = &analysis.base_alignments[base_index];
        return vec![
            present_alternative(
                SourceRevision::Base,
                &base.elements[alignment.base_index].owned_region,
            ),
            alternative_for_element(SourceRevision::Ours, &ours.elements, alignment.ours_index),
            alternative_for_element(
                SourceRevision::Theirs,
                &theirs.elements,
                alignment.theirs_index,
            ),
        ];
    }

    vec![
        absent_alternative(SourceRevision::Base),
        alternative_for_node_id(SourceRevision::Ours, &ours.elements, ours_node_ids.first()),
        alternative_for_node_id(SourceRevision::Theirs, &theirs.elements, theirs_node_ids.first()),
    ]
}

fn alternative_for_element(
    revision: SourceRevision,
    elements: &[JsonSyntaxValue],
    index: Option<usize>,
) -> ConflictAlternative {
    index.map_or_else(
        || absent_alternative(revision),
        |index| present_alternative(revision, &elements[index].owned_region),
    )
}

fn alternative_for_node_id(
    revision: SourceRevision,
    elements: &[JsonSyntaxValue],
    node_id: Option<&String>,
) -> ConflictAlternative {
    node_id
        .and_then(|node_id| elements.iter().find(|element| element.node_id == *node_id))
        .map_or_else(
            || absent_alternative(revision),
            |element| present_alternative(revision, &element.owned_region),
        )
}

fn alternative_for_member(
    revision: SourceRevision,
    member: Option<&JsonSyntaxMember>,
) -> ConflictAlternative {
    member.map_or_else(
        || absent_alternative(revision),
        |member| present_alternative(revision, &member.owned_region),
    )
}

fn render_plan(source: &str, mut plan: MergePlan) -> Result<(String, Vec<SourceEdit>), String> {
    for additions in plan.additions.into_values() {
        plan.edits.extend(addition_edits(source, additions)?);
    }
    let output = apply_source_edits(source, &plan.edits).map_err(|error| error.to_string())?;
    Ok((output, plan.edits))
}

fn addition_edits(source: &str, additions: ObjectAdditions) -> Result<Vec<SourceEdit>, String> {
    let newline = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let object_source = source
        .get(additions.object_range.start_byte..additions.object_range.end_byte)
        .ok_or_else(|| format!("invalid object span for {}", additions.node_id))?;
    if !object_source.contains('\n') {
        let edit = if source.contains('\n') {
            expanded_inline_object_edit(source, additions, newline)?
        } else {
            compact_inline_object_edit(additions)?
        };
        return Ok(vec![edit]);
    }

    let mut grouped = BTreeMap::<Option<usize>, Vec<String>>::new();
    for addition in &additions.additions {
        grouped.entry(addition.before_byte).or_default().push(addition.pair_source.clone());
    }
    let mut edits = Vec::new();
    for (before_byte, pair_sources) in grouped {
        if let Some(before_byte) = before_byte {
            let indent = line_indent(source, before_byte);
            let joined = pair_sources.join(&format!(",{newline}{indent}"));
            edits.push(SourceEdit::insert(before_byte, format!("{joined},{newline}{indent}")));
        } else {
            edits.push(appended_members_edit(source, &additions, pair_sources, newline)?);
        }
    }
    Ok(edits)
}

fn compact_inline_object_edit(additions: ObjectAdditions) -> Result<SourceEdit, String> {
    let mut additions_by_anchor = BTreeMap::<Option<usize>, Vec<String>>::new();
    for addition in additions.additions {
        additions_by_anchor.entry(addition.before_byte).or_default().push(addition.pair_source);
    }
    let mut members = Vec::new();
    for (byte, pair_source) in additions.existing_members {
        if let Some(prefix) = additions_by_anchor.remove(&Some(byte)) {
            members.extend(prefix);
        }
        members.push(pair_source);
    }
    if let Some(suffix) = additions_by_anchor.remove(&None) {
        members.extend(suffix);
    }
    if !additions_by_anchor.is_empty() {
        return Err(format!("missing insertion anchor for {}", additions.node_id));
    }
    Ok(SourceEdit::replace(
        additions.object_range.start_byte,
        additions.object_range.end_byte,
        format!("{{{}}}", members.join(",")),
    ))
}

fn expanded_inline_object_edit(
    source: &str,
    additions: ObjectAdditions,
    newline: &str,
) -> Result<SourceEdit, String> {
    let object_indent = line_indent(source, additions.object_range.start_byte);
    let member_indent = format!("{object_indent}  ");
    let mut additions_by_anchor = BTreeMap::<Option<usize>, Vec<String>>::new();
    for addition in additions.additions {
        additions_by_anchor.entry(addition.before_byte).or_default().push(addition.pair_source);
    }
    let mut members = Vec::new();
    for (byte, pair_source) in additions.existing_members {
        if let Some(prefix) = additions_by_anchor.remove(&Some(byte)) {
            members.extend(prefix);
        }
        members.push(pair_source);
    }
    if let Some(suffix) = additions_by_anchor.remove(&None) {
        members.extend(suffix);
    }
    if !additions_by_anchor.is_empty() {
        return Err(format!("missing insertion anchor for {}", additions.node_id));
    }
    let body = members
        .into_iter()
        .map(|member| format!("{member_indent}{member}"))
        .collect::<Vec<_>>()
        .join(&format!(",{newline}"));
    Ok(SourceEdit::replace(
        additions.object_range.start_byte,
        additions.object_range.end_byte,
        format!("{{{newline}{body}{newline}{object_indent}}}"),
    ))
}

fn appended_members_edit(
    source: &str,
    additions: &ObjectAdditions,
    pair_sources: Vec<String>,
    newline: &str,
) -> Result<SourceEdit, String> {
    let indent =
        additions.first_member_byte.map(|byte| line_indent(source, byte)).unwrap_or_else(|| {
            let closing_indent = line_indent(source, additions.closing_byte);
            format!("{closing_indent}  ")
        });
    let joined = pair_sources.join(&format!(",{newline}{indent}"));
    let (start, replacement) = if let Some(last_member_end) = additions.last_member_end {
        let gap = source
            .get(last_member_end..additions.closing_byte)
            .ok_or_else(|| format!("invalid object insertion span for {}", additions.node_id))?;
        if gap.contains('\n') {
            (last_member_end, format!(",{newline}{indent}{joined}{gap}"))
        } else {
            (last_member_end, format!(", {joined}{gap}"))
        }
    } else {
        let start = additions.object_range.start_byte + 1;
        let gap = source.get(start..additions.closing_byte).ok_or_else(|| {
            format!("invalid empty object insertion span for {}", additions.node_id)
        })?;
        if gap.contains('\n') {
            (start, format!("{newline}{indent}{joined}{gap}"))
        } else {
            (start, format!("{joined}{gap}"))
        }
    };
    Ok(SourceEdit::replace(start, additions.closing_byte, replacement))
}

fn line_indent(source: &str, byte: usize) -> String {
    let line_start = source[..byte].rfind('\n').map_or(0, |index| index + 1);
    source[line_start..byte]
        .chars()
        .take_while(|character| {
            character.is_whitespace() && *character != '\r' && *character != '\n'
        })
        .collect()
}

fn member<'a>(members: &'a [JsonSyntaxMember], key: &str) -> Option<&'a JsonSyntaxMember> {
    members.iter().find(|member| member.key == key)
}

fn join_path(path: &str, key: &str) -> String {
    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn diagnostic(category: DiagnosticCategory, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category,
        message: message.into(),
        path: None,
        review: None,
    }
}

fn conflict_diagnostics(conflicts: &[MergeConflict]) -> Vec<Diagnostic> {
    conflicts
        .iter()
        .map(|conflict| Diagnostic {
            path: Some(conflict.path.clone()),
            ..diagnostic(DiagnosticCategory::MergeConflict, conflict.message.clone())
        })
        .collect()
}

fn two_way_parse_failure(message: String, destination: bool) -> MergeResult<String> {
    MergeResult {
        ok: false,
        diagnostics: vec![diagnostic(
            if destination {
                DiagnosticCategory::DestinationParseError
            } else {
                DiagnosticCategory::ParseError
            },
            message,
        )],
        output: None,
        policies: vec![],
    }
}

fn render_failure(message: impl Into<String>) -> MergeResult<String> {
    MergeResult {
        ok: false,
        diagnostics: vec![diagnostic(DiagnosticCategory::UnsupportedFeature, message)],
        output: None,
        policies: vec![],
    }
}

fn clean_three_way(output: String) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Clean,
        diagnostics: vec![],
        conflicts: vec![],
        output: Some(output),
        policies: vec![],
    }
}

fn conflicted_three_way(conflicts: Vec<MergeConflict>) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Conflict,
        diagnostics: conflict_diagnostics(&conflicts),
        conflicts,
        output: None,
        policies: vec![],
    }
}

fn three_way_parse_failure(role: &str, message: String) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Error,
        diagnostics: vec![diagnostic(
            DiagnosticCategory::ParseError,
            format!("{role} parse error: {message}"),
        )],
        conflicts: vec![],
        output: None,
        policies: vec![],
    }
}

fn three_way_render_failure(message: impl Into<String>) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Error,
        diagnostics: vec![diagnostic(DiagnosticCategory::UnsupportedFeature, message)],
        conflicts: vec![],
        output: None,
        policies: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_strict_json_members_from_tree_haver() {
        let source = "{\r\n  \"name\" : \"structuredmerge\",\r\n  \"enabled\": true\r\n}\r\n";
        let document = parse_document(source, JsonDialect::Json).unwrap();

        assert_eq!(document.source, source);
        assert_eq!(document.root.source, source.trim());
        assert_eq!(document.root.members.len(), 2);
        assert_eq!(document.root.members[0].key, "name");
        assert_eq!(document.root.members[0].pair_source, "\"name\" : \"structuredmerge\"");
        assert_eq!(document.root.members[0].value.source, "\"structuredmerge\"");
    }

    #[test]
    fn uses_the_json5_grammar_for_jsonc_and_json5() {
        let jsonc =
            parse_document("{\n  // note\n  \"enabled\": true\n}\n", JsonDialect::Jsonc).unwrap();
        let json5 =
            parse_document("{\n  first: 1,\n  second: 2,\n}\n", JsonDialect::Json5).unwrap();

        assert_eq!(jsonc.root.members[0].key, "enabled");
        assert_eq!(
            json5.root.members.iter().map(|member| member.key.as_str()).collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    #[test]
    fn rejects_malformed_source_through_tree_haver() {
        let error = parse_document("{\"enabled\": tru", JsonDialect::Json).unwrap_err();
        assert!(error.contains("tree-sitter-language-pack reported syntax errors for json"));
    }

    #[test]
    fn preserves_destination_bytes_for_two_way_matches() {
        let incoming = "{\n  \"managed\": {\n    \"version\": 2\n  }\n}\n";
        let current = "{\n  // retained project note\n\n  \"managed\": {\n    \"version\": 1\n  },\n\n  \"local\": true\n}\n";
        let result = merge_json_source_preserving(incoming, current, JsonDialect::Jsonc);

        assert!(result.ok);
        assert_eq!(result.output.as_deref(), Some(current));
    }

    #[test]
    fn preserves_owned_comments_and_blank_lines_while_adding_a_member() {
        let incoming = "{\n  \"managed\": 1,\n  \"added\": true\n}\n";
        let current = "{\n  // retained owner note\n  \"managed\": 1,\n\n  \"local\": true\n}\n";
        let result = merge_json_source_preserving(incoming, current, JsonDialect::Jsonc);

        assert!(result.ok);
        assert_eq!(
            result.output.as_deref(),
            Some(
                "{\n  // retained owner note\n  \"managed\": 1,\n\n  \"local\": true,\n  \"added\": true\n}\n"
            )
        );
    }

    #[test]
    fn combines_independent_object_additions_without_reformatting() {
        let result = merge_json_three_way(
            "{\n  \"shared\": true\n}\n",
            "{\n  \"shared\": true,\n  \"ours\": 1\n}\n",
            "{\n  \"shared\": true,\n  \"theirs\": 2\n}\n",
            JsonDialect::Json,
        );

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(
            result.output.as_deref(),
            Some("{\n  \"shared\": true,\n  \"ours\": 1,\n  \"theirs\": 2\n}\n")
        );
    }

    #[test]
    fn applies_independent_jsonc_value_edits_to_ours() {
        let result = merge_json_three_way(
            "{\n  // left stays\n  \"left\": 1,\n  \"right\": 1\n}\n",
            "{\n  // left stays\n  \"left\": 2,\n  \"right\": 1\n}\n",
            "{\n  // left stays\n  \"left\": 1,\n  \"right\": 2\n}\n",
            JsonDialect::Jsonc,
        );

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(
            result.output.as_deref(),
            Some("{\n  // left stays\n  \"left\": 2,\n  \"right\": 2\n}\n")
        );
    }

    #[test]
    fn applies_independent_json5_value_edits_without_reordering() {
        let result = merge_json_three_way(
            "{\n  first: 1,\n  second: 1,\n}\n",
            "{\n  first: 2,\n  second: 1,\n}\n",
            "{\n  first: 1,\n  second: 2,\n}\n",
            JsonDialect::Json5,
        );

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some("{\n  first: 2,\n  second: 2,\n}\n"));
    }

    #[test]
    fn conflicts_on_same_owner_changes_and_preserves_duplicate_array_occurrences() {
        let same_owner = merge_json_three_way(
            "{\"region\":\"east\"}\n",
            "{\"region\":\"west\"}\n",
            "{\"region\":\"north\"}\n",
            JsonDialect::Json,
        );
        assert_eq!(same_owner.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(same_owner.conflicts[0].path, "/region");
        assert_eq!(same_owner.conflicts[0].alternatives.len(), 3);
        assert!(
            same_owner.conflicts[0]
                .alternatives
                .iter()
                .all(|alternative| alternative.state == ConflictAlternativeState::Present)
        );
        assert_eq!(same_owner.conflicts[0].alternatives[1].regions[0].start_line, 1);

        let duplicate = merge_json_three_way(
            "{\"items\":[{\"id\":\"same\",\"v\":1},{\"id\":\"same\",\"v\":1}]}\n",
            "{\"items\":[{\"id\":\"same\",\"v\":2},{\"id\":\"same\",\"v\":1}]}\n",
            "{\"items\":[{\"id\":\"same\",\"v\":1},{\"id\":\"same\",\"v\":2}]}\n",
            JsonDialect::Json,
        );
        assert_eq!(duplicate.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(duplicate.conflicts[0].category, "atomic_array_edit");
        assert_eq!(duplicate.conflicts[0].alternatives.len(), 3);
    }

    #[test]
    fn classifies_array_delete_modify_and_order_conflicts_with_owned_regions() {
        let delete_modify = merge_json_three_way(
            "{\"items\":[{\"id\":\"a\",\"v\":1},{\"id\":\"b\",\"v\":1}]}\n",
            "{\"items\":[{\"id\":\"b\",\"v\":1}]}\n",
            "{\"items\":[{\"id\":\"a\",\"v\":2},{\"id\":\"b\",\"v\":1}]}\n",
            JsonDialect::Json,
        );
        assert_eq!(delete_modify.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(delete_modify.conflicts[0].category, "delete_modify");
        assert_eq!(delete_modify.conflicts[0].path, "/items/0");
        assert_eq!(
            delete_modify.conflicts[0].alternatives[1].state,
            ConflictAlternativeState::Absent
        );

        let reorder = merge_json_three_way(
            "{\"items\":[{\"id\":\"a\"},{\"id\":\"b\"},{\"id\":\"c\"}]}\n",
            "{\"items\":[{\"id\":\"b\"},{\"id\":\"a\"},{\"id\":\"c\"}]}\n",
            "{\"items\":[{\"id\":\"a\"},{\"id\":\"c\"},{\"id\":\"b\"}]}\n",
            JsonDialect::Json,
        );
        assert_eq!(reorder.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(reorder.conflicts[0].category, "order");
        assert_eq!(reorder.conflicts[0].path, "/items");
        assert!(
            reorder.conflicts[0]
                .alternatives
                .iter()
                .all(|alternative| alternative.regions[0].region_kind == "node")
        );
    }

    #[test]
    fn records_absent_conflict_alternatives_without_inventing_a_region() {
        let result = merge_json_three_way(
            "{\n  \"enabled\": true,\n  \"stable\": 1\n}\n",
            "{\n  \"enabled\": true,\n  \"stable\": 2\n}\n",
            "{\n  \"stable\": 1\n}\n",
            JsonDialect::Json,
        );

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Conflict);
        let theirs = result.conflicts[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.revision == SourceRevision::Theirs)
            .unwrap();
        assert_eq!(theirs.state, ConflictAlternativeState::Absent);
        assert!(theirs.regions.is_empty());
    }

    #[test]
    fn reports_the_malformed_three_way_role_without_fallback() {
        let result = merge_json_three_way(
            "{\"ok\":true}\n",
            "{\"ok\": tru\n",
            "{\"ok\":false}\n",
            JsonDialect::Json,
        );

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
        assert!(result.diagnostics[0].message.starts_with("ours parse error:"));
    }

    #[test]
    fn semantic_array_identity_includes_object_keys() {
        let left = JsonSemanticValue::Object(BTreeMap::from([(
            "left".to_string(),
            JsonSemanticValue::Scalar { kind: "number".to_string(), value: "1".to_string() },
        )]));
        let right = JsonSemanticValue::Object(BTreeMap::from([(
            "right".to_string(),
            JsonSemanticValue::Scalar { kind: "number".to_string(), value: "1".to_string() },
        )]));

        assert_ne!(semantic_identity(&left), semantic_identity(&right));
    }
}
