//! Git-specific policy and evidence on the common operation carrier. Neither
//! host adapters nor the carrier infer merge decisions from rendered text.
use crate::{operation_result::*, *};
use ast_merge_git::typed::{ConflictRenderEvidence, ConflictRenderOptions};

pub(crate) fn options(policy: &OperationPolicy) -> Result<ConflictRenderOptions, String> {
    let OperationPolicy::Merge3(policy) = policy else {
        return Err("Git profile supports merge3 only".into());
    };
    if policy.render_policy != "source-preserving"
        || !policy.extra.is_empty()
        || policy.fallback_policy.as_deref().is_some_and(|p| p != "none")
    {
        return Err("unsupported Git merge policy".into());
    }
    let mut options = ConflictRenderOptions::default();
    if let Some(size) = policy.conflict_marker_size {
        options.marker_size = usize::try_from(size).map_err(|e| e.to_string())?;
    }
    for (role, label) in policy.labels.iter().flatten() {
        match role.as_str() {
            "base" => options.labels.base = label.clone(),
            "ours" => options.labels.ours = label.clone(),
            "theirs" => options.labels.theirs = label.clone(),
            _ => return Err("unknown Git conflict label role".into()),
        }
    }
    options.validate()?;
    Ok(options)
}

pub(crate) fn project_render(
    result: &mut OperationResult,
    render: Option<ConflictRenderEvidence>,
    error: Option<String>,
) {
    result.render_report = [
        (
            "strategy".into(),
            serde_json::json!(if render.is_some() {
                "git-localized-conflict-review"
            } else {
                "git-unrendered-conflict"
            }),
        ),
        ("artifact_kind".into(), serde_json::json!("unresolved-conflict-review")),
        ("outside_conflicts".into(), serde_json::json!("ours-not-partially-merged")),
        ("evidence".into(), serde_json::to_value(&render).unwrap()),
        ("render_error".into(), serde_json::to_value(error).unwrap()),
    ]
    .into();
    result.conflicted_output = render.map(|evidence| evidence.rendered.content);
    // Markers are not parsed as JSON; do not fabricate a failed reparse either.
    result.verification.output_reparsed = None;
    result.verification.structural_equivalence = None;
}

pub(crate) fn validate(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    // Selection/parser failures may precede classification. They must not carry
    // an output or rendering claim from this profile.
    if result.verification.classification_reached != Some(true) {
        return if !result.ok
            && result.output.is_none()
            && result.conflicted_output.is_none()
            && result.conflicts.is_empty()
            && result.render_report.is_empty()
        {
            Ok(())
        } else {
            Err(Invalid)
        };
    }
    let input = request.request();
    let options = options(&input.operation).map_err(|_| Invalid)?;
    if result.operation != OperationKind::Merge3
        || result.provider.provider_id.as_deref() != Some("kernel.git.json")
        || result.profile.profile_id.as_deref() != Some(crate::profiles::GIT_JSON)
        || result.verification.base_participated != Some(true)
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
    if result.ok {
        return Ok(());
    } // Clean edit/output-parse evidence checked by json_operation.
    let execution =
        ast_merge_git::typed::merge3(&parses[0], &parses[1], &parses[2], dialect, &options, |_| {
            Err("conflict validation cannot authorize clean output".into())
        })
        .map_err(|_| Invalid)?;
    if execution.merge.result.conflicts.is_empty() {
        return if result.conflicts.is_empty()
            && result.conflicted_output.is_none()
            && result.render_report.is_empty()
        {
            Ok(())
        } else {
            Err(Invalid)
        };
    }
    let mut expected = crate::native_operation::empty_result(request);
    project_render(&mut expected, execution.conflict_render, execution.conflict_render_error);
    for conflict in execution.merge.result.conflicts {
        crate::json_operation::project_conflict(&mut expected, request, conflict)
            .map_err(|_| Invalid)?;
    }
    if result.conflicted_output != expected.conflicted_output
        || result.output.is_some()
        || result.verification.output_reparsed.is_some()
        || result.verification.structural_equivalence.is_some()
        || result.verification.preservation.is_some()
        || result.verification.retained_source_regions.is_some()
    {
        return Err(Invalid);
    }
    for (expected, actual) in [
        (serde_json::to_value(expected.render_report), serde_json::to_value(&result.render_report)),
        (serde_json::to_value(expected.conflicts), serde_json::to_value(&result.conflicts)),
    ] {
        if !crate::json_diff::contains_fields(
            &expected.map_err(|_| Invalid)?,
            &actual.map_err(|_| Invalid)?,
        ) {
            return Err(Invalid);
        }
    }
    Ok(())
}
