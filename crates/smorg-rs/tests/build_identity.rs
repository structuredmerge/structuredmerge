#[path = "../build_support.rs"]
mod build_support;
use std::collections::BTreeMap;

fn environment() -> BTreeMap<String, String> {
    [
        ("TARGET", "test-target"),
        ("HOST", "test-host"),
        ("PROFILE", "debug"),
        ("OPT_LEVEL", "0"),
        ("DEBUG", "false"),
    ]
    .into_iter()
    .map(|(key, value)| (key.into(), value.into()))
    .collect()
}

#[test]
fn absent_source_identity_stays_unknown_and_does_not_read_a_checkout() {
    let rendered = build_support::render(&environment()).unwrap();
    assert!(rendered.contains("SOURCE_REVISION: Option<&str> = None"));
    assert!(rendered.contains("SOURCE_STATE: &str = \"unknown\""));
}

#[test]
fn source_identity_requires_explicit_complete_revision_and_valid_state() {
    let mut vars = environment();
    for (revision, state, valid) in [
        ("a".repeat(40), "dirty", true),
        ("b".repeat(64), "clean", true),
        ("c".repeat(40), "unknown", true),
        ("a".repeat(7), "clean", false),
        ("G".repeat(40), "clean", false),
        ("d".repeat(40), "verified", false),
        (String::new(), "unknown", false),
    ] {
        vars.insert("SMORG_BUILD_REVISION".into(), revision);
        vars.insert("SMORG_BUILD_SOURCE_STATE".into(), state.into());
        assert_eq!(build_support::render(&vars).is_ok(), valid);
    }
    vars.remove("SMORG_BUILD_REVISION");
    vars.insert("SMORG_BUILD_SOURCE_STATE".into(), "clean".into());
    assert!(build_support::render(&vars).is_err());
    vars = environment();
    vars.remove("TARGET");
    assert!(build_support::render(&vars).is_err());
}

#[test]
fn build_fields_are_escaped_and_feature_lists_are_deterministic() {
    let mut vars = environment();
    vars.insert("TARGET".into(), "target\"\ntext".into());
    vars.insert("CARGO_FEATURE_ZED".into(), "1".into());
    vars.insert("CARGO_FEATURE_ALPHA_BETA".into(), "1".into());
    vars.insert("CARGO_CFG_TARGET_FEATURE".into(), "sse2,,sse,sse2".into());
    let rendered = build_support::render(&vars).unwrap();
    assert!(rendered.contains("target\\\"\\ntext"));
    assert!(rendered.contains("[\"CARGO_FEATURE_ALPHA_BETA\", \"CARGO_FEATURE_ZED\"]"));
    assert!(rendered.contains("[\"sse\", \"sse2\"]"));
    assert_eq!(rendered, build_support::render(&vars).unwrap());
}
