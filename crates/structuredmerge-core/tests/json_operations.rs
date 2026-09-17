use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(operation: &str, dialect: &str, texts: &[&str]) -> OperationRequest {
    let roles: &[&str] = match operation {
        "diff2" => &["before", "after"],
        "merge2" => &["incoming", "current"],
        _ => &["base", "ours", "theirs"],
    };
    let mut sources = serde_json::Map::new();
    for (role, text) in roles.iter().zip(texts) {
        let input = source_input(
            role.to_string(),
            serde_json::from_value(json!(role)).unwrap(),
            SourceEncoding::Utf8,
            text.as_bytes().to_vec(),
        )
        .unwrap();
        sources.insert(role.to_string(), json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8","byte_length":input.descriptor.byte_length,"sha256":input.descriptor.sha256}));
    }
    serde_json::from_value(json!({"schema": OPERATION_SCHEMA,"request_id":"json-common",
        "operation":operation,"provider_selection":{"provider_id":"kernel.json","family":"json","dialect":dialect,"profile_id":"kernel.json.nested.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"json.common","preference":[],"required_capabilities":[]},"sources":sources,
        "policy":match operation { "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}), "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) },
        "extensions":[],"metadata":{}})).unwrap()
}
fn run(request: OperationRequest) -> (OperationResult, ValidatedOperationRequest) {
    let language = if request.provider_selection.dialect.as_deref() == Some("json") {
        "json"
    } else {
        "json5"
    };
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(
            LanguagePackProvider::new("json.common".into(), language.into()).unwrap(),
        ))
        .unwrap();
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
    };
    let validated = request.validate(100000, |_, _| panic!()).unwrap();
    let result = native_operation::execute_native_operation(
        &validated,
        &registry.snapshot().unwrap(),
        &context,
    )
    .unwrap();
    result.validate_against(&validated).unwrap();
    (result, validated)
}

#[test]
fn common_json_diff_compares_complete_bytes_and_native_nested_subjects() {
    let (result, _) = run(request(
        "diff2",
        "json5",
        &["// old\r\n{a:{x:1},b:{x:1},gone:0}", "// new\r\n{a:{x:1},b:{x:2},added:0}"],
    ));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.output.is_none());
    assert!(result.render_report.is_empty());
    assert_ne!(result.verification.output_reparsed, Some(true));
    assert_eq!(
        result
            .changes
            .iter()
            .map(|c| (c.path.as_deref(), c.classification.as_str()))
            .collect::<Vec<_>>(),
        [
            (None, "edited"),
            (Some(""), "edited"),
            (Some("/added"), "added"),
            (Some("/b"), "edited"),
            (Some("/b/x"), "edited"),
            (Some("/gone"), "deleted")
        ]
    );
    let nested = &result.changes[4];
    assert_eq!(nested.role_states["before"]["owner"]["path"], "/b/x");
    assert_eq!(nested.role_states["before"]["source"]["role"], "before");
    assert_eq!(result.diff.unwrap().extra["document_bytes_compared"], true);
}

#[test]
fn common_json_diff_covers_trivia_only_edits_noops_scalars_and_positional_arrays() {
    for (left, right, count) in [
        ("// old\n{}", "// new\n{}", 1),
        ("{}", "{}\r\n", 1),
        ("1", "2", 2),
        ("[1,2]", "[0,1,2]", 5),
        ("{x:1}", "{x:1}", 0),
    ] {
        let (result, _) = run(request("diff2", "json5", &[left, right]));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.changes.len(), count, "{left:?} -> {right:?}");
        assert_eq!(
            result.verification.consumed_source_roles,
            Some(vec![SourceRole::Before, SourceRole::After])
        );
    }
}

#[test]
fn common_json_diff_rejects_ambiguous_keys_and_unsupported_equivalence() {
    let (result, _) = run(request("diff2", "json", &[r#"{"x":1,"x":2}"#, "{}"]));
    assert!(!result.ok);
    assert!(result.diff.is_none());
    let mut unsupported = request("diff2", "json", &["{}", "{}"]);
    let OperationPolicy::Diff2(policy) = &mut unsupported.operation else { panic!() };
    policy.equivalence = Some(vec!["ignore-whitespace".into()]);
    let (result, _) = run(unsupported);
    assert!(!result.ok);
    assert!(!result.extra.contains_key("input_parses"));
}

#[test]
fn common_json_diff_recomputes_evidence_and_preserves_compatible_unknown_fields() {
    let (result, request) = run(request("diff2", "json", &[r#"{"x":1}"#, r#"{"x":2}"#]));
    assert!(result.ok);
    for mutation in 0..8 {
        let mut forged = result.clone();
        match mutation {
            0 => forged.changes[1].classification = "added".into(),
            1 => {
                forged.changes[1].role_states.get_mut("before").unwrap()["owner"]["sha256"] =
                    json!("0".repeat(64))
            }
            2 => {
                forged.changes.remove(0);
                forged.diff.as_mut().unwrap().change_ids.remove(0);
            }
            3 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["source"]["role"] =
                    json!("after")
            }
            4 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["backend"]["languages"] =
                    json!(["yaml"])
            }
            5 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["nodes"][0]["span"]["range"]
                    ["end_byte"] = json!(99999)
            }
            6 => forged
                .diff
                .as_mut()
                .unwrap()
                .extra
                .insert("document_bytes_compared".into(), json!(false))
                .map(|_| ())
                .unwrap(),
            _ => forged.verification.output_reparsed = Some(true),
        }
        assert!(forged.validate_against(&request).is_err(), "mutation {mutation}");
    }
    let mut extended = result.clone();
    extended.changes[0].extra.insert("future".into(), json!({"passive":true}));
    extended.changes[1].role_states.get_mut("before").unwrap()["owner"]["future"] =
        json!(["retained"]);
    extended.diff.as_mut().unwrap().extra.insert("future".into(), json!(true));
    extended.validate_against(&request).unwrap();
    let roundtrip: OperationResult =
        serde_json::from_value(serde_json::to_value(&extended).unwrap()).unwrap();
    assert_eq!(roundtrip, extended);
}

#[test]
fn common_directional_nested_json_has_replayed_source_evidence() {
    let (result, request) = run(request(
        "merge2",
        "json5",
        &["{x:{added:'é'},arr:[3]}", "// note\r\n{\r\n x:{keep:1},\r\n arr:[1,2]\r\n}"],
    ));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.output.as_ref().unwrap().contains("// note\r\n"));
    assert!(result.output.as_ref().unwrap().contains("added"));
    assert_eq!(result.verification.directional_roles_preserved, Some(true));
    assert_eq!(result.verification.base_participated, None);
    let proof: json_merge::render_evidence::JsonRenderEvidence =
        serde_json::from_value(result.render_report["evidence"].clone()).unwrap();
    proof
        .validate(request.sources().get("current").unwrap(), result.output.as_ref().unwrap())
        .unwrap();
    assert!(!result.verification.retained_source_regions.unwrap().is_empty());
}

#[test]
fn common_json_rejects_tampered_render_and_output_parse_evidence() {
    let (result, request) = run(request("merge2", "json", &["{\"add\":1}", "{\n\"keep\":2\n}"]));
    assert!(result.ok);
    for mutation in 0..5 {
        let mut forged = result.clone();
        match mutation {
            0 => {
                forged.render_report.get_mut("evidence").unwrap()["retained"][0]["sha256"] =
                    json!("0".repeat(64))
            }
            1 => forged.output.as_mut().unwrap().push(' '),
            2 => forged.verification.retained_source_regions.as_mut().unwrap().clear(),
            3 => forged.extra.get_mut("output_parse").unwrap()["backend"]["id"] = json!("wrong"),
            _ => {
                forged.render_report.remove("evidence");
            }
        }
        assert!(forged.validate_against(&request).is_err());
    }
}

#[test]
fn common_merge3_proves_selected_theirs_and_nested_independent_changes() {
    let (selected, _) =
        run(request("merge3", "json", &["{\"x\":1}", "{\"x\":1}", "{\r\n\"x\":2\r\n}"]));
    assert!(selected.ok);
    assert_eq!(selected.render_report["evidence"]["baseline"]["role"], "theirs");
    assert_eq!(selected.verification.base_participated, Some(true));
    let (merged, _) = run(request(
        "merge3",
        "json",
        &[r#"{"x":{"a":1,"b":2}}"#, r#"{"x":{"a":3,"b":2}}"#, r#"{"x":{"a":1,"b":4}}"#],
    ));
    assert!(merged.ok, "{:?}", merged.diagnostics);
    assert_eq!(
        serde_json::from_str::<Value>(merged.output.as_ref().unwrap()).unwrap(),
        json!({"x":{"a":3,"b":4}})
    );
}

#[test]
fn common_conflicts_keep_native_categories_and_real_present_absent_regions() {
    for texts in [
        [r#"{"x":1}"#, r#"{"x":2}"#, r#"{"x":3}"#],
        [r#"{"x":1}"#, "{}", r#"{"x":3}"#],
        ["1", "2", "3"],
    ] {
        let (result, _) = run(request("merge3", "json", &texts));
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(!result.conflicts.is_empty());
        let portable_conflict::ConflictRecord::Canonical(conflict) = &result.conflicts[0] else {
            panic!()
        };
        assert!(
            !conflict.classification.extra["native_conflict"]["category"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        assert_eq!(conflict.alternatives.len(), 3);
        for alternative in &conflict.alternatives {
            if alternative.state == portable_conflict::AlternativeState::Absent {
                assert!(alternative.regions.is_empty());
            } else {
                assert!(!alternative.regions.is_empty());
            }
        }
    }
}

#[test]
fn common_json_rejects_invalid_dialects_syntax_and_unimplemented_constraints() {
    for (dialect, texts) in [
        ("json", ["{}", "{"]),
        ("json", ["{}", "// forbidden\n{}"]),
        ("jsonc", ["{}", "{unquoted:1}"]),
    ] {
        let (result, _) = run(request("merge2", dialect, &texts));
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(!result.diagnostics.is_empty());
    }
    let mut unsupported = request("merge2", "json", &["{}", "{}"]);
    unsupported.provider_selection.required_capabilities.push("invented".into());
    unsupported.provider_selection.required_capabilities.sort();
    let (result, _) = run(unsupported);
    assert!(!result.ok);
    assert!(!result.extra.contains_key("input_parses"));
}

#[test]
fn exported_facade_routes_json_and_honors_cancellation() {
    register_language_pack_parser("json.common.api".into(), "json".into()).unwrap();
    let limits = ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let control = create_operation_control();
    control.cancel();
    for operation in ["merge2", "diff2"] {
        let mut request = request(operation, "json", &["{\"add\":1}", "{}"]);
        request.parser_selection.backend = Some("json.common.api".into());
        assert_eq!(
            execute_operation_controlled(request.clone(), limits.clone(), &control)
                .unwrap_err()
                .code,
            "execution.cancelled"
        );
        assert!(execute_operation(request, limits.clone()).unwrap().ok);
    }
    unregister_parser_provider("json.common.api".into()).unwrap();
}

#[test]
fn output_parser_failure_never_exposes_success_or_a_render() {
    struct FailingOutput(LanguagePackProvider);
    impl ParserProvider for FailingOutput {
        fn descriptor(&self) -> &ParserProviderDescriptor {
            self.0.descriptor()
        }
        fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
            self.0.probe(request)
        }
        fn parse_batch(
            &self,
            requests: Vec<ParseRequest>,
            context: &ExecutionContext,
        ) -> Result<Vec<ParseOutput>, ProviderFault> {
            if requests.iter().any(|request| request.source.descriptor.role == SourceRole::Output) {
                return Err(ProviderFault {
                    code: "test.output_refused".into(),
                    message: "private parser exception".into(),
                });
            }
            self.0.parse_batch(requests, context)
        }
    }
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(FailingOutput(
            LanguagePackProvider::new("json.common".into(), "json".into()).unwrap(),
        )))
        .unwrap();
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 10000,
        max_nodes: 1000,
        max_diagnostics: 20,
    };
    for operation in ["merge2", "merge3"] {
        let texts = if operation == "merge2" { vec!["{}", "{}"] } else { vec!["{}", "{}", "{}"] };
        let request = request(operation, "json", &texts).validate(10000, |_, _| panic!()).unwrap();
        let result = native_operation::execute_native_operation(
            &request,
            &registry.snapshot().unwrap(),
            &context,
        )
        .unwrap();
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(result.render_report.is_empty());
        let portable_diagnostic::DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0]
        else {
            panic!()
        };
        assert_eq!(diagnostic.origin.native_code.as_deref(), Some("test.output_refused"));
        assert!(!diagnostic.message.contains("private"));
        result.validate_against(&request).unwrap();
    }
}
