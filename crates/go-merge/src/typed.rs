//! Existing conservative Go function ownership over validated TreeHaver facts.
//! Package/import bytes remain unowned layout, not newly supported merge owners.
use ast_merge::{
    SourcePreservingMergeEvidence, SourcePreservingOwnerDocument, ThreeWayMergeOutcome,
    native_analysis::NativeOwnerAnalysis,
};
use tree_haver::{service::ParsedResult, source::SourceRole};

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    if !parsed.backend.languages.iter().any(|language| language == "go") {
        return Err("Go analysis requires a Go parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root = parsed.document.output().root_id.as_deref().ok_or("Go parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis =
        ast_merge::project_named_top_level_analysis(source, root, &nodes, super::owner_policy())?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
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

#[derive(Clone, Debug)]
pub struct GoMergeExecution {
    pub evidence: SourcePreservingMergeEvidence,
    /// Only a clean result has a fresh parse bound to the exact output bytes.
    pub output_parse: Option<ParsedResult>,
}

fn verified_document(
    parsed: &ParsedResult,
    text: &str,
    ours: &ParsedResult,
    input_ids: &std::collections::HashSet<&String>,
) -> Result<SourcePreservingOwnerDocument, String> {
    if parsed.source.descriptor().role != SourceRole::Output
        || input_ids.contains(&parsed.source.descriptor().source_id)
        || parsed.source.bytes() != text.as_bytes()
        || parsed.backend != ours.backend
    {
        return Err(
            "Go output verification changed source bytes, identity or selected parser".into()
        );
    }
    owners(parsed)
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    mut parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<GoMergeExecution, String> {
    let mut ids = std::collections::HashSet::new();
    for (parsed, role) in [base, ours, theirs].into_iter().zip([
        SourceRole::Base,
        SourceRole::Ours,
        SourceRole::Theirs,
    ]) {
        if parsed.source.descriptor().role != role
            || !ids.insert(&parsed.source.descriptor().source_id)
        {
            return Err("Go merge requires distinct base/ours/theirs source identities".into());
        }
    }
    let [base_document, ours_document, theirs_document] =
        [owners(base)?, owners(ours)?, owners(theirs)?];
    // This family guard predates typed transport. Do not bypass it via the
    // generic owner classifier or claim a classification that never happened.
    if let Some(result) = membership_conflict(&base_document, &ours_document, &theirs_document) {
        return Ok(GoMergeExecution {
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
    if evidence.result.outcome == ThreeWayMergeOutcome::Clean {
        let text = evidence.result.output.as_deref().ok_or("clean Go merge omitted output")?;
        // Whole-source selections/no-ops bypass the legacy renderer callback.
        if output_parse.is_none() {
            let parsed = parse_output(text)?;
            verified_document(&parsed, text, ours, &ids)?;
            output_parse = Some(parsed);
        }
    } else {
        output_parse = None;
    }
    Ok(GoMergeExecution { evidence, output_parse })
}
