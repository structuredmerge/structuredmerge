//! Source and registry consistency for successful common native merge results.
//! This does not authenticate parser facts or independently prove merge semantics.
use crate::{operation_result::*, *};

pub(crate) fn validate(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    if !result.ok
        || !matches!(result.operation, OperationKind::Merge2 | OperationKind::Merge3)
        || !matches!(
            result.profile.profile_id.as_deref(),
            Some(
                profiles::YAML_MAPPING
                    | profiles::PYTHON_DECLARATIONS
                    | profiles::BASH_OWNERS
                    | profiles::GO_OWNERS
                    | profiles::RUST_OWNERS
                    | profiles::TYPESCRIPT_OWNERS
            )
        )
    {
        return Ok(());
    }
    let inputs: Vec<CoreParseResult> =
        serde_json::from_value(result.extra.get("input_parses").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    let roles = result.operation.source_roles();
    if inputs.len() != roles.len() || inputs.is_empty() {
        return Err(Invalid);
    }
    let first = &inputs[0].selection;
    for (parsed, role) in inputs.iter().zip(roles) {
        let source = request
            .sources()
            .get(&request.request().sources.get(role).ok_or(Invalid)?.source_id)
            .map_err(|_| Invalid)?;
        validate_parse(parsed, source)?;
        if parsed.selection.generation != first.generation
            || parsed.selection.digest != first.digest
        {
            return Err(Invalid);
        }
    }
    let output: CoreParseResult =
        serde_json::from_value(result.extra.get("output_parse").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    let text = result.output.as_ref().ok_or(Invalid)?;
    if output.parsed.source.role != SourceRole::Output
        || request
            .request()
            .sources
            .values()
            .any(|source| source.source_id == output.parsed.source.source_id)
        || output.selection.generation != first.generation
        || output.selection.digest != first.digest
        || !inputs.iter().any(|input| input.backend == output.backend)
    {
        return Err(Invalid);
    }
    let source = SourceDocument::validate(
        SourceInput { descriptor: output.parsed.source.clone(), bytes: text.as_bytes().to_vec() },
        text.len() as u64,
    )
    .map_err(|_| Invalid)?;
    validate_parse(&output, &source)
}

fn validate_parse(
    core: &CoreParseResult,
    source: &SourceDocument,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let selected: Vec<_> =
        core.selection.candidates.iter().filter(|candidate| candidate.selected).collect();
    if core.schema != service::PARSE_RESULT_SCHEMA
        || !core.parsed.ok
        || core.selection.selected_backend.as_ref() != Some(&core.backend.id)
        || selected.len() != 1
        || selected[0].backend_id != core.backend.id
        || selected[0].available != Some(true)
        || selected[0].loadable != Some(true)
        || !selected[0].rejections.is_empty()
        || selected[0].probe_fault.is_some()
    {
        return Err(Invalid);
    }
    parsed::ParsedDocument::validate(
        core.parsed.clone(),
        &core.parsed.request_id,
        source,
        parsed::ParseValidationLimits {
            max_nodes: core.parsed.nodes.len(),
            max_diagnostics: core.parsed.diagnostics.len(),
            partial_tree_allowed: false,
            comments_supported: core
                .backend
                .capabilities
                .iter()
                .any(|capability| capability == "comments"),
        },
    )
    .map_err(|_| Invalid)?;
    Ok(())
}
