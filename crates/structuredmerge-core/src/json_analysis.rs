//! Common projection of Rust JSON family analysis, retaining native parse facts.
use crate::{operation_result::*, *};
use serde::Serialize;
use tree_haver::service::ParsedResult;

pub(crate) fn supports(policy: &operation::AnalyzePolicy) -> bool {
    policy.extra.is_empty()
        && policy.analysis_depth.as_deref().is_none_or(|p| p == "exact-source-owners")
        && policy.comments != Some(false)
        && policy.ownership != Some(false)
        && policy.native_extensions != Some(false)
        && policy.tokens != Some(true)
}

#[derive(Serialize)]
struct Owner {
    #[serde(flatten)]
    fact: json_merge::typed::JsonOwnerFact,
    node_ids: Vec<String>,
    logical_identity: Vec<String>,
    match_keys: Vec<String>,
    source_sha256: String,
}

#[derive(Serialize)]
struct Analysis {
    schema: &'static str,
    request_id: String,
    ok: bool,
    parse_result_ref: String,
    parse_result: CoreParseResult,
    owners: Vec<Owner>,
    comment_regions: Vec<json_merge::typed_analysis::CommentRegion>,
    family_comment_regions: Vec<json_merge::typed_analysis::FamilyCommentRegion>,
    layout_gaps: Vec<json_merge::typed_analysis::LayoutGap>,
    attachments: Vec<json_merge::typed_analysis::Attachment>,
    ownership: Vec<json_merge::typed_analysis::Ownership>,
    diagnostics: Vec<ParseDiagnostic>,
    extensions: Vec<NativeExtension>,
    metadata: Metadata,
}

pub(crate) fn project(
    parsed: &ParsedResult,
    dialect: json_merge::JsonDialect,
) -> Result<ResultAnalysis, String> {
    let facts = json_merge::typed_analysis::analyze(parsed, dialect)?;
    let mut diagnostics = parsed.document.output().diagnostics.clone();
    for (index, node_id) in facts.unresolved_comment_node_ids.iter().enumerate() {
        let node = parsed.document.node(node_id).ok_or("unresolved comment diagnostic")?;
        let mut id = format!("json.analysis.comment.{index}");
        while diagnostics.iter().any(|diagnostic| diagnostic.id == id) {
            id.push('_');
        }
        diagnostics.push(ParseDiagnostic {
            id,
            severity: parsed::ParseSeverity::Warning,
            category: "ambiguous_attachment".into(),
            code: Some("json.comment_attachment_unresolved".into()),
            message: "Native comment retained at document scope; family attachment is unresolved"
                .into(),
            source_role: parsed.source.descriptor().role,
            span: Some(node.span.clone()),
            node_id: Some(node_id.clone()),
            blocking: false,
            metadata: Metadata::new(),
            extra: Metadata::new(),
        });
    }
    let owners = facts
        .owners
        .into_iter()
        .map(|fact| {
            let mut nodes = vec![fact.node_id.clone()];
            if fact.value_node_id != fact.node_id {
                nodes.push(fact.value_node_id.clone());
            }
            Owner {
                node_ids: nodes,
                logical_identity: vec!["json".into(), fact.path.clone()],
                match_keys: fact.match_key.iter().cloned().collect(),
                source_sha256: fact.sha256.clone(),
                fact,
            }
        })
        .collect();
    let request_id = parsed.document.output().request_id.clone();
    let analysis = Analysis {
        schema: "structuredmerge.analysis-result/v1",
        request_id: request_id.clone(),
        ok: true,
        parse_result_ref: request_id,
        parse_result: parsed.clone().into(),
        owners,
        comment_regions: facts.comment_regions,
        family_comment_regions: facts.family_comment_regions,
        layout_gaps: facts.layout_gaps,
        attachments: facts.attachments,
        ownership: facts.ownership,
        diagnostics,
        extensions: vec![],
        metadata: [
            ("analysis_depth".into(), serde_json::json!("exact-source-owners")),
            ("layout_policy".into(), serde_json::json!("existing-family-blank-runs")),
            ("overlapping_subjects".into(), serde_json::json!(true)),
            (
                "unresolved_comment_node_ids".into(),
                serde_json::json!(facts.unresolved_comment_node_ids),
            ),
            ("token_analysis".into(), serde_json::json!("not-requested")),
        ]
        .into(),
    };
    serde_json::from_value(serde_json::to_value(analysis).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub(crate) fn validate(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let input = request.request();
    let OperationPolicy::Analyze(policy) = &input.operation else {
        return Err(Invalid);
    };
    if !supports(policy)
        || result.provider.provider_id.as_deref() != Some("kernel.json")
        || result.provider.family.as_deref() != Some("json")
        || input.provider_selection.provider_id.as_deref().is_some_and(|p| p != "kernel.json")
        || input.provider_selection.family.as_deref().is_some_and(|p| p != "json")
        || !input.provider_selection.extra.is_empty()
        || input.provider_selection.required_capabilities.iter().any(|c| c != "analyze")
        || input.parser_selection.profile_id.is_some()
        || input.parser_selection.language_version.is_some()
        || !input.parser_selection.extra.is_empty()
        || input.extensions.iter().any(|extension| !extension.capabilities.is_empty())
        || result.output.is_some()
        || !result.changes.is_empty()
        || !result.render_report.is_empty()
        || result.verification.output_reparsed == Some(true)
        || result.verification.structural_equivalence == Some(true)
    {
        return Err(Invalid);
    }
    let dialect = match input.provider_selection.dialect.as_deref().unwrap_or("json") {
        "json" => json_merge::JsonDialect::Json,
        "jsonc" => json_merge::JsonDialect::Jsonc,
        "json5" => json_merge::JsonDialect::Json5,
        _ => return Err(Invalid),
    };
    let parses = crate::json_diff::validated_parses(result, request, dialect)?;
    if ["comments", "native_extensions", "diagnostics"]
        .iter()
        .any(|required| !parses[0].backend.capabilities.iter().any(|c| c == required))
    {
        return Err(Invalid);
    }
    let expected = project(&parses[0], dialect).map_err(|_| Invalid)?;
    let actual = result.analysis.as_ref().ok_or(Invalid)?;
    let extensions: Vec<NativeExtension> =
        serde_json::from_value(actual.extra.get("extensions").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    parsed::validate_extensions(&extensions).map_err(|_| Invalid)?;
    let mut expected = serde_json::to_value(expected).map_err(|_| Invalid)?;
    expected.as_object_mut().ok_or(Invalid)?.remove("extensions");
    if !crate::json_diff::contains_fields(
        &expected,
        &serde_json::to_value(actual).map_err(|_| Invalid)?,
    ) {
        return Err(Invalid);
    }
    Ok(())
}
