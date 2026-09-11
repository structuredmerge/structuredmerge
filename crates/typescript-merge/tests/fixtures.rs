use std::{fs, path::PathBuf};

use serde_json::Value;
use typescript_merge::{
    TypeScriptBackend, TypeScriptDialect, match_typescript_owners, merge_typescript,
    merge_typescript_three_way, parse_typescript, typescript_backend_feature_profile,
    typescript_backends, typescript_feature_profile, typescript_plan_context,
};

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
    let base = "function left(): number { return 1; }\nfunction right(): number { return 1; }\n";
    let ours = "function left(): number { return 2; }\nfunction right(): number { return 1; }\n";
    let theirs = "function left(): number { return 1; }\nfunction right(): number { return 2; }\n";

    let result = merge_typescript_three_way(base, ours, theirs, TypeScriptDialect::TypeScript);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean);
    assert_eq!(
        result.output.as_deref(),
        Some("function left(): number { return 2; }\nfunction right(): number { return 2; }\n")
    );
}

#[test]
fn reports_incompatible_function_edits_as_a_local_conflict() {
    let base = "function value(): number { return 1; }\n";
    let ours = "function value(): number { return 2; }\n";
    let theirs = "function value(): number { return 3; }\n";

    let result = merge_typescript_three_way(base, ours, theirs, TypeScriptDialect::TypeScript);

    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Conflict);
    assert_eq!(result.conflicts[0].path, "/function:value");
    assert_eq!(result.conflicts[0].fallback_scope, "owner");
}

#[test]
fn serializes_all_supported_typescript_declaration_kinds() {
    let source = concat!(
        "class User {}\n",
        "enum Kind { One }\n",
        "function run(): void {}\n",
        "interface Config {}\n",
        "declare namespace Nested {}\n",
        "type Alias = string;\n",
    );
    let result = parse_typescript(source, TypeScriptDialect::TypeScript);

    assert!(result.ok, "diagnostics: {:?}", result.diagnostics);
    let declarations = result.analysis.unwrap().declarations;
    let kinds = declarations
        .iter()
        .map(|declaration| declaration.declaration_kind.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        kinds,
        ["class", "enum", "function", "interface", "internal_module", "type_alias"]
            .into_iter()
            .collect()
    );
}

#[test]
fn preserves_destination_bytes_when_two_way_merge_has_no_additions() {
    let template =
        "function left(): number { return 1; }\nfunction right(): number { return 1; }\n";
    let destination =
        "function left(): number { return 2; }\nfunction right(): number { return 1; }\n";

    let result =
        typescript_merge::merge_typescript(template, destination, TypeScriptDialect::TypeScript);

    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some(destination));
}

#[test]
fn rejects_malformed_and_changed_layout_inputs_without_fallback() {
    let valid = "function left(): number { return 1; }\nfunction right(): number { return 1; }\n";
    let malformed = "function left(: number { return 1; }\n";
    let malformed_result =
        merge_typescript_three_way(valid, valid, malformed, TypeScriptDialect::TypeScript);
    assert_eq!(malformed_result.outcome, ast_merge::ThreeWayMergeOutcome::Error);
    assert_eq!(malformed_result.diagnostics[0].category, ast_merge::DiagnosticCategory::ParseError);

    let ours = "function left(): number { return 2; }\nfunction right(): number { return 1; }\n";
    let theirs =
        "function left(): number { return 1; }\n\nfunction right(): number { return 2; }\n";
    let layout_result =
        merge_typescript_three_way(valid, ours, theirs, TypeScriptDialect::TypeScript);
    assert_eq!(layout_result.outcome, ast_merge::ThreeWayMergeOutcome::Error);
    assert_eq!(
        layout_result.diagnostics[0].category,
        ast_merge::DiagnosticCategory::UnsupportedFeature
    );
}

fn read_fixture(parts: &[&str]) -> Value {
    let source = fs::read_to_string(fixture_path(parts)).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
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
fn conforms_to_typescript_fixtures() {
    let profile_fixture = read_fixture(&[
        "diagnostics",
        "slice-101-typescript-family-feature-profile",
        "typescript-feature-profile.json",
    ]);
    let profile = typescript_feature_profile();
    assert_eq!(profile.family, profile_fixture["feature_profile"]["family"].as_str().unwrap());

    let analysis_fixture =
        read_fixture(&["typescript", "slice-102-analysis", "module-owners.json"]);
    let analysis = parse_typescript(
        analysis_fixture["source"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
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
                        typescript_merge::TypeScriptOwnerKind::Import => "import",
                        typescript_merge::TypeScriptOwnerKind::Declaration => "declaration",
                    },
                    "match_key": owner.match_key,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(owners), analysis_fixture["expected"]["owners"]);
    } else {
        assert_structured_import_failure(&analysis.diagnostics);
    }

    let matching_fixture =
        read_fixture(&["typescript", "slice-103-matching", "path-equality.json"]);
    let template = parse_typescript(
        matching_fixture["template"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    let destination = parse_typescript(
        matching_fixture["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    if template.ok && destination.ok {
        let matched = match_typescript_owners(
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

    let merge_fixture = read_fixture(&["typescript", "slice-104-merge", "module-merge.json"]);
    let merge_result = merge_typescript(
        merge_fixture["template"].as_str().unwrap(),
        merge_fixture["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    if merge_result.ok {
        assert_eq!(
            merge_result.output,
            merge_fixture["expected"]["output"].as_str().map(str::to_string)
        );
    } else {
        assert_structured_import_failure(&merge_result.diagnostics);
    }

    let invalid_template =
        read_fixture(&["typescript", "slice-104-merge", "invalid-template.json"]);
    let invalid_template_result = merge_typescript(
        invalid_template["template"].as_str().unwrap(),
        invalid_template["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    assert!(!invalid_template_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_template_result.diagnostics),
        invalid_template["expected"]["diagnostics"]
    );

    let invalid_destination =
        read_fixture(&["typescript", "slice-104-merge", "invalid-destination.json"]);
    let invalid_destination_result = merge_typescript(
        invalid_destination["template"].as_str().unwrap(),
        invalid_destination["destination"].as_str().unwrap(),
        TypeScriptDialect::TypeScript,
    );
    assert!(!invalid_destination_result.ok);
    assert_eq!(
        diagnostic_shape(&invalid_destination_result.diagnostics),
        invalid_destination["expected"]["diagnostics"]
    );

    let backends_fixture = read_fixture(&[
        "diagnostics",
        "slice-115-typescript-family-backends",
        "typescript-backends.json",
    ]);
    assert_eq!(typescript_backends(), vec![TypeScriptBackend::TreeSitter]);
    assert_eq!(
        Value::Array(vec![Value::String("kreuzberg-language-pack".to_string())]),
        backends_fixture["backends"]
    );

    let backend_fixture = read_fixture(&[
        "diagnostics",
        "slice-122-source-family-backend-feature-profiles",
        "typescript-backend-feature-profiles.json",
    ]);
    let backend_profile = typescript_backend_feature_profile(TypeScriptBackend::TreeSitter);
    let backend_ref = backend_profile
        .backend_ref
        .as_ref()
        .map(|reference| {
            serde_json::json!({
                "id": reference.id,
                "family": reference.family,
            })
        })
        .unwrap_or(Value::Null);
    assert_eq!(
        serde_json::json!({
            "backend": backend_profile.backend,
            "supports_dialects": backend_profile.supports_dialects,
            "supported_policies": backend_profile.supported_policies,
            "backend_ref": backend_ref,
        }),
        backend_fixture["tree_sitter"]
    );

    let plan_fixture = read_fixture(&[
        "diagnostics",
        "slice-123-source-family-plan-contexts",
        "typescript-plan-contexts.json",
    ]);
    assert_eq!(
        serde_json::to_value(typescript_plan_context(TypeScriptBackend::TreeSitter))
            .expect("typescript plan context should serialize"),
        plan_fixture["tree_sitter"]
    );

    let source_manifest_fixture = read_fixture(&[
        "conformance",
        "slice-124-source-family-manifest",
        "source-family-manifest.json",
    ]);
    let source_manifest =
        serde_json::from_value::<ast_merge::ConformanceManifest>(source_manifest_fixture)
            .expect("source manifest should deserialize");
    assert_eq!(
        ast_merge::conformance_family_feature_profile_path(&source_manifest, "typescript"),
        Some(vec![
            "diagnostics".to_string(),
            "slice-101-typescript-family-feature-profile".to_string(),
            "typescript-feature-profile.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "typescript", "analysis"),
        Some(vec![
            "typescript".to_string(),
            "slice-102-analysis".to_string(),
            "module-owners.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "typescript", "matching"),
        Some(vec![
            "typescript".to_string(),
            "slice-103-matching".to_string(),
            "path-equality.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&source_manifest, "typescript", "merge"),
        Some(vec![
            "typescript".to_string(),
            "slice-104-merge".to_string(),
            "module-merge.json".to_string(),
        ])
        .as_deref()
    );

    let canonical_manifest_fixture =
        read_fixture(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let canonical_manifest =
        serde_json::from_value::<ast_merge::ConformanceManifest>(canonical_manifest_fixture)
            .expect("canonical manifest should deserialize");
    assert_eq!(
        ast_merge::conformance_family_feature_profile_path(&canonical_manifest, "typescript"),
        Some(vec![
            "diagnostics".to_string(),
            "slice-101-typescript-family-feature-profile".to_string(),
            "typescript-feature-profile.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "typescript", "analysis"),
        Some(vec![
            "typescript".to_string(),
            "slice-102-analysis".to_string(),
            "module-owners.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "typescript", "matching"),
        Some(vec![
            "typescript".to_string(),
            "slice-103-matching".to_string(),
            "path-equality.json".to_string(),
        ])
        .as_deref()
    );
    assert_eq!(
        ast_merge::conformance_fixture_path(&canonical_manifest, "typescript", "merge"),
        Some(vec![
            "typescript".to_string(),
            "slice-104-merge".to_string(),
            "module-merge.json".to_string(),
        ])
        .as_deref()
    );
}
