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
    serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":"go-common",
        "operation":operation,"sources":sources,"extensions":[],"metadata":{},
        "provider_selection":{"provider_id":"kernel.go","family":"go","dialect":"go","profile_id":"kernel.go.owners.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"go.common","preference":[],"required_capabilities":[]},
        "policy": match operation { "analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}),
            "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) }
    })).unwrap()
}

fn setup() -> (ParserRegistrySnapshot, ExecutionContext) {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(LanguagePackProvider::new("go.common".into(), "go".into()).unwrap()))
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

const BASE: &str = "package main\n// é\nfunc f() { println(1) }\nfunc g() { println(1) }\n";
const OURS: &str = "package main\n// é\nfunc f() { println(2) }\nfunc g() { println(1) }\n";
const THEIRS: &str = "package main\n// é\nfunc f() { println(1) }\nfunc g() { println(2) }\n";

#[test]
fn exported_go_operations_honor_selection_and_cancellation() {
    register_language_pack_parser("go.facade".into(), "go".into()).unwrap();
    let limits = ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let control = create_operation_control();
    control.cancel();
    for operation in ["analyze", "diff2", "merge2", "merge3"] {
        let mut input = request(operation, &[BASE; 3]);
        input.parser_selection.backend = Some("go.facade".into());
        assert_eq!(
            execute_operation_controlled(input.clone(), limits.clone(), &control).unwrap_err().code,
            "execution.cancelled"
        );
        assert!(execute_operation(input.clone(), limits.clone()).unwrap().ok);
        input.parser_selection.backend = Some("go.missing".into());
        assert!(!execute_operation(input, limits.clone()).unwrap().ok);
    }
    unregister_parser_provider("go.facade".into()).unwrap();
}

#[test]
fn common_analysis_retains_native_references_and_revalidates_owner_claims() {
    let (result, input) = run(request("analyze", &[BASE]));
    assert!(result.ok, "{:?}", result.diagnostics);
    let facts = &result.analysis.as_ref().unwrap().extra;
    assert_eq!(facts["owners"][0]["id"], "/function:f");
    assert_eq!(facts["owners"][1]["id"], "/function:g");
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
fn diff_reports_owner_and_whole_document_changes() {
    let (result, _) = run(request("diff2", &[BASE, OURS]));
    assert!(result.ok);
    assert!(result.changes.iter().any(|change| change.path.as_deref() == Some("/function:f")));
    assert_eq!(result.changes.len(), 2);
    for changed in
        [BASE.replace("// é", "// documentation"), BASE.replace("package main", "package other")]
    {
        let (result, _) = run(request("diff2", &[BASE, &changed]));
        assert!(result.ok);
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].subject_ref.as_deref(), Some("document"));
    }
}

#[test]
fn merge3_preserves_native_decisions_and_verifies_every_clean_output() {
    for texts in [[BASE, OURS, THEIRS], [BASE; 3], [BASE, BASE, OURS]] {
        let expected =
            go_merge::merge_go_three_way(texts[0], texts[1], texts[2], go_merge::GoDialect::Go);
        let (result, _) = run(request("merge3", &texts));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.output, expected.output);
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(result.verification.base_participated, Some(true));
        assert!(!result.verification.retained_source_regions.unwrap().is_empty());
    }
    let other = BASE.replace("println(1)", "println(3)");
    let (result, _) = run(request("merge3", &[BASE, OURS, &other]));
    assert!(!result.ok);
    assert!(!result.conflicts.is_empty());
    assert!(result.output.is_none());
}

#[test]
fn family_guard_is_a_whole_document_conflict_without_invented_owner_decisions() {
    for texts in [
        [BASE.to_string(), OURS.to_string(), format!("{BASE}func added() {{}}\n")],
        [BASE.to_string(), format!("{OURS}func added() {{}}\n"), BASE.to_string()],
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
        assert_eq!(conflict.category, ConflictCategory::Ownership);
        assert_eq!(conflict.localization.status, LocalizationStatus::WholeDocument);
        assert!(conflict.decision_ids.is_empty());
        assert!(conflict.classification.decision_ids.is_empty());
        for (alternative, text) in conflict.alternatives.iter().zip(&texts) {
            assert_eq!(alternative.regions[0].byte_length, text.len() as u64);
        }
        // No trusted owner decision ID is needed for a whole-document guard.
        result.validate_against(&input).unwrap();
    }
}

#[test]
fn unsupported_operations_syntax_selection_and_cancellation_remain_fail_closed() {
    let (result, _) = run(request("analyze", &["package main\nvar x=1\n"]));
    assert!(!result.ok);
    assert!(!result.diagnostics.is_empty());
    let mut input = request("analyze", &[BASE]);
    input.provider_selection.dialect = Some("bash".into());
    assert!(!run(input).0.ok);
    let mut input = request("merge3", &[BASE; 3]);
    let OperationPolicy::Merge3(policy) = &mut input.operation else { panic!() };
    policy.conflict_marker_size = Some(8);
    assert!(!run(input).0.ok);
    let (snapshot, context) = setup();
    context.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    let input = request("analyze", &[BASE]).validate(100000, |_, _| panic!()).unwrap();
    let result = native_operation::execute_native_operation(&input, &snapshot, &context).unwrap();
    assert!(!result.ok);
    assert!(result.analysis.is_none());
}

#[test]
fn directional_go_retains_current_bytes_and_native_function_comments() {
    for (incoming, current, expected) in [
        (
            "package main\nfunc f() { println(1) }\nfunc g() {}\n",
            "package main\nfunc f() { println(9) }\n",
            "package main\nfunc f() { println(9) }\nfunc g() {}\n",
        ),
        (
            "package main\n\n// é new\nfunc added() {}\nfunc f() {}\n",
            "// current module\npackage main\n\n// current f\nfunc f() { println(9) }\n// footer\n",
            "// current module\npackage main\n\n// é new\nfunc added() {}\n\n// current f\nfunc f() { println(9) }\n// footer\n",
        ),
        (
            "package main\n\n// first\nfunc f() {}\n",
            "package main\n// footer\n",
            "package main\n\n// first\nfunc f() {}\n// footer\n",
        ),
        ("package main\n", "package main\nfunc f() {}", "package main\nfunc f() {}"),
        (
            "package other\nfunc f() { println(1) }\n",
            "package main\nfunc f() {}",
            "package main\nfunc f() {}",
        ),
        (
            "package main\nimport \"fmt\"\nfunc f() {}\n// g\nfunc g() { fmt.Println(1) }\n",
            "package main\nimport \"fmt\"\nfunc f() {} // current\n// footer\n",
            "package main\nimport \"fmt\"\nfunc f() {} // current\n// g\nfunc g() { fmt.Println(1) }\n// footer\n",
        ),
    ] {
        let (result, _) = run(request("merge2", &[incoming, current]));
        assert!(result.ok, "{incoming:?} into {current:?}: {:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), Some(expected));
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(result.verification.directional_roles_preserved, Some(true));
        assert_eq!(result.verification.base_participated, None);
        let mut retained_current = vec![];
        for region in result.verification.retained_source_regions.unwrap() {
            let source =
                if region.source_role == SourceRole::Incoming { incoming } else { current };
            let output = &region.extra["output_range"];
            assert_eq!(
                &source.as_bytes()[region.range.start_byte..region.range.end_byte],
                &expected.as_bytes()[output["start_byte"].as_u64().unwrap() as usize
                    ..output["end_byte"].as_u64().unwrap() as usize]
            );
            if region.source_role == SourceRole::Current {
                retained_current.extend_from_slice(
                    &current.as_bytes()[region.range.start_byte..region.range.end_byte],
                );
            }
        }
        assert_eq!(retained_current, current.as_bytes());
    }
}

#[test]
fn directional_go_rejects_unproven_headers_and_ambiguous_placement() {
    for (incoming, current) in [
        ("package other\nfunc g() {}\n", "package main\nfunc f() {}\n"),
        (
            "package main\nimport \"fmt\"\nfunc g() { fmt.Println(1) }\n",
            "package main\nfunc f() {}\n",
        ),
        (
            "package main\nfunc b() {}\nfunc new() {}\nfunc a() {}\n",
            "package main\nfunc a() {}\nfunc b() {}\n",
        ),
        ("package main\nfunc f() {}; func g() {}\n", "package main\nfunc f() {}\n"),
        ("package main\nfunc g() {}", "package main\nfunc f() {}\n"),
        ("package main\nfunc g() {}\n", "package main\nfunc f() {}"),
        ("package main\nfunc g() {}\n", ""),
        ("package main\nvar x=1\nfunc g() {}\n", "package main\nfunc f() {}\n"),
    ] {
        let (result, _) = run(request("merge2", &[incoming, current]));
        assert!(!result.ok, "{incoming:?} into {current:?}");
        assert!(result.output.is_none());
        assert!(result.verification.retained_source_regions.is_none());
    }
}

#[test]
fn guard_projection_rejects_forged_messages_sources_and_provider_identity() {
    use ast_merge::typed_merge::{NativeOwnerEngine, merge_native_sources_with_engine};
    let added = format!("{BASE}func added() {{}}\n");
    let input = request("merge3", &[BASE, OURS, &added]).validate(100000, |_, _| panic!()).unwrap();
    let parses = [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
        .map(|role| {
            let source = &input.request().sources[&role];
            let bytes = input.sources().get(&source.source_id).unwrap().bytes().to_vec();
            ParseRequest {
                schema: PARSE_REQUEST_SCHEMA.into(),
                request_id: format!("{role:?}"),
                source: source_input(source.source_id.clone(), role, SourceEncoding::Utf8, bytes)
                    .unwrap(),
                language: "go".into(),
                dialect: None,
                selection: ParserSelection {
                    backend_id: Some("go.common".into()),
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
        "go",
        parses,
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        NativeOwnerEngine {
            analyze: go_merge::typed::owners,
            merge: go_merge::typed::merge_documents,
        },
    )
    .unwrap();
    let project = |execution: &ast_merge::typed_merge::NativeMergeExecution, provider| {
        native_conflict_projection::project_native_merge_conflicts(execution, &input, provider)
    };
    assert!(project(&execution, "kernel.go").is_ok());
    assert!(project(&execution, "kernel.bash").is_err());
    let original = execution.rendered.result.conflicts[0].message.clone();
    execution.rendered.result.conflicts[0].message.push_str(" forged");
    assert!(project(&execution, "kernel.go").is_err());
    execution.rendered.result.conflicts[0].message = original;
    assert!(project(&execution, "kernel.go").is_ok());
    execution.input_parses.swap(0, 1);
    assert!(project(&execution, "kernel.go").is_err());
}
