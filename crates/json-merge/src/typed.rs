//! Existing JSON semantics over validated TreeHaver facts. No parser loading,
//! registry, source-text discovery or host-owned merge decisions live here.
use crate::{JsonAnalysis, JsonDialect, source_preserving::*};
use ast_merge::{
    ConflictAlternative, ConflictAlternativeState, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeConflict, MergeResult, OwnedSourceRegion, SourceRevision,
    ThreeWayMergeOutcome, ThreeWayMergeResult,
};
use tree_haver::{service::ParsedResult, source::SourceRole};

/// Nested source owners may overlap their descendants. They are comparison
/// subjects, not a non-overlapping render partition. Array identity is positional.
/// IDs are local to this analysis and must be qualified by its source descriptor.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JsonOwnerFact {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub node_id: String,
    /// Value node used by the existing family comment/layout owner policy.
    pub value_node_id: String,
    pub parent_id: Option<String>,
    pub match_key: Option<String>,
    pub span: tree_haver::SourceSpan,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JsonOwnerAnalysis {
    pub source: tree_haver::source::SourceDescriptor,
    pub owners: Vec<JsonOwnerFact>,
    /// Exact native-node provenance for the existing family region decisions.
    /// A multiline node can participate in multiple regions; callers must not
    /// assume these references form disjoint emission ranges.
    pub comment_region_node_ids: std::collections::BTreeMap<String, Vec<String>>,
    pub unclaimed_comment_node_ids: Vec<String>,
    /// Exact bytes of the legacy blank-run gaps (not all document trivia).
    pub layout_gap_sources: std::collections::BTreeMap<String, JsonLayoutGapSource>,
    /// Legacy family comment/layout analysis retains its own native-node owner
    /// namespace. It is not yet a Slice 1024 projection of `owners` above.
    pub family: JsonAnalysis,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JsonLayoutGapSource {
    pub span: tree_haver::SourceSpan,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JsonOwnerChange {
    pub path: String,
    pub classification: String,
    pub before: Option<JsonOwnerFact>,
    pub after: Option<JsonOwnerFact>,
}

pub fn owner_analysis(
    parsed: &ParsedResult,
    dialect: JsonDialect,
) -> Result<JsonOwnerAnalysis, String> {
    let document = document(parsed, dialect)?;
    let mut owners = vec![];
    let mut paths = std::collections::BTreeSet::new();
    // Node ID and span are supplied together by family syntax analysis. Never
    // search source text to relocate repeated or identical fragments.
    let mut pending = vec![(
        &document.root,
        "".to_string(),
        "root",
        document.root.node_id.as_str(),
        document.root.range.clone(),
        None,
        None,
    )];
    while let Some((value, path, kind, node_id, range, parent_id, match_key)) = pending.pop() {
        if !paths.insert(path.clone()) {
            return Err("duplicate JSON owner identity is ambiguous".into());
        }
        let node = parsed.document.node(node_id).ok_or("unresolved JSON owner node")?;
        if node.span.range != range {
            return Err("JSON owner range differs from native node".into());
        }
        let id = format!("json:{path}");
        owners.push(JsonOwnerFact {
            id: id.clone(),
            path: path.clone(),
            kind: kind.into(),
            node_id: node_id.into(),
            value_node_id: value.node_id.clone(),
            parent_id,
            match_key,
            span: node.span.clone(),
            sha256: parsed.source.range_digest(range).map_err(|error| error.to_string())?,
        });
        for (index, element) in value.elements.iter().enumerate().rev() {
            pending.push((
                element,
                format!("{path}/{index}"),
                "element",
                element.node_id.as_str(),
                element.range.clone(),
                Some(id.clone()),
                Some(index.to_string()),
            ));
        }
        for member in value.members.iter().rev() {
            let key = member.key.replace('~', "~0").replace('/', "~1");
            pending.push((
                &member.value,
                format!("{path}/{key}"),
                "member",
                member.node_id.as_str(),
                member.pair_range.clone(),
                Some(id.clone()),
                Some(member.key.clone()),
            ));
        }
    }
    // Convert the existing line-based decisions using a byte line index. This
    // indexes newline boundaries only; it does not discover syntax or comments.
    let mut line_starts = vec![0];
    line_starts.extend(
        parsed
            .source
            .bytes()
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b'\n').then_some(index + 1)),
    );
    let mut layout_gap_sources = std::collections::BTreeMap::new();
    for gap in &document.comment_augmentation.augmentation.gaps {
        let start = gap
            .start_line
            .checked_sub(1)
            .and_then(|index| line_starts.get(index))
            .copied()
            .ok_or("invalid JSON layout gap start line")?;
        let end = line_starts.get(gap.end_line).copied().unwrap_or(parsed.source.bytes().len());
        let range = tree_haver::ByteRange { start_byte: start, end_byte: end };
        let span = tree_haver::SourceSpan {
            start_point: parsed.source.point(start).map_err(|e| e.to_string())?,
            end_point: parsed.source.point(end).map_err(|e| e.to_string())?,
            range: range.clone(),
        };
        layout_gap_sources.insert(
            gap.id.clone(),
            JsonLayoutGapSource {
                span,
                sha256: parsed.source.range_digest(range).map_err(|e| e.to_string())?,
            },
        );
    }
    Ok(JsonOwnerAnalysis {
        source: parsed.source.descriptor().clone(),
        owners,
        comment_region_node_ids: document.comment_augmentation.region_node_ids.clone(),
        unclaimed_comment_node_ids: document.comment_augmentation.unclaimed_node_ids.clone(),
        layout_gap_sources,
        family: analyze_syntax(document, dialect),
    })
}

/// Exact-source diff of nested owner subjects, including the root. An edited
/// descendant also edits containing owners; this is not an edit script or a
/// semantic array move detector. Equal text never supplies an owner's location.
/// This helper deliberately excludes trivia outside the syntax root and is not
/// a complete document diff. Common diff2 must also compare layout/comments.
pub fn diff_owner_sources(
    before: &ParsedResult,
    after: &ParsedResult,
    dialect: JsonDialect,
) -> Result<Vec<JsonOwnerChange>, String> {
    require_role(before, SourceRole::Before)?;
    require_role(after, SourceRole::After)?;
    if before.source.descriptor().source_id == after.source.descriptor().source_id {
        return Err("JSON diff source IDs must be distinct".into());
    }
    let left = diff_owners(before, dialect)?;
    let right = diff_owners(after, dialect)?;
    let left = left
        .into_iter()
        .map(|owner| (owner.path.clone(), owner))
        .collect::<std::collections::BTreeMap<_, _>>();
    let right = right
        .into_iter()
        .map(|owner| (owner.path.clone(), owner))
        .collect::<std::collections::BTreeMap<_, _>>();
    let paths = left.keys().chain(right.keys()).collect::<std::collections::BTreeSet<_>>();
    let mut changes = vec![];
    for path in paths {
        let before_owner = left.get(path);
        let after_owner = right.get(path);
        if let (Some(before_owner), Some(after_owner)) = (before_owner, after_owner) {
            if before_owner.kind == after_owner.kind
                && before
                    .source
                    .slice(before_owner.span.range.clone())
                    .map_err(|e| e.to_string())?
                    == after
                        .source
                        .slice(after_owner.span.range.clone())
                        .map_err(|e| e.to_string())?
            {
                continue;
            }
        }
        changes.push(JsonOwnerChange {
            path: path.clone(),
            classification: if before_owner.is_none() {
                "added"
            } else if after_owner.is_none() {
                "deleted"
            } else {
                "edited"
            }
            .into(),
            before: before_owner.cloned(),
            after: after_owner.cloned(),
        });
    }
    Ok(changes)
}

// A zero-byte document has no JSON owners for source comparison. This is
// deliberately diff-only: analyze and merge still require a JSON value. Keep
// parser/language and native-root evidence checks even for the empty side.
fn diff_owners(parsed: &ParsedResult, dialect: JsonDialect) -> Result<Vec<JsonOwnerFact>, String> {
    if !parsed.source.bytes().is_empty() {
        return Ok(owner_analysis(parsed, dialect)?.owners);
    }
    if !parsed.backend.languages.iter().any(|language| language == parser_language(dialect)) {
        return Err("typed JSON input uses an incompatible parser language".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root =
        parsed.document.output().root_id.as_deref().ok_or("typed JSON input omitted its root")?;
    let native = parsed.document.node(root).ok_or("unresolved empty JSON root")?;
    if nodes.len() != 1
        || native.native_type != "document"
        || native.missing
        || native.has_error
        || !native.children.is_empty()
        || native.span.range.start_byte != 0
        || native.span.range.end_byte != 0
    {
        return Err("empty JSON diff requires an empty native document".into());
    }
    Ok(vec![])
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JsonMergeExecution<T> {
    pub result: T,
    pub render: Option<crate::render_evidence::JsonRenderEvidence>,
}

fn document(parsed: &ParsedResult, dialect: JsonDialect) -> Result<JsonSyntaxDocument, String> {
    if !parsed.backend.languages.iter().any(|language| language == parser_language(dialect)) {
        return Err("typed JSON input uses an incompatible parser language".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let root =
        parsed.document.output().root_id.as_deref().ok_or("typed JSON input omitted its root")?;
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
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<MergeResult<String>, String> {
    Ok(merge2_with_evidence(incoming, current, dialect, parse_output)?.result)
}

pub fn merge2_with_evidence(
    incoming: &ParsedResult,
    current: &ParsedResult,
    dialect: JsonDialect,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<JsonMergeExecution<MergeResult<String>>, String> {
    require_role(incoming, SourceRole::Incoming)?;
    require_role(current, SourceRole::Current)?;
    if incoming.source.descriptor().source_id == current.source.descriptor().source_id {
        return Err("typed JSON inputs must have distinct source IDs".into());
    }
    if incoming.source.bytes() != current.source.bytes() {
        owner_analysis(incoming, dialect)?;
        owner_analysis(current, dialect)?;
    }
    let mut render = None;
    let mut result = merge_documents_two_way(
        document(incoming, dialect)?,
        document(current, dialect)?,
        |source, role, edits| {
            if role != SourceRole::Current {
                return Err("invalid directional render baseline".into());
            }
            let document = verify_output(
                source,
                parse_output(source)?,
                current,
                &[incoming, current],
                dialect,
            )?;
            render = Some(crate::render_evidence::JsonRenderEvidence::from_edits(
                &current.source,
                source,
                edits,
            )?);
            Ok(document)
        },
    );
    if result.ok {
        result.policies.push(crate::destination_wins_array_policy());
    }
    if !result.ok {
        render = None;
    }
    Ok(JsonMergeExecution { result, render })
}

fn duplicate_member_paths(value: &JsonSyntaxValue) -> Vec<String> {
    fn visit(value: &JsonSyntaxValue, path: &str, paths: &mut std::collections::BTreeSet<String>) {
        let mut seen = std::collections::HashSet::new();
        for member in &value.members {
            let escaped = member.key.replace('~', "~0").replace('/', "~1");
            let child_path = format!("{path}/{escaped}");
            if !seen.insert(member.key.as_str()) {
                paths.insert(child_path.clone());
            }
            visit(&member.value, &child_path, paths);
        }
        for (index, element) in value.elements.iter().enumerate() {
            visit(element, &format!("{path}/{index}"), paths);
        }
    }

    let mut paths = std::collections::BTreeSet::new();
    visit(value, "", &mut paths);
    paths.into_iter().collect()
}

fn path_segments(path: &str) -> Vec<String> {
    path.strip_prefix('/').unwrap_or(path).split('/').map(str::to_string).collect()
}

fn decode_path_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

fn descend<'a>(current: Vec<&'a JsonSyntaxValue>, segments: &[String]) -> Vec<&'a JsonSyntaxValue> {
    let Some((segment, rest)) = segments.split_first() else {
        return current;
    };
    let mut next = Vec::new();
    for value in current {
        if !value.members.is_empty() || matches!(value.semantic, JsonSemanticValue::Object(_)) {
            let key = decode_path_segment(segment);
            next.extend(
                value.members.iter().filter(|member| member.key == key).map(|member| &member.value),
            );
        } else if let Ok(index) = segment.parse::<usize>() {
            if let Some(element) = value.elements.get(index) {
                next.push(element);
            }
        }
    }
    descend(next, rest)
}

fn member_values_at_path(root: &JsonSyntaxValue, path: &str) -> Vec<JsonSemanticValue> {
    descend(vec![root], &path_segments(path))
        .into_iter()
        .map(|value| value.semantic.clone())
        .collect()
}

fn duplicate_conflict_alternative(
    root: &JsonSyntaxValue,
    path: &str,
    revision: SourceRevision,
) -> ConflictAlternative {
    let mut segments = path_segments(path);
    let key = segments.pop().unwrap_or_default();
    let regions = descend(vec![root], &segments)
        .into_iter()
        .flat_map(|parent| {
            parent
                .members
                .iter()
                .filter(|member| member.key == decode_path_segment(&key))
                .map(|member| member.owned_region.clone())
        })
        .collect::<Vec<OwnedSourceRegion>>();
    ConflictAlternative {
        revision,
        state: if regions.is_empty() {
            ConflictAlternativeState::Absent
        } else {
            ConflictAlternativeState::Present
        },
        regions,
    }
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    dialect: JsonDialect,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<ThreeWayMergeResult<String>, String> {
    Ok(merge3_with_evidence(base, ours, theirs, dialect, parse_output)?.result)
}

pub fn merge3_with_evidence(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    dialect: JsonDialect,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<JsonMergeExecution<ThreeWayMergeResult<String>>, String> {
    require_role(base, SourceRole::Base)?;
    require_role(ours, SourceRole::Ours)?;
    require_role(theirs, SourceRole::Theirs)?;
    let ids = [base, ours, theirs].map(|parsed| &parsed.source.descriptor().source_id);
    if ids[0] == ids[1] || ids[0] == ids[2] || ids[1] == ids[2] {
        return Err("typed JSON inputs must have distinct source IDs".into());
    }
    let base_document = document(base, dialect)?;
    let ours_document = document(ours, dialect)?;
    let theirs_document = document(theirs, dialect)?;
    let duplicate_paths = duplicate_member_paths(&base_document.root)
        .into_iter()
        .chain(duplicate_member_paths(&ours_document.root))
        .chain(duplicate_member_paths(&theirs_document.root))
        .collect::<std::collections::BTreeSet<_>>();
    let contested_paths = duplicate_paths
        .into_iter()
        .filter(|path| {
            let base_values = member_values_at_path(&base_document.root, path);
            let ours_values = member_values_at_path(&ours_document.root, path);
            let theirs_values = member_values_at_path(&theirs_document.root, path);
            ours_values != base_values
                && theirs_values != base_values
                && ours_values != theirs_values
        })
        .collect::<Vec<_>>();
    if !contested_paths.is_empty() {
        let conflicts = contested_paths
            .into_iter()
            .map(|path| MergeConflict {
                conflict_id: format!("json:duplicate-owner:{}", path),
                category: "edit_edit".into(),
                fallback_scope: path.clone(),
                message: format!("both sides changed duplicate JSON member {path}"),
                alternatives: vec![
                    duplicate_conflict_alternative(
                        &base_document.root,
                        &path,
                        SourceRevision::Base,
                    ),
                    duplicate_conflict_alternative(
                        &ours_document.root,
                        &path,
                        SourceRevision::Ours,
                    ),
                    duplicate_conflict_alternative(
                        &theirs_document.root,
                        &path,
                        SourceRevision::Theirs,
                    ),
                ],
                path,
            })
            .collect::<Vec<_>>();
        let diagnostics = conflicts
            .iter()
            .map(|conflict| Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::MergeConflict,
                message: conflict.message.clone(),
                path: Some(conflict.path.clone()),
                review: None,
            })
            .collect();
        return Ok(JsonMergeExecution {
            result: ThreeWayMergeResult {
                outcome: ThreeWayMergeOutcome::Conflict,
                diagnostics,
                conflicts,
                output: None,
                policies: vec![],
            },
            render: None,
        });
    }
    let mut render = None;
    let result = merge_documents_three_way(
        base_document,
        ours_document,
        theirs_document,
        |source, role, edits| {
            let baseline = match role {
                SourceRole::Ours => ours,
                SourceRole::Theirs => theirs,
                _ => return Err("invalid three-way render baseline".into()),
            };
            let document =
                verify_output(source, parse_output(source)?, ours, &[base, ours, theirs], dialect)?;
            render = Some(crate::render_evidence::JsonRenderEvidence::from_edits(
                &baseline.source,
                source,
                edits,
            )?);
            Ok(document)
        },
    );
    if result.outcome != ast_merge::ThreeWayMergeOutcome::Clean {
        render = None;
    }
    Ok(JsonMergeExecution { result, render })
}
