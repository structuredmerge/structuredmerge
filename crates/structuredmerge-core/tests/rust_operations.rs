use serde_json::json;
use std::sync::{Arc, atomic::AtomicBool};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(operation: &str, texts: &[&str]) -> OperationRequest {
    let roles = match operation {
        "analyze" => vec!["source"],
        "diff2" => vec!["before", "after"],
        "merge2" => vec!["incoming", "current"],
        _ => vec!["base", "ours", "theirs"],
    };
    let sources: serde_json::Map<_, _> = roles
        .into_iter()
        .zip(texts)
        .map(|(role, text)| {
            let source = source_input(
                role.into(),
                serde_json::from_value(json!(role)).unwrap(),
                SourceEncoding::Utf8,
                text.as_bytes().to_vec(),
            )
            .unwrap();
            (
                role.into(),
                json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8",
            "byte_length":source.descriptor.byte_length,"sha256":source.descriptor.sha256}),
            )
        })
        .collect();
    serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":"rust-common",
        "operation":operation,"sources":sources,"extensions":[],"metadata":{},
        "provider_selection":{"provider_id":"kernel.rust","family":"rust","dialect":"rust","profile_id":"kernel.rust.owners.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"rust.common","preference":[],"required_capabilities":[]},
        "policy": match operation { "analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}),
            "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) }
    })).unwrap()
}

fn setup() -> (ParserRegistrySnapshot, ExecutionContext) {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(LanguagePackProvider::new("rust.common".into(), "rust".into()).unwrap()))
        .unwrap();
    (
        registry.snapshot().unwrap(),
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 3,
            max_input_bytes: 100000,
            max_nodes: 10000,
            max_diagnostics: 100,
        },
    )
}

fn run(input: OperationRequest) -> (OperationResult, ValidatedOperationRequest) {
    let (snapshot, context) = setup();
    let input = input.validate(100000, |_, _| panic!()).unwrap();
    let result = native_operation::execute_native_operation(&input, &snapshot, &context).unwrap();
    if result.conflicts.is_empty() {
        result.validate_against(&input).unwrap();
    }
    (result, input)
}

const BASE: &str = "use std::fmt;\n// é\nfn f() -> i32 { 1 }\nfn g() -> i32 { 1 }\n";
const OURS: &str = "use std::fmt;\n// é\nfn f() -> i32 { 2 }\nfn g() -> i32 { 1 }\n";
const THEIRS: &str = "use std::fmt;\n// é\nfn f() -> i32 { 1 }\nfn g() -> i32 { 2 }\n";

#[test]
fn exported_rust_operations_honor_selection_and_cancellation() {
    register_language_pack_parser("rust.facade".into(), "rust".into()).unwrap();
    let limits = ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let control = create_operation_control();
    control.cancel();
    for operation in ["analyze", "diff2", "merge3"] {
        let mut input = request(operation, &[BASE; 3]);
        input.parser_selection.backend = Some("rust.facade".into());
        assert_eq!(
            execute_operation_controlled(input.clone(), limits.clone(), &control).unwrap_err().code,
            "execution.cancelled"
        );
        assert!(execute_operation(input.clone(), limits.clone()).unwrap().ok);
        input.parser_selection.backend = Some("rust.missing".into());
        assert!(!execute_operation(input, limits.clone()).unwrap().ok);
    }
    unregister_parser_provider("rust.facade".into()).unwrap();
}

#[test]
fn analysis_revalidates_native_ownership_and_preserves_passive_metadata() {
    let (result, input) = run(request("analyze", &[BASE]));
    assert!(result.ok, "{:?}", result.diagnostics);
    let facts = &result.analysis.as_ref().unwrap().extra;
    assert_eq!(facts["owners"][0]["id"], "/function:f");
    assert!(!facts["owners"][0]["node_ids"].as_array().unwrap().is_empty());
    let mut wrong = result.clone();
    wrong.analysis.as_mut().unwrap().extra.get_mut("owners").unwrap()[0]["node_id"] =
        json!("invented");
    assert!(wrong.validate_against(&input).is_err());
    let mut future = result;
    future.analysis.as_mut().unwrap().extra.insert("future_passive".into(), json!(true));
    future.validate_against(&input).unwrap();
}

#[test]
fn diff_includes_complete_source_summary_for_use_and_comment_changes() {
    let (result, _) = run(request("diff2", &[BASE, OURS]));
    assert!(result.ok);
    assert_eq!(result.changes.len(), 2);
    assert!(result.changes.iter().any(|change| change.path.as_deref() == Some("/function:f")));
    for changed in [BASE.replace("// é", "// documentation"), BASE.replace("std::fmt", "std::io")]
    {
        let (result, _) = run(request("diff2", &[BASE, &changed]));
        assert!(result.ok);
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].subject_ref.as_deref(), Some("document"));
    }
    assert!(run(request("diff2", &[BASE, BASE])).0.changes.is_empty());
}

#[test]
fn merge3_matches_native_decisions_and_reparses_even_unchanged_output() {
    for texts in [[BASE, OURS, THEIRS], [BASE; 3], [BASE, BASE, OURS]] {
        let expected = rust_merge::merge_rust_three_way(
            texts[0],
            texts[1],
            texts[2],
            rust_merge::RustDialect::Rust,
        );
        let (result, _) = run(request("merge3", &texts));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.output, expected.output);
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(result.verification.base_participated, Some(true));
        assert!(!result.verification.retained_source_regions.unwrap().is_empty());
    }
    let other = BASE.replace("{ 1 }", "{ 3 }");
    let (result, _) = run(request("merge3", &[BASE, OURS, &other]));
    assert!(!result.ok);
    assert!(!result.conflicts.is_empty());
    assert!(result.output.is_none());
}

#[test]
fn family_guard_precedes_shortcuts_and_does_not_fabricate_owner_decisions() {
    for texts in [
        [BASE.to_string(), OURS.to_string(), format!("{BASE}fn added() {{}}\n")],
        [BASE.to_string(), format!("{OURS}fn added() {{}}\n"), BASE.to_string()],
        [BASE.to_string(), OURS.to_string(), BASE.replace("fn g() -> i32 { 1 }\n", "")],
    ] {
        let (result, input) = run(request("merge3", &[&texts[0], &texts[1], &texts[2]]));
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert_eq!(result.verification.classification_reached, Some(true));
        assert!(result.verification.extra["owner_classification"].is_null());
        assert_eq!(result.verification.base_participated, Some(true));
        assert_eq!(result.verification.consumed_source_roles.as_ref().unwrap().len(), 3);
        assert_eq!(result.conflicts.len(), 1);
        let ConflictRecord::Canonical(conflict) = &result.conflicts[0] else { panic!() };
        assert_eq!(conflict.code, "rust.membership_with_owner_edit");
        assert_eq!(conflict.category, ConflictCategory::Ownership);
        assert_eq!(conflict.localization.status, LocalizationStatus::WholeDocument);
        assert_eq!(conflict.subject.whole_document, Some(true));
        assert!(conflict.subject.owner_ref.is_none());
        assert!(conflict.decision_ids.is_empty());
        assert!(conflict.classification.decision_ids.is_empty());
        for (alternative, text) in conflict.alternatives.iter().zip(&texts) {
            assert_eq!(alternative.regions[0].byte_length, text.len() as u64);
        }
        result.validate_against(&input).unwrap();
    }
}

#[test]
fn unsupported_shapes_policies_and_directional_merge_fail_closed() {
    for source in [
        "use std::fmt;\n",
        "impl X {}\nfn f() {}\n",
        "#[test]\nfn f() {}\n",
        "fn f( {",
        "fn f() {}\nfn f() {}\n",
        "macro_rules! x { () => {} }\nfn f() {}\n",
    ] {
        let (result, _) = run(request("analyze", &[source]));
        assert!(!result.ok, "{source}");
        assert!(!result.diagnostics.is_empty());
    }
    assert!(!run(request("merge2", &[OURS, BASE])).0.ok);
    let mut input = request("analyze", &[BASE]);
    input.provider_selection.dialect = Some("go".into());
    assert!(!run(input).0.ok);
    let mut input = request("merge3", &[BASE; 3]);
    let OperationPolicy::Merge3(policy) = &mut input.operation else { panic!() };
    policy.conflict_marker_size = Some(8);
    assert!(!run(input).0.ok);
}

#[test]
fn guard_projection_recomputes_evidence_and_rejects_forged_results() {
    use ast_merge::typed_merge::{NativeOwnerEngine, merge_native_sources_with_engine};
    let added = format!("{BASE}fn added() {{}}\n");
    let input = request("merge3", &[BASE, OURS, &added]).validate(100000, |_, _| panic!()).unwrap();
    let parses = [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
        .map(|role| {
            let source = &input.request().sources[&role];
            ParseRequest {
                schema: PARSE_REQUEST_SCHEMA.into(),
                request_id: format!("{role:?}"),
                source: source_input(
                    source.source_id.clone(),
                    role,
                    SourceEncoding::Utf8,
                    input.sources().get(&source.source_id).unwrap().bytes().to_vec(),
                )
                .unwrap(),
                language: "rust".into(),
                dialect: None,
                selection: ParserSelection {
                    backend_id: Some("rust.common".into()),
                    preference: vec![],
                    required_capabilities: vec![],
                },
                options: ParseOptions::default(),
                metadata: Default::default(),
                extra: Default::default(),
            }
        })
        .to_vec();
    let (snapshot, context) = setup();
    let mut execution = merge_native_sources_with_engine(
        "rust",
        parses,
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        NativeOwnerEngine {
            analyze: rust_merge::typed::owners,
            merge: rust_merge::typed::merge_documents,
        },
    )
    .unwrap();
    let project = |execution: &ast_merge::typed_merge::NativeMergeExecution, provider| {
        native_conflict_projection::project_native_merge_conflicts(execution, &input, provider)
    };
    assert!(project(&execution, "kernel.rust").is_ok());
    assert!(project(&execution, "kernel.go").is_err());
    let original = execution.rendered.result.conflicts[0].message.clone();
    execution.rendered.result.conflicts[0].message.push_str(" forged");
    assert!(project(&execution, "kernel.rust").is_err());
    execution.rendered.result.conflicts[0].message = original;
    assert!(project(&execution, "kernel.rust").is_ok());
    execution.input_parses.swap(0, 1);
    assert!(project(&execution, "kernel.rust").is_err());
}
