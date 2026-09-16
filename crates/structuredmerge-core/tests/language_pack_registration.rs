use structuredmerge_core::*;

#[test]
fn native_registration_is_explicit_non_loading_and_shares_removal() {
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
