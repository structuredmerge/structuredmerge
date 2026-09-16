//! Real typed TSLP integration; explicitly enabled where grammar loading is allowed.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tree_haver::{NodeRole, language_pack_provider::LanguagePackProvider, service::*, source::*};

fn context() -> ExecutionContext {
    ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 10000,
        max_nodes: 1000,
        max_diagnostics: 20,
    }
}
fn request(text: &str, id: &str) -> ParseRequest {
    ParseRequest {
        schema: PARSE_REQUEST_SCHEMA.into(),
        request_id: id.into(),
        source: source_input(
            id.into(),
            SourceRole::Source,
            SourceEncoding::Utf8,
            text.as_bytes().to_vec(),
        )
        .unwrap(),
        language: "json".into(),
        dialect: None,
        selection: ParserSelection {
            backend_id: Some("typed.tslp.json".into()),
            preference: vec![],
            required_capabilities: vec![],
        },
        options: ParseOptions {
            comments: true,
            diagnostics: true,
            tokens: false,
            native_extensions: false,
        },
        metadata: Default::default(),
        extra: Default::default(),
    }
}
fn registry() -> ParserRegistrySnapshot {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(
            LanguagePackProvider::new("typed.tslp.json".into(), "json".into()).unwrap(),
        ))
        .unwrap();
    registry.snapshot().unwrap()
}

#[test]
fn unsupported_selection_and_controls_fail_without_grammar_loading() {
    assert!(LanguagePackProvider::new("".into(), "json".into()).is_err());
    let parser = LanguagePackProvider::new("typed.tslp.json".into(), "json".into()).unwrap();
    assert!(
        !parser
            .probe(&ParserProbeRequest { language: "python".into(), dialect: None })
            .unwrap()
            .available
    );
    let registry = registry();
    let mut request = request("{}", "r");
    request.options.tokens = true;
    assert!(matches!(
        TreeHaverParseService::default().parse_batch(vec![request.clone()], &registry, &context()),
        Err(ServiceError::Selection(_))
    ));
    request.options.tokens = false;
    let cancelled = context();
    cancelled.cancelled.store(true, Ordering::Release);
    assert!(matches!(
        TreeHaverParseService::default().parse_batch(vec![request], &registry, &cancelled),
        Err(ServiceError::Cancelled)
    ));
}

#[test]
#[ignore = "requires language-pack grammar cache/download access"]
fn real_json_preserves_fields_spans_identity_and_request_order() {
    let source = "{\r\n  \"é\": [true, 2]\r\n}";
    let results = TreeHaverParseService::default()
        .parse_batch(
            vec![request(source, "first"), request("{}", "second")],
            &registry(),
            &context(),
        )
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].document.output().request_id, "first");
    assert_eq!(results[1].document.output().request_id, "second");
    let parsed = &results[0];
    assert!(parsed.document.output().ok);
    assert_eq!(parsed.backend.runtime, "rust");
    assert_eq!(parsed.selection.selected_backend.as_deref(), Some("typed.tslp.json"));
    assert!(
        parsed
            .document
            .output()
            .nodes
            .iter()
            .any(|n| n.children.iter().any(|e| e.field_name.as_deref() == Some("key")))
    );
    for node in &parsed.document.output().nodes {
        let range = &node.span.range;
        assert_eq!(
            parsed.source.slice(range.clone()).unwrap(),
            &source.as_bytes()[range.start_byte..range.end_byte]
        );
        assert!(!node.missing && !node.has_error);
    }
}

#[test]
#[ignore = "requires language-pack grammar cache/download access"]
fn syntax_failure_keeps_partial_tree_and_native_error_flags() {
    let result = TreeHaverParseService::default()
        .parse_batch(vec![request("{\"x\":", "bad")], &registry(), &context())
        .unwrap();
    let output = result[0].document.output();
    assert!(!output.ok);
    assert!(output.root_id.is_some());
    assert!(output.nodes.iter().any(|n| n.has_error || n.missing || n.role == NodeRole::Error));
    assert!(output.diagnostics.iter().any(|d| d.blocking && d.source_role == SourceRole::Source));
}

#[test]
#[ignore = "requires language-pack grammar cache/download access"]
fn projection_budget_rejects_without_exposing_a_truncated_tree() {
    let mut context = context();
    context.max_nodes = 1;
    let result = TreeHaverParseService::default().parse_batch(
        vec![request("{\"a\":1}", "limited")],
        &registry(),
        &context,
    );
    assert!(
        matches!(result, Err(ServiceError::Provider { fault, .. }) if fault.code == "resource.limit")
    );
}

#[test]
#[ignore = "requires language-pack grammar cache/download access"]
fn comments_are_native_facts_and_forward_fields_cannot_shadow_output() {
    let mut input = request("// note\n{\"x\": 1}", "comments");
    input.extra.insert("ok".into(), serde_json::json!(false));
    let result =
        TreeHaverParseService::default().parse_batch(vec![input], &registry(), &context()).unwrap();
    let output = result[0].document.output();
    assert!(output.ok);
    assert_eq!(output.comments.len(), 1);
    let comment = &output.comments[0];
    assert_eq!(comment.attachment_hint, tree_haver::parsed::AttachmentHint::Unknown);
    let node = result[0].document.node(&comment.node_id).unwrap();
    assert_eq!(result[0].source.slice(node.span.range.clone()).unwrap(), b"// note");
    assert!(!output.extra.contains_key("ok"));
    assert_eq!(output.extra["request_extra"]["ok"], false);
}

#[test]
#[ignore = "requires language-pack grammar cache/download access"]
fn native_extra_flag_is_versioned_and_opt_in() {
    let mut request = request("// note\n{}", "flags");
    request.options.native_extensions = true;
    let results = TreeHaverParseService::default()
        .parse_batch(vec![request.clone()], &registry(), &context())
        .unwrap();
    let output = results[0].document.output();
    let comment = output.nodes.iter().find(|node| node.role == NodeRole::Comment).unwrap();
    assert_eq!(comment.extensions[0].schema, "tree-haver.tree-sitter.node/v1");
    assert_eq!(comment.extensions[0].payload["extra"], true);
    assert_eq!(output.nodes[0].extensions[0].payload["extra"], false);
    request.options.native_extensions = false;
    let results = TreeHaverParseService::default()
        .parse_batch(vec![request], &registry(), &context())
        .unwrap();
    assert!(results[0].document.output().nodes.iter().all(|node| node.extensions.is_empty()));
}
