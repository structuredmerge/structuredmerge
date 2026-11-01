//! Conservative Go declaration ownership over validated Tree-sitter facts.
//! Package clauses, imports, and supported declarations are whole owners.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument,
    native_analysis::NativeOwnerAnalysis,
};
use tree_haver::service::ParsedResult;

pub fn validate_package_clause(parsed: &ParsedResult) -> Result<(), String> {
    if !parsed.backend.languages.iter().any(|language| language == "go") {
        return Err("Go analysis requires a Go parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    super::validate_go_package_clause(&index, root)
}

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    analysis_with_diagnostics(parsed).map_err(|error| error.message)
}

pub fn analysis_with_diagnostics(
    parsed: &ParsedResult,
) -> Result<NativeOwnerAnalysis, ast_merge::typed_merge::NativeAnalysisError> {
    let analysis = project(parsed).map_err(|message| {
        if message.starts_with("Go document has duplicate top-level owner identity ") {
            ast_merge::typed_merge::NativeAnalysisError::ambiguous_owner(message)
        } else {
            ast_merge::typed_merge::NativeAnalysisError::unsupported(message)
        }
    })?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| {
        ast_merge::typed_merge::NativeAnalysisError::unsupported(error.to_string())
    })?;
    validate_native_source_ranges(source, &analysis.document)
        .map_err(ast_merge::typed_merge::NativeAnalysisError::unsupported)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    project(parsed).map(|analysis| analysis.document)
}

fn project(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    validate_package_clause(parsed)?;
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis = super::project_go_owners(source, root, &nodes)?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

/// Preserve the family's conservative boundary independently of transport.
pub fn membership_conflict(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> Option<ast_merge::ThreeWayMergeResult<String>> {
    super::go_membership_change_with_owner_edit(base, ours, theirs)
        .then(super::conservative_membership_conflict)
}

pub fn merge_documents(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: &mut dyn FnMut(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> SourcePreservingMergeEvidence {
    if let Some(result) = membership_conflict(&base, &ours, &theirs) {
        return SourcePreservingMergeEvidence {
            result,
            source_segments: vec![],
            classification: None,
        };
    }
    ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
}

pub type GoMergeExecution = ast_merge::typed_merge::ParsedOwnerMergeExecution;

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<GoMergeExecution, String> {
    ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze_with_diagnostics: None,
            analyze: owners,
            merge: merge_documents,
            postprocess: Some(line_selection_postprocessor),
        },
        parse_output,
    )
}

fn validate_native_source_ranges(
    source: &str,
    document: &SourcePreservingOwnerDocument,
) -> Result<(), String> {
    let bytes = source.as_bytes();
    let horizontal_whitespace =
        |slice: &[u8]| slice.iter().all(|byte| matches!(byte, b'\t' | b'\r' | b' '));
    for (index, owner) in document.owners.iter().enumerate() {
        let start = owner.start_byte;
        let end = owner.end_byte;
        if start > end || end > bytes.len() {
            return Err(format!("Go owner {:?} has an invalid source range", owner.id));
        }
        let line_start =
            bytes[..start].iter().rposition(|byte| *byte == b'\n').map_or(0, |i| i + 1);
        let line_end =
            bytes[end..].iter().position(|byte| *byte == b'\n').map_or(bytes.len(), |i| end + i);
        if !horizontal_whitespace(&bytes[line_start..start])
            || !horizontal_whitespace(&bytes[end..line_end])
            || (index > 0 && document.owners[index - 1].end_line >= owner.start_line)
        {
            return Err(format!("Go owner {:?} has an unsafe source range", owner.id));
        }
    }
    Ok(())
}

/// Line-level ownership facts for one validated Go parse. Comments outside owner ranges stay
/// document-owned layout, matching the Go owner model that leaves them unmanaged.
pub fn source_line_analysis(
    parsed: &ParsedResult,
) -> Result<ast_merge::line_selection::LineSelectionAnalysis, String> {
    use ast_merge::{
        CommentAttachmentPolicy, CommentScope, LayoutOwner, augment_layout,
        augment_normalized_comments_with_evidence_scope,
        line_selection::{remap_comment_owner_ids, remap_layout_owner_ids, source_line_contents},
        source_line_dispositions_from_augmentations,
    };
    use std::collections::HashMap;
    let owners = project(parsed)?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let nodes = parsed.normalized_nodes()?;
    let mut tree_to_semantic = HashMap::<String, String>::new();
    let mut native_owners = Vec::with_capacity(owners.document.owners.len());
    let mut semantic_owners = Vec::with_capacity(owners.document.owners.len());
    for owner in &owners.document.owners {
        let node_ids = owners
            .owner_node_ids
            .get(&owner.id)
            .ok_or_else(|| format!("Go owner {} has no native node references", owner.id))?;
        let structural = node_ids
            .iter()
            .filter_map(|id| nodes.iter().find(|node| node.id == *id))
            .filter(|node| node.role != tree_haver::NodeRole::Comment && node.kind != "comment")
            .collect::<Vec<_>>();
        let [declaration] = structural.as_slice() else {
            return Err(format!("Go owner {} does not map to one declaration node", owner.id));
        };
        if tree_to_semantic.insert(declaration.id.clone(), owner.id.clone()).is_some() {
            return Err(format!("native Go node {} maps to multiple owners", declaration.id));
        }
        native_owners.push(LayoutOwner {
            owner_id: declaration.id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        });
        semantic_owners.push(LayoutOwner {
            owner_id: owner.id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        });
    }
    let ranges = owners
        .document
        .owners
        .iter()
        .map(|owner| (owner.start_line, owner.end_line))
        .collect::<Vec<_>>();
    let mut comments = augment_normalized_comments_with_evidence_scope(
        source,
        &native_owners,
        &nodes,
        "go_comment",
        CommentAttachmentPolicy::FollowingOwner,
        |node| {
            if node.role != tree_haver::NodeRole::Comment && node.kind != "comment" {
                return None;
            }
            let line = node.span.start_point.row + 1;
            let inside = ranges.iter().any(|(start, end)| *start <= line && line <= *end);
            Some(if inside { CommentScope::Owner } else { CommentScope::Root })
        },
        normalize_go_comment,
    )?;
    remap_comment_owner_ids(&mut comments.augmentation, &tree_to_semantic)?;
    let lines = source_line_contents(source);
    let mut layout = augment_layout(&lines, &native_owners)?;
    remap_layout_owner_ids(&mut layout, &tree_to_semantic)?;
    let dispositions = source_line_dispositions_from_augmentations(
        source,
        &semantic_owners,
        &comments.augmentation,
        &layout,
    )?;
    Ok(ast_merge::line_selection::LineSelectionAnalysis { owners, comments, dispositions })
}

fn normalize_go_comment(text: &str) -> String {
    let text = text.trim();
    let body = match text.strip_prefix("/*") {
        Some(rest) => rest.strip_suffix("*/").unwrap_or(rest),
        None => text.trim_start_matches('/'),
    };
    body.trim().to_string()
}

/// Stopgap for Go's intentional membership guard: when only individually named owners were added
/// or removed (no package or grouped-declaration change), rerun the generic classification and
/// select whole source lines per owner decision. The result must be byte-proven, reparse with the
/// same parser, and contain exactly the surviving owners; anything else keeps the guard.
pub fn line_selection_postprocessor(
    inputs: &[ParsedResult],
    evidence: &mut SourcePreservingMergeEvidence,
    parse_output: &mut dyn FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<bool, String> {
    use ast_merge::line_selection::{
        is_recoverable_owner_layout_error, recover_with_line_selection,
    };
    use tree_haver::source::SourceRole;

    let guarded = evidence.result.outcome == ast_merge::ThreeWayMergeOutcome::Conflict
        && evidence
            .result
            .conflicts
            .iter()
            .any(|conflict| conflict.conflict_id == super::GO_MEMBERSHIP_CONFLICT_ID);
    if inputs.len() != 3 || !(guarded || is_recoverable_owner_layout_error(evidence)) {
        return Ok(false);
    }
    let find = |role| inputs.iter().find(|parsed| parsed.source.descriptor().role == role);
    let (Some(base), Some(ours), Some(theirs)) =
        (find(SourceRole::Base), find(SourceRole::Ours), find(SourceRole::Theirs))
    else {
        return Ok(false);
    };
    let (Ok(base_lines), Ok(ours_lines), Ok(theirs_lines)) =
        (source_line_analysis(base), source_line_analysis(ours), source_line_analysis(theirs))
    else {
        return Ok(false);
    };
    let analyses = [base_lines, ours_lines, theirs_lines];
    let mut candidate = evidence.clone();
    if guarded {
        if !super::go_plain_membership_change(
            &analyses[0].owners.document,
            &analyses[1].owners.document,
            &analyses[2].owners.document,
        ) || analyses
            .iter()
            .any(|analysis| analysis.dispositions.iter().any(|line| line.kind == "unmanaged"))
        {
            return Ok(false);
        }
        let mut verify = |source: &str| {
            let parsed = parse_output(source)?;
            if !parsed.document.output().ok {
                return Err("Go parser rejected rendered output".to_string());
            }
            owners(&parsed)
        };
        candidate = ast_merge::merge_source_preserving_owners_with_evidence(
            analyses[0].owners.document.clone(),
            analyses[1].owners.document.clone(),
            analyses[2].owners.document.clone(),
            &mut verify,
        );
        if candidate.result.outcome != ast_merge::ThreeWayMergeOutcome::Clean
            && !is_recoverable_owner_layout_error(&candidate)
        {
            return Ok(false);
        }
    }
    if recover_with_line_selection(inputs, &analyses, &mut candidate, parse_output, &|parsed| {
        owners(parsed)
            .map(|document| document.owners.iter().map(|owner| owner.id.clone()).collect())
    }) {
        *evidence = candidate;
        return Ok(true);
    }
    Ok(false)
}
