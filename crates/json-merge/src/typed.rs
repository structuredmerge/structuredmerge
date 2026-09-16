//! Existing JSON semantics over validated TreeHaver facts. No parser loading,
//! registry, source-text discovery or host-owned merge decisions live here.
use crate::{JsonAnalysis, JsonDialect, source_preserving::*};
use ast_merge::{MergeResult, ThreeWayMergeResult};
use tree_haver::{NormalizedTreeNode, service::ParsedResult, source::SourceRole};

fn document(parsed: &ParsedResult, dialect: JsonDialect) -> Result<JsonSyntaxDocument, String> {
    let output = parsed.document.output();
    if !output.ok || output.source != *parsed.source.descriptor() {
        return Err("typed JSON input is unsuccessful or has mismatched source identity".into());
    }
    if !parsed.backend.languages.iter().any(|language| language == parser_language(dialect)) {
        return Err("typed JSON input uses an incompatible parser language".into());
    }
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let root = output.root_id.as_deref().ok_or("typed JSON input omitted its root")?;
    let fields = output
        .nodes
        .iter()
        .flat_map(|node| &node.children)
        .map(|edge| (edge.node_id.as_str(), edge.field_name.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    // Internal adaptation to the established family analyzer, not a public
    // normalized/JSON transport. All text and topology come from validated facts.
    let nodes = output
        .nodes
        .iter()
        .map(|node| {
            let field_name = fields.get(node.id.as_str()).cloned().flatten();
            Ok(NormalizedTreeNode {
                id: node.id.clone(),
                kind: node.native_type.clone(),
                role: node.role,
                parent_id: node.parent_id.clone(),
                child_ids: node.children.iter().map(|edge| edge.node_id.clone()).collect(),
                span: node.span.clone(),
                field_name,
                named: node.named,
                anonymous: !node.named,
                has_source_text: true,
                source_fragment: std::str::from_utf8(
                    parsed
                        .source
                        .slice(node.span.range.clone())
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?
                .to_string(),
                backend_kind: Some(node.native_type.clone()),
                semantic_roles: node.semantic_roles.clone(),
                backend_roles: vec![],
                unsupported_features: node.unsupported_features.clone(),
                metadata: Default::default(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    document_from_nodes(source, dialect, root, &nodes)
}

fn require_role(parsed: &ParsedResult, role: SourceRole) -> Result<(), String> {
    if parsed.source.descriptor().role != role {
        return Err(format!("typed JSON input requires {role:?} source role"));
    }
    Ok(())
}

fn verify_output(
    source: &str,
    result: ParsedResult,
    selected: &ParsedResult,
    inputs: &[&ParsedResult],
    dialect: JsonDialect,
) -> Result<JsonSyntaxDocument, String> {
    require_role(&result, SourceRole::Output)?;
    if inputs
        .iter()
        .any(|input| input.source.descriptor().source_id == result.source.descriptor().source_id)
    {
        return Err("typed JSON output source ID collides with an input".into());
    }
    if result.source.bytes() != source.as_bytes() || result.backend != selected.backend {
        return Err("typed JSON verification changed output bytes or selected parser".into());
    }
    document(&result, dialect)
}

pub fn analyze(parsed: &ParsedResult, dialect: JsonDialect) -> Result<JsonAnalysis, String> {
    Ok(analyze_syntax(document(parsed, dialect)?, dialect))
}

/// Incoming/current direction is explicit; never invent a base for merge2.
/// The caller supplies output parsing through the same TreeHaver snapshot.
pub fn merge2(
    incoming: &ParsedResult,
    current: &ParsedResult,
    dialect: JsonDialect,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<MergeResult<String>, String> {
    require_role(incoming, SourceRole::Incoming)?;
    require_role(current, SourceRole::Current)?;
    if incoming.source.descriptor().source_id == current.source.descriptor().source_id {
        return Err("typed JSON inputs must have distinct source IDs".into());
    }
    let mut result = merge_documents_two_way(
        document(incoming, dialect)?,
        document(current, dialect)?,
        |source| {
            verify_output(source, parse_output(source)?, current, &[incoming, current], dialect)
        },
    );
    if result.ok {
        result.policies.push(crate::destination_wins_array_policy());
    }
    Ok(result)
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    dialect: JsonDialect,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<ThreeWayMergeResult<String>, String> {
    require_role(base, SourceRole::Base)?;
    require_role(ours, SourceRole::Ours)?;
    require_role(theirs, SourceRole::Theirs)?;
    let ids = [base, ours, theirs].map(|parsed| &parsed.source.descriptor().source_id);
    if ids[0] == ids[1] || ids[0] == ids[2] || ids[1] == ids[2] {
        return Err("typed JSON inputs must have distinct source IDs".into());
    }
    Ok(merge_documents_three_way(
        document(base, dialect)?,
        document(ours, dialect)?,
        document(theirs, dialect)?,
        |source| verify_output(source, parse_output(source)?, ours, &[base, ours, theirs], dialect),
    ))
}
