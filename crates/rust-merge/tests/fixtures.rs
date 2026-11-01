use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceManifest, conformance_family_feature_profile_path, conformance_fixture_path,
};
use rust_merge::{
    RustBackend, RustDialect, match_rust_owners, merge_rust, merge_rust_three_way,
    merge_rust_three_way_with_backend, merge_rust_with_backend, parse_rust,
    parse_rust_with_backend, rust_backend_feature_profile, rust_backends, rust_feature_profile,
    rust_plan_context,
};
use serde_json::Value;

fn fixture_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("..");
    path.push("fixtures");
    for part in parts {
        path.push(part);
    }
    path
}

#[test]
fn merges_independent_function_edits_with_exact_source_preservation() {
    let base = "use std::fmt;\n\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
    let ours = "use std::fmt;\n\nfn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
    let theirs = "use std::fmt;\n\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 2 }\n";

    let result = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean);
    assert_eq!(
        result.output.as_deref(),
        Some("use std::fmt;\n\nfn left() -> i32 { 2 }\n\nfn right() -> i32 { 2 }\n")
    );
}

#[test]
fn consumes_shared_malformed_owner_fixture() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_merge3_malformed_owner.json"]);
    let input = &fixture["input"];
    let result = merge_rust_three_way(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
    );
    assert_ne!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean);
    assert!(!fixture["assertions"][0]["value"].as_bool().unwrap());
}

#[test]
fn rejects_owner_membership_changes_with_independent_edits() {
    let base = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
    let ours = "fn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
    let theirs = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n\nfn added() -> i32 { 3 }\n";

    let result = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(result.conflicts[0].category, "unmanaged_source_change");
    assert_eq!(result.conflicts[0].path, "<unmanaged-source>");
}

#[test]
fn merges_independent_named_item_edits_with_exact_source_preservation() {
    let base = "const LIMIT: usize = 1;\n\nstruct Config {\n    value: usize,\n}\n";
    let ours = "const LIMIT: usize = 2;\n\nstruct Config {\n    value: usize,\n}\n";
    let theirs = "const LIMIT: usize = 1;\n\nstruct Config {\n    value: u64,\n}\n";

    let result = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean);
    assert_eq!(
        result.output.as_deref(),
        Some("const LIMIT: usize = 2;\n\nstruct Config {\n    value: u64,\n}\n")
    );
}

#[test]
fn rejects_an_independent_edit_with_a_deleted_rust_owner() {
    let base = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
    let ours = "fn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
    let theirs = "fn left() -> i32 { 1 }\n";

    let result = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(result.conflicts[0].category, "unmanaged_source_change");
    assert_eq!(result.conflicts[0].path, "<unmanaged-source>");
}

#[test]
fn rejects_an_independent_edit_with_an_added_rust_owner() {
    let base = "fn left() -> i32 { 1 }\n";
    let ours = "fn left() -> i32 { 2 }\n";
    let theirs = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 3 }\n";

    let result = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(result.conflicts[0].category, "unmanaged_source_change");
    assert_eq!(result.conflicts[0].path, "<unmanaged-source>");
}

#[test]
fn rejects_ambiguous_and_malformed_rust_three_way_inputs_and_checks_native_conflicts() {
    let base = "fn value() -> i32 { 1 }\n";
    let ours = "fn value() -> i32 { 2 }\n";
    let theirs = "fn value() -> i32 { 3 }\n";
    let conflict = merge_rust_three_way(base, ours, theirs, RustDialect::Rust);
    assert_eq!(conflict.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(conflict.conflicts[0].path, "/function:value");

    let malformed = merge_rust_three_way(base, ours, "fn value( {\n", RustDialect::Rust);
    assert_eq!(malformed.outcome, ast_merge::ThreeWayMergeOutcome::Error);
    assert_eq!(malformed.diagnostics[0].category, ast_merge::DiagnosticCategory::ParseError);

    let native = merge_rust_three_way_with_backend(
        base,
        ours,
        theirs,
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(native.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(native.conflicts[0].path, "/function:value");
}

fn read_fixture(parts: &[&str]) -> Value {
    let source = fs::read_to_string(fixture_path(parts)).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

fn read_manifest(parts: &[&str]) -> ConformanceManifest {
    let source = fs::read_to_string(fixture_path(parts)).expect("manifest should be readable");
    serde_json::from_str(&source).expect("manifest should be valid json")
}

fn diagnostic_shape(diagnostics: &[ast_merge::Diagnostic]) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|diagnostic| {
                serde_json::json!({
                    "severity": match diagnostic.severity {
                        ast_merge::DiagnosticSeverity::Info => "info",
                        ast_merge::DiagnosticSeverity::Warning => "warning",
                        ast_merge::DiagnosticSeverity::Error => "error",
                    },
                    "category": match diagnostic.category {
                        ast_merge::DiagnosticCategory::ParseError => "parse_error",
                        ast_merge::DiagnosticCategory::DestinationParseError => "destination_parse_error",
                        ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                        ast_merge::DiagnosticCategory::UnsafeSourceRange => "unsafe_source_range",
                        ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                        ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                        ast_merge::DiagnosticCategory::MergeConflict => "merge_conflict",
                        ast_merge::DiagnosticCategory::AssumedDefault => "assumed_default",
                        ast_merge::DiagnosticCategory::ConfigurationError => "configuration_error",
                        ast_merge::DiagnosticCategory::ReplayRejected => "replay_rejected",
                        ast_merge::DiagnosticCategory::KindMismatch => "kind_mismatch",
                        ast_merge::DiagnosticCategory::UnsupportedVersion => "unsupported_version",
                    }
                })
            })
            .collect(),
    )
}

fn assert_structured_import_failure(diagnostics: &[ast_merge::Diagnostic]) {
    assert_eq!(
        diagnostics.first().map(|diagnostic| diagnostic.category),
        Some(ast_merge::DiagnosticCategory::UnsupportedFeature)
    );
    assert!(
        diagnostics.first().is_some_and(|diagnostic| diagnostic
            .message
            .contains("structured import module fields"))
    );
}

#[test]
fn conforms_to_rust_fixtures() {
    let profile_fixture = read_fixture(&[
        "diagnostics",
        "slice-105-rust-family-feature-profile",
        "rust-feature-profile.json",
    ]);
    let profile = rust_feature_profile();
    assert_eq!(profile.family, profile_fixture["feature_profile"]["family"].as_str().unwrap());

    let analysis_fixture = read_fixture(&["rust", "slice-106-analysis", "module-owners.json"]);
    let analysis = parse_rust(analysis_fixture["source"].as_str().unwrap(), RustDialect::Rust);
    if analysis.ok {
        let owners = analysis
            .analysis
            .as_ref()
            .unwrap()
            .owners
            .iter()
            .map(|owner| {
                serde_json::json!({
                    "path": owner.path,
                    "owner_kind": match owner.owner_kind {
                        rust_merge::RustOwnerKind::Import => "import",
                        rust_merge::RustOwnerKind::Declaration => "declaration",
                    },
                    "match_key": owner.match_key,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(owners), analysis_fixture["expected"]["owners"]);
    } else {
        assert_structured_import_failure(&analysis.diagnostics);
    }

    let matching_fixture = read_fixture(&["rust", "slice-107-matching", "path-equality.json"]);
    let template = parse_rust(matching_fixture["template"].as_str().unwrap(), RustDialect::Rust);
    let destination =
        parse_rust(matching_fixture["destination"].as_str().unwrap(), RustDialect::Rust);
    if template.ok && destination.ok {
        let matched = match_rust_owners(
            template.analysis.as_ref().unwrap(),
            destination.analysis.as_ref().unwrap(),
        );
        assert_eq!(
            Value::Array(
                matched
                    .matched
                    .iter()
                    .map(|entry| serde_json::json!([entry.template_path, entry.destination_path]))
                    .collect()
            ),
            matching_fixture["expected"]["matched"]
        );
    } else {
        assert_structured_import_failure(if template.ok {
            &destination.diagnostics
        } else {
            &template.diagnostics
        });
    }

    let merge_fixture = read_fixture(&["rust", "slice-108-merge", "module-merge.json"]);
    let merge_result = merge_rust(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        RustDialect::Rust,
    );
    if merge_result.ok {
        assert_eq!(
            merge_result.output,
            merge_fixture["expected"]["output"].as_str().map(str::to_string)
        );
    } else {
        assert_structured_import_failure(&merge_result.diagnostics);
    }

    let invalid_template = read_fixture(&["rust", "slice-108-merge", "invalid-template.json"]);
    let invalid_template_result = merge_rust(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        RustDialect::Rust,
    );
    assert!(!invalid_template_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_template_result.diagnostics),
        invalid_template["expected"]["diagnostics"]
    );

    let invalid_destination =
        read_fixture(&["rust", "slice-108-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_rust(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        RustDialect::Rust,
    );
    assert!(!invalid_destination_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_destination_result.diagnostics),
        invalid_destination["expected"]["diagnostics"]
    );
}

#[test]
fn classic_merge_backends_consume_shared_doc_owner_fixture() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_merge2_comment_owner.json"]);
    let input = &fixture["input"];
    for backend in [RustBackend::Native, RustBackend::TreeSitter] {
        let result = merge_rust_with_backend(
            input["template"].as_str().unwrap(),
            input["destination"].as_str().unwrap(),
            RustDialect::Rust,
            backend,
        );
        assert!(result.ok, "{backend:?} classic merge failed: {:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), fixture["expected"]["output"].as_str());
    }
}

#[test]
fn classic_merge_backends_consume_shared_block_doc_owner_fixture() {
    let fixture =
        read_fixture(&["polyglot", "typed-core", "rust_merge2_block_doc_comment_owner.json"]);
    let input = &fixture["input"];
    for backend in [RustBackend::Native, RustBackend::TreeSitter] {
        let result = merge_rust_with_backend(
            input["template"].as_str().unwrap(),
            input["destination"].as_str().unwrap(),
            RustDialect::Rust,
            backend,
        );
        assert!(result.ok, "{backend:?} classic merge failed: {:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), fixture["expected"]["output"].as_str());
    }
}

#[test]
fn rust_backends_conform_to_shared_owner_profile_failure_boundary() {
    let fixture = read_fixture(&[
        "polyglot",
        "typed-core",
        "rust_parser_owner_profile_failure_boundary.json",
    ]);
    for case in fixture["cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        for backend in [RustBackend::Native, RustBackend::TreeSitter] {
            let result = parse_rust_with_backend(source, RustDialect::Rust, backend);
            assert_eq!(
                result.ok,
                case["expected"]["ok"].as_bool().unwrap(),
                "{backend:?}: {}",
                case["name"]
            );
            let diagnostics = diagnostic_shape(&result.diagnostics);
            assert!(
                diagnostics.as_array().unwrap().iter().any(|diagnostic| {
                    diagnostic["category"] == case["expected"]["rust_family_category"]
                }),
                "{backend:?} lacked the expected failure category for {}: {:?}",
                case["name"],
                diagnostics
            );
        }
    }
}

#[test]
fn rust_classic_backends_merge_named_macro_and_function_owners_independently() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_macro_definition_merge3.json"]);
    let input = &fixture["input"];
    for backend in [RustBackend::Native, RustBackend::TreeSitter] {
        let result = merge_rust_three_way_with_backend(
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
            RustDialect::Rust,
            backend,
        );
        assert_eq!(
            result.outcome,
            ast_merge::ThreeWayMergeOutcome::Clean,
            "{backend:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
    }
}

#[test]
fn rust_native_backend_distinguishes_impl_polarity_and_inherent_owners() {
    let fixture =
        read_fixture(&["polyglot", "typed-core", "rust_impl_polarity_inherent_merge3.json"]);
    let input = &fixture["input"];
    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_native_backend_merges_trait_impl_and_function_owners_independently() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_impl_owner_merge3.json"]);
    let input = &fixture["input"];
    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_native_backend_distinguishes_concrete_generic_impl_arguments() {
    let fixture = read_fixture(&[
        "polyglot",
        "typed-core",
        "rust_impl_concrete_generic_identity_merge3.json",
    ]);
    let input = &fixture["input"];
    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_native_backend_merges_trait_aliases_with_independent_function_edits() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_trait_alias_merge3.json"]);
    let input = &fixture["input"];
    let native_parse = parse_rust_with_backend(
        input["base"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert!(native_parse.ok, "explicit syn parser should accept: {:?}", native_parse.diagnostics);
    let tree_sitter_parse = parse_rust_with_backend(
        input["base"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::TreeSitter,
    );
    assert!(!tree_sitter_parse.ok, "fixture pins this parser-specific acceptance boundary");
    assert_eq!(
        tree_sitter_parse.diagnostics.first().map(|diagnostic| diagnostic.category),
        Some(ast_merge::DiagnosticCategory::ParseError)
    );
    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_backends_merge_extern_crate_alias_edits_with_independent_function_edits() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_extern_crate_owner_merge3.json"]);
    let input = &fixture["input"];
    for backend in [RustBackend::Native, RustBackend::TreeSitter] {
        let result = merge_rust_three_way_with_backend(
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
            RustDialect::Rust,
            backend,
        );
        assert_eq!(
            result.outcome,
            ast_merge::ThreeWayMergeOutcome::Clean,
            "{backend:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
    }
}

#[test]
fn rust_native_backend_merges_unique_macro_invocations_as_owners() {
    let fixture = read_fixture(&["polyglot", "typed-core", "rust_macro_invocation_merge3.json"]);
    let input = &fixture["input"];
    let native_parse = parse_rust_with_backend(
        input["base"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert!(
        native_parse.ok,
        "native syn analysis should support macro invocation owners: {:?}",
        native_parse.diagnostics
    );
    let native_analysis = native_parse.analysis.unwrap();
    assert!(
        native_analysis
            .owners
            .iter()
            .any(|owner| owner.path == fixture["expected"]["left_owner_path"])
    );
    assert!(
        native_analysis
            .owners
            .iter()
            .any(|owner| owner.path == fixture["expected"]["right_owner_path"])
    );
    let tree_sitter_parse = parse_rust_with_backend(
        input["base"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::TreeSitter,
    );
    assert!(
        !tree_sitter_parse.ok,
        "Tree-sitter source profile should retain its explicit boundary"
    );
    assert_eq!(
        tree_sitter_parse.diagnostics.first().map(|diagnostic| diagnostic.category),
        Some(ast_merge::DiagnosticCategory::UnsupportedFeature)
    );

    let duplicate_source = fixture["duplicate_identity"]["source"].as_str().unwrap();
    let duplicate_parse =
        parse_rust_with_backend(duplicate_source, RustDialect::Rust, RustBackend::Native);
    assert!(!duplicate_parse.ok);
    assert_eq!(
        duplicate_parse.diagnostics.first().map(|diagnostic| diagnostic.category),
        Some(ast_merge::DiagnosticCategory::Ambiguity)
    );

    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_native_syn_preserves_comments_around_macro_invocation_owners() {
    let fixture =
        read_fixture(&["polyglot", "typed-core", "rust_syn_macro_invocation_comments_merge3.json"]);
    let input = &fixture["input"];
    let result = merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), Some(fixture["expected"]["output"].as_str().unwrap()));
}

#[test]
fn rust_classic_backends_merge_block_doc_two_tip_additions() {
    let fixture =
        read_fixture(&["polyglot", "typed-core", "rust_merge3_block_doc_two_tip_additions.json"]);
    let input = &fixture["input"];
    for backend in [RustBackend::Native, RustBackend::TreeSitter] {
        let result = merge_rust_three_way_with_backend(
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
            RustDialect::Rust,
            backend,
        );
        assert_eq!(
            result.outcome,
            ast_merge::ThreeWayMergeOutcome::Clean,
            "{backend:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            result.output.as_deref(),
            Some(fixture["expected"]["output"].as_str().unwrap()),
            "{backend:?}"
        );
    }
}

#[test]
fn conforms_to_rust_backend_fixtures() {
    let backends_fixture =
        read_fixture(&["diagnostics", "slice-117-rust-family-backends", "rust-backends.json"]);
    assert_eq!(
        serde_json::json!(
            rust_backends()
                .iter()
                .map(|backend| match backend {
                    RustBackend::TreeSitter => "tree-sitter",
                    RustBackend::Native => "native",
                })
                .collect::<Vec<_>>()
        ),
        backends_fixture["backends"]
    );

    let parity_fixture = read_fixture(&["rust", "slice-118-native", "module-parity.json"]);
    let native_result = parse_rust_with_backend(
        parity_fixture["source"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert!(
        native_result.ok,
        "native syn parser should project Rust owners: {:?}",
        native_result.diagnostics
    );
    let native_analysis = native_result.analysis.as_ref().unwrap();
    let native_owners = native_analysis
        .owners
        .iter()
        .map(|owner| {
            serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    rust_merge::RustOwnerKind::Import => "import",
                    rust_merge::RustOwnerKind::Declaration => "declaration",
                },
                "match_key": owner.match_key,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(native_owners), parity_fixture["expected"]["owners"]);
    assert!(
        native_analysis.declarations.iter().any(|item| item.text.contains("pub struct Greeter;"))
    );

    let merge_result = merge_rust_with_backend(
        parity_fixture["template"].as_str().unwrap(),
        parity_fixture["destination"].as_str().unwrap(),
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert!(
        merge_result.ok,
        "native syn merge2 should consume native owner analysis: {:?}",
        merge_result.diagnostics
    );
    assert_eq!(
        merge_result.output.as_deref(),
        Some(parity_fixture["expected"]["output"].as_str().unwrap())
    );

    let backend_profiles = read_fixture(&[
        "diagnostics",
        "slice-122-source-family-backend-feature-profiles",
        "rust-backend-feature-profiles.json",
    ]);
    assert_eq!(
        serde_json::json!({
            "backend": rust_backend_feature_profile(RustBackend::TreeSitter).backend,
            "supports_dialects": rust_backend_feature_profile(RustBackend::TreeSitter).supports_dialects,
            "supported_policies": rust_backend_feature_profile(RustBackend::TreeSitter).supported_policies.iter().map(|policy| serde_json::json!({
                "surface": match policy.surface {
                    ast_merge::PolicySurface::Fallback => "fallback",
                    ast_merge::PolicySurface::Array => "array",
                },
                "name": policy.name
            })).collect::<Vec<_>>(),
            "backend_ref": rust_backend_feature_profile(RustBackend::TreeSitter).backend_ref.as_ref().map(|backend| serde_json::json!({
                "id": backend.id,
                "family": backend.family,
            }))
        }),
        backend_profiles["tree_sitter"]
    );
    assert_eq!(
        serde_json::json!({
            "backend": rust_backend_feature_profile(RustBackend::Native).backend,
            "supports_dialects": rust_backend_feature_profile(RustBackend::Native).supports_dialects,
            "supported_policies": rust_backend_feature_profile(RustBackend::Native).supported_policies.iter().map(|policy| serde_json::json!({
                "surface": match policy.surface {
                    ast_merge::PolicySurface::Fallback => "fallback",
                    ast_merge::PolicySurface::Array => "array",
                },
                "name": policy.name
            })).collect::<Vec<_>>()
        }),
        backend_profiles["native"]
    );

    let plan_contexts = read_fixture(&[
        "diagnostics",
        "slice-123-source-family-plan-contexts",
        "rust-plan-contexts.json",
    ]);
    assert_eq!(
        serde_json::json!({
            "family_profile": {
                "family": rust_plan_context(RustBackend::TreeSitter).family_profile.family,
                "supported_dialects": rust_plan_context(RustBackend::TreeSitter).family_profile.supported_dialects,
                "supported_policies": rust_plan_context(RustBackend::TreeSitter).family_profile.supported_policies.iter().map(|policy| serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })).collect::<Vec<_>>()
            },
            "feature_profile": {
                "backend": rust_plan_context(RustBackend::TreeSitter).feature_profile.as_ref().unwrap().backend,
                "supports_dialects": rust_plan_context(RustBackend::TreeSitter).feature_profile.as_ref().unwrap().supports_dialects,
                "supported_policies": rust_plan_context(RustBackend::TreeSitter).feature_profile.as_ref().unwrap().supported_policies.iter().map(|policy| serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })).collect::<Vec<_>>()
            }
        }),
        plan_contexts["tree_sitter"]
    );
    assert_eq!(
        serde_json::json!({
            "family_profile": {
                "family": rust_plan_context(RustBackend::Native).family_profile.family,
                "supported_dialects": rust_plan_context(RustBackend::Native).family_profile.supported_dialects,
                "supported_policies": rust_plan_context(RustBackend::Native).family_profile.supported_policies.iter().map(|policy| serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })).collect::<Vec<_>>()
            },
            "feature_profile": {
                "backend": rust_plan_context(RustBackend::Native).feature_profile.as_ref().unwrap().backend,
                "supports_dialects": rust_plan_context(RustBackend::Native).feature_profile.as_ref().unwrap().supports_dialects,
                "supported_policies": rust_plan_context(RustBackend::Native).feature_profile.as_ref().unwrap().supported_policies.iter().map(|policy| serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })).collect::<Vec<_>>()
            }
        }),
        plan_contexts["native"]
    );
}

#[test]
fn conforms_to_slice_124_source_family_manifest_fixture() {
    let manifest = read_manifest(&[
        "conformance",
        "slice-124-source-family-manifest",
        "source-family-manifest.json",
    ]);

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "rust"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-105-rust-family-feature-profile".to_string(),
                "rust-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "analysis"),
        Some(
            &[
                "rust".to_string(),
                "slice-106-analysis".to_string(),
                "module-owners.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "matching"),
        Some(
            &[
                "rust".to_string(),
                "slice-107-matching".to_string(),
                "path-equality.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "merge"),
        Some(
            &["rust".to_string(), "slice-108-merge".to_string(), "module-merge.json".to_string(),]
                [..],
        )
    );
}

#[test]
fn conforms_to_slice_131_canonical_manifest_fixture() {
    let manifest =
        read_manifest(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);

    assert_eq!(
        conformance_family_feature_profile_path(&manifest, "rust"),
        Some(
            &[
                "diagnostics".to_string(),
                "slice-105-rust-family-feature-profile".to_string(),
                "rust-feature-profile.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "analysis"),
        Some(
            &[
                "rust".to_string(),
                "slice-106-analysis".to_string(),
                "module-owners.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "matching"),
        Some(
            &[
                "rust".to_string(),
                "slice-107-matching".to_string(),
                "path-equality.json".to_string(),
            ][..],
        )
    );
    assert_eq!(
        conformance_fixture_path(&manifest, "rust", "merge"),
        Some(
            &["rust".to_string(), "slice-108-merge".to_string(), "module-merge.json".to_string(),]
                [..],
        )
    );
}

#[test]
fn native_syn_uses_doc_attributes_and_exact_item_source_spans() {
    let source =
        "//! crate docs\n\n/// Public API docs\n#[inline]\npub fn answer() -> u32 { 42 }\n";
    let parsed = parse_rust_with_backend(source, RustDialect::Rust, RustBackend::Native);
    assert!(parsed.ok, "native parse should accept Rust doc attributes: {:?}", parsed.diagnostics);
    let declaration = &parsed.analysis.as_ref().unwrap().declarations[0];
    assert!(declaration.text.starts_with("/// Public API docs\n#[inline]\npub fn answer"));
    assert!(declaration.text.ends_with("\n"));
}

#[test]
fn native_syn_merge3_uses_shared_source_line_selection() {
    let base =
        "//! crate docs\n\n// left docs\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
    let ours =
        "//! crate docs\n\n// left docs\nfn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
    let theirs =
        "//! crate docs\n\n// left docs\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 2 }\n";
    let result = merge_rust_three_way_with_backend(
        base,
        ours,
        theirs,
        RustDialect::Rust,
        RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean);
    assert_eq!(
        result.output.as_deref(),
        Some("//! crate docs\n\n// left docs\nfn left() -> i32 { 2 }\n\nfn right() -> i32 { 2 }\n")
    );
}

#[test]
fn native_syn_merge3_consumes_shared_two_tip_addition_fixtures() {
    for fixture_name in [
        "rust_merge3_two_tip_additions.json",
        "rust_merge3_two_tip_additions_distinct_source_gaps.json",
        "rust_merge3_two_tip_additions_same_internal_gap.json",
    ] {
        let fixture = read_fixture(&["polyglot", "typed-core", fixture_name]);
        let input = &fixture["input"];
        let result = merge_rust_three_way_with_backend(
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
            RustDialect::Rust,
            RustBackend::Native,
        );
        assert_eq!(
            result.outcome,
            ast_merge::ThreeWayMergeOutcome::Clean,
            "{fixture_name}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            result.output.as_deref(),
            Some(fixture["expected"]["output"].as_str().unwrap()),
            "{fixture_name}"
        );
    }
}
