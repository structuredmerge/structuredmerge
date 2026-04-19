use std::{fs, path::PathBuf};

use ast_merge::{
    ConformanceManifest, conformance_family_feature_profile_path, conformance_fixture_path,
};
use json_merge::{
    JsonDialect, JsonOwner, JsonOwnerKind, JsonRootKind, json_feature_profile, match_json_owners,
    merge_json, parse_json, parse_json_with_language_pack,
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

fn read_manifest() -> ConformanceManifest {
    let path = fixture_path(&["conformance", "slice-24-manifest", "family-feature-profiles.json"]);
    let source = fs::read_to_string(path).expect("manifest should be readable");
    serde_json::from_str(&source).expect("manifest should be valid json")
}

fn path_buf_from_segments(segments: &[String]) -> PathBuf {
    let mut path = fixture_path(&[]);
    for segment in segments {
        path.push(segment);
    }

    path
}

fn family_feature_profile_fixture_path(family: &str) -> PathBuf {
    let manifest = read_manifest();
    let path = conformance_family_feature_profile_path(&manifest, family)
        .expect("family feature profile entry should be present");

    path_buf_from_segments(path)
}

fn read_fixture_from_path(path: PathBuf) -> Value {
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

fn json_fixture_path(role: &str) -> PathBuf {
    let manifest = read_manifest();
    let path = conformance_fixture_path(&manifest, "json", role)
        .expect("json fixture entry should be present");

    path_buf_from_segments(path)
}

#[test]
fn conforms_to_jsonc_comments_accepted_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("parse_comments"));
    let source = fixture["source"].as_str().expect("source should be present");
    let result = parse_json(source, JsonDialect::Jsonc);

    assert_eq!(result.ok, fixture["expected"]["ok"].as_bool().unwrap_or(false));
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.allows_comments).unwrap_or(false),
        fixture["expected"]["allows_comments"].as_bool().unwrap_or(false)
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn conforms_to_slice_07_structure_fixtures() {
    let object_fixture = read_fixture_from_path(json_fixture_path("structure_json"));
    let object_source = object_fixture["source"].as_str().expect("source should be present");
    let object_result = parse_json(object_source, JsonDialect::Json);

    assert!(object_result.ok);
    assert_eq!(
        object_result.analysis.as_ref().map(|analysis| match analysis.root_kind {
            JsonRootKind::Object => "object",
            JsonRootKind::Array => "array",
            JsonRootKind::Scalar => "scalar",
        }),
        object_fixture["expected"]["root_kind"].as_str()
    );
    let object_owners = object_result
        .analysis
        .as_ref()
        .expect("analysis should be present")
        .owners
        .iter()
        .map(|owner| {
            let mut value = serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    JsonOwnerKind::Member => "member",
                    JsonOwnerKind::Element => "element",
                }
            });
            if let Some(match_key) = &owner.match_key {
                value["match_key"] = serde_json::json!(match_key);
            }
            value
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(object_owners), object_fixture["expected"]["owners"]);

    let jsonc_fixture = read_fixture_from_path(json_fixture_path("structure_jsonc"));
    let jsonc_source = jsonc_fixture["source"].as_str().expect("source should be present");
    let jsonc_result = parse_json(jsonc_source, JsonDialect::Jsonc);

    assert!(jsonc_result.ok);
    assert_eq!(
        jsonc_result.analysis.as_ref().map(|analysis| match analysis.root_kind {
            JsonRootKind::Object => "object",
            JsonRootKind::Array => "array",
            JsonRootKind::Scalar => "scalar",
        }),
        jsonc_fixture["expected"]["root_kind"].as_str()
    );
    let jsonc_owners = jsonc_result
        .analysis
        .as_ref()
        .expect("analysis should be present")
        .owners
        .iter()
        .map(|owner| {
            let mut value = serde_json::json!({
                "path": owner.path,
                "owner_kind": match owner.owner_kind {
                    JsonOwnerKind::Member => "member",
                    JsonOwnerKind::Element => "element",
                }
            });
            if let Some(match_key) = &owner.match_key {
                value["match_key"] = serde_json::json!(match_key);
            }
            value
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(jsonc_owners), jsonc_fixture["expected"]["owners"]);
}

#[test]
fn conforms_to_slice_08_path_matching_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("matching"));
    let template = parse_json(
        fixture["template"].as_str().expect("template should be present"),
        JsonDialect::Json,
    );
    let destination = parse_json(
        fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    let result = match_json_owners(
        template.analysis.as_ref().expect("template analysis should be present"),
        destination.analysis.as_ref().expect("destination analysis should be present"),
    );

    let matched = result
        .matched
        .iter()
        .map(|entry| serde_json::json!([entry.template_path, entry.destination_path]))
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(matched), fixture["expected"]["matched"]);
    assert_eq!(
        result.unmatched_template,
        fixture["expected"]["unmatched_template"]
            .as_array()
            .expect("unmatched_template should be an array")
            .iter()
            .map(|value| value.as_str().expect("paths should be strings").to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.unmatched_destination,
        fixture["expected"]["unmatched_destination"]
            .as_array()
            .expect("unmatched_destination should be an array")
            .iter()
            .map(|value| value.as_str().expect("paths should be strings").to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn conforms_to_slice_09_object_merge_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("merge_object"));
    let template = fixture["template"].as_str().expect("template should be present");
    let destination = fixture["destination"].as_str().expect("destination should be present");
    let result = merge_json(template, destination, JsonDialect::Json);

    assert!(result.ok);
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_09_invalid_merge_fixtures() {
    let invalid_template_fixture =
        read_fixture_from_path(json_fixture_path("merge_invalid_template"));
    let invalid_template_result = merge_json(
        invalid_template_fixture["template"].as_str().expect("template should be present"),
        invalid_template_fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert!(!invalid_template_result.ok);
    assert!(invalid_template_result.output.is_none());
    let invalid_template_diagnostics = invalid_template_result
        .diagnostics
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
                    ast_merge::DiagnosticCategory::DestinationParseError => {
                        "destination_parse_error"
                    }
                    ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                    ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                    ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                }
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        Value::Array(invalid_template_diagnostics),
        invalid_template_fixture["expected"]["diagnostics"]
    );

    let invalid_destination_fixture =
        read_fixture_from_path(json_fixture_path("merge_invalid_destination"));
    let invalid_destination_result = merge_json(
        invalid_destination_fixture["template"].as_str().expect("template should be present"),
        invalid_destination_fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert!(!invalid_destination_result.ok);
    assert!(invalid_destination_result.output.is_none());
    let invalid_destination_diagnostics = invalid_destination_result
        .diagnostics
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
                    ast_merge::DiagnosticCategory::DestinationParseError => {
                        "destination_parse_error"
                    }
                    ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                    ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                    ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                }
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        Value::Array(invalid_destination_diagnostics),
        invalid_destination_fixture["expected"]["diagnostics"]
    );
}

#[test]
fn conforms_to_slice_14_fallback_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("fallback"));
    let result = merge_json(
        fixture["template"].as_str().expect("template should be present"),
        fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert_eq!(result.ok, fixture["expected"]["ok"].as_bool().unwrap_or(false));
    let diagnostics = result
        .diagnostics
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
                    ast_merge::DiagnosticCategory::DestinationParseError => {
                        "destination_parse_error"
                    }
                    ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                    ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                    ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                }
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(diagnostics), fixture["expected"]["diagnostics"]);
    assert_eq!(
        result.policies,
        vec![
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Array,
                name: "destination_wins_array".to_string()
            },
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Fallback,
                name: "trailing_comma_destination_fallback".to_string()
            }
        ]
    );
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
}

#[test]
fn conforms_to_slice_15_fallback_boundary_fixtures() {
    let template_fixture = read_fixture_from_path(json_fixture_path("fallback_boundary_template"));
    let template_result = merge_json(
        template_fixture["template"].as_str().expect("template should be present"),
        template_fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert_eq!(template_result.ok, template_fixture["expected"]["ok"].as_bool().unwrap_or(false));
    let template_diagnostics = template_result
        .diagnostics
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
                    ast_merge::DiagnosticCategory::DestinationParseError => {
                        "destination_parse_error"
                    }
                    ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                    ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                    ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                }
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(template_diagnostics), template_fixture["expected"]["diagnostics"]);
    assert!(template_result.output.is_none());

    let comments_fixture = read_fixture_from_path(json_fixture_path("fallback_boundary_comments"));
    let comments_result = merge_json(
        comments_fixture["template"].as_str().expect("template should be present"),
        comments_fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert_eq!(comments_result.ok, comments_fixture["expected"]["ok"].as_bool().unwrap_or(false));
    let comments_diagnostics = comments_result
        .diagnostics
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
                    ast_merge::DiagnosticCategory::DestinationParseError => {
                        "destination_parse_error"
                    }
                    ast_merge::DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
                    ast_merge::DiagnosticCategory::FallbackApplied => "fallback_applied",
                    ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                }
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(comments_diagnostics), comments_fixture["expected"]["diagnostics"]);
    assert!(comments_result.output.is_none());
}

#[test]
fn conforms_to_slice_16_array_policy_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("array_policy"));
    let result = merge_json(
        fixture["template"].as_str().expect("template should be present"),
        fixture["destination"].as_str().expect("destination should be present"),
        JsonDialect::Json,
    );

    assert_eq!(result.ok, fixture["expected"]["ok"].as_bool().unwrap_or(false));
    assert_eq!(
        result.policies,
        vec![ast_merge::PolicyReference {
            surface: ast_merge::PolicySurface::Array,
            name: "destination_wins_array".to_string()
        }]
    );
    assert_eq!(result.output, fixture["expected"]["output"].as_str().map(str::to_string));
    assert!(result.diagnostics.is_empty());
}

#[test]
fn conforms_to_slice_21_family_feature_profile_fixture_via_the_conformance_manifest() {
    let fixture = read_fixture_from_path(family_feature_profile_fixture_path("json"));
    let profile = json_feature_profile();

    assert_eq!(
        serde_json::json!({
            "family": profile.family,
            "supported_dialects": profile.supported_dialects.iter().map(|dialect| match dialect {
                JsonDialect::Json => "json",
                JsonDialect::Jsonc => "jsonc",
            }).collect::<Vec<_>>(),
            "supported_policies": profile.supported_policies.iter().map(|policy| {
                serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })
            }).collect::<Vec<_>>(),
        }),
        fixture["feature_profile"]
    );
}

#[test]
fn conforms_to_slice_26_tree_sitter_adapter_fixture() {
    let fixture = read_fixture_from_path(json_fixture_path("tree_sitter_adapter"));
    let cases = fixture["cases"].as_array().expect("cases should be an array");

    for case in cases {
        let dialect = match case["dialect"].as_str().expect("dialect should be present") {
            "jsonc" => JsonDialect::Jsonc,
            _ => JsonDialect::Json,
        };
        let source = case["source"].as_str().expect("source should be present");
        let result = parse_json_with_language_pack(source, dialect);

        assert_eq!(result.ok, case["expected"]["ok"].as_bool().unwrap_or(false));
        assert_eq!(
            Value::Array(
                result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| {
                        let mut value = serde_json::json!({
                            "severity": match diagnostic.severity {
                                ast_merge::DiagnosticSeverity::Info => "info",
                                ast_merge::DiagnosticSeverity::Warning => "warning",
                                ast_merge::DiagnosticSeverity::Error => "error",
                            },
                            "category": match diagnostic.category {
                                ast_merge::DiagnosticCategory::ParseError => "parse_error",
                                ast_merge::DiagnosticCategory::DestinationParseError => {
                                    "destination_parse_error"
                                }
                                ast_merge::DiagnosticCategory::UnsupportedFeature => {
                                    "unsupported_feature"
                                }
                                ast_merge::DiagnosticCategory::FallbackApplied => {
                                    "fallback_applied"
                                }
                                ast_merge::DiagnosticCategory::Ambiguity => "ambiguity",
                            }
                        });
                        if !diagnostic.message.is_empty() {
                            value["message"] = serde_json::json!(diagnostic.message);
                        }
                        value
                    })
                    .collect::<Vec<_>>()
            ),
            case["expected"]["diagnostics"]
        );

        if result.ok {
            assert_eq!(
                result.analysis.as_ref().map(|analysis| match analysis.root_kind {
                    JsonRootKind::Object => "object",
                    JsonRootKind::Array => "array",
                    JsonRootKind::Scalar => "scalar",
                }),
                case["expected"]["root_kind"].as_str()
            );
            let owners = result
                .analysis
                .as_ref()
                .expect("analysis should be present")
                .owners
                .iter()
                .map(|owner| {
                    let mut value = serde_json::json!({
                        "path": owner.path,
                        "owner_kind": match owner.owner_kind {
                            JsonOwnerKind::Member => "member",
                            JsonOwnerKind::Element => "element",
                        }
                    });
                    if let Some(match_key) = &owner.match_key {
                        value["match_key"] = serde_json::json!(match_key);
                    }
                    value
                })
                .collect::<Vec<_>>();
            assert_eq!(Value::Array(owners), case["expected"]["owners"]);
        } else {
            assert!(result.analysis.is_none());
        }
    }
}

#[test]
fn parses_strict_json_through_language_pack_and_preserves_analysis() {
    let result = parse_json_with_language_pack("{\"alpha\":{\"beta\":1}}", JsonDialect::Json);

    assert!(result.ok);
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.root_kind),
        Some(JsonRootKind::Object)
    );
    assert_eq!(
        result.analysis.as_ref().map(|analysis| analysis.owners.clone()),
        Some(vec![
            JsonOwner {
                path: "/alpha".to_string(),
                owner_kind: JsonOwnerKind::Member,
                match_key: Some("alpha".to_string()),
            },
            JsonOwner {
                path: "/alpha/beta".to_string(),
                owner_kind: JsonOwnerKind::Member,
                match_key: Some("beta".to_string()),
            },
        ])
    );
}

#[test]
fn reports_strict_json_syntax_errors_through_language_pack() {
    let result = parse_json_with_language_pack("{\"alpha\":1,}", JsonDialect::Json);

    assert!(!result.ok);
    assert_eq!(
        result.diagnostics[0].message,
        "tree-sitter-language-pack reported syntax errors for json."
    );
}

#[test]
fn rejects_jsonc_through_language_pack_for_now() {
    let result =
        parse_json_with_language_pack("{\n  // note\n  \"alpha\":1\n}", JsonDialect::Jsonc);

    assert!(!result.ok);
    assert_eq!(
        result.diagnostics[0].message,
        "tree-sitter-language-pack json parsing currently supports only the json dialect."
    );
}
