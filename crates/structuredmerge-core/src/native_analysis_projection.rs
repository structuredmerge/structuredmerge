//! Projection of executed native owner analysis, not parser-supplied ownership.
use crate::{CoreParseResult, Metadata, SourceSpan, operation_result::ResultAnalysis};
use ast_merge::native_analysis::NativeOwnerAnalysis;
use serde::Serialize;
use tree_haver::service::ParsedResult;

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
    if !parsed.document.output().comments.is_empty() {
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
