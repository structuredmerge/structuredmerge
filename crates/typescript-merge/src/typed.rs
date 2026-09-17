//! Existing TypeScript/TSX declaration ownership over validated native facts.
//! Import/comment bytes remain layout; wrapper spans remain whole source owners.
use ast_merge::{SourcePreservingOwnerDocument, native_analysis::NativeOwnerAnalysis};
use tree_haver::service::ParsedResult;

pub fn analysis(parsed: &ParsedResult) -> Result<NativeOwnerAnalysis, String> {
    if !parsed
        .backend
        .languages
        .iter()
        .any(|language| matches!(language.as_str(), "typescript" | "tsx"))
    {
        return Err("TypeScript analysis requires a TypeScript or TSX parser".into());
    }
    let nodes = parsed.normalized_nodes()?;
    let root =
        parsed.document.output().root_id.as_deref().ok_or("TypeScript parse omitted its root")?;
    let source = std::str::from_utf8(parsed.source.bytes()).map_err(|error| error.to_string())?;
    let analysis =
        ast_merge::project_named_top_level_analysis(source, root, &nodes, super::owner_policy())?;
    analysis.validate(parsed)?;
    Ok(analysis)
}

pub fn owners(parsed: &ParsedResult) -> Result<SourcePreservingOwnerDocument, String> {
    analysis(parsed).map(|analysis| analysis.document)
}

pub type TypeScriptMergeExecution = ast_merge::typed_merge::ParsedOwnerMergeExecution;

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<TypeScriptMergeExecution, String> {
    // TypeScript uses the generic owner engine, unlike the Go/Rust family guards.
    ast_merge::typed_merge::merge_parsed_sources(
        base,
        ours,
        theirs,
        ast_merge::typed_merge::NativeOwnerEngine {
            analyze: owners,
            merge: |base, ours, theirs, verify| {
                ast_merge::merge_source_preserving_owners_with_evidence(base, ours, theirs, verify)
            },
        },
        parse_output,
    )
}
