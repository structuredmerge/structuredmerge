use structuredmerge_core::*;

static REGISTRATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn cached_registration_is_explicit_and_cannot_be_replaced_by_legacy_registration() {
    let _guard = REGISTRATION_LOCK.lock().unwrap();
    let id = "core.registration.cached".to_string();
    assert_eq!(
        register_cached_language_pack_parser("".into(), "json".into()).unwrap_err().code,
        "request.invalid"
    );
    let descriptor =
        register_cached_language_pack_parser(id.clone(), "not-a-real-grammar".into()).unwrap();
    assert_eq!(descriptor.metadata["grammar_policy"], "cached-only");
    assert!(
        register_language_pack_parser(id.clone(), "json".into())
            .unwrap_err()
            .message
            .contains("DuplicateId")
    );
    let report = parser_selection_report(
        ParserSelectionRequest {
            language: "not-a-real-grammar".into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: Some(id.clone()),
                preference: vec![],
                required_capabilities: vec![],
            },
            options: ParseOptions::default(),
        },
        ParseLimits {
            max_batch_items: 1,
            max_input_bytes: 0,
            max_nodes: 0,
            max_diagnostics: 0,
            timeout_millis: None,
        },
    )
    .unwrap();
    assert!(report.selected_backend.is_none());
    unregister_parser_provider(id).unwrap();
}

#[test]
fn native_registration_is_explicit_non_loading_and_shares_removal() {
    let _guard = REGISTRATION_LOCK.lock().unwrap();
    assert_eq!(
        register_language_pack_parser("".into(), "json".into()).unwrap_err().code,
        "request.invalid"
    );
    let id = "core.registration.nonloading".to_string();
    // An unavailable grammar can be registered: only probe/parse may load it.
    let descriptor =
        register_language_pack_parser(id.clone(), "not-a-real-grammar".into()).unwrap();
    assert_eq!(descriptor.id, id);
    assert_eq!(descriptor.runtime, "rust");
    assert_eq!(descriptor.languages, vec!["not-a-real-grammar"]);
    assert!(
        register_language_pack_parser(id.clone(), "json".into())
            .unwrap_err()
            .message
            .contains("DuplicateId")
    );
    unregister_parser_host(id.clone()).unwrap();
    assert!(unregister_parser_provider(id.clone()).unwrap_err().message.contains("UnknownId"));
    register_language_pack_parser(id.clone(), "json".into()).unwrap();
    unregister_parser_provider(id).unwrap();
}
