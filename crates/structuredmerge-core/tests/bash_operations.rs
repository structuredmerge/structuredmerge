use serde_json::json;
use std::sync::{Arc, atomic::AtomicBool};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(operation: &str, texts: &[&str]) -> OperationRequest {
    let roles: &[&str] = match operation {
        "analyze" => &["source"],
        "diff2" => &["before", "after"],
        "merge2" => &["incoming", "current"],
        _ => &["base", "ours", "theirs"],
    };
    let sources: serde_json::Map<_, _> = roles
        .iter()
        .zip(texts)
        .map(|(role, text)| {
            let source = source_input(
                role.to_string(),
                serde_json::from_value(json!(role)).unwrap(),
                SourceEncoding::Utf8,
                text.as_bytes().to_vec(),
            )
            .unwrap();
            (
                role.to_string(),
                json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8",
            "byte_length":source.descriptor.byte_length,"sha256":source.descriptor.sha256}),
            )
        })
        .collect();
    serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":"bash-common",
        "operation":operation,"sources":sources,"extensions":[],"metadata":{},
        "provider_selection":{"provider_id":"kernel.bash","family":"bash","dialect":"bash",
            "profile_id":"kernel.bash.owners.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"bash.common","preference":[],"required_capabilities":[]},
        "policy":match operation {"analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}),
            "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"})}
    })).unwrap()
}

fn run(input: OperationRequest) -> (OperationResult, ValidatedOperationRequest) {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(LanguagePackProvider::new("bash.common".into(), "bash".into()).unwrap()))
        .unwrap();
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
    };
    let input = input.validate(100000, |_, _| panic!()).unwrap();
    let result =
        native_operation::execute_native_operation(&input, &registry.snapshot().unwrap(), &context)
            .unwrap();
    // Canonical decision references require trusted executor evidence; finalize
    // already validated them. The public data-only validator cannot invent it.
    if result.conflicts.is_empty() {
        result.validate_against(&input).unwrap();
    }
    (result, input)
}

#[test]
fn bash_analysis_retains_native_owner_ids_and_exact_layout() {
    let (result, input) = run(request("analyze", &["# é\nx=1\n\n# next\nf() { :; }\n"]));
    assert!(result.ok, "{:?}", result.diagnostics);
    let facts = &result.analysis.as_ref().unwrap().extra;
    let owners = facts["owners"].as_array().unwrap();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0]["id"], "/variable:x");
    assert_eq!(owners[1]["id"], "/function:f");
    let nodes = facts["parse_result"]["parsed"]["nodes"].as_array().unwrap();
    for owner in owners {
        let node = nodes.iter().find(|node| node["id"] == owner["node_id"]).unwrap();
        assert_eq!(node["span"], owner["span"]);
    }
    assert!(!facts["layout_gaps"].as_array().unwrap().is_empty());
    // Comments remain exact layout bytes, not invented attachment decisions.
    assert!(facts["comment_regions"].as_array().unwrap().is_empty());
    let mut bad = result.clone();
    bad.analysis.as_mut().unwrap().extra.get_mut("owners").unwrap()[0]["node_id"] =
        json!("invented");
    assert!(bad.validate_against(&input).is_err());
    let mut future = result.clone();
    future.analysis.as_mut().unwrap().extra.insert("future_passive".into(), json!(1));
    future.validate_against(&input).unwrap();
}

#[test]
fn bash_diff_is_rust_owned_and_includes_document_trivia() {
    let (result, _) =
        run(request("diff2", &["# before\nx=1\nf() { :; }\n", "# after\nx=2\nf() { :; }\n"]));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.changes.iter().any(|change| change.path.as_deref() == Some("/variable:x")));
    assert!(!result.changes.iter().any(|change| change.path.as_deref() == Some("/function:f")));
    assert!(result.output.is_none());
    let (trivia, _) = run(request("diff2", &["# before\nx=1\n", "# after\nx=1\n"]));
    assert!(trivia.ok);
    assert!(!trivia.changes.is_empty());
}

#[test]
fn bash_merge3_has_exact_render_evidence_and_fresh_noop_parse() {
    for texts in [["x=1\ny=1\n", "x=2\ny=1\n", "x=1\ny=2\n"], ["# é\nx=1\n"; 3]] {
        let (result, _) = run(request("merge3", &texts));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.verification.base_participated, Some(true));
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert!(!result.verification.retained_source_regions.as_ref().unwrap().is_empty());
        assert_eq!(result.extra["output_parse"]["parsed"]["source"]["role"], "output");
        assert_eq!(result.render_report["producer"], "kernel.bash");
    }
}

#[test]
fn bash_conflicts_and_failures_do_not_invent_outputs_or_capabilities() {
    let (result, _) = run(request("merge3", &["x=1\n", "x=2\n", "x=3\n"]));
    assert!(!result.ok);
    assert_eq!(result.conflicts.len(), 1);
    assert!(result.output.is_none() && result.conflicted_output.is_none());
    for text in ["echo unsupported\n", "f() {", "x=1\nx=2\n"] {
        let (result, _) = run(request("analyze", &[text]));
        assert!(!result.ok, "{text}");
    }
    let (result, _) = run(request("merge2", &["x=1\n", "x=2\n"]));
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("x=2\n"));
    for policy in [json!({"comments":true}), json!({"tokens":true}), json!({"ownership":false})] {
        let mut value = serde_json::to_value(request("analyze", &["x=1\n"])).unwrap();
        value["policy"] = policy;
        assert!(!run(serde_json::from_value(value).unwrap()).0.ok);
    }
    let mut input = request("merge3", &["x=1\n"; 3]);
    let OperationPolicy::Merge3(policy) = &mut input.operation else { panic!() };
    policy.conflict_marker_size = Some(9);
    assert!(!run(input).0.ok);
    let mut input = request("analyze", &["x=1\n"]);
    input.provider_selection.dialect = Some("zsh".into());
    assert!(!run(input).0.ok);
}

#[test]
fn exported_bash_profile_honors_explicit_selection_and_cancellation() {
    register_language_pack_parser("bash.facade".into(), "bash".into()).unwrap();
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
        let mut input = request(operation, &["x=1\n"; 3]);
        input.parser_selection.backend = Some("bash.facade".into());
        assert_eq!(
            execute_operation_controlled(input.clone(), limits.clone(), &control).unwrap_err().code,
            "execution.cancelled"
        );
        assert!(execute_operation(input.clone(), limits.clone()).unwrap().ok);
        input.parser_selection.backend = Some("bash.missing".into());
        assert!(!execute_operation(input, limits.clone()).unwrap().ok);
    }
    unregister_parser_provider("bash.facade".into()).unwrap();
}

#[test]
fn bash_merge2_preserves_current_and_inserts_native_owner_fragments() {
    for (incoming, current, expected) in [
        ("x=1\n", "", "x=1\n"),
        ("", "x=9\n", "x=9\n"),
        ("x=1\n", "# current header\n", "# current header\nx=1\n"),
        ("x=1\ny=2\n", "x=9\n", "x=9\ny=2\n"),
        (
            "x=1\n# incoming é\ny=2 # inline\nz=3\n",
            "# current header\nx=9 # keep inline\n# current z\nz=8\n# footer\n",
            "# current header\nx=9 # keep inline\n# incoming é\ny=2 # inline\n# current z\nz=8\n# footer\n",
        ),
        (
            "#!/bin/bash\nnew=2\nx=1\n",
            "#!/usr/bin/env bash\nx=9\n",
            "#!/usr/bin/env bash\nnew=2\nx=9\n",
        ),
        ("x=1\na=2\nb=3\n", "x=9\n# footer\n", "x=9\na=2\nb=3\n# footer\n"),
        ("x=1\n", "x=9", "x=9"),
    ] {
        let (result, _) = run(request("merge2", &[incoming, current]));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), Some(expected));
        assert_eq!(result.verification.directional_roles_preserved, Some(true));
        assert_eq!(result.verification.base_participated, None);
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(
            result.verification.consumed_source_roles,
            Some(vec![SourceRole::Incoming, SourceRole::Current])
        );
        for retained in result.verification.retained_source_regions.unwrap() {
            let source =
                if retained.source_role == SourceRole::Incoming { incoming } else { current };
            let output = &retained.extra["output_range"];
            let start = output["start_byte"].as_u64().unwrap() as usize;
            let end = output["end_byte"].as_u64().unwrap() as usize;
            assert_eq!(
                &source.as_bytes()[retained.range.start_byte..retained.range.end_byte],
                &expected.as_bytes()[start..end]
            );
        }
    }
}

#[test]
fn bash_merge2_rejects_ambiguous_placement_without_exposing_output() {
    for (incoming, current) in [
        ("b=2\nnew=3\na=1\n", "a=9\nb=8\n"),
        ("x=1\ny=2", "x=9\n"),
        ("x=1\ny=2\n", "x=9"),
        ("x=1; y=2\n", "x=9\n"),
        ("x=1\ny=2\n", "x=9; z=8\n"),
    ] {
        let (result, _) = run(request("merge2", &[incoming, current]));
        assert!(!result.ok, "{incoming:?} into {current:?}");
        assert!(result.output.is_none());
        assert!(result.verification.retained_source_regions.is_none());
        assert!(!result.diagnostics.is_empty());
    }
}
