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
    let analysis = super::project_bash_analysis(source, root, &nodes)?;
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
