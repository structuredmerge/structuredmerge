//! Existing Bash ownership/merge semantics over validated TreeHaver facts.
//! Parsing and output verification are supplied by the shared service caller.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument, ThreeWayMergeOutcome,
};
use tree_haver::{service::ParsedResult, source::SourceRole};

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
}

pub fn analysis(
    parsed: &ParsedResult,
) -> Result<ast_merge::native_analysis::NativeOwnerAnalysis, String> {
    if !parsed.backend.languages.iter().any(|language| language == "bash") {
        return Err("Bash analysis requires a Bash parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Bash parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|e| e.to_string())?;
    // Empty/comment-only documents are valid directional endpoints. Derive this
    // from the native root's children, never from textual source classification.
    let index = tree_haver::NormalizedTreeIndex::new(&nodes)?;
    let root_node = index.root(root)?;
    let analysis = if index
        .children(root_node)
        .iter()
        .all(|node| node.role == tree_haver::NodeRole::Comment)
    {
        ast_merge::native_analysis::NativeOwnerAnalysis {
            document: SourcePreservingOwnerDocument { source: source.into(), owners: vec![] },
            owner_node_ids: Default::default(),
        }
    } else {
        super::project_bash_analysis(source, root, &nodes)?
    };
    analysis.validate(parsed)?;
    Ok(analysis)
}

#[derive(Clone, Debug)]
pub struct BashMergeExecution {
    pub evidence: SourcePreservingMergeEvidence,
    /// Present only for a clean, freshly verified result. No input parse is
    /// relabeled as an output parse, including whole-source/no-op selections.
    pub output_parse: Option<ParsedResult>,
}

fn verified_document(
    parsed: &ParsedResult,
    text: &str,
    ours: &ParsedResult,
    ids: &std::collections::HashSet<&String>,
) -> Result<SourcePreservingOwnerDocument, String> {
    if parsed.source.descriptor().role != SourceRole::Output
        || ids.contains(&parsed.source.descriptor().source_id)
        || parsed.source.bytes() != text.as_bytes()
        || parsed.backend != ours.backend
    {
        return Err(
            "Bash output verification changed source bytes, identity or selected parser".into()
        );
    }
    owners(parsed)
}

/// Return the Ruby-compatible whole-document guard when unmanaged source differs.
pub fn unmanaged_source_conflict(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> Option<ast_merge::ThreeWayMergeResult<String>> {
    if ours.source == theirs.source || base.source == theirs.source || base.source == ours.source {
        return None;
    }
    (ast_merge::unmanaged_source_fingerprint(ours)
        != ast_merge::unmanaged_source_fingerprint(theirs))
    .then(crate::unmanaged_source_conflict_result)
}

/// Shared-core merge callback with Bash's Ruby-compatible unmanaged-source guard.
pub fn merge_documents(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: &mut dyn FnMut(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> SourcePreservingMergeEvidence {
    if let Some(result) = unmanaged_source_conflict(&base, &ours, &theirs) {
        return SourcePreservingMergeEvidence {
            result,
            source_segments: vec![],
            classification: None,
        };
    }
    ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<BashMergeExecution, String> {
    let inputs = [base, ours, theirs];
    let mut ids = std::collections::HashSet::new();
    for (parsed, role) in
        inputs.into_iter().zip([SourceRole::Base, SourceRole::Ours, SourceRole::Theirs])
    {
        if parsed.source.descriptor().role != role
            || !ids.insert(&parsed.source.descriptor().source_id)
        {
            return Err("Bash merge requires distinct base/ours/theirs source identities".into());
        }
    }
    let documents = [owners(base)?, owners(ours)?, owners(theirs)?];
    let [base_document, ours_document, theirs_document] = documents;
    if let Some(result) =
        unmanaged_source_conflict(&base_document, &ours_document, &theirs_document)
    {
        return Ok(BashMergeExecution {
            evidence: SourcePreservingMergeEvidence {
                result,
                source_segments: vec![],
                classification: None,
            },
            output_parse: None,
        });
    }
    let mut output_parse = None;
    let mut verify = |text: &str| {
        let parsed = parse_output(text)?;
        let document = verified_document(&parsed, text, ours, &ids)?;
        output_parse = Some(parsed);
        Ok(document)
    };
    let evidence = ast_merge::merge_source_preserving_owners_with_evidence(
        base_document,
        ours_document,
        theirs_document,
        &mut verify,
    );
    // Existing whole-source shortcuts do not call verify. Typed execution must
    // verify these too, while preserving the engine's decisions and segments.
    if evidence.result.outcome == ThreeWayMergeOutcome::Clean {
        let text = evidence.result.output.as_deref().ok_or("clean Bash merge omitted output")?;
        if output_parse.is_none() {
            let parsed = parse_output(text)?;
            verified_document(&parsed, text, ours, &ids)?;
            output_parse = Some(parsed);
        }
    } else {
        output_parse = None;
    }
    Ok(BashMergeExecution { evidence, output_parse })
}
