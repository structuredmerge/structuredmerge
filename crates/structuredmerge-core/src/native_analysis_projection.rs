//! Projection of executed native owner analysis, not parser-supplied ownership.
use crate::{CoreParseResult, Metadata, SourceSpan, operation_result::ResultAnalysis};
use ast_merge::native_analysis::NativeOwnerAnalysis;
use serde::Serialize;
use tree_haver::service::ParsedResult;

pub(crate) fn supports(policy: &crate::operation::AnalyzePolicy) -> bool {
    policy.extra.is_empty()
        && policy.analysis_depth.as_deref().is_none_or(|depth| depth == "exact-source-owners")
        && policy.comments != Some(true)
        && policy.tokens != Some(true)
        && policy.ownership != Some(false)
        && policy.native_extensions != Some(false)
}

/// Reconstruct family decisions from validated embedded syntax, not from the
/// result's owner/layout claims. This checks evidence consistency; it does not
/// authenticate a foreign parser or prove a registry snapshot was executed.
pub(crate) fn validate_embedded(
    result: &crate::operation_result::OperationResult,
    request: &crate::operation::ValidatedOperationRequest,
) -> Result<(), crate::operation_result::ResultContractError> {
    use crate::operation_result::ResultContractError::InvalidSourceEvidence as Invalid;
    let (family, provider) = match result.profile.profile_id.as_deref() {
        Some(crate::profiles::YAML_MAPPING) => ("yaml", "kernel.yaml"),
        Some(crate::profiles::PYTHON_DECLARATIONS) => ("python", "kernel.python"),
        Some(crate::profiles::BASH_OWNERS) => ("bash", "kernel.bash"),
        Some(crate::profiles::GO_OWNERS) => ("go", "kernel.go"),
        Some(crate::profiles::RUST_OWNERS) => ("rust", "kernel.rust"),
        _ => return Ok(()), // Other profiles need their own analysis validator.
    };
    let analysis = result.analysis.as_ref().ok_or(Invalid)?;
    let crate::operation::OperationPolicy::Analyze(policy) = &request.request().operation else {
        return Err(Invalid);
    };
    if !supports(policy)
        || !result.ok
        || result.provider.family.as_deref() != Some(family)
        || result.provider.provider_id.as_deref() != Some(provider)
        || request
            .request()
            .provider_selection
            .dialect
            .as_deref()
            .is_some_and(|dialect| !matches!(family, "bash" | "go" | "rust") || dialect != family)
        || !request.request().provider_selection.extra.is_empty()
        || request
            .request()
            .provider_selection
            .required_capabilities
            .iter()
            .any(|capability| capability != "analyze")
        || request.request().parser_selection.profile_id.is_some()
        || request.request().parser_selection.language_version.is_some()
        || !request.request().parser_selection.extra.is_empty()
        || request.request().extensions.iter().any(|extension| !extension.capabilities.is_empty())
    {
        return Err(Invalid);
    }
    let core: CoreParseResult =
        serde_json::from_value(analysis.extra.get("parse_result").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    let selection = &request.request().parser_selection;
    let parser = result.profile.parser.as_ref().ok_or(Invalid)?;
    if core.schema != crate::service::PARSE_RESULT_SCHEMA
        || !core.parsed.ok
        || core.backend.id.is_empty()
        || core.selection.selected_backend.as_deref() != Some(&core.backend.id)
        || core.selection.requested.backend_id != selection.backend
        || core.selection.requested.preference != selection.preference
        || core.selection.requested.required_capabilities != selection.required_capabilities
        || selection.backend.as_ref().is_some_and(|id| id != &core.backend.id)
        || parser.selected_backend.as_ref() != Some(&core.backend.id)
        || parser.requested_backend != selection.backend
        || parser.selection_mode.as_deref()
            != Some(if selection.backend.is_some() { "explicit" } else { "policy" })
        || !core.backend.languages.iter().any(|language| language == family)
        || !core
            .backend
            .contracts
            .iter()
            .any(|schema| schema == crate::service::PARSE_RESULT_SCHEMA)
        || !core.backend.capabilities.iter().any(|capability| capability == "native_extensions")
        || selection
            .required_capabilities
            .iter()
            .any(|capability| !core.backend.capabilities.contains(capability))
    {
        return Err(Invalid);
    }
    let selected: Vec<_> =
        core.selection.candidates.iter().filter(|candidate| candidate.selected).collect();
    if selected.len() != 1
        || selected[0].backend_id != core.backend.id
        || !selected[0].rejections.is_empty()
        || selected[0].available != Some(true)
        || selected[0].loadable != Some(true)
        || selected[0].probe_fault.is_some()
    {
        return Err(Invalid);
    }
    let source_id =
        &request.request().sources.get(&crate::SourceRole::Source).ok_or(Invalid)?.source_id;
    let source = request.sources().get(source_id).map_err(|_| Invalid)?.clone();
    let parse_id = core.parsed.request_id.clone();
    let limits = crate::parsed::ParseValidationLimits {
        max_nodes: core.parsed.nodes.len(),
        max_diagnostics: core.parsed.diagnostics.len(),
        partial_tree_allowed: false,
        comments_supported: core
            .backend
            .capabilities
            .iter()
            .any(|capability| capability == "comments"),
    };
    let document = crate::parsed::ParsedDocument::validate(core.parsed, &parse_id, &source, limits)
        .map_err(|_| Invalid)?;
    let parsed = ParsedResult {
        schema: core.schema,
        selection: core.selection,
        backend: core.backend,
        document,
        source,
    };
    let owners = match family {
        "yaml" => yaml_merge::typed::mapping_analysis(&parsed),
        "python" => python_merge::declaration_analysis(&parsed),
        "bash" => bash_merge::typed::analysis(&parsed),
        "go" => go_merge::typed::analysis(&parsed),
        "rust" => rust_merge::typed::analysis(&parsed),
        _ => unreachable!(),
    }
    .map_err(|_| Invalid)?;
    let expected = project(&parsed, &owners, family).map_err(|_| Invalid)?;
    let mut expected = serde_json::to_value(expected).map_err(|_| Invalid)?;
    // Passive compatible analysis extensions are retained, not mistaken for
    // known ownership decisions. Deserialization still checks their shape.
    let extensions: Vec<crate::NativeExtension> =
        serde_json::from_value(analysis.extra.get("extensions").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    crate::parsed::validate_extensions(&extensions).map_err(|_| Invalid)?;
    expected.as_object_mut().ok_or(Invalid)?.remove("extensions");
    if !contains_expected(&serde_json::to_value(analysis).map_err(|_| Invalid)?, &expected) {
        return Err(Invalid);
    }
    Ok(())
}

fn contains_expected(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    match (actual, expected) {
        (serde_json::Value::Object(actual), serde_json::Value::Object(expected)) => {
            expected.iter().all(|(key, value)| {
                actual.get(key).is_some_and(|actual| contains_expected(actual, value))
            })
        }
        (serde_json::Value::Array(actual), serde_json::Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| contains_expected(actual, expected))
        }
        _ => actual == expected,
    }
}

#[derive(Serialize)]
struct Owner {
    id: String,
    node_id: String,
    node_ids: Vec<String>,
    logical_identity: Vec<String>,
    match_keys: Vec<String>,
    span: SourceSpan,
    source_sha256: String,
    metadata: Metadata,
}

#[derive(Serialize)]
struct Gap {
    id: String,
    kind: &'static str,
    span: SourceSpan,
    source_sha256: String,
    before_owner_id: Option<String>,
    after_owner_id: Option<String>,
    controller_side: &'static str,
    fallback_controller_side: Option<String>,
    metadata: Metadata,
}

#[derive(Serialize)]
struct Attachment {
    owner_id: String,
    leading_comment_region_ids: Vec<String>,
    inline_comment_region_ids: Vec<String>,
    trailing_comment_region_ids: Vec<String>,
    orphan_comment_region_ids: Vec<String>,
    leading_gap_id: Option<String>,
    trailing_gap_id: Option<String>,
}

#[derive(Serialize)]
struct Ownership {
    subject_ref: String,
    selected_owner_ref: String,
    relation: &'static str,
    basis: &'static str,
    confidence: &'static str,
    alternatives: Vec<String>,
}

#[derive(Serialize)]
struct Analysis {
    schema: &'static str,
    request_id: String,
    ok: bool,
    parse_result_ref: String,
    parse_result: CoreParseResult,
    owners: Vec<Owner>,
    comment_regions: Vec<Metadata>,
    layout_gaps: Vec<Gap>,
    attachments: Vec<Attachment>,
    ownership: Vec<Ownership>,
    diagnostics: Vec<crate::ParseDiagnostic>,
    extensions: Vec<crate::NativeExtension>,
    metadata: Metadata,
}

fn span(parsed: &ParsedResult, range: crate::ByteRange) -> Result<SourceSpan, String> {
    parsed.source.slice(range.clone()).map_err(|error| error.to_string())?;
    Ok(SourceSpan {
        start_point: parsed.source.point(range.start_byte).map_err(|error| error.to_string())?,
        end_point: parsed.source.point(range.end_byte).map_err(|error| error.to_string())?,
        range,
    })
}

/// Comments are intentionally unsupported by this bounded analysis policy.
/// Exact source gaps may contain comments, but are not classified as comments.
pub(crate) fn project(
    parsed: &ParsedResult,
    analysis: &NativeOwnerAnalysis,
    family: &str,
) -> Result<ResultAnalysis, String> {
    analysis.validate(parsed)?;
    // Bash retains native comments in the embedded parse and exact layout gaps.
    // This exact-owner profile does not request semantic comment attachment.
    if !matches!(family, "bash" | "go" | "rust") && !parsed.document.output().comments.is_empty() {
        return Err("native comment attachment requires another analysis policy".into());
    }
    let mut owners = vec![];
    for owner in &analysis.document.owners {
        let nodes = &analysis.owner_node_ids[&owner.id];
        let range = crate::ByteRange { start_byte: owner.start_byte, end_byte: owner.end_byte };
        owners.push(Owner {
            id: owner.id.clone(),
            node_id: nodes[0].clone(),
            node_ids: nodes.clone(),
            logical_identity: vec![family.into(), owner.id.clone()],
            match_keys: vec![owner.id.clone()],
            span: span(parsed, range.clone())?,
            source_sha256: parsed.source.range_digest(range).map_err(|error| error.to_string())?,
            metadata: [(
                "node_reference_policy".into(),
                serde_json::json!("family-native-boundaries"),
            )]
            .into(),
        });
    }
    let mut layout_gaps = vec![];
    let mut ownership = vec![];
    for gap in analysis.layout_gaps()?.into_iter().filter(|gap| !gap.range.is_empty()) {
        let controller = gap
            .controller_owner_id
            .ok_or("ownerless nonempty layout requires an explicit document ownership policy")?;
        let (kind, side) = match (&gap.before_owner_id, &gap.after_owner_id) {
            (None, Some(_)) => ("preamble", "after"),
            (Some(_), Some(_)) => ("interstitial", "after"),
            (Some(_), None) => ("postlude", "before"),
            (None, None) => return Err("unowned layout gap".into()),
        };
        ownership.push(Ownership {
            subject_ref: gap.id.clone(),
            selected_owner_ref: controller,
            relation: "controls-output",
            basis: "exact-source-baseline-emission",
            confidence: "deterministic",
            alternatives: vec![],
        });
        layout_gaps.push(Gap {
            id: gap.id,
            kind,
            span: span(parsed, gap.range)?,
            source_sha256: gap.source_sha256,
            before_owner_id: gap.before_owner_id,
            after_owner_id: gap.after_owner_id,
            controller_side: side,
            fallback_controller_side: None,
            metadata: [("deletion_policy".into(), serde_json::json!("no-implicit-transfer"))]
                .into(),
        });
    }
    let attachments = analysis
        .layout_attachments()?
        .into_iter()
        .map(|attachment| Attachment {
            owner_id: attachment.owner_id,
            leading_comment_region_ids: vec![],
            inline_comment_region_ids: vec![],
            trailing_comment_region_ids: vec![],
            orphan_comment_region_ids: vec![],
            leading_gap_id: attachment.leading_gap_id,
            trailing_gap_id: attachment.trailing_gap_id,
        })
        .collect();
    let request_id = parsed.document.output().request_id.clone();
    let result = Analysis {
        schema: "structuredmerge.analysis-result/v1",
        request_id: request_id.clone(),
        ok: true,
        parse_result_ref: request_id,
        parse_result: parsed.clone().into(),
        owners,
        comment_regions: vec![],
        layout_gaps,
        attachments,
        ownership,
        diagnostics: parsed.document.output().diagnostics.clone(),
        extensions: vec![],
        metadata: [
            ("analysis_depth".into(), serde_json::json!("exact-source-owners")),
            ("comment_analysis".into(), serde_json::json!("not-requested")),
            ("token_analysis".into(), serde_json::json!("not-requested")),
        ]
        .into(),
    };
    // The common envelope preserves compatible fields. Projection is typed;
    // this conversion does not transport an opaque whole-operation JSON string.
    serde_json::from_value(serde_json::to_value(result).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}
