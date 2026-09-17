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
    validate_parse(&output, &source)?;
    validate_partition(result, request, text.as_bytes())
}

fn validate_partition(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
    output: &[u8],
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    use ast_merge::{SourceRevision, byte_evidence, directional_render};
    let source = |role| {
        request
            .sources()
            .get(&request.request().sources.get(&role).ok_or(Invalid)?.source_id)
            .map_err(|_| Invalid)
    };
    let raw = result.render_report.get("source_segments").ok_or(Invalid)?;
    let regions = result.verification.retained_source_regions.as_ref().ok_or(Invalid)?;
    let strategy;
    if result.operation == OperationKind::Merge2 {
        strategy = "directional-source-plan";
        let segments: Vec<directional_render::DirectionalByteSegment> =
            serde_json::from_value(raw.clone()).map_err(|_| Invalid)?;
        directional_render::verify_directional_segments(
            output,
            source(SourceRole::Incoming)?,
            source(SourceRole::Current)?,
            &segments,
        )
        .map_err(|_| Invalid)?;
        if regions.len() != segments.len() {
            return Err(Invalid);
        }
        for (region, segment) in regions.iter().zip(&segments) {
            validate_region(
                region,
                &segment.source_id,
                segment.source_role,
                &segment.source_range,
                &segment.output_range,
                &segment.sha256,
            )?;
        }
        validate_property(result, "all-current-bytes-retained")?;
    } else {
        strategy = "source-plan";
        let segments: Vec<byte_evidence::SourceByteSegment> =
            serde_json::from_value(raw.clone()).map_err(|_| Invalid)?;
        let sources = [
            (SourceRevision::Base, source(SourceRole::Base)?.bytes()),
            (SourceRevision::Ours, source(SourceRole::Ours)?.bytes()),
            (SourceRevision::Theirs, source(SourceRole::Theirs)?.bytes()),
        ]
        .into();
        byte_evidence::verify_source_byte_segments(output, &sources, &segments)
            .map_err(|_| Invalid)?;
        if regions.len() != segments.len() {
            return Err(Invalid);
        }
        for (region, segment) in regions.iter().zip(&segments) {
            let role = match segment.revision {
                SourceRevision::Base => SourceRole::Base,
                SourceRevision::Ours => SourceRole::Ours,
                SourceRevision::Theirs => SourceRole::Theirs,
            };
            validate_region(
                region,
                &request.request().sources[&role].source_id,
                role,
                &segment.source_range,
                &segment.output_range,
                &segment.sha256,
            )?;
        }
    }
    if result.render_report.get("strategy").and_then(|value| value.as_str()) != Some(strategy)
        || result.render_report.get("producer").and_then(|value| value.as_str())
            != Some(result.provider.provider_id.as_deref().ok_or(Invalid)?)
    {
        return Err(Invalid);
    }
    validate_property(result, "exact-source-partition")
}

fn validate_region(
    region: &ResultSourceRegion,
    source_id: &str,
    role: SourceRole,
    source_range: &ByteRange,
    output_range: &ByteRange,
    sha256: &str,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let projected: ByteRange =
        serde_json::from_value(region.extra.get("output_range").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    if region.source_id != source_id
        || region.source_role != role
        || region.range.start_byte != source_range.start_byte
        || region.range.end_byte != source_range.end_byte
        || region.sha256.as_deref() != Some(sha256)
        || projected != *output_range
    {
        return Err(Invalid);
    }
    Ok(())
}

fn validate_property(result: &OperationResult, name: &str) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let mut properties = result
        .verification
        .preservation
        .as_ref()
        .ok_or(Invalid)?
        .iter()
        .filter(|property| property.property == name);
    let property = properties.next().ok_or(Invalid)?;
    if !property.required || property.status != "passed" || properties.next().is_some() {
        return Err(Invalid);
    }
    Ok(())
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
