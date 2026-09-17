//! Exact-source JSON comparison over validated native facts, never host text lookup.
use crate::{operation_result::*, *};
use tree_haver::service::ParsedResult;

pub(crate) fn supports(policy: &operation::DiffPolicy) -> bool {
    policy.extra.is_empty()
        && policy.comparison_profile.as_deref().is_none_or(|p| p == "exact-source-owners")
        && policy
            .equivalence
            .as_ref()
            .is_none_or(|rules| rules.is_empty() || rules == &["exact-source"])
}

pub(crate) fn project(
    before: &ParsedResult,
    after: &ParsedResult,
    dialect: json_merge::JsonDialect,
) -> Result<(Vec<ResultChange>, ResultDiff), String> {
    let owners = json_merge::typed::diff_owner_sources(before, after, dialect)?;
    let mut changes = vec![];
    // Always compare the complete documents too. This summary covers comments,
    // delimiters and layout outside owner spans without inventing ownership.
    // It overlaps nested subjects and must not be interpreted as an edit script.
    if before.source.bytes() != after.source.bytes() {
        let mut states = Metadata::new();
        let mut spans = std::collections::BTreeMap::new();
        for (name, parsed) in [("before", before), ("after", after)] {
            let range = ByteRange { start_byte: 0, end_byte: parsed.source.bytes().len() };
            let span = SourceSpan {
                start_point: parsed.source.point(0).map_err(|e| e.to_string())?,
                end_point: parsed.source.point(range.end_byte).map_err(|e| e.to_string())?,
                range,
            };
            states.insert(
                name.into(),
                serde_json::json!({"source":parsed.source.descriptor(), "span":span}),
            );
            spans.insert(parsed.source.descriptor().role, result_span(&span)?);
        }
        changes.push(ResultChange {
            id: "json.change.document".into(),
            classification: "edited".into(),
            subject_ref: Some("json.document".into()),
            path: None,
            role_states: states,
            source_spans: spans,
            metadata: [("scope".into(), serde_json::json!("whole-document-summary"))].into(),
            extra: Metadata::new(),
        });
    }
    for change in owners {
        let mut source_spans = std::collections::BTreeMap::new();
        let mut role_states = Metadata::new();
        for (name, parsed, owner) in
            [("before", before, &change.before), ("after", after, &change.after)]
        {
            role_states.insert(
                name.into(),
                match owner {
                    Some(owner) => {
                        source_spans
                            .insert(parsed.source.descriptor().role, result_span(&owner.span)?);
                        serde_json::json!({"source": parsed.source.descriptor(), "owner":owner})
                    }
                    None => serde_json::Value::Null,
                },
            );
        }
        changes.push(ResultChange {
            id: format!("json.change.owner:{}", change.path),
            classification: change.classification,
            subject_ref: Some(format!("json:{}", change.path)),
            path: Some(change.path),
            role_states,
            source_spans,
            metadata: [("scope".into(), serde_json::json!("nested-owner-source"))].into(),
            extra: Metadata::new(),
        });
    }
    let diff = ResultDiff {
        change_ids: changes.iter().map(|change| change.id.clone()).collect(),
        extra: [
            ("comparison".into(), serde_json::json!("exact-source-owners")),
            ("document_bytes_compared".into(), serde_json::json!(true)),
            ("array_identity".into(), serde_json::json!("positional")),
            ("overlapping_subjects".into(), serde_json::json!(true)),
        ]
        .into(),
    };
    Ok((changes, diff))
}

fn result_span(span: &SourceSpan) -> Result<ResultSpan, String> {
    serde_json::from_value(serde_json::to_value(span).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Recompute comparison from validated embedded syntax and request bytes. This
/// verifies evidence consistency, not foreign parser authenticity or authority.
pub(crate) fn validate(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let input = request.request();
    let OperationPolicy::Diff2(policy) = &input.operation else {
        return Err(Invalid);
    };
    if !supports(policy)
        || result.provider.provider_id.as_deref() != Some("kernel.json")
        || result.provider.family.as_deref() != Some("json")
        || input.provider_selection.provider_id.as_deref().is_some_and(|id| id != "kernel.json")
        || input.provider_selection.family.as_deref().is_some_and(|id| id != "json")
        || !input.provider_selection.extra.is_empty()
        || input.provider_selection.required_capabilities.iter().any(|c| c != "diff2")
        || input.parser_selection.profile_id.is_some()
        || input.parser_selection.language_version.is_some()
        || !input.parser_selection.extra.is_empty()
        || input.extensions.iter().any(|extension| !extension.capabilities.is_empty())
        || result.output.is_some()
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
    let parses = validated_parses(result, request, dialect)?;
    let (changes, diff) = project(&parses[0], &parses[1], dialect).map_err(|_| Invalid)?;
    if changes.len() != result.changes.len()
        || changes.iter().zip(&result.changes).any(|(expected, actual)| {
            expected.path != actual.path
                || expected.subject_ref != actual.subject_ref
                || expected.source_spans.keys().ne(actual.source_spans.keys())
                || expected.role_states.keys().ne(actual.role_states.keys())
        })
    {
        return Err(Invalid);
    }
    for (expected, actual) in [
        (serde_json::to_value(changes), serde_json::to_value(&result.changes)),
        (serde_json::to_value(diff), serde_json::to_value(result.diff.as_ref().ok_or(Invalid)?)),
    ] {
        if !contains_fields(&expected.map_err(|_| Invalid)?, &actual.map_err(|_| Invalid)?) {
            return Err(Invalid);
        }
    }
    Ok(())
}

pub(crate) fn validated_parses(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
    dialect: json_merge::JsonDialect,
) -> Result<Vec<ParsedResult>, ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    let input = request.request();
    let roles = input.operation.kind().source_roles();
    let cores: Vec<CoreParseResult> =
        serde_json::from_value(result.extra.get("input_parses").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    if cores.len() != roles.len() || cores.is_empty() {
        return Err(Invalid);
    }
    // Execution parses the entire request against one immutable registry snapshot.
    // These fields prove internal consistency, not authenticity of a foreign report.
    let generation = cores[0].selection.generation;
    let digest = cores[0].selection.digest.clone();
    let selection = &input.parser_selection;
    let mut parses = vec![];
    for (core, role) in cores.into_iter().zip(roles) {
        let source = request
            .sources()
            .get(&input.sources.get(role).ok_or(Invalid)?.source_id)
            .map_err(|_| Invalid)?
            .clone();
        let language = if dialect == json_merge::JsonDialect::Json { "json" } else { "json5" };
        let candidates =
            core.selection.candidates.iter().filter(|c| c.selected).collect::<Vec<_>>();
        if core.schema != service::PARSE_RESULT_SCHEMA
            || core.selection.generation != generation
            || core.selection.digest != digest
            || !core.parsed.ok
            || core.parsed.request_id != format!("{}:{role:?}", input.request_id)
            || core.backend.id.is_empty()
            || !core.backend.languages.iter().any(|l| l == language)
            || !core.backend.contracts.iter().any(|c| c == service::PARSE_RESULT_SCHEMA)
            || core.selection.selected_backend.as_ref() != Some(&core.backend.id)
            || core.selection.requested.backend_id != selection.backend
            || core.selection.requested.preference != selection.preference
            || core.selection.requested.required_capabilities != selection.required_capabilities
            || selection.backend.as_ref().is_some_and(|id| id != &core.backend.id)
            || selection
                .required_capabilities
                .iter()
                .any(|c| !core.backend.capabilities.contains(c))
            || candidates.len() != 1
            || candidates[0].backend_id != core.backend.id
            || !candidates[0].rejections.is_empty()
            || candidates[0].available != Some(true)
            || candidates[0].loadable != Some(true)
            || candidates[0].probe_fault.is_some()
        {
            return Err(Invalid);
        }
        let limits = parsed::ParseValidationLimits {
            max_nodes: core.parsed.nodes.len(),
            max_diagnostics: core.parsed.diagnostics.len(),
            partial_tree_allowed: false,
            comments_supported: core.backend.capabilities.iter().any(|c| c == "comments"),
        };
        let document = parsed::ParsedDocument::validate(
            core.parsed.clone(),
            &core.parsed.request_id,
            &source,
            limits,
        )
        .map_err(|_| Invalid)?;
        parses.push(ParsedResult {
            schema: core.schema,
            selection: core.selection,
            backend: core.backend,
            document,
            source,
        });
    }
    let parser = result.profile.parser.as_ref().ok_or(Invalid)?;
    if parser.selected_backend.as_ref() != Some(&parses[0].backend.id)
        || parser.requested_backend != selection.backend
        || parser.selection_mode.as_deref()
            != Some(if selection.backend.is_some() { "explicit" } else { "policy" })
    {
        return Err(Invalid);
    }
    Ok(parses)
}

// Compatible unknown fields are passive and survive forwarding; required
// evidence is checked recursively rather than rejecting every extra key.
pub(crate) fn contains_fields(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => left
            .iter()
            .all(|(key, value)| right.get(key).is_some_and(|other| contains_fields(value, other))),
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(a, b)| contains_fields(a, b))
        }
        _ => expected == actual,
    }
}
