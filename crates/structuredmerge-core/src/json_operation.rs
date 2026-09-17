//! Explicit JSON-family execution through the shared TreeHaver snapshot.
use crate::{
    native_operation::{diagnostic, empty_result, finalize, retain_parses, service_failure},
    operation_result::*,
    portable_conflict::*,
    portable_diagnostic::*,
    *,
};
use tree_haver::service::{
    ExecutionContext, PARSE_REQUEST_SCHEMA, ParseService, ParserRegistrySnapshot,
    TreeHaverParseService,
};

/// Verify exported edit/retention evidence against request bytes. This does not
/// turn a foreign result into authority for the semantic decisions in its plan.
pub(crate) fn validate_render(
    result: &OperationResult,
    request: &ValidatedOperationRequest,
) -> Result<(), ResultContractError> {
    use ResultContractError::InvalidSourceEvidence as Invalid;
    if result.profile.profile_id.as_deref() != Some(crate::profiles::JSON_NESTED) || !result.ok {
        return Ok(());
    }
    if result.operation == OperationKind::Diff2 {
        return crate::json_diff::validate(result, request);
    }
    if result.operation == OperationKind::Analyze {
        return crate::json_analysis::validate(result, request);
    }
    let output = result.output.as_deref().ok_or(Invalid)?;
    let render: json_merge::render_evidence::JsonRenderEvidence =
        serde_json::from_value(result.render_report.get("evidence").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    let baseline = request.sources().get(&render.baseline.source_id).map_err(|_| Invalid)?;
    let expected_roles = match result.operation {
        OperationKind::Merge2 => vec![SourceRole::Current],
        OperationKind::Merge3 => vec![SourceRole::Ours, SourceRole::Theirs],
        _ => return Err(Invalid),
    };
    if !expected_roles.contains(&render.baseline.role)
        || result.render_report.get("strategy") != Some(&serde_json::json!("json-source-edits"))
    {
        return Err(Invalid);
    }
    render.validate(baseline, output).map_err(|_| Invalid)?;
    let retained = result.verification.retained_source_regions.as_ref().ok_or(Invalid)?;
    if retained.len() != render.retained.len() {
        return Err(Invalid);
    }
    for (reported, actual) in retained.iter().zip(&render.retained) {
        if reported.source_id != render.baseline.source_id
            || reported.source_role != render.baseline.role
            || reported.range.start_byte != actual.source_range.start_byte
            || reported.range.end_byte != actual.source_range.end_byte
            || reported.sha256.as_ref() != Some(&actual.sha256)
            || reported.extra.get("output_range")
                != Some(&serde_json::to_value(&actual.output_range).map_err(|_| Invalid)?)
        {
            return Err(Invalid);
        }
    }
    let core: CoreParseResult =
        serde_json::from_value(result.extra.get("output_parse").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    if !core.parsed.ok
        || core.schema != service::PARSE_RESULT_SCHEMA
        || core.parsed.source.role != SourceRole::Output
        || request
            .request()
            .sources
            .values()
            .any(|source| source.source_id == core.parsed.source.source_id)
        || core.selection.selected_backend.as_ref() != Some(&core.backend.id)
    {
        return Err(Invalid);
    }
    let inputs: Vec<CoreParseResult> =
        serde_json::from_value(result.extra.get("input_parses").ok_or(Invalid)?.clone())
            .map_err(|_| Invalid)?;
    let selected = inputs.get(1).ok_or(Invalid)?;
    if core.backend != selected.backend {
        return Err(Invalid);
    }
    let source = SourceDocument::validate(
        SourceInput { descriptor: core.parsed.source.clone(), bytes: output.as_bytes().to_vec() },
        output.len() as u64,
    )
    .map_err(|_| Invalid)?;
    let limits = parsed::ParseValidationLimits {
        max_nodes: core.parsed.nodes.len(),
        max_diagnostics: core.parsed.diagnostics.len(),
        partial_tree_allowed: false,
        comments_supported: core
            .backend
            .capabilities
            .iter()
            .any(|capability| capability == "comments"),
    };
    parsed::ParsedDocument::validate(core.parsed.clone(), &core.parsed.request_id, &source, limits)
        .map_err(|_| Invalid)?;
    Ok(())
}

pub(crate) fn execute(
    request: &ValidatedOperationRequest,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
) -> Result<OperationResult, CoreError> {
    let mut result = empty_result(request);
    let evidence = ConflictEvidence::default();
    let input = request.request();
    let finish = |result| finalize(result, request, &evidence, context);
    let dialect = match input.provider_selection.dialect.as_deref().unwrap_or("json") {
        "json" => json_merge::JsonDialect::Json,
        "jsonc" => json_merge::JsonDialect::Jsonc,
        "json5" => json_merge::JsonDialect::Json5,
        _ => {
            diagnostic(
                &mut result,
                PortableCategory::SelectionError,
                "json.unsupported_dialect",
                "unsupported JSON dialect",
                DiagnosticLayer::Registry,
                vec![],
            );
            return finish(result);
        }
    };
    let operation = match &input.operation {
        OperationPolicy::Analyze(policy) if crate::json_analysis::supports(policy) => "analyze",
        OperationPolicy::Diff2(policy) if crate::json_diff::supports(policy) => "diff2",
        OperationPolicy::Merge2(policy)
            if policy.directional_merge == "template-into-current"
                && policy.render_policy == "source-preserving"
                && policy.extra.is_empty()
                && policy.fallback_policy.as_deref().is_none_or(|p| p == "none") =>
        {
            "merge2"
        }
        OperationPolicy::Merge3(policy)
            if policy.render_policy == "source-preserving"
                && policy.labels.is_none()
                && policy.conflict_marker_size.is_none()
                && policy.extra.is_empty()
                && policy.fallback_policy.as_deref().is_none_or(|p| p == "none") =>
        {
            "merge3"
        }
        _ => "unsupported",
    };
    if operation == "unsupported"
        || input.provider_selection.provider_id.as_deref().is_some_and(|id| id != "kernel.json")
        || input.provider_selection.family.as_deref().is_some_and(|family| family != "json")
        || !input.provider_selection.extra.is_empty()
        || input.parser_selection.profile_id.is_some()
        || input.parser_selection.language_version.is_some()
        || !input.parser_selection.extra.is_empty()
        || input
            .provider_selection
            .required_capabilities
            .iter()
            .any(|capability| capability != operation)
        || input.extensions.iter().any(|extension| !extension.capabilities.is_empty())
    {
        diagnostic(
            &mut result,
            PortableCategory::UnsupportedFeature,
            "json.unsupported_requirements",
            "JSON profile cannot honor the requested operation or constraints",
            DiagnosticLayer::Registry,
            vec![],
        );
        return finish(result);
    }
    result.provider.provider_id = Some("kernel.json".into());
    result.provider.family = Some("json".into());
    result.profile.profile_id = Some(crate::profiles::JSON_NESTED.into());
    let language = if dialect == json_merge::JsonDialect::Json { "json" } else { "json5" };
    let roles = input.operation.kind().source_roles();
    let mut requests = vec![];
    for role in roles {
        let source = request
            .sources()
            .get(&input.sources[role].source_id)
            .map_err(|e| CoreError::new(e.to_string()))?;
        requests.push(ParseRequest {
            schema: PARSE_REQUEST_SCHEMA.into(),
            request_id: format!("{}:{role:?}", input.request_id),
            source: SourceInput {
                descriptor: source.descriptor().clone(),
                bytes: source.bytes().to_vec(),
            },
            language: language.into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: input.parser_selection.backend.clone(),
                preference: input.parser_selection.preference.clone(),
                required_capabilities: input.parser_selection.required_capabilities.clone(),
            },
            options: if operation == "analyze" {
                ParseOptions {
                    comments: true,
                    tokens: false,
                    diagnostics: true,
                    native_extensions: true,
                }
            } else {
                ParseOptions::default()
            },
            metadata: input.metadata.clone(),
            extra: Metadata::new(),
        });
    }
    let service = TreeHaverParseService::default();
    let parses = match service.parse_batch(requests.clone(), snapshot, context) {
        Ok(parses) => parses,
        Err(error) => {
            service_failure(&mut result, error);
            return finish(result);
        }
    };
    retain_parses(&mut result, &parses);
    for parsed in &parses {
        if !parsed.document.output().ok {
            diagnostic(
                &mut result,
                PortableCategory::ParseError,
                "json.parse_rejected",
                "JSON parser rejected input",
                DiagnosticLayer::Parser,
                vec![DiagnosticSourceRef {
                    source_id: parsed.source.descriptor().source_id.clone(),
                    role: parsed.source.descriptor().role,
                    span: None,
                    extra: Metadata::new(),
                }],
            );
        }
    }
    if !result.diagnostics.is_empty() {
        return finish(result);
    }
    if let Err(error) = context.check() {
        service_failure(&mut result, error);
        return finish(result);
    }
    if operation == "analyze" {
        match crate::json_analysis::project(&parses[0], dialect) {
            Ok(analysis) => {
                result.ok = true;
                result.analysis = Some(analysis);
                result.verification.classification_reached = Some(true);
                result.verification.consumed_source_roles = Some(roles.to_vec());
            }
            Err(_) => diagnostic(
                &mut result,
                PortableCategory::UnsupportedFeature,
                "json.analysis_rejected",
                "JSON family analysis rejected the parsed input",
                DiagnosticLayer::Analysis,
                vec![],
            ),
        }
        return finish(result);
    }
    if operation == "diff2" {
        match crate::json_diff::project(&parses[0], &parses[1], dialect) {
            Ok((changes, diff)) => {
                result.ok = true;
                result.changes = changes;
                result.diff = Some(diff);
                result.verification.classification_reached = Some(true);
                result.verification.consumed_source_roles = Some(roles.to_vec());
            }
            Err(_) => diagnostic(
                &mut result,
                PortableCategory::UnsupportedFeature,
                "json.analysis_rejected",
                "JSON family analysis rejected the parsed input",
                DiagnosticLayer::Analysis,
                vec![],
            ),
        }
        return finish(result);
    }
    let mut verification_error = None;
    let mut output_parse = None;
    let baseline_index = 1; // Current for merge2; Ours for merge3.
    let mut output_id = "json-output".to_string();
    while input.sources.values().any(|source| source.source_id == output_id) {
        output_id.push('_');
    }
    let mut verify = |output: &str| {
        let mut parse_request = requests[baseline_index].clone();
        parse_request.request_id = format!("{}:output", input.request_id);
        parse_request.source = source_input(
            output_id.clone(),
            SourceRole::Output,
            SourceEncoding::Utf8,
            output.as_bytes().to_vec(),
        )
        .map_err(|e| e.to_string())?;
        parse_request.selection.backend_id = Some(parses[baseline_index].backend.id.clone());
        match service.parse_batch(vec![parse_request], snapshot, context) {
            Ok(mut values) => {
                let parsed = values.remove(0);
                output_parse = Some(parsed.clone());
                Ok(parsed)
            }
            Err(error) => {
                verification_error = Some(error);
                Err("output parser service rejected verification".into())
            }
        }
    };
    let execution = if operation == "merge2" {
        json_merge::typed::merge2_with_evidence(&parses[0], &parses[1], dialect, &mut verify).map(
            |execution| {
                (execution.result.output, execution.result.diagnostics, vec![], execution.render)
            },
        )
    } else {
        json_merge::typed::merge3_with_evidence(
            &parses[0],
            &parses[1],
            &parses[2],
            dialect,
            &mut verify,
        )
        .map(|execution| {
            (
                execution.result.output,
                execution.result.diagnostics,
                execution.result.conflicts,
                execution.render,
            )
        })
    };
    let (output, diagnostics, conflicts, render) = match execution {
        Ok(execution) => execution,
        Err(_) => {
            diagnostic(
                &mut result,
                PortableCategory::UnsupportedFeature,
                "json.analysis_rejected",
                "JSON family analysis rejected the parsed input",
                DiagnosticLayer::Analysis,
                vec![],
            );
            return finish(result);
        }
    };
    result.verification.classification_reached = Some(true);
    result.verification.consumed_source_roles = Some(roles.to_vec());
    if operation == "merge2" {
        result.verification.directional_roles_preserved = Some(true);
    } else {
        result.verification.base_participated = Some(true);
    }
    result.extra.insert(
        "output_parse".into(),
        serde_json::to_value(output_parse.map(CoreParseResult::from)).unwrap(),
    );
    if let Some(error) = verification_error {
        service_failure(&mut result, error);
        return finish(result);
    }
    if !conflicts.is_empty() {
        for conflict in conflicts {
            project_conflict(&mut result, request, conflict)?;
        }
        return finish(result);
    }
    if let (Some(output), Some(render)) = (output, render) {
        let baseline = request
            .sources()
            .get(&render.baseline.source_id)
            .map_err(|e| CoreError::new(e.to_string()))?;
        render.validate(baseline, &output).map_err(CoreError::new)?;
        result.ok = true;
        result.output = Some(output);
        result.verification.output_reparsed = Some(true);
        result.verification.structural_equivalence = Some(true);
        result.verification.preservation = Some(vec![PreservationProperty {
            property: "exact-bytes-outside-executed-edits".into(),
            required: true,
            status: "passed".into(),
            extra: Metadata::new(),
        }]);
        result.verification.retained_source_regions = Some(
            render
                .retained
                .iter()
                .map(|region| ResultSourceRegion {
                    source_id: render.baseline.source_id.clone(),
                    source_role: render.baseline.role,
                    range: ResultRange {
                        start_byte: region.source_range.start_byte,
                        end_byte: region.source_range.end_byte,
                        extra: Metadata::new(),
                    },
                    sha256: Some(region.sha256.clone()),
                    extra: [(
                        "output_range".into(),
                        serde_json::to_value(&region.output_range).unwrap(),
                    )]
                    .into(),
                })
                .collect(),
        );
        result.render_report = [
            ("strategy".into(), serde_json::json!("json-source-edits")),
            ("evidence".into(), serde_json::to_value(render).unwrap()),
        ]
        .into();
    } else {
        result
            .extra
            .insert("native_diagnostics".into(), serde_json::to_value(diagnostics).unwrap());
        diagnostic(
            &mut result,
            PortableCategory::VerificationError,
            "json.merge_not_accepted",
            "JSON merge did not produce a verified render",
            DiagnosticLayer::Verifier,
            vec![],
        );
    }
    finish(result)
}

fn project_conflict(
    result: &mut OperationResult,
    request: &ValidatedOperationRequest,
    native: ast_merge::MergeConflict,
) -> Result<(), CoreError> {
    let mut alternatives = vec![];
    let mut localized = !native.alternatives.is_empty();
    for (revision, role) in [
        (SourceRevision::Base, SourceRole::Base),
        (SourceRevision::Ours, SourceRole::Ours),
        (SourceRevision::Theirs, SourceRole::Theirs),
    ] {
        let source_id = &request.request().sources[&role].source_id;
        let source = request.sources().get(source_id).map_err(|e| CoreError::new(e.to_string()))?;
        let alternative =
            native.alternatives.iter().find(|alternative| alternative.revision == revision);
        let state = match alternative.map(|alternative| alternative.state) {
            Some(ConflictAlternativeState::Present) => AlternativeState::Present,
            Some(ConflictAlternativeState::Absent) => AlternativeState::Absent,
            _ => {
                localized = false;
                AlternativeState::Opaque
            }
        };
        let mut regions = vec![];
        for region in alternative.into_iter().flat_map(|alternative| &alternative.regions) {
            let range = ByteRange { start_byte: region.start_byte, end_byte: region.end_byte };
            let bytes = source.slice(range.clone()).map_err(|e| CoreError::new(e.to_string()))?;
            regions.push(ExactConflictRegion {
                range: ResultRange {
                    start_byte: range.start_byte,
                    end_byte: range.end_byte,
                    extra: Metadata::new(),
                },
                byte_length: bytes.len() as u64,
                sha256: source.range_digest(range).map_err(|e| CoreError::new(e.to_string()))?,
                extra: Metadata::new(),
            });
        }
        alternatives.push(ConflictSourceAlternative {
            role,
            state,
            source_id: (state != AlternativeState::Absent).then(|| source_id.clone()),
            regions,
            change_ids: vec![],
            extra: Metadata::new(),
        });
    }
    diagnostic(
        result,
        PortableCategory::MergeConflict,
        "json.merge_conflict",
        "JSON family reported an unresolved structural conflict",
        DiagnosticLayer::Provider,
        vec![],
    );
    let diagnostic_id = result.diagnostics.last().unwrap().id().to_string();
    result.conflicts.push(ConflictRecord::Canonical(Box::new(PortableConflict {
        schema: CONFLICT_SCHEMA.into(),
        id: native.conflict_id.clone(),
        operation: OperationKind::Merge3,
        category: ConflictCategory::ProviderSpecific,
        code: "json.structural_conflict".into(),
        message: Some(native.message.clone()),
        subject: ConflictSubject {
            structural_path: (!native.path.is_empty()).then(|| native.path.clone()),
            owner_ref: None,
            node_ref: None,
            archive_entry: None,
            binary_region: None,
            whole_document: Some(native.path.is_empty()),
            extra: Metadata::new(),
        },
        roles: OperationKind::Merge3.source_roles().to_vec(),
        alternatives,
        classification: ConflictClassification {
            base_participated: Some(true),
            change_ids: vec![],
            decision_ids: vec![],
            extra: [("native_conflict".into(), serde_json::to_value(native).unwrap())].into(),
        },
        localization: ConflictLocalization {
            status: if localized {
                LocalizationStatus::Owner
            } else {
                LocalizationStatus::Unavailable
            },
            verified: localized,
            output_regions: vec![],
            extra: Metadata::new(),
        },
        resolution: ConflictResolution {
            status: ResolutionStatus::Unresolved,
            strategy: ResolutionStrategy::None,
            selected_roles: vec![],
            decision_id: None,
            resolver: None,
            reason: None,
            extra: Metadata::new(),
        },
        diagnostic_ids: vec![diagnostic_id],
        change_ids: vec![],
        decision_ids: vec![],
        render_fragment_ids: vec![],
        extensions: vec![],
        metadata: Metadata::new(),
        extra: Metadata::new(),
    })));
    Ok(())
}
