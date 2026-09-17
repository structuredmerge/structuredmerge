use serde_json::json;
use std::sync::{Arc, atomic::AtomicBool};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(dialect: &str, operation: &str, texts: &[&str]) -> OperationRequest {
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
    serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":"typescript-common",
        "operation":operation,"sources":sources,"extensions":[],"metadata":{},
        "provider_selection":{"provider_id":"kernel.typescript","family":"typescript","dialect":dialect,"profile_id":"kernel.typescript.owners.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":format!("{dialect}.common"),"preference":[],"required_capabilities":[]},
        "policy": match operation { "analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}),
            "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) }
    })).unwrap()
}

fn setup() -> (ParserRegistrySnapshot, ExecutionContext) {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(
            LanguagePackProvider::new("typescript.common".into(), "typescript".into()).unwrap(),
        ))
        .unwrap();
    registry
        .register(Arc::new(LanguagePackProvider::new("tsx.common".into(), "tsx".into()).unwrap()))
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

const BASE: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 1; }\nfunction g() { return 1; }\n";
const OURS: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 2; }\nfunction g() { return 1; }\n";
const THEIRS: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 1; }\nfunction g() { return 2; }\n";

#[test]
fn analysis_revalidates_native_wrappers_and_requested_dialect() {
    for dialect in ["typescript", "tsx"] {
        let (result, input) = run(request(dialect, "analyze", &[BASE]));
        assert!(result.ok, "{:?}", result.diagnostics);
        let facts = &result.analysis.as_ref().unwrap().extra;
        assert_eq!(facts["owners"][0]["id"], "/function:f");
        assert!(!facts["owners"][0]["node_ids"].as_array().unwrap().is_empty());
        let mut wrong = result.clone();
        wrong.analysis.as_mut().unwrap().extra.get_mut("owners").unwrap()[0]["node_id"] =
            json!("invented");
        assert!(wrong.validate_against(&input).is_err());
        let other = if dialect == "tsx" { "typescript" } else { "tsx" };
        let mut wrong_request = input.request().clone();
        wrong_request.provider_selection.dialect = Some(other.into());
        let wrong_request = wrong_request.validate(100000, |_, _| panic!()).unwrap();
        assert!(result.validate_against(&wrong_request).is_err());
        let mut future = result;
        future.analysis.as_mut().unwrap().extra.insert("future_passive".into(), json!(true));
        future.validate_against(&input).unwrap();
    }
}

#[test]
fn diff_reports_owners_and_import_comment_layout_for_both_dialects() {
    for dialect in ["typescript", "tsx"] {
        let (result, _) = run(request(dialect, "diff2", &[BASE, OURS]));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.changes.len(), 2);
        assert!(result.changes.iter().any(|change| change.path.as_deref() == Some("/function:f")));
        for changed in [BASE.replace("// é", "// changed"), BASE.replace("'x'", "'y'")] {
            let (result, _) = run(request(dialect, "diff2", &[BASE, &changed]));
            assert!(result.ok);
            assert_eq!(result.changes.len(), 1);
            assert_eq!(result.changes[0].subject_ref.as_deref(), Some("document"));
        }
        assert!(run(request(dialect, "diff2", &[BASE, BASE])).0.changes.is_empty());
    }
}

#[test]
fn merge3_retains_native_decisions_and_fresh_output_verification() {
    for (dialect, native_dialect) in [
        ("typescript", typescript_merge::TypeScriptDialect::TypeScript),
        ("tsx", typescript_merge::TypeScriptDialect::Tsx),
    ] {
        let added = format!("{BASE}function added() {{}}\n");
        let other = BASE.replace("return 1", "return 3");
        for texts in [
            [BASE, OURS, THEIRS],
            [BASE; 3],
            [BASE, BASE, OURS],
            [BASE, OURS, added.as_str()],
            [BASE, OURS, other.as_str()],
        ] {
            let native = typescript_merge::merge_typescript_three_way(
                texts[0],
                texts[1],
                texts[2],
                native_dialect,
            );
            let (result, _) = run(request(dialect, "merge3", &texts));
            assert_eq!(result.output, native.output);
            if native.outcome == ast_merge::ThreeWayMergeOutcome::Clean {
                assert!(result.ok, "{:?}", result.diagnostics);
                assert_eq!(result.verification.output_reparsed, Some(true));
                assert_eq!(result.verification.base_participated, Some(true));
                assert!(!result.verification.retained_source_regions.unwrap().is_empty());
            } else {
                assert!(!result.ok);
                assert!(!result.conflicts.is_empty());
                assert!(matches!(&result.conflicts[0], ConflictRecord::Canonical(_)));
            }
        }
    }
}

#[test]
fn jsx_requires_tsx_and_selects_tsx_for_output_reparse() {
    let base = "function View() { return <div>one</div>; }\nfunction f() { return 1; }\n";
    let ours = base.replace(">one<", ">two<");
    let theirs = base.replace("return 1", "return 2");
    let (result, _) = run(request("tsx", "merge3", &[base, &ours, &theirs]));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert_eq!(result.output, Some(ours.replace("return 1", "return 2")));
    assert_eq!(result.verification.output_reparsed, Some(true));
    assert!(run(request("tsx", "analyze", &[base])).0.ok);
    assert!(run(request("tsx", "diff2", &[base, &ours])).0.ok);
    assert!(!run(request("typescript", "analyze", &[base])).0.ok);
    let mut wrong = request("tsx", "analyze", &[base]);
    wrong.parser_selection.backend = Some("typescript.common".into());
    assert!(!run(wrong).0.ok);
    let mut omitted = request("typescript", "analyze", &[BASE]);
    omitted.provider_selection.dialect = None;
    assert!(run(omitted).0.ok);
}

#[test]
fn unsupported_policies_dialects_and_merge2_fail_closed() {
    for dialect in ["typescript", "tsx"] {
        assert!(!run(request(dialect, "merge2", &[OURS, BASE])).0.ok);
        for source in ["export const v = 1;\n", "const a = 1, b = 2;\n", "function f( {"] {
            assert!(!run(request(dialect, "analyze", &[source])).0.ok);
        }
        let mut input = request(dialect, "merge3", &[BASE; 3]);
        let OperationPolicy::Merge3(policy) = &mut input.operation else { panic!() };
        policy.conflict_marker_size = Some(8);
        assert!(!run(input).0.ok);
    }
    let mut input = request("typescript", "analyze", &[BASE]);
    input.provider_selection.dialect = Some("javascript".into());
    assert!(!run(input).0.ok);
    let mut input = request("tsx", "analyze", &[BASE]);
    input.provider_selection.family = Some("tsx".into());
    assert!(!run(input).0.ok);
}

#[test]
fn exported_facade_honors_dialect_selection_and_cancellation() {
    let limits = ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let control = create_operation_control();
    control.cancel();
    for dialect in ["typescript", "tsx"] {
        let id = format!("{dialect}.facade");
        register_language_pack_parser(id.clone(), dialect.into()).unwrap();
        for operation in ["analyze", "diff2", "merge3"] {
            let mut input = request(dialect, operation, &[BASE; 3]);
            input.parser_selection.backend = Some(id.clone());
            assert_eq!(
                execute_operation_controlled(input.clone(), limits.clone(), &control)
                    .unwrap_err()
                    .code,
                "execution.cancelled"
            );
            assert!(execute_operation(input.clone(), limits.clone()).unwrap().ok);
            input.parser_selection.backend = Some("missing".into());
            assert!(!execute_operation(input, limits.clone()).unwrap().ok);
        }
        unregister_parser_provider(id).unwrap();
    }
}
