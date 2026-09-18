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

fn isolated_cached_only(mode: &str, grammar: Option<&std::path::Path>) {
    use std::{
        fs,
        net::TcpListener,
        process::Command,
        time::{Duration, Instant},
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(&root).unwrap();
    let libs = dir.path().join("libs");
    let cache = dir.path().join("cache");
    fs::create_dir(&libs).unwrap();
    fs::create_dir(&cache).unwrap();
    let library = libs.join(tree_sitter_language_pack::registry::library_file_name("json"));
    if let Some(grammar) = grammar {
        fs::copy(grammar, &library).unwrap();
    } else if mode == "corrupt" {
        fs::write(&library, "not a grammar library").unwrap();
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cached_only_isolated_child", "--nocapture"])
        .env("TREE_HAVER_CACHE_ONLY_CHILD", mode)
        .env("TREE_SITTER_LANGUAGE_PACK_LIBS_DIR", &libs)
        .env("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", &cache)
        .env("TREE_SITTER_LANGUAGE_PACK_CACHE_DIR", &cache)
        .env("HTTPS_PROXY", &proxy)
        .env("HTTP_PROXY", &proxy)
        .env("ALL_PROXY", &proxy)
        .env("https_proxy", &proxy)
        .env("http_proxy", &proxy)
        .env("all_proxy", &proxy)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("cached-only child timed out: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{mode}: {output:?}");
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "unexpected network proxy connection"
    );
    assert_eq!(fs::read_dir(cache).unwrap().count(), 0, "no grammar/cache acquisition");
}

#[test]
fn cached_only_cold_and_corrupt_grammars_do_not_acquire() {
    isolated_cached_only("cold", None);
    isolated_cached_only("corrupt", None);
}

#[test]
#[ignore = "requires an explicitly supplied existing JSON grammar, never downloads"]
fn cached_only_warm_grammar_survives_cache_file_removal() {
    let grammar = std::env::var_os("TREE_HAVER_CACHED_JSON_LIBRARY")
        .expect("supply a preinstalled grammar library");
    isolated_cached_only("warm", Some(std::path::Path::new(&grammar)));
}

#[test]
fn cached_only_isolated_child() {
    let Ok(mode) = std::env::var("TREE_HAVER_CACHE_ONLY_CHILD") else { return };
    let parser =
        LanguagePackProvider::new_cached_only("typed.tslp.json".into(), "json".into()).unwrap();
    assert_eq!(parser.descriptor().metadata["grammar_policy"], "cached-only");
    let probe = parser.probe(&ParserProbeRequest { language: "json".into(), dialect: None });
    if mode != "warm" {
        assert_eq!(probe.unwrap_err().code, "parser.local_unavailable");
        assert_eq!(
            parser.parse_batch(vec![request("{}", "cold")], &context()).unwrap_err().code,
            "parser.local_unavailable"
        );
        return;
    }
    assert!(probe.unwrap().available);
    let libs = std::env::var_os("TREE_SITTER_LANGUAGE_PACK_LIBS_DIR").unwrap();
    std::fs::remove_file(
        std::path::Path::new(&libs)
            .join(tree_sitter_language_pack::registry::library_file_name("json")),
    )
    .unwrap();
    let registry = ParserRegistry::default();
    registry.register(Arc::new(parser)).unwrap();
    let text = "{\r\n  \"é\": true\r\n}";
    let results = TreeHaverParseService::default()
        .parse_batch(vec![request(text, "warm")], &registry.snapshot().unwrap(), &context())
        .unwrap();
    assert!(results[0].document.output().ok);
    assert_eq!(results[0].document.output().source.byte_length, text.len() as u64);
    assert_eq!(results[0].selection.selected_backend.as_deref(), Some("typed.tslp.json"));
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
    request.options.comments = false;
    request.options.native_extensions = true;
    let results = TreeHaverParseService::default()
        .parse_batch(vec![request.clone()], &registry(), &context())
        .unwrap();
    let output = results[0].document.output();
    assert_eq!(
        output.comments.len(),
        1,
        "native comment index is required even without enrichment"
    );
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
