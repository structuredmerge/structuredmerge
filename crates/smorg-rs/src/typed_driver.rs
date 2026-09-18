//! Explicit typed migration lane. Legacy dispatch is never a fallback here.
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
};

use core::*;
use structuredmerge_core as core;

use crate::{MergeDriverOptions, path_safety, staged_file::StagedFile};

const INPUT_BUDGET: u64 = 8 * 1024 * 1024;

fn limits() -> ParseLimits {
    ParseLimits {
        max_batch_items: 3,
        max_input_bytes: INPUT_BUDGET,
        max_nodes: 200_000,
        max_diagnostics: 100,
        timeout_millis: Some(10_000),
    }
}

struct Registration(String);
impl Drop for Registration {
    fn drop(&mut self) {
        let _ = unregister_parser_provider(self.0.clone());
    }
}

pub(super) fn run_merge(options: &MergeDriverOptions, stderr: &mut dyn Write) -> i32 {
    match merge(options, stderr) {
        Ok(code) => code,
        Err(failure) => {
            let _ = writeln!(stderr, "typed merge-driver: {}", failure.message);
            if let (Some(diagnostic), Some(path)) = (&failure.diagnostic, &options.report_path) {
                // Common argument/path preflight has already accepted this destination.
                // An empty query never probes providers or acquires a grammar.
                let reported =
                    capability_manifest(vec![], limits()).map_err(internal).and_then(|manifest| {
                        write_report(
                            path,
                            &report(
                                &manifest.kernel_version,
                                failure.exit_code,
                                "error",
                                None,
                                vec![diagnostic.as_ref().clone()],
                            ),
                        )
                    });
                if let Err(error) = reported {
                    let _ = writeln!(stderr, "cannot write typed error report: {}", error.message);
                    return 3;
                }
            }
            failure.exit_code
        }
    }
}

struct Failure {
    exit_code: i32,
    message: String,
    // Only failures before a validated operation result may use a null envelope.
    diagnostic: Option<Box<PortableDiagnostic>>,
}
fn invalid(message: impl ToString) -> Failure {
    Failure { exit_code: 2, message: message.to_string(), diagnostic: None }
}
fn internal(message: impl ToString) -> Failure {
    Failure { exit_code: 3, message: message.to_string(), diagnostic: None }
}

fn rejected(category: PortableCategory, code: &str, message: impl ToString) -> Failure {
    let message = message.to_string();
    Failure {
        exit_code: 2,
        message: message.clone(),
        diagnostic: Some(Box::new(PortableDiagnostic {
            schema: DIAGNOSTIC_SCHEMA.into(),
            id: "cli.failure".into(),
            sequence: 0,
            severity: DiagnosticSeverity::Error,
            category,
            code: code.into(),
            message,
            blocking: true,
            operation: Some(OperationKind::Merge3),
            request_id: Some("cli.merge3".into()),
            source_refs: vec![],
            subject_refs: None,
            cause_ids: vec![],
            related_ids: vec![],
            origin: DiagnosticOrigin {
                layer: DiagnosticLayer::Adapter,
                provider_id: None,
                backend_id: None,
                package: Some(env!("CARGO_PKG_NAME").into()),
                package_version: Some(env!("CARGO_PKG_VERSION").into()),
                native_code: None,
                extra: BTreeMap::new(),
            },
            data: BTreeMap::new(),
            extensions: vec![],
            metadata: BTreeMap::new(),
            extra: BTreeMap::new(),
        })),
    }
}

fn selection(message: impl ToString) -> Failure {
    rejected(PortableCategory::SelectionError, "cli.selection_rejected", message)
}

fn source_failure(message: impl ToString) -> Failure {
    rejected(PortableCategory::InvalidRequest, "cli.source_rejected", message)
}

fn kernel_failure(error: CoreError) -> Failure {
    // Classify stable error codes, never human text. Unknown codes are opaque
    // origin evidence, not permission to invent a more specific category.
    let category = match error.code.as_str() {
        "request.invalid"
        | "operation.invalid_request"
        | "source.invalid"
        | "source.unresolved_reference" => PortableCategory::InvalidRequest,
        "capability.unknown_profile" | "selection.no_parser" => PortableCategory::SelectionError,
        "resource.limit" => PortableCategory::ResourceLimit,
        "execution.cancelled" => PortableCategory::Cancelled,
        "execution.deadline_exceeded" => PortableCategory::DeadlineExceeded,
        _ => PortableCategory::InternalError,
    };
    let mut failure = rejected(category, "cli.kernel_rejected", &error.message);
    if category == PortableCategory::InternalError {
        failure.exit_code = 3;
    }
    failure.diagnostic.as_mut().unwrap().origin.native_code = Some(error.code);
    failure
}

fn report(
    kernel_version: &str,
    code: i32,
    outcome: &str,
    result: Option<&OperationResult>,
    diagnostics: Vec<PortableDiagnostic>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "structuredmerge.cli-report/v1", "command": "merge-driver",
        "cli": {"executable": env!("CARGO_BIN_NAME"), "package": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"), "kernel_version": kernel_version,
            "cli_contract": "structuredmerge.cli/v1"},
        "outcome": outcome, "exit_code": code, "operation_result": result,
        "availability": null, "conflict_review": null, "git_install": null,
        "diagnostics": diagnostics, "output_commit_verified": false,
    })
}

fn write_report(path: &str, report: &serde_json::Value) -> Result<(), Failure> {
    let mut bytes = serde_json::to_vec(report).map_err(internal)?;
    bytes.push(b'\n');
    StagedFile::new(path, &bytes).map_err(internal)?.commit().map_err(internal)
}

fn merge(options: &MergeDriverOptions, stderr: &mut dyn Write) -> Result<i32, Failure> {
    let provider = options
        .provider_id
        .as_ref()
        .ok_or_else(|| invalid("--provider is required for typed execution"))?;
    let backend = options
        .backend_id
        .as_ref()
        .ok_or_else(|| invalid("--backend is required for typed execution"))?;
    let profile_id = options
        .profile_id
        .as_ref()
        .ok_or_else(|| invalid("--profile is required for typed execution"))?;
    if options.exit_code && !options.check_only {
        return Err(invalid("--exit-code requires --check-only"));
    }
    if options.profile_report {
        return Err(invalid("--profile-report is legacy-only; use --report for the typed result"));
    }
    if options.fallback_explicit && options.fallback != "none" {
        return Err(invalid("typed execution currently supports only --fallback none"));
    }
    if options.require_profile_status.as_deref().is_some_and(|s| s != "available") {
        return Err(invalid("typed profiles have no recommended/default authority"));
    }
    let write_conflict = match options.conflict_policy.as_deref().unwrap_or("leave-ours") {
        "leave-ours" => false,
        "write" => true,
        _ => return Err(invalid("--conflict-policy must be leave-ours or write")),
    };
    let output_path = options.output.as_deref().unwrap_or(&options.current);
    if !path_safety::report_is_distinct(output_path, &[&options.ancestor, &options.other])
        .map_err(invalid)?
    {
        return Err(invalid("output path aliases base or theirs"));
    }
    let catalog = operation_profile_catalog();
    let profile = catalog
        .profiles
        .iter()
        .find(|p| &p.id == profile_id)
        .ok_or_else(|| selection("unknown typed profile"))?;
    if &profile.provider_id != provider
        || options.family.as_ref().is_some_and(|f| f != &profile.family)
    {
        return Err(selection("provider/family constraints do not match the selected profile"));
    }
    // In this standalone process no providers have been registered yet. Ask the
    // kernel for its parser request; do not duplicate its dialect/language map.
    // This is NOT an availability lease or source-specific execution evidence.
    let manifest = capability_manifest(
        vec![CapabilityQuery {
            profile_id: profile_id.clone(),
            operation: OperationKind::Merge3,
            dialect: options.dialect.clone(),
            parser_selection: ParserSelection {
                backend_id: Some(backend.clone()),
                preference: vec![],
                required_capabilities: vec![],
            },
        }],
        limits(),
    )
    .map_err(kernel_failure)?;
    let language = &manifest.observations[0]
        .parser_request
        .as_ref()
        .ok_or_else(|| selection("profile does not declare the requested operation/dialect"))?
        .language;
    if backend != &format!("kernel.tslp.{language}") {
        return Err(selection("backend is not an explicitly supported cached kernel parser"));
    }
    let mut remaining = INPUT_BUDGET;
    let mut sources = BTreeMap::new();
    for (role, id, path) in [
        (SourceRole::Base, "base", &options.ancestor),
        (SourceRole::Ours, "ours", &options.current),
        (SourceRole::Theirs, "theirs", &options.other),
    ] {
        if !std::fs::metadata(path).map_err(source_failure)?.is_file() {
            return Err(source_failure("sources must be regular files"));
        }
        let file = File::open(path).map_err(source_failure)?;
        if !file.metadata().map_err(source_failure)?.is_file() {
            return Err(source_failure("sources must be regular files"));
        }
        let mut bytes = Vec::new();
        file.take(remaining + 1).read_to_end(&mut bytes).map_err(source_failure)?;
        if bytes.len() as u64 > remaining {
            return Err(rejected(
                PortableCategory::ResourceLimit,
                "cli.source_limit",
                "aggregate source size exceeds 8 MiB",
            ));
        }
        remaining -= bytes.len() as u64;
        let source =
            source_input(id.into(), role, SourceEncoding::Utf8, bytes).map_err(source_failure)?;
        sources.insert(
            role,
            OperationSource {
                source_id: id.into(),
                role,
                byte_length: source.descriptor.byte_length,
                sha256: source.descriptor.sha256,
                encoding: "utf-8".into(),
                content: None,
                bytes: Some(source.bytes),
                reference: None,
                bom: None,
                line_endings: None,
                final_newline: None,
                extra: BTreeMap::new(),
            },
        );
    }
    let ours = sources[&SourceRole::Ours].bytes.as_ref().unwrap().clone();
    // Wire capabilities are a sorted set; CLI flag order is not semantic.
    let mut required_capabilities = options.required_capabilities.clone();
    required_capabilities.sort();
    required_capabilities.dedup();
    register_cached_language_pack_parser(backend.clone(), language.clone())
        .map_err(kernel_failure)?;
    let _registration = Registration(backend.clone());
    let result = execute_operation(
        OperationRequest {
            schema: "structuredmerge.operation-request/v1".into(),
            request_id: "cli.merge3".into(),
            operation: OperationPolicy::Merge3(ThreeWayMergePolicy {
                render_policy: "source-preserving".into(),
                fallback_policy: Some("none".into()),
                conflict_marker_size: None,
                labels: None,
                extra: BTreeMap::new(),
            }),
            provider_selection: MergeProviderSelection {
                provider_id: Some(provider.clone()),
                family: options.family.clone(),
                dialect: options.dialect.clone(),
                profile_id: Some(profile_id.clone()),
                required_capabilities,
                extra: BTreeMap::new(),
            },
            parser_selection: OperationParserSelection {
                backend: Some(backend.clone()),
                preference: vec![],
                required_capabilities: vec![],
                profile_id: None,
                language_version: None,
                extra: BTreeMap::new(),
            },
            sources,
            path_name: options.path_name.clone(),
            extensions: vec![],
            metadata: BTreeMap::new(),
            extra: BTreeMap::new(),
        },
        limits(),
    )
    .map_err(kernel_failure)?;
    let conflict = result.conflicts.iter().any(|c| c.unresolved());
    for diagnostic in &result.diagnostics {
        let (code, message) = match diagnostic {
            DiagnosticRecord::Canonical(d) => (d.code.as_str(), d.message.as_str()),
            DiagnosticRecord::Migration(d) => (d.code.as_str(), d.message.as_str()),
        };
        writeln!(stderr, "{code}: {message}").map_err(internal)?;
    }
    let changed = result.output.as_ref().is_some_and(|s| s.as_bytes() != ours);
    let (code, outcome) = if conflict {
        (1, "conflict")
    } else if !result.ok {
        (2, "error")
    } else if changed {
        (if options.check_only && options.exit_code { 1 } else { 0 }, "changed")
    } else {
        (0, "clean")
    };
    let output = if options.check_only {
        None
    } else if conflict && write_conflict {
        result.conflicted_output.as_deref()
    } else if result.ok && !conflict {
        result.output.as_deref()
    } else {
        None
    };
    if conflict && write_conflict && !options.check_only && output.is_none() {
        return Err(invalid("selected profile supplied no validated conflicted_output"));
    }
    if result.ok && !conflict && result.output.is_none() {
        return Err(invalid("successful merge supplied no output"));
    }
    let staged_output = output
        .map(|text| StagedFile::new(output_path, text.as_bytes()))
        .transpose()
        .map_err(internal)?;
    let report = report(&manifest.kernel_version, code, outcome, Some(&result), vec![]);
    if let Some(path) = &options.report_path {
        write_report(path, &report)?;
    }
    if let Some(staged) = staged_output {
        staged.commit().map_err(internal)?;
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_failure_classification_uses_codes_not_messages() {
        for (native, category, exit) in [
            ("operation.invalid_request", PortableCategory::InvalidRequest, 2),
            ("selection.no_parser", PortableCategory::SelectionError, 2),
            ("resource.limit", PortableCategory::ResourceLimit, 2),
            ("execution.cancelled", PortableCategory::Cancelled, 2),
            ("execution.deadline_exceeded", PortableCategory::DeadlineExceeded, 2),
            ("future.unrecognized", PortableCategory::InternalError, 3),
        ] {
            let failure = kernel_failure(CoreError {
                code: native.into(),
                message: "merge conflict parse error success".into(),
            });
            assert_eq!(failure.exit_code, exit);
            let diagnostic = failure.diagnostic.unwrap();
            assert_eq!(diagnostic.category, category);
            assert_eq!(diagnostic.origin.native_code.as_deref(), Some(native));
            validate_diagnostics(
                &[&diagnostic],
                "cli.merge3",
                OperationKind::Merge3,
                &SourceMap::default(),
                |_| false,
            )
            .unwrap();
            let value = report("test-kernel", exit, "error", None, vec![*diagnostic]);
            assert!(value["operation_result"].is_null());
            assert_eq!(value["outcome"], "error");
            assert_eq!(value["cli"]["kernel_version"], "test-kernel");
        }
        assert!(invalid("bad syntax").diagnostic.is_none());
        assert!(internal("failed commit").diagnostic.is_none());
    }
}
