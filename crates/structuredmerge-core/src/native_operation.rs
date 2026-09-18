//! Explicit native-profile execution of the common request/result contract.
//! Not a replacement provider registry: profiles must be explicitly requested;
//! no family-default selection, host workflow merge, or fallback is introduced.

use crate::{
    CoreError, CoreParseResult, DiagnosticSeverity, Metadata, OperationKind, ParseRequest,
    ParserSelection, SourceEncoding, SourceInput, SourceRole,
    operation::{OperationPolicy, ValidatedOperationRequest},
    operation_result::*,
    portable_conflict::{ConflictEvidence, ConflictRecord},
    portable_diagnostic::*,
};
use ast_merge::{
    SourcePreservingOwnerDocument, SourceRevision, ThreeWayMergeOutcome,
    typed_merge::{NativeMergeError, NativeMergeExecution, merge_native_sources_with_evidence},
};
use tree_haver::service::{
    ExecutionContext, PARSE_REQUEST_SCHEMA, ParseService, ParsedResult, ParserRegistrySnapshot,
    ServiceError, TreeHaverParseService,
};

type Analyzer = fn(&ParsedResult) -> Result<SourcePreservingOwnerDocument, String>;

fn diff_span(
    region: &ast_merge::owner_diff::DiffSourceRegion,
    request: &ValidatedOperationRequest,
) -> Result<ResultSpan, CoreError> {
    let invalid = || CoreError {
        code: "operation.invalid_evidence".into(),
        message: "diff source region does not match the validated request".into(),
    };
    let input = request.request().sources.get(&region.source_role).ok_or_else(invalid)?;
    if input.source_id != region.source_id {
        return Err(invalid());
    }
    let document = request.sources().get(&region.source_id).map_err(|_| invalid())?;
    if document.range_digest(region.range.clone()).map_err(|_| invalid())? != region.sha256 {
        return Err(invalid());
    }
    let start = document.point(region.range.start_byte).map_err(|_| invalid())?;
    let end = document.point(region.range.end_byte).map_err(|_| invalid())?;
    Ok(ResultSpan {
        range: ResultRange {
            start_byte: region.range.start_byte,
            end_byte: region.range.end_byte,
            extra: Metadata::new(),
        },
        start_point: ResultPoint { row: start.row, column: start.column, extra: Metadata::new() },
        end_point: ResultPoint { row: end.row, column: end.column, extra: Metadata::new() },
        extra: Metadata::new(),
    })
}

pub(crate) fn empty_result(request: &ValidatedOperationRequest) -> OperationResult {
    let input = request.request();
    OperationResult {
        schema: OPERATION_RESULT_SCHEMA.into(),
        request_id: input.request_id.clone(),
        operation: input.operation.kind(),
        ok: false,
        provider: ResultProvider {
            provider_id: None,
            family: None,
            delegation: None,
            extra: Metadata::new(),
        },
        profile: ResultProfile { profile_id: None, parser: None, extra: Metadata::new() },
        diagnostics: vec![],
        changes: vec![],
        conflicts: vec![],
        fallbacks: vec![],
        render_report: Metadata::new(),
        verification: ResultVerification {
            consumed_source_roles: None,
            directional_roles_preserved: None,
            base_participated: None,
            classification_reached: Some(false),
            output_reparsed: None,
            structural_equivalence: None,
            preservation: None,
            retained_source_regions: None,
            extra: Metadata::new(),
        },
        analysis: None,
        diff: None,
        output: None,
        conflicted_output: None,
        extensions: input.extensions.clone(),
        metadata: input.metadata.clone(),
        // Request-only forward fields must not shadow result-reserved fields.
        extra: [("request_forwarding".into(), serde_json::json!({
            "extra": input.extra, "path_name": input.path_name, "policy": input.operation,
            "provider_selection": input.provider_selection, "parser_selection": input.parser_selection,
        }))].into(),
    }
}

pub(crate) fn diagnostic(
    result: &mut OperationResult,
    category: PortableCategory,
    code: &str,
    message: impl Into<String>,
    layer: DiagnosticLayer,
    source_refs: Vec<DiagnosticSourceRef>,
) {
    result.diagnostics.push(DiagnosticRecord::Canonical(PortableDiagnostic {
        schema: DIAGNOSTIC_SCHEMA.into(),
        id: format!("diagnostic.{}", result.diagnostics.len()),
        sequence: result.diagnostics.len() as u64,
        severity: DiagnosticSeverity::Error,
        category,
        code: code.into(),
        message: message.into(),
        blocking: true,
        operation: Some(result.operation),
        request_id: Some(result.request_id.clone()),
        source_refs,
        subject_refs: None,
        cause_ids: vec![],
        related_ids: vec![],
        origin: DiagnosticOrigin {
            layer,
            provider_id: result.provider.provider_id.clone(),
            backend_id: None,
            package: None,
            package_version: None,
            native_code: None,
            extra: Metadata::new(),
        },
        data: Metadata::new(),
        extensions: vec![],
        metadata: Metadata::new(),
        extra: Metadata::new(),
    }));
}

pub(crate) fn service_failure(result: &mut OperationResult, error: ServiceError) {
    let category = match &error {
        ServiceError::Cancelled => PortableCategory::Cancelled,
        ServiceError::DeadlineExceeded => PortableCategory::DeadlineExceeded,
        ServiceError::LimitExceeded
        | ServiceError::Source(crate::source::SourceError {
            code: crate::source::SourceErrorCode::LimitExceeded,
            ..
        })
        | ServiceError::InvalidResult {
            error: crate::parsed::ParseValidationError::LimitExceeded,
            ..
        } => PortableCategory::ResourceLimit,
        ServiceError::Selection(_) => PortableCategory::SelectionError,
        ServiceError::Source(_) | ServiceError::InvalidRequest => PortableCategory::InvalidRequest,
        _ => PortableCategory::InternalError,
    };
    let failure = crate::ParserFailure::from(error);
    // Do not copy native exception text into the public diagnostic message.
    diagnostic(
        result,
        category,
        &failure.code,
        "operation could not complete the parser service call",
        DiagnosticLayer::Runner,
        vec![],
    );
    let DiagnosticRecord::Canonical(record) = result.diagnostics.last_mut().unwrap() else {
        unreachable!()
    };
    record.origin.backend_id = failure.backend_id;
    record.origin.native_code = failure.native_code;
    if let Some(selection) = failure.selection {
        record.data.insert("selection".into(), serde_json::to_value(selection).unwrap());
    }
}

pub(crate) fn retain_parses(result: &mut OperationResult, parses: &[ParsedResult]) {
    if let Some(first) = parses.first() {
        result.profile.parser = Some(ResultParserSelection {
            requested_backend: first.selection.requested.backend_id.clone(),
            selected_backend: Some(first.backend.id.clone()),
            selection_mode: Some(
                if first.selection.requested.backend_id.is_some() { "explicit" } else { "policy" }
                    .into(),
            ),
            extra: Metadata::new(),
        });
    }
    result.extra.insert(
        "input_parses".into(),
        serde_json::to_value(parses.iter().cloned().map(CoreParseResult::from).collect::<Vec<_>>())
            .unwrap(),
    );
}

fn execution_failure(result: &mut OperationResult, error: NativeMergeError) {
    match error {
        NativeMergeError::Parse(error) | NativeMergeError::InputParseFailed { error, .. } => {
            service_failure(result, error)
        }
        NativeMergeError::NativeParseRejected { parses, .. } => {
            retain_parses(result, &parses);
            for parsed in parses.iter().filter(|parsed| !parsed.document.output().ok) {
                diagnostic(
                    result,
                    PortableCategory::ParseError,
                    "parse.rejected",
                    "native parser rejected input",
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
        NativeMergeError::AnalysisRejected { failures, parses, .. } => {
            retain_parses(result, &parses);
            for failure in failures {
                diagnostic(
                    result,
                    PortableCategory::UnsupportedFeature,
                    "analysis.unsupported",
                    failure.message,
                    DiagnosticLayer::Analysis,
                    vec![DiagnosticSourceRef {
                        source_id: failure.source_id,
                        role: failure.source_role,
                        span: None,
                        extra: Metadata::new(),
                    }],
                );
            }
        }
        NativeMergeError::InvalidInputs => diagnostic(
            result,
            PortableCategory::InvalidRequest,
            "request.invalid",
            "invalid native operation inputs",
            DiagnosticLayer::Transport,
            vec![],
        ),
        NativeMergeError::Unsupported(message) => diagnostic(
            result,
            PortableCategory::UnsupportedFeature,
            "operation.unsupported",
            message,
            DiagnosticLayer::Provider,
            vec![],
        ),
    }
}

pub(crate) fn finalize(
    mut result: OperationResult,
    request: &ValidatedOperationRequest,
    evidence: &ConflictEvidence,
    context: &ExecutionContext,
) -> Result<OperationResult, CoreError> {
    if let Err(error) = context.check() {
        result = empty_result(request);
        service_failure(&mut result, error);
    }
    result.validate_with_conflict_evidence(request, evidence).map_err(|error| CoreError {
        code: "operation.invalid_evidence".into(),
        message: error.to_string(),
    })?;
    if let Err(error) = context.check() {
        result = empty_result(request);
        service_failure(&mut result, error);
    }
    Ok(result)
}

/// Execute the explicitly selected YAML mapping/Python declaration profile.
/// Source resolution/validation has already completed. TreeHaver owns parser
/// selection; `context` covers the whole call, including output verification.
pub fn execute_native_operation(
    request: &ValidatedOperationRequest,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
) -> Result<OperationResult, CoreError> {
    execute_native_operation_with_service(
        request,
        snapshot,
        context,
        &TreeHaverParseService::default(),
    )
}

/// Registry-negotiated dispatch supplies conjunctive parser constraints without
/// rewriting the caller's requested backend or explicit/policy provenance.
pub(crate) fn execute_native_operation_with_service(
    request: &ValidatedOperationRequest,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    service: &TreeHaverParseService,
) -> Result<OperationResult, CoreError> {
    if matches!(
        request.request().provider_selection.profile_id.as_deref(),
        Some(crate::profiles::JSON_NESTED | crate::profiles::GIT_JSON)
    ) {
        return crate::json_operation::execute(request, snapshot, context, service);
    }
    let finish = |result, request, evidence| finalize(result, request, evidence, context);
    let mut result = empty_result(request);
    let evidence = ConflictEvidence::default();
    if let Err(error) = context.check() {
        service_failure(&mut result, error);
        return finish(result, request, &evidence);
    }
    let input = request.request();
    let (family, provider, analyzer): (&str, &str, Analyzer) = match input
        .provider_selection
        .profile_id
        .as_deref()
    {
        Some(crate::profiles::YAML_MAPPING) => {
            ("yaml", "kernel.yaml", yaml_merge::typed::mapping_owners)
        }
        Some(crate::profiles::PYTHON_DECLARATIONS) => {
            ("python", "kernel.python", python_merge::declaration_owners)
        }
        Some(crate::profiles::BASH_OWNERS) => ("bash", "kernel.bash", bash_merge::typed::owners),
        Some(crate::profiles::GO_OWNERS) => ("go", "kernel.go", go_merge::typed::owners),
        Some(crate::profiles::RUST_OWNERS) => ("rust", "kernel.rust", rust_merge::typed::owners),
        Some(crate::profiles::TYPESCRIPT_OWNERS) => {
            ("typescript", "kernel.typescript", typescript_merge::typed::owners)
        }
        _ => {
            diagnostic(
                &mut result,
                PortableCategory::UnsupportedFeature,
                "selection.unsupported_profile",
                "an explicit implemented native profile is required",
                DiagnosticLayer::Registry,
                vec![],
            );
            return finish(result, request, &evidence);
        }
    };
    if input.provider_selection.provider_id.as_deref().is_some_and(|id| id != provider)
        || input.provider_selection.family.as_deref().is_some_and(|name| name != family)
        || crate::profiles::native_language(family, input.provider_selection.dialect.as_deref())
            .is_none()
        || !input.provider_selection.extra.is_empty()
        || input.parser_selection.profile_id.is_some()
        || input.parser_selection.language_version.is_some()
        || !input.parser_selection.extra.is_empty()
    {
        diagnostic(
            &mut result,
            PortableCategory::SelectionError,
            "selection.unsupported_constraints",
            "native execution cannot honor the requested selection constraints",
            DiagnosticLayer::Registry,
            vec![],
        );
        return finish(result, request, &evidence);
    }
    let language =
        crate::profiles::native_language(family, input.provider_selection.dialect.as_deref())
            .expect("selection checked above");
    let analyzer = if family == "go" && input.operation.kind() == OperationKind::Merge2 {
        go_merge::directional::owners as Analyzer
    } else if family == "rust" && input.operation.kind() == OperationKind::Merge2 {
        rust_merge::directional::owners as Analyzer
    } else if family == "typescript" && input.operation.kind() == OperationKind::Merge2 {
        typescript_merge::directional::owners as Analyzer
    } else {
        analyzer
    };
    let supported = crate::profiles::profile_operations(
        input.provider_selection.profile_id.as_deref().unwrap_or(""),
    )
    .contains(&input.operation.kind())
        && match &input.operation {
            OperationPolicy::Merge2(policy) => {
                policy.directional_merge == "template-into-current"
                    && policy.render_policy == "source-preserving"
                    && policy.extra.is_empty()
                    && policy.fallback_policy.as_deref().is_none_or(|p| p == "none")
            }
            OperationPolicy::Analyze(policy) => crate::native_analysis_projection::supports(policy),
            OperationPolicy::Merge3(policy) => {
                policy.render_policy == "source-preserving"
                    && (!matches!(family, "bash" | "go" | "rust" | "typescript")
                        || (policy.labels.is_none() && policy.conflict_marker_size.is_none()))
                    && policy.extra.is_empty()
                    && policy.fallback_policy.as_deref().is_none_or(|policy| policy == "none")
            }
            OperationPolicy::Diff2(policy) => {
                policy.extra.is_empty()
                    && policy
                        .comparison_profile
                        .as_deref()
                        .is_none_or(|profile| profile == "exact-source-owners")
                    && policy
                        .equivalence
                        .as_ref()
                        .is_none_or(|rules| rules.is_empty() || rules == &["exact-source"])
            }
        };
    let operation = match input.operation.kind() {
        OperationKind::Analyze => "analyze",
        OperationKind::Merge3 => "merge3",
        OperationKind::Diff2 => "diff2",
        OperationKind::Merge2 => "merge2",
    };
    if !supported
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
            "operation.unsupported_requirements",
            "operation or policy requirements are not implemented by the selected native profile",
            DiagnosticLayer::Registry,
            vec![],
        );
        return finish(result, request, &evidence);
    }
    result.provider.provider_id = Some(provider.into());
    result.provider.family = Some(family.into());
    result.profile.profile_id = input.provider_selection.profile_id.clone();
    let mut parses = vec![];
    for &role in input.operation.kind().source_roles() {
        let source = request
            .sources()
            .get(&input.sources[&role].source_id)
            .map_err(|error| CoreError::new(error.to_string()))?;
        parses.push(ParseRequest {
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
            options: crate::profiles::operation_parse_options(
                input.provider_selection.profile_id.as_deref().unwrap_or_default(),
                input.operation.kind(),
            ),
            metadata: input.metadata.clone(),
            extra: Metadata::new(),
        });
    }
    if input.operation.kind() == OperationKind::Analyze {
        let expected_source = parses[0].source.descriptor.clone();
        let parsed = service.parse_batch(parses, snapshot, context);
        if let Err(error) = context.check() {
            service_failure(&mut result, error);
            return finish(result, request, &evidence);
        }
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                service_failure(&mut result, error);
                return finish(result, request, &evidence);
            }
        };
        if parsed.len() != 1 || parsed[0].source.descriptor() != &expected_source {
            execution_failure(&mut result, NativeMergeError::InvalidInputs);
            return finish(result, request, &evidence);
        }
        retain_parses(&mut result, &parsed);
        if !parsed[0].document.output().ok {
            execution_failure(
                &mut result,
                NativeMergeError::NativeParseRejected {
                    sources: vec![parsed[0].source.descriptor().clone()],
                    parses: parsed,
                },
            );
            return finish(result, request, &evidence);
        }
        let analysis = match family {
            "yaml" => yaml_merge::typed::mapping_analysis(&parsed[0]),
            "python" => python_merge::declaration_analysis(&parsed[0]),
            "bash" => bash_merge::typed::analysis(&parsed[0]),
            "go" => go_merge::typed::analysis(&parsed[0]),
            "rust" => rust_merge::typed::analysis(&parsed[0]),
            "typescript" => typescript_merge::typed::analysis(&parsed[0]),
            _ => unreachable!("profile already selected"),
        }
        .and_then(|analysis| {
            crate::native_analysis_projection::project(&parsed[0], &analysis, family)
        });
        match analysis {
            Ok(analysis) => {
                result.analysis = Some(analysis);
                result.ok = true;
                result.verification.classification_reached = Some(true);
                result.verification.consumed_source_roles = Some(vec![SourceRole::Source]);
            }
            Err(message) => diagnostic(
                &mut result,
                PortableCategory::UnsupportedFeature,
                "analysis.unsupported",
                message,
                DiagnosticLayer::Analysis,
                vec![DiagnosticSourceRef {
                    source_id: parsed[0].source.descriptor().source_id.clone(),
                    role: SourceRole::Source,
                    span: None,
                    extra: Metadata::new(),
                }],
            ),
        }
        return finish(result, request, &evidence);
    }
    if input.operation.kind() == OperationKind::Diff2 {
        match ast_merge::typed_diff::diff_native_sources_with_evidence(
            language, parses, service, snapshot, context, analyzer,
        ) {
            Ok(execution) => {
                retain_parses(&mut result, &execution.input_parses);
                result.ok = true;
                result.verification.classification_reached = Some(true);
                result.verification.consumed_source_roles =
                    Some(input.operation.kind().source_roles().to_vec());
                for change in &execution.diff.changes {
                    let mut source_spans = std::collections::BTreeMap::new();
                    for region in [&change.before, &change.after].into_iter().flatten() {
                        source_spans.insert(region.source_role, diff_span(region, request)?);
                    }
                    result.changes.push(ResultChange {
                        id: change.id.clone(),
                        classification: match change.kind {
                            ast_merge::owner_diff::OwnerChangeKind::Added => "added",
                            ast_merge::owner_diff::OwnerChangeKind::Deleted => "deleted",
                            ast_merge::owner_diff::OwnerChangeKind::Edited => "edited",
                        }
                        .into(),
                        subject_ref: Some(change.owner_id.clone()),
                        path: change.after_path.clone().or(change.before_path.clone()),
                        role_states: [
                            ("before".into(), serde_json::to_value(&change.before).unwrap()),
                            ("after".into(), serde_json::to_value(&change.after).unwrap()),
                        ]
                        .into(),
                        source_spans,
                        metadata: Metadata::new(),
                        extra: Metadata::new(),
                    });
                }
                // Exact owner comparisons alone omit comments/layout changes.
                // Named declaration profiles include a complete-byte summary too;
                // it overlaps owner changes and is not an executable edit.
                if matches!(family, "bash" | "go" | "rust" | "typescript")
                    && execution.input_parses[0].source.bytes()
                        != execution.input_parses[1].source.bytes()
                {
                    let mut source_spans = std::collections::BTreeMap::new();
                    let mut role_states = Metadata::new();
                    for (name, parsed) in
                        ["before", "after"].into_iter().zip(&execution.input_parses)
                    {
                        let source = &parsed.source;
                        let region = ast_merge::owner_diff::DiffSourceRegion {
                            source_id: source.descriptor().source_id.clone(),
                            source_role: source.descriptor().role,
                            range: crate::ByteRange {
                                start_byte: 0,
                                end_byte: source.bytes().len(),
                            },
                            sha256: source.descriptor().sha256.clone(),
                        };
                        source_spans.insert(region.source_role, diff_span(&region, request)?);
                        role_states.insert(name.into(), serde_json::to_value(region).unwrap());
                    }
                    result.changes.push(ResultChange {
                        id: "document-change".into(),
                        classification: "edited".into(),
                        subject_ref: Some("document".into()),
                        path: None,
                        role_states,
                        source_spans,
                        metadata: [("scope".into(), serde_json::json!("complete-source-summary"))]
                            .into(),
                        extra: Metadata::new(),
                    });
                }
                result.diff = Some(ResultDiff {
                    change_ids: result.changes.iter().map(|change| change.id.clone()).collect(),
                    extra: [("owner_diff".into(), serde_json::to_value(&execution.diff).unwrap())]
                        .into(),
                });
            }
            Err(error) => execution_failure(&mut result, error),
        }
        return finish(result, request, &evidence);
    }
    if input.operation.kind() == OperationKind::Merge2 {
        use ast_merge::typed_merge2::{DirectionalFailureStage, merge_directional_native_sources};
        let execution = match merge_directional_native_sources(
            language,
            parses,
            service,
            snapshot,
            context,
            analyzer,
            if family == "bash" {
                bash_merge::directional::plan_insertions
            } else if family == "go" {
                go_merge::directional::plan_insertions
            } else if family == "rust" {
                rust_merge::directional::plan_insertions
            } else if family == "typescript" {
                typescript_merge::directional::plan_insertions
            } else {
                python_merge::directional::plan_insertions
            },
        ) {
            Ok(execution) => execution,
            Err(error) => {
                execution_failure(&mut result, error);
                return finish(result, request, &evidence);
            }
        };
        retain_parses(&mut result, &execution.input_parses);
        result.extra.insert(
            "output_parse".into(),
            serde_json::to_value(execution.output_parse.map(CoreParseResult::from)).unwrap(),
        );
        match execution.rendered {
            Ok(rendered) => {
                result.ok = true;
                result.output = Some(rendered.output);
                result.verification.classification_reached = Some(true);
                result.verification.directional_roles_preserved = Some(true);
                result.verification.consumed_source_roles =
                    Some(input.operation.kind().source_roles().to_vec());
                result.verification.output_reparsed = Some(true);
                result.verification.structural_equivalence = Some(true);
                result.verification.preservation = Some(
                    ["exact-source-partition", "all-current-bytes-retained"]
                        .into_iter()
                        .map(|property| PreservationProperty {
                            property: property.into(),
                            required: true,
                            status: "passed".into(),
                            extra: Metadata::new(),
                        })
                        .collect(),
                );
                result.verification.retained_source_regions = Some(
                    rendered
                        .segments
                        .iter()
                        .map(|segment| ResultSourceRegion {
                            source_id: segment.source_id.clone(),
                            source_role: segment.source_role,
                            range: ResultRange {
                                start_byte: segment.source_range.start_byte,
                                end_byte: segment.source_range.end_byte,
                                extra: Metadata::new(),
                            },
                            sha256: Some(segment.sha256.clone()),
                            extra: [(
                                "output_range".into(),
                                serde_json::to_value(&segment.output_range).unwrap(),
                            )]
                            .into(),
                        })
                        .collect(),
                );
                for decision in &rendered.classification.decisions {
                    if decision.action
                        != ast_merge::owner_merge2::DirectionalOwnerAction::AddIncomingOnly
                    {
                        continue;
                    }
                    let region = decision.incoming.as_ref().expect("classified incoming addition");
                    let span = diff_span(
                        &ast_merge::owner_diff::DiffSourceRegion {
                            source_id: region.source_id.clone(),
                            source_role: region.source_role,
                            range: region.range.clone(),
                            sha256: region.sha256.clone(),
                        },
                        request,
                    )?;
                    result.changes.push(ResultChange {
                        id: format!("change-{}", result.changes.len()),
                        classification: "added".into(),
                        subject_ref: Some(decision.owner_id.clone()),
                        path: Some(decision.path.clone()),
                        role_states: [
                            ("incoming".into(), serde_json::to_value(region).unwrap()),
                            ("current".into(), serde_json::Value::Null),
                        ]
                        .into(),
                        source_spans: [(SourceRole::Incoming, span)].into(),
                        metadata: Metadata::new(),
                        extra: Metadata::new(),
                    });
                }
                result.verification.extra.insert(
                    "owner_classification".into(),
                    serde_json::to_value(rendered.classification).unwrap(),
                );
                result.render_report = [
                    ("producer".into(), serde_json::json!(provider)),
                    ("strategy".into(), serde_json::json!("directional-source-plan")),
                    ("source_segments".into(), serde_json::to_value(rendered.segments).unwrap()),
                ]
                .into();
            }
            Err(_) => {
                if let Some(error) = execution.verification_error {
                    service_failure(&mut result, error);
                } else {
                    let (category, code, layer) = match execution.failure_stage {
                        Some(DirectionalFailureStage::Planning) => (
                            PortableCategory::UnsupportedFeature,
                            "merge2.plan_unsupported",
                            DiagnosticLayer::Analysis,
                        ),
                        Some(DirectionalFailureStage::Rendering) => (
                            PortableCategory::RenderError,
                            "merge2.render_rejected",
                            DiagnosticLayer::Renderer,
                        ),
                        _ => (
                            PortableCategory::VerificationError,
                            "merge2.verification_rejected",
                            DiagnosticLayer::Verifier,
                        ),
                    };
                    diagnostic(
                        &mut result,
                        category,
                        code,
                        "directional merge could not prove an accepted output",
                        layer,
                        vec![],
                    );
                }
            }
        }
        return finish(result, request, &evidence);
    }
    let executed = if matches!(family, "go" | "rust") {
        ast_merge::typed_merge::merge_native_sources_with_engine(
            language,
            parses.clone(),
            service,
            snapshot,
            context,
            ast_merge::typed_merge::NativeOwnerEngine {
                analyze: analyzer,
                merge: if family == "rust" {
                    rust_merge::typed::merge_documents
                } else {
                    go_merge::typed::merge_documents
                },
            },
        )
    } else {
        merge_native_sources_with_evidence(
            language,
            parses.clone(),
            service,
            snapshot,
            context,
            analyzer,
        )
    };
    let mut execution = match executed {
        Ok(execution) => execution,
        Err(error) => {
            execution_failure(&mut result, error);
            return finish(result, request, &evidence);
        }
    };
    retain_parses(&mut result, &execution.input_parses);
    let classified = execution.rendered.classification.is_some();
    result.verification.classification_reached = Some(classified);
    result.verification.base_participated = Some(classified);
    if classified {
        result.verification.consumed_source_roles =
            Some(input.operation.kind().source_roles().to_vec());
    }
    result.verification.extra.insert(
        "owner_classification".into(),
        serde_json::to_value(&execution.rendered.classification).unwrap(),
    );
    match execution.rendered.result.outcome {
        ThreeWayMergeOutcome::Conflict => {
            let projection = crate::native_conflict_projection::project_native_merge_conflicts(
                &execution, request, provider,
            )
            .map_err(|error| CoreError {
                code: "operation.invalid_evidence".into(),
                message: error.to_string(),
            })?;
            result.conflicts = projection
                .conflicts
                .into_iter()
                .map(|conflict| ConflictRecord::Canonical(Box::new(conflict)))
                .collect();
            result.diagnostics =
                projection.diagnostics.into_iter().map(DiagnosticRecord::Canonical).collect();
            if matches!(family, "go" | "rust") && !classified {
                // The family guard classified an ownership conflict, but did
                // not run the generic owner classifier (owner_classification
                // remains null). The common contract counts either decision.
                result.verification.classification_reached = Some(true);
                result.verification.base_participated = Some(true);
                result.verification.consumed_source_roles =
                    Some(input.operation.kind().source_roles().to_vec());
            }
            finish(result, request, &projection.evidence)
        }
        ThreeWayMergeOutcome::Clean => {
            if execution.output_parse.is_none() {
                if let Err(error) = reparse_selected_output(
                    &mut execution,
                    parses.remove(0),
                    snapshot,
                    context,
                    analyzer,
                    service,
                ) {
                    execution_failure(&mut result, error);
                    result.extra.insert(
                        "output_parse".into(),
                        serde_json::to_value(execution.output_parse.map(CoreParseResult::from))
                            .unwrap(),
                    );
                    return finish(result, request, &evidence);
                }
            }
            if let Err(error) = context.check() {
                service_failure(&mut result, error);
                return finish(result, request, &evidence);
            }
            result.extra.insert(
                "output_parse".into(),
                serde_json::to_value(execution.output_parse.clone().map(CoreParseResult::from))
                    .unwrap(),
            );
            result.ok = true;
            result.output = execution.rendered.result.output;
            result.verification.output_reparsed = Some(true);
            result.verification.structural_equivalence = Some(true);
            result.verification.preservation = Some(vec![PreservationProperty {
                property: "exact-source-partition".into(),
                required: true,
                status: "passed".into(),
                extra: Metadata::new(),
            }]);
            let mut retained = vec![];
            for segment in &execution.rendered.source_segments {
                let role = revision_role(segment.revision);
                retained.push(ResultSourceRegion {
                    source_id: input.sources[&role].source_id.clone(),
                    source_role: role,
                    range: ResultRange {
                        start_byte: segment.source_range.start_byte,
                        end_byte: segment.source_range.end_byte,
                        extra: Metadata::new(),
                    },
                    sha256: Some(segment.sha256.clone()),
                    extra: [(
                        "output_range".into(),
                        serde_json::to_value(&segment.output_range).unwrap(),
                    )]
                    .into(),
                });
            }
            result.verification.retained_source_regions = Some(retained);
            result.render_report = [
                ("producer".into(), serde_json::json!(provider)),
                ("strategy".into(), serde_json::json!("source-plan")),
                (
                    "source_segments".into(),
                    serde_json::to_value(execution.rendered.source_segments).unwrap(),
                ),
            ]
            .into();
            finish(result, request, &evidence)
        }
        ThreeWayMergeOutcome::Error => {
            if let Some(error) = execution.verification_error {
                service_failure(&mut result, error);
            } else {
                diagnostic(
                    &mut result,
                    PortableCategory::VerificationError,
                    "merge.not_accepted",
                    "Rust owner merge could not prove an accepted output",
                    DiagnosticLayer::Verifier,
                    vec![],
                );
            }
            result.extra.insert(
                "merge_diagnostics".into(),
                serde_json::to_value(execution.rendered.result.diagnostics).unwrap(),
            );
            result.extra.insert(
                "output_parse".into(),
                serde_json::to_value(execution.output_parse.map(CoreParseResult::from)).unwrap(),
            );
            finish(result, request, &evidence)
        }
    }
}

fn revision_role(revision: SourceRevision) -> SourceRole {
    match revision {
        SourceRevision::Base => SourceRole::Base,
        SourceRevision::Ours => SourceRole::Ours,
        SourceRevision::Theirs => SourceRole::Theirs,
    }
}

fn reparse_selected_output(
    execution: &mut NativeMergeExecution,
    mut parse: ParseRequest,
    snapshot: &ParserRegistrySnapshot,
    context: &ExecutionContext,
    analyzer: Analyzer,
    service: &TreeHaverParseService,
) -> Result<(), NativeMergeError> {
    let selection = execution
        .rendered
        .classification
        .as_ref()
        .and_then(|c| c.whole_source_selection)
        .ok_or(NativeMergeError::InvalidInputs)?;
    let selected = execution
        .input_parses
        .iter()
        .find(|p| p.source.descriptor().role == revision_role(selection))
        .ok_or(NativeMergeError::InvalidInputs)?;
    let output =
        execution.rendered.result.output.as_ref().ok_or(NativeMergeError::InvalidInputs)?;
    parse.request_id = format!("{}:output", parse.request_id);
    parse.source = crate::source_input(
        execution.output_source.as_ref().ok_or(NativeMergeError::InvalidInputs)?.source_id.clone(),
        SourceRole::Output,
        SourceEncoding::Utf8,
        output.as_bytes().to_vec(),
    )
    .map_err(|error| NativeMergeError::Parse(ServiceError::Source(error)))?;
    parse.selection.backend_id = Some(selected.backend.id.clone());
    let mut parsed =
        service.parse_batch(vec![parse], snapshot, context).map_err(NativeMergeError::Parse)?;
    context.check().map_err(NativeMergeError::Parse)?;
    let parsed = parsed.pop().ok_or(NativeMergeError::InvalidInputs)?;
    let accepted = parsed.document.output().ok
        && analyzer(&parsed)
            .ok()
            .zip(analyzer(selected).ok())
            .is_some_and(|(output, input)| output == input);
    execution.output_parse = Some(parsed);
    if !accepted {
        return Err(NativeMergeError::Unsupported(
            "selected output failed native reparse/owner verification".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod span_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn service_failure_diagnostics_distinguish_limits_from_invalid_input_and_native_claims() {
        use crate::parsed::ParseValidationError;
        use crate::source::{SourceError, SourceErrorCode};
        use tree_haver::service::ProviderFault;

        let source =
            crate::source_input("source".into(), SourceRole::Source, SourceEncoding::Utf8, vec![])
                .unwrap();
        let request: crate::OperationRequest = serde_json::from_value(json!({
            "schema": crate::OPERATION_SCHEMA, "request_id": "service-failure",
            "operation": "analyze",
            "provider_selection": {"family": "yaml", "required_capabilities": []},
            "parser_selection": {"preference": [], "required_capabilities": []},
            "policy": {}, "extensions": [], "metadata": {},
            "sources": {"source": {"source_id": "source", "role": "source",
                "content": "", "encoding": "utf-8", "byte_length": 0,
                "sha256": source.descriptor.sha256}}
        }))
        .unwrap();
        let request = request.validate(1024, |_, _| panic!()).unwrap();
        let cases = [
            (ServiceError::LimitExceeded, PortableCategory::ResourceLimit, "resource.limit"),
            (
                ServiceError::Source(SourceError {
                    code: SourceErrorCode::LimitExceeded,
                    source_id: "source".into(),
                }),
                PortableCategory::ResourceLimit,
                "resource.limit",
            ),
            (
                ServiceError::InvalidResult {
                    backend_id: "native".into(),
                    error: ParseValidationError::LimitExceeded,
                },
                PortableCategory::ResourceLimit,
                "resource.limit",
            ),
            (
                ServiceError::Source(SourceError {
                    code: SourceErrorCode::DescriptorMismatch,
                    source_id: "source".into(),
                }),
                PortableCategory::InvalidRequest,
                "source.invalid",
            ),
            (
                ServiceError::InvalidResult {
                    backend_id: "native".into(),
                    error: ParseValidationError::IdentityMismatch,
                },
                PortableCategory::InternalError,
                "parser.invalid_result",
            ),
            (
                ServiceError::Provider {
                    backend_id: "native".into(),
                    fault: ProviderFault {
                        code: "resource.limit".into(),
                        message: "secret native source text".into(),
                    },
                },
                PortableCategory::InternalError,
                "parser.provider_fault",
            ),
        ];
        for (error, category, code) in cases {
            let failure = crate::ParserFailure::from(error.clone());
            let mut result = empty_result(&request);
            service_failure(&mut result, error);
            assert!(!result.ok);
            assert!(result.output.is_none());
            let [DiagnosticRecord::Canonical(record)] = result.diagnostics.as_slice() else {
                panic!("expected one canonical diagnostic");
            };
            assert_eq!(record.category, category);
            assert_eq!(record.code, code);
            assert_eq!(record.origin.backend_id, failure.backend_id);
            assert_eq!(record.origin.native_code, failure.native_code);
            assert!(record.blocking);
            assert_eq!(record.severity, DiagnosticSeverity::Error);
            assert!(!serde_json::to_string(record).unwrap().contains("secret native source text"));
            result.validate_against(&request).unwrap();
        }
    }

    #[test]
    fn diff_projection_rejects_wrong_identity_digest_and_out_of_bounds_regions() {
        let text = "a: été\r\n";
        let digest = crate::source_input(
            "before".into(),
            SourceRole::Before,
            SourceEncoding::Utf8,
            text.as_bytes().to_vec(),
        )
        .unwrap()
        .descriptor
        .sha256;
        let request: crate::operation::OperationRequest = serde_json::from_value(json!({
            "schema": crate::OPERATION_SCHEMA, "request_id": "span-test", "operation": "diff2",
            "provider_selection": {"family": "yaml", "required_capabilities": []},
            "parser_selection": {"preference": [], "required_capabilities": []},
            "policy": {}, "extensions": [], "metadata": {},
            "sources": {
                "before": {"source_id": "before", "role": "before", "content": text,
                    "encoding": "utf-8", "byte_length": text.len(), "sha256": digest},
                "after": {"source_id": "after", "role": "after", "content": text,
                    "encoding": "utf-8", "byte_length": text.len(), "sha256": digest}
            }
        }))
        .unwrap();
        let request = request.validate(1024, |_, _| panic!()).unwrap();
        let region = ast_merge::owner_diff::DiffSourceRegion {
            source_id: "before".into(),
            source_role: SourceRole::Before,
            range: crate::ByteRange { start_byte: 0, end_byte: text.len() },
            sha256: digest,
        };
        let span = diff_span(&region, &request).unwrap();
        assert_eq!((span.end_point.row, span.end_point.column), (1, 0));
        let mut invalid = region.clone();
        invalid.source_id = "after".into();
        assert!(diff_span(&invalid, &request).is_err());
        invalid = region.clone();
        invalid.source_role = SourceRole::After;
        assert!(diff_span(&invalid, &request).is_err());
        invalid = region.clone();
        invalid.sha256 = "0".repeat(64);
        assert!(diff_span(&invalid, &request).is_err());
        invalid = region;
        invalid.range.end_byte += 1;
        assert!(diff_span(&invalid, &request).is_err());
    }
}
