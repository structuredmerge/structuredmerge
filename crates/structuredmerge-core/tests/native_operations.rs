//! Real Psych facts through the common Rust operation dispatcher. Explicit
//! ignored gate: requires Ruby/Psych, not a generated-binding artifact test.
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
use structuredmerge_core::{
    native_operation::execute_native_operation,
    operation::{OperationRequest, ValidatedOperationRequest},
    portable_diagnostic::DiagnosticRecord,
    *,
};
use tree_haver::service::*;

#[derive(Clone, Copy)]
enum Behavior {
    Normal,
    Cancel,
    FailOutput,
}
struct Psych {
    descriptor: ParserProviderDescriptor,
    calls: AtomicUsize,
    behavior: Behavior,
}
impl ParserProvider for Psych {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        Ok(ParserProbeResult { available: true, loadable: true })
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let fault = |message: String| ProviderFault { code: "test.psych".into(), message };
        if matches!(self.behavior, Behavior::FailOutput)
            && requests[0].source.descriptor.role == SourceRole::Output
        {
            return Err(fault(
                "private parser exception that must not become a public message".into(),
            ));
        }
        let mut child = Command::new(
            std::env::var("STRUCTUREDMERGE_NATIVE_RUBY").unwrap_or_else(|_| "ruby".into()),
        )
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../yaml-merge/tests/support/psych_facts.rb"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| fault(error.to_string()))?;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&requests).unwrap())
            .map_err(|error| fault(error.to_string()))?;
        let output = child.wait_with_output().map_err(|error| fault(error.to_string()))?;
        if !output.status.success() {
            return Err(fault(String::from_utf8_lossy(&output.stderr).into()));
        }
        if matches!(self.behavior, Behavior::Cancel) {
            context.cancelled.store(true, Ordering::Release);
        }
        serde_json::from_slice(&output.stdout).map_err(|error| fault(error.to_string()))
    }
}
fn setup(behavior: Behavior) -> (Arc<Psych>, ParserRegistrySnapshot, ExecutionContext) {
    let provider = Arc::new(Psych {
        descriptor: serde_json::from_value(json!({
            "id": "test.psych", "family": "native", "runtime": "ruby", "package": "psych", "package_version": "test-runtime",
            "parser": "psych", "parser_version": "test-runtime", "grammar": null, "grammar_version": null,
            "languages": ["yaml"], "dialects": [], "contracts": [PARSE_RESULT_SCHEMA],
            "capabilities": ["native_extensions", "source_spans"], "probe_id": "test.psych", "priority": 0, "metadata": {}, "extensions": []
        })).unwrap(), calls: AtomicUsize::new(0), behavior,
    });
    let registry = ParserRegistry::default();
    registry.register(provider.clone()).unwrap();
    (
        provider,
        registry.snapshot().unwrap(),
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 3,
            max_input_bytes: 4096,
            max_nodes: 1000,
            max_diagnostics: 20,
        },
    )
}
fn wire(operation: &str, sources: &[&str]) -> Value {
    let roles = match operation {
        "merge3" => vec!["base", "ours", "theirs"],
        "merge2" => vec!["incoming", "current"],
        "diff2" => vec!["before", "after"],
        "analyze" => vec!["source"],
        _ => unreachable!(),
    };
    let mut inputs = serde_json::Map::new();
    for (role, source) in roles.into_iter().zip(sources) {
        let input = source_input(
            role.into(),
            serde_json::from_value(json!(role)).unwrap(),
            SourceEncoding::Utf8,
            source.as_bytes().to_vec(),
        )
        .unwrap();
        inputs.insert(role.into(), json!({"source_id": role, "role": role, "content": source, "byte_length": input.descriptor.byte_length, "sha256": input.descriptor.sha256, "encoding": "utf-8"}));
    }
    json!({"schema": OPERATION_SCHEMA, "request_id": "native-op-1", "operation": operation,
        "provider_selection": {"provider_id": "kernel.yaml", "family": "yaml", "profile_id": "kernel.yaml.native_mapping.v1", "required_capabilities": [operation]},
        "parser_selection": {"backend": "test.psych", "preference": [], "required_capabilities": []}, "sources": inputs,
        "policy": match operation { "merge3" => json!({"render_policy": "source-preserving", "fallback_policy": "none"}), "merge2" => json!({"directional_merge": "template-into-current", "render_policy": "source-preserving"}), _ => json!({}) },
        "extensions": [], "metadata": {"caller": "integration"}, "future": {"opaque": [null, false, 7]}})
}
fn request(value: Value) -> ValidatedOperationRequest {
    serde_json::from_value::<OperationRequest>(value)
        .unwrap()
        .validate(4096, |_, _| panic!())
        .unwrap()
}
fn code(result: &operation_result::OperationResult) -> &str {
    let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
    &diagnostic.code
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn common_merge_executes_rust_composition_with_real_reparse_and_exact_evidence() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "merge3",
        &[
            "# header\r\na: one\r\nb: two",
            "# header\r\na: ours\r\nb: two",
            "# header\r\na: one\r\nb: theirs",
        ],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("# header\r\na: ours\r\nb: theirs"));
    assert_eq!(result.verification.base_participated, Some(true));
    assert_eq!(result.verification.output_reparsed, Some(true));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert!(result.extra["output_parse"]["parsed"]["ok"].as_bool().unwrap());
    assert!(!result.verification.retained_source_regions.as_ref().unwrap().is_empty());
    assert_eq!(
        result.extra["request_forwarding"]["extra"]["future"],
        json!({"opaque": [null, false, 7]})
    );
    assert_eq!(result.metadata["caller"], "integration");
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn whole_source_selection_is_actually_reparsed_not_reported_as_a_reparse_by_assumption() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut input = wire("merge3", &["a: one\n", "a: two\n", "a: one\n"]);
    input["sources"]["base"]["source_id"] = json!("merge-output");
    input["ok"] = json!(false);
    input["output"] = json!("untrusted request extension");
    let request = request(input);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("a: two\n"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.extra["output_parse"]["parsed"]["source"]["role"], "output");
    assert_eq!(result.verification.output_reparsed, Some(true));
    assert_ne!(result.extra["output_parse"]["parsed"]["source"]["source_id"], "merge-output");
    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized["ok"], true);
    assert_eq!(serialized["output"], "a: two\n");
    assert_eq!(serialized["request_forwarding"]["extra"]["output"], "untrusted request extension");
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn real_conflicts_use_canonical_records_and_executed_decision_catalogs() {
    let (_, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "merge3",
        &["anchor: kept\na: one\n", "anchor: kept\n", "anchor: kept\na: theirs\n"],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(!result.ok);
    assert_eq!(code(&result), "merge.delete_edit");
    assert!(result.output.is_none());
    assert!(result.conflicted_output.is_none());
    let portable_conflict::ConflictRecord::Canonical(conflict) = &result.conflicts[0] else {
        panic!()
    };
    assert_eq!(conflict.classification.base_participated, Some(true));
    assert_eq!(conflict.alternatives[1].state, portable_conflict::AlternativeState::Absent);
    assert!(conflict.alternatives[1].regions.is_empty());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn diff_has_both_roles_and_preserves_exact_changes_and_layout_without_output() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let request = request(wire("diff2", &["# header\na: one\n", "# changed\na: two\nb: three\n"]));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.changes.len(), 2);
    assert!(result.output.is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        result.verification.consumed_source_roles,
        Some(vec![SourceRole::Before, SourceRole::After])
    );
    assert!(
        !result.diff.unwrap().extra["owner_diff"]["before_layout"].as_array().unwrap().is_empty()
    );
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn diff_spans_use_exact_utf8_bytes_and_do_not_invent_absent_revision_ranges() {
    let (_, registry, context) = setup(Behavior::Normal);
    let request = request(wire(
        "diff2",
        &["# café\r\na: ancien\r\ngone: oui\r\n", "# café\r\na: été\r\nnew: oui\r\n"],
    ));
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(result.changes.len(), 3);
    let edited = &result.changes[0];
    assert_eq!(edited.classification, "edited");
    assert_eq!(edited.source_spans.len(), 2);
    let after = &edited.source_spans[&SourceRole::After];
    assert_eq!(after.range.start_byte, "# café\r\n".len());
    assert_eq!((after.start_point.row, after.start_point.column), (1, 0));
    assert_eq!((after.end_point.row, after.end_point.column), (1, "a: été".len()));
    for change in &result.changes {
        for (role, span) in &change.source_spans {
            let key = if *role == SourceRole::Before { "before" } else { "after" };
            assert_eq!(
                serde_json::to_value(&span.range).unwrap(),
                change.role_states[key]["range"]
            );
        }
    }
    assert_eq!(result.changes[1].classification, "deleted");
    assert!(result.changes[1].source_spans.contains_key(&SourceRole::Before));
    assert!(!result.changes[1].source_spans.contains_key(&SourceRole::After));
    assert_eq!(result.changes[2].classification, "added");
    assert!(!result.changes[2].source_spans.contains_key(&SourceRole::Before));
    assert!(result.changes[2].source_spans.contains_key(&SourceRole::After));
    result.validate_against(&request).unwrap();
    let mut corrupt = result;
    corrupt.changes[0].source_spans.get_mut(&SourceRole::After).unwrap().end_point.column -= 1;
    assert!(corrupt.validate_against(&request).is_err());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn unsupported_operations_policies_and_selection_never_invoke_the_parser() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut cases = vec![wire("merge2", &["a: one", "a: two"])];
    for (key, value) in [
        ("comments", json!(true)),
        ("tokens", json!(true)),
        ("ownership", json!(false)),
        ("native_extensions", json!(false)),
        ("analysis_depth", json!("full")),
    ] {
        let mut analysis = wire("analyze", &["a: one"]);
        analysis["policy"][key] = value;
        cases.push(analysis);
    }
    let base = wire("merge3", &["a: one", "a: two", "a: three"]);
    for pointer in [
        "/policy/fallback_policy",
        "/policy/render_policy",
        "/provider_selection/provider_id",
        "/provider_selection/profile_id",
    ] {
        let mut value = base.clone();
        *value.pointer_mut(pointer).unwrap() = json!("unknown");
        cases.push(value);
    }
    let mut constraints = base.clone();
    constraints["parser_selection"]["language_version"] = json!("9.9");
    cases.push(constraints);
    let mut policy = base;
    policy["policy"]["unknown_required_behavior"] = json!(true);
    cases.push(policy);
    for value in cases {
        let request = request(value);
        let result = execute_native_operation(&request, &registry, &context).unwrap();
        assert!(!result.ok);
        assert!(code(&result).contains("unsupported"));
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn native_parse_and_analysis_failures_keep_exact_revision_and_original_parse_evidence() {
    let (_, registry, context) = setup(Behavior::Normal);
    for (text, expected) in [("a: [", "parse.rejected"), ("a: one\na: two", "analysis.unsupported")]
    {
        let request = request(wire("merge3", &["a: one", text, "a: three"]));
        let result = execute_native_operation(&request, &registry, &context).unwrap();
        assert!(!result.ok);
        assert_eq!(code(&result), expected);
        let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
        assert_eq!(diagnostic.source_refs[0].role, SourceRole::Ours);
        assert_eq!(result.extra["input_parses"].as_array().unwrap().len(), 3);
        assert_eq!(result.verification.classification_reached, Some(false));
    }
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn analysis_retains_native_nodes_and_exact_shared_layout_without_rendering() {
    let (provider, registry, context) = setup(Behavior::Normal);
    let mut value = wire("analyze", &["# café\r\na: one\r\nb: two"]);
    value["policy"] = json!({"analysis_depth": "exact-source-owners", "comments": false,
        "tokens": false, "ownership": true, "native_extensions": true});
    let input = request(value);
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(result.output.is_none());
    assert!(result.diff.is_none());
    assert!(result.changes.is_empty());
    assert!(result.conflicts.is_empty());
    assert_eq!(result.verification.consumed_source_roles, Some(vec![SourceRole::Source]));
    assert_eq!(result.verification.output_reparsed, None);
    let analysis = serde_json::to_value(result.analysis.as_ref().unwrap()).unwrap();
    assert_eq!(analysis["schema"], "structuredmerge.analysis-result/v1");
    assert_eq!(analysis["parse_result_ref"], analysis["parse_result"]["parsed"]["request_id"]);
    assert_eq!(analysis["parse_result"]["parsed"]["source"]["role"], "source");
    let nodes = analysis["parse_result"]["parsed"]["nodes"].as_array().unwrap();
    let owners = analysis["owners"].as_array().unwrap();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0]["logical_identity"], json!(["yaml", "/a"]));
    for owner in owners {
        let ids = owner["node_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(owner["node_id"], ids[0]);
        for id in ids {
            assert!(nodes.iter().any(|node| node["id"] == *id));
        }
    }
    let gaps = analysis["layout_gaps"].as_array().unwrap();
    assert_eq!(gaps.len(), 2); // zero-width suffix is not a reported gap
    assert_eq!(gaps[0]["kind"], "preamble");
    assert_eq!(gaps[0]["span"]["range"]["end_byte"], "# café\r\n".len());
    assert_eq!(gaps[1]["kind"], "interstitial");
    assert!(gaps.iter().all(|gap| gap["fallback_controller_side"].is_null()));
    let source = input.sources().get("source").unwrap();
    for gap in gaps {
        let range: ByteRange = serde_json::from_value(gap["span"]["range"].clone()).unwrap();
        assert_eq!(gap["source_sha256"], source.range_digest(range).unwrap());
    }
    assert_eq!(
        analysis["attachments"][0]["trailing_gap_id"],
        analysis["attachments"][1]["leading_gap_id"]
    );
    assert_eq!(analysis["ownership"][1]["selected_owner_ref"], "/b");
    assert!(analysis["comment_regions"].as_array().unwrap().is_empty());
    assert_eq!(analysis["metadata"]["comment_analysis"], "not-requested");
    result.validate_against(&input).unwrap();
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn common_analysis_validation_rejects_forged_owner_layout_and_parser_evidence() {
    let (_, registry, context) = setup(Behavior::Normal);
    let input = request(wire("analyze", &["# header\na: one\nb: two\n"]));
    let result = execute_native_operation(&input, &registry, &context).unwrap();
    assert!(result.ok);
    let original = serde_json::to_value(result.analysis.as_ref().unwrap()).unwrap();
    for (pointer, value) in [
        ("/parse_result_ref", json!("different-parse")),
        ("/request_id", json!("different-request")),
        ("/parse_result/schema", json!("unknown/v9")),
        ("/parse_result/parsed/source/sha256", json!("0".repeat(64))),
        ("/parse_result/parsed/source/role", json!("before")),
        ("/parse_result/selection/selected_backend", json!("invented.backend")),
        ("/parse_result/selection/requested/backend_id", json!("invented.backend")),
        ("/owners/0/node_id", json!("missing-node")),
        ("/owners/0/node_ids", json!([])),
        ("/owners/0/logical_identity", json!(["yaml", "/forged"])),
        ("/owners/0/match_keys", json!(["forged"])),
        ("/owners/0/source_sha256", json!("0".repeat(64))),
        ("/owners/0/span/start_point/column", json!(1)),
        ("/layout_gaps/0/source_sha256", json!("0".repeat(64))),
        ("/layout_gaps/1/controller_side", json!("before")),
        ("/layout_gaps/1/fallback_controller_side", json!("before")),
        ("/attachments/0/trailing_gap_id", json!("unknown-gap")),
        ("/ownership/1/selected_owner_ref", json!("/a")),
        ("/ownership/1/confidence", json!("guessed")),
        ("/comment_regions", json!([{"invented": true}])),
        (
            "/extensions",
            json!([{"schema": "example/v1", "namespace": "", "capabilities": [], "payload": {}}]),
        ),
    ] {
        let mut corrupted = original.clone();
        *corrupted.pointer_mut(pointer).unwrap_or_else(|| panic!("missing pointer: {pointer}")) =
            value;
        let mut changed = result.clone();
        changed.analysis = Some(serde_json::from_value(corrupted).unwrap());
        assert!(changed.validate_against(&input).is_err(), "accepted mutation at {pointer}");
    }
    let mut changed = result.clone();
    changed.analysis.as_mut().unwrap().extra.remove("parse_result");
    assert!(changed.validate_against(&input).is_err());
    let mut unsupported = input.request().clone();
    unsupported.parser_selection.language_version = Some("unimplemented-version".into());
    let unsupported = unsupported.validate(4096, |_, _| panic!()).unwrap();
    assert!(result.validate_against(&unsupported).is_err());
    let mut future = original;
    future["future"] = json!({"opaque": [null, false, 7]});
    future["owners"][0]["future"] = json!("retained");
    future["extensions"] = json!([{"schema": "example.analysis/v1", "namespace": "example.analysis",
        "capabilities": [], "payload": {"opaque": "preserved"}}]);
    let mut changed = result;
    changed.analysis = Some(serde_json::from_value(future.clone()).unwrap());
    changed.validate_against(&input).unwrap();
    assert_eq!(serde_json::to_value(changed.analysis.unwrap()).unwrap(), future);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn analysis_failures_preserve_source_role_and_discard_late_results() {
    for (text, expected) in [("a: [", "parse.rejected"), ("a: one\na: two", "analysis.unsupported")]
    {
        let (_, registry, context) = setup(Behavior::Normal);
        let input = request(wire("analyze", &[text]));
        let result = execute_native_operation(&input, &registry, &context).unwrap();
        assert!(!result.ok);
        assert!(result.analysis.is_none());
        assert_eq!(code(&result), expected);
        let DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0] else { panic!() };
        assert_eq!(diagnostic.source_refs[0].role, SourceRole::Source);
        assert_eq!(result.extra["input_parses"].as_array().unwrap().len(), 1);
    }
    let (_, registry, context) = setup(Behavior::Cancel);
    let result =
        execute_native_operation(&request(wire("analyze", &["a: one"])), &registry, &context)
            .unwrap();
    assert_eq!(code(&result), "execution.cancelled");
    assert!(!result.ok);
    assert!(result.analysis.is_none());
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn missing_explicit_backend_and_input_budget_fail_without_parser_substitution() {
    let (provider, registry, mut context) = setup(Behavior::Normal);
    let mut input = wire("diff2", &["a: one", "a: two"]);
    input["parser_selection"]["backend"] = json!("missing.backend");
    let result = execute_native_operation(&request(input), &registry, &context).unwrap();
    assert!(!result.ok);
    assert!(code(&result).starts_with("selection."));
    context.max_input_bytes = 1;
    let result = execute_native_operation(
        &request(wire("diff2", &["a: one", "a: two"])),
        &registry,
        &context,
    )
    .unwrap();
    assert!(!result.ok);
    assert!(code(&result).contains("limit"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "native Ruby/Psych common-operation integration gate"]
fn controls_and_output_failures_do_not_fabricate_success_or_expose_native_exception_text() {
    let request = request(wire("merge3", &["a: one", "a: two", "a: one"]));
    let (_, registry, context) = setup(Behavior::Cancel);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert_eq!(code(&result), "execution.cancelled");
    assert!(!result.ok);
    assert!(result.output.is_none());
    let (provider, registry, mut context) = setup(Behavior::Normal);
    context.deadline = Some(Instant::now());
    assert_eq!(
        code(&execute_native_operation(&request, &registry, &context).unwrap()),
        "execution.deadline_exceeded"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let (_, registry, context) = setup(Behavior::FailOutput);
    let result = execute_native_operation(&request, &registry, &context).unwrap();
    assert!(!result.ok);
    assert!(result.output.is_none());
    assert_eq!(code(&result), "parser.provider_fault");
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(!encoded.contains("private parser exception"));
}
