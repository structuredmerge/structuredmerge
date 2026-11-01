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

fn merge3_output(base: &str, ours: &str, theirs: &str) -> (bool, Option<String>) {
    let (snapshot, context) = setup();
    let input = request("merge3", &[base, ours, theirs]).validate(100000, |_, _| panic!()).unwrap();
    let result = native_operation::execute_native_operation(&input, &snapshot, &context).unwrap();
    if result.ok {
        result.validate_against(&input).unwrap();
    }
    (result.ok, result.output)
}

#[test]
fn owner_membership_changes_with_independent_edits_merge_by_whole_line_selection() {
    // (name, base, ours, theirs, expected output; None = must stay fail-closed)
    let cases: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        (
            "documented owner deleted",
            "package main\n\n// a docs\nfunc a() {}\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc b() { return 1 }\n",
            "package main\n\n// a docs\nfunc a() {}\n\nfunc b() { return 2 }\n",
            Some("package main\n\nfunc b() { return 2 }\n"),
        ),
        (
            "trailing comment leaves with its owner",
            "package main\n\nfunc a() {} // note\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc a() {} // note\n\nfunc b() { return 2 }\n",
            Some("package main\n\nfunc b() { return 2 }\n"),
        ),
        (
            "delete with independent addition",
            "package main\n\nfunc a() {}\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc a() {}\n\nfunc b() { return 1 }\n\nfunc c() {}\n",
            Some("package main\n\nfunc b() { return 1 }\n\nfunc c() {}\n"),
        ),
        (
            "delete the last owner",
            "package main\n\nfunc b() { return 1 }\n\n// a\nfunc a() {}\n",
            "package main\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc b() { return 2 }\n\n// a\nfunc a() {}\n",
            Some("package main\n\nfunc b() { return 2 }\n"),
        ),
        (
            "file header stays",
            "// Copyright\n\npackage main\n\n// a\nfunc a() {}\n\nfunc b() { return 1 }\n",
            "// Copyright\n\npackage main\n\nfunc b() { return 1 }\n",
            "// Copyright\n\npackage main\n\n// a\nfunc a() {}\n\nfunc b() { return 2 }\n",
            Some("// Copyright\n\npackage main\n\nfunc b() { return 2 }\n"),
        ),
        (
            "method deleted",
            "package main\n\ntype T struct{}\n\nfunc (t T) M() {}\n\nfunc b() { return 1 }\n",
            "package main\n\ntype T struct{}\n\nfunc b() { return 1 }\n",
            "package main\n\ntype T struct{}\n\nfunc (t T) M() {}\n\nfunc b() { return 2 }\n",
            Some("package main\n\ntype T struct{}\n\nfunc b() { return 2 }\n"),
        ),
        (
            "delete versus edit of the same owner",
            "package main\n\n// a\nfunc a() { 1 }\n\nfunc b() {}\n",
            "package main\n\nfunc b() {}\n",
            "package main\n\n// a\nfunc a() { 2 }\n\nfunc b() {}\n",
            None,
        ),
        (
            "floating comment beside a deleted owner stays fail-closed",
            "package main\n\nfunc a() {}\n\n// floating note\n\nfunc b() { return 1 }\n",
            "package main\n\n// floating note\n\nfunc b() { return 1 }\n",
            "package main\n\nfunc a() {}\n\n// floating note\n\nfunc b() { return 2 }\n",
            None,
        ),
        (
            "grouped declaration membership keeps the Go guard",
            "package main\n\nvar (\n\tx = 1\n)\n\nfunc b() { return 1 }\n",
            "package main\n\nvar (\n\tx = 1\n\ty = 2\n)\n\nfunc b() { return 1 }\n",
            "package main\n\nvar (\n\tx = 1\n)\n\nfunc b() { return 2 }\n",
            None,
        ),
    ];
    for (name, base, ours, theirs, expected) in cases {
        let (ok, output) = merge3_output(base, ours, theirs);
        assert_eq!(ok, expected.is_some(), "{name}");
        assert_eq!(output.as_deref(), expected, "{name}");
    }
}
