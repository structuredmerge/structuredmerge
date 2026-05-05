use std::{fs, path::PathBuf};

use ast_merge::{ConformanceManifest, conformance_fixture_path};
use serde_json::Value;
use tree_haver::{
    AdapterInfo, BackendReference, ByteRange, FeatureProfile, KaitaiByteSpan, KaitaiTreeAnalysis,
    KaitaiTreeNode, ParserRequest, ProcessRequest, SourcePoint, byte_offset_for_point,
    current_backend_id, kaitai_adapter_info, kaitai_feature_profile, kaitai_struct_backend,
    pest_adapter_info, pest_backend, pest_feature_profile, process_with_language_pack,
    register_backend, registered_backends, slice_byte_range, with_backend,
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

fn diagnostics_fixture_path(role: &str) -> PathBuf {
    let manifest = read_manifest();
    let path = conformance_fixture_path(&manifest, "diagnostics", role)
        .expect("diagnostics fixture entry should be present");

    path_buf_from_segments(path)
}

fn read_fixture_from_path(path: PathBuf) -> Value {
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_slice_06_parser_request_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("parser_request"));

    let request = ParserRequest {
        source: fixture["request"]["source"]
            .as_str()
            .expect("source should be present")
            .to_string(),
        language: fixture["request"]["language"]
            .as_str()
            .expect("language should be present")
            .to_string(),
        dialect: fixture["request"]["dialect"].as_str().map(str::to_string),
    };

    let adapter_info = AdapterInfo {
        backend: fixture["adapter_info"]["backend"]
            .as_str()
            .expect("backend should be present")
            .to_string(),
        backend_ref: None,
        supports_dialects: fixture["adapter_info"]["supports_dialects"]
            .as_bool()
            .expect("supports_dialects should be boolean"),
        supported_policies: vec![],
    };

    assert_eq!(
        serde_json::json!({
            "source": request.source,
            "language": request.language,
            "dialect": request.dialect,
        }),
        fixture["request"]
    );
    assert_eq!(
        serde_json::json!({
            "backend": adapter_info.backend,
            "supports_dialects": adapter_info.supports_dialects,
        }),
        fixture["adapter_info"]
    );
}

#[test]
fn conforms_to_slice_19_adapter_policy_support_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("adapter_policy_support"));

    let adapter_info = AdapterInfo {
        backend: fixture["adapter_info"]["backend"]
            .as_str()
            .expect("backend should be present")
            .to_string(),
        backend_ref: None,
        supports_dialects: fixture["adapter_info"]["supports_dialects"]
            .as_bool()
            .expect("supports_dialects should be boolean"),
        supported_policies: vec![
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            },
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Fallback,
                name: "trailing_comma_destination_fallback".to_string(),
            },
        ],
    };

    assert_eq!(
        serde_json::json!({
            "backend": adapter_info.backend,
            "supports_dialects": adapter_info.supports_dialects,
            "supported_policies": adapter_info.supported_policies.iter().map(|policy| {
                serde_json::json!({
                    "surface": match policy.surface {
                        ast_merge::PolicySurface::Fallback => "fallback",
                        ast_merge::PolicySurface::Array => "array",
                    },
                    "name": policy.name
                })
            }).collect::<Vec<_>>(),
        }),
        fixture["adapter_info"]
    );
}

#[test]
fn conforms_to_slice_20_adapter_feature_profile_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("adapter_feature_profile"));

    let profile = FeatureProfile {
        backend: fixture["feature_profile"]["backend"]
            .as_str()
            .expect("backend should be present")
            .to_string(),
        backend_ref: None,
        supports_dialects: fixture["feature_profile"]["supports_dialects"]
            .as_bool()
            .expect("supports_dialects should be boolean"),
        supported_policies: vec![
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            },
            ast_merge::PolicyReference {
                surface: ast_merge::PolicySurface::Fallback,
                name: "trailing_comma_destination_fallback".to_string(),
            },
        ],
    };

    assert_eq!(
        serde_json::json!({
            "backend": profile.backend,
            "supports_dialects": profile.supports_dialects,
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
fn conforms_to_slice_25_backend_registry_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("backend_registry"));

    let backends = [
        BackendReference { id: "native".to_string(), family: "builtin".to_string() },
        BackendReference { id: "tree-sitter".to_string(), family: "tree-sitter".to_string() },
    ];
    let profile = FeatureProfile {
        backend: "tree-sitter".to_string(),
        backend_ref: Some(backends[1].clone()),
        supports_dialects: true,
        supported_policies: vec![],
    };

    assert_eq!(
        serde_json::json!(
            backends
                .iter()
                .map(|backend| serde_json::json!({ "id": backend.id, "family": backend.family }))
                .collect::<Vec<_>>()
        ),
        fixture["backends"]
    );
    assert_eq!(
        serde_json::json!({
            "id": profile.backend_ref.as_ref().map(|backend| backend.id.clone()),
            "family": profile.backend_ref.as_ref().map(|backend| backend.family.clone()),
        }),
        serde_json::json!({
            "id": "tree-sitter",
            "family": "tree-sitter",
        })
    );
}

#[test]
fn conforms_to_slice_721_kaitai_tree_haver_substrate_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("kaitai_tree_haver_substrate"));

    let backend = kaitai_struct_backend();
    assert_eq!(backend.id, fixture["backend"]["id"].as_str().unwrap());
    assert_eq!(backend.family, fixture["backend"]["family"].as_str().unwrap());

    let info = kaitai_adapter_info();
    assert_eq!(info.backend, fixture["adapter_info"]["backend"].as_str().unwrap());
    assert_eq!(info.backend_ref.as_ref().unwrap().family, "kaitai");

    let profile = kaitai_feature_profile();
    assert_eq!(profile.backend, fixture["feature_profile"]["backend"].as_str().unwrap());
    assert_eq!(profile.backend_ref.as_ref().unwrap().id, "kaitai-struct");

    let tree_node = &fixture["tree_node"];
    let child = &tree_node["children"][0];
    let mut fields = std::collections::HashMap::new();
    for (key, value) in tree_node["fields"].as_object().unwrap() {
        fields.insert(key.clone(), value.as_str().unwrap().to_string());
    }
    let mut child_fields = std::collections::HashMap::new();
    for (key, value) in child["fields"].as_object().unwrap() {
        child_fields.insert(key.clone(), value.as_str().unwrap().to_string());
    }

    let analysis = KaitaiTreeAnalysis {
        schema: "png.ksy".to_string(),
        root: KaitaiTreeNode {
            kind: tree_node["kind"].as_str().unwrap().to_string(),
            schema_path: tree_node["schema_path"].as_str().unwrap().to_string(),
            span: KaitaiByteSpan {
                start_byte: tree_node["span"]["start_byte"].as_u64().unwrap() as usize,
                end_byte: tree_node["span"]["end_byte"].as_u64().unwrap() as usize,
            },
            fields,
            children: vec![KaitaiTreeNode {
                kind: child["kind"].as_str().unwrap().to_string(),
                schema_path: child["schema_path"].as_str().unwrap().to_string(),
                span: KaitaiByteSpan {
                    start_byte: child["span"]["start_byte"].as_u64().unwrap() as usize,
                    end_byte: child["span"]["end_byte"].as_u64().unwrap() as usize,
                },
                fields: child_fields,
                children: vec![],
            }],
        },
        backend_ref: backend,
    };

    assert_eq!(tree_haver::AnalysisHandle::kind(&analysis), "kaitai-tree");
    assert_eq!(analysis.root.schema_path, "/chunks/1");
    assert_eq!(analysis.root.children[0].fields["value"], "Template");
}

#[test]
fn conforms_to_slice_722_portable_byte_location_contract_fixture() {
    let fixture =
        read_fixture_from_path(diagnostics_fixture_path("portable_byte_location_contract"));

    let byte_range = ByteRange {
        start_byte: fixture["byte_range"]["start_byte"].as_u64().unwrap() as usize,
        end_byte: fixture["byte_range"]["end_byte"].as_u64().unwrap() as usize,
    };
    let point = SourcePoint {
        row: fixture["source_point"]["row"].as_u64().unwrap() as usize,
        column: fixture["source_point"]["column"].as_u64().unwrap() as usize,
    };
    let overlapping_range = ByteRange {
        start_byte: fixture["comparison_ranges"]["overlapping"]["start_byte"].as_u64().unwrap()
            as usize,
        end_byte: fixture["comparison_ranges"]["overlapping"]["end_byte"].as_u64().unwrap()
            as usize,
    };
    let disjoint_range = ByteRange {
        start_byte: fixture["comparison_ranges"]["disjoint"]["start_byte"].as_u64().unwrap()
            as usize,
        end_byte: fixture["comparison_ranges"]["disjoint"]["end_byte"].as_u64().unwrap() as usize,
    };
    let source = fixture["source"].as_str().unwrap();

    assert_eq!(byte_range.len(), fixture["expected"]["length"].as_u64().unwrap() as usize);
    assert_eq!(
        slice_byte_range(source, &byte_range).unwrap(),
        fixture["expected"]["slice"].as_str().unwrap()
    );
    assert_eq!(
        byte_range.contains_byte(byte_range.start_byte),
        fixture["expected"]["contains_start"].as_bool().unwrap()
    );
    assert_eq!(
        byte_range.contains_byte(byte_range.end_byte),
        fixture["expected"]["contains_end"].as_bool().unwrap()
    );
    assert_eq!(
        byte_range.overlaps(&overlapping_range),
        fixture["expected"]["overlaps"].as_bool().unwrap()
    );
    assert_eq!(
        byte_range.overlaps(&disjoint_range),
        fixture["expected"]["disjoint"].as_bool().unwrap()
    );
    assert_eq!(
        byte_offset_for_point(source, &point).unwrap(),
        fixture["expected"]["line_column_offset"].as_u64().unwrap() as usize
    );
}

#[test]
fn conforms_to_slice_100_process_baseline_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("process_baseline"));

    let result = process_with_language_pack(&ProcessRequest {
        source: fixture["request"]["source"]
            .as_str()
            .expect("source should be present")
            .to_string(),
        language: fixture["request"]["language"]
            .as_str()
            .expect("language should be present")
            .to_string(),
    });

    assert!(result.ok);
    let analysis = result.analysis.expect("analysis should be present");
    assert_eq!(analysis.language, fixture["expected"]["language"].as_str().unwrap());
    assert_eq!(
        serde_json::json!(
            analysis
                .structure
                .iter()
                .map(|item| {
                    let mut value = serde_json::json!({ "kind": item.kind });
                    if let Some(name) = &item.name {
                        value["name"] = serde_json::json!(name);
                    }
                    value
                })
                .collect::<Vec<_>>()
        ),
        fixture["expected"]["structure"]
    );
    assert_eq!(
        serde_json::json!(
            analysis
                .imports
                .iter()
                .map(|item| serde_json::json!({ "source": item.source, "items": item.items }))
                .collect::<Vec<_>>()
        ),
        fixture["expected"]["imports"]
    );
}

#[test]
fn exposes_peg_backend_references_for_plurality_slices() {
    assert_eq!(
        serde_json::json!({ "id": pest_backend().id, "family": pest_backend().family }),
        serde_json::json!({ "id": "pest", "family": "peg" })
    );
    assert_eq!(
        serde_json::json!({
            "id": pest_adapter_info().backend_ref.as_ref().map(|backend| backend.id.clone()),
            "family": pest_adapter_info().backend_ref.as_ref().map(|backend| backend.family.clone()),
        }),
        serde_json::json!({ "id": "pest", "family": "peg" })
    );
    assert_eq!(
        serde_json::json!({
            "id": pest_feature_profile().backend_ref.as_ref().map(|backend| backend.id.clone()),
            "family": pest_feature_profile().backend_ref.as_ref().map(|backend| backend.family.clone()),
        }),
        serde_json::json!({ "id": "pest", "family": "peg" })
    );
}

#[test]
fn supports_temporary_backend_context_selection() {
    assert_eq!(current_backend_id(), None);

    with_backend("pest", || {
        assert_eq!(current_backend_id(), Some("pest".to_string()));
        with_backend("kreuzberg-language-pack", || {
            assert_eq!(current_backend_id(), Some("kreuzberg-language-pack".to_string()));
        })
        .expect("nested backend context should be valid");
        assert_eq!(current_backend_id(), Some("pest".to_string()));
    })
    .expect("pest backend should be valid");

    assert_eq!(current_backend_id(), None);
}

#[test]
fn supports_runtime_backend_registration() {
    register_backend(BackendReference {
        id: "custom-toml".to_string(),
        family: "native".to_string(),
    });

    assert_eq!(
        tree_haver::backend_reference("custom-toml"),
        Some(BackendReference { id: "custom-toml".to_string(), family: "native".to_string() })
    );
    assert!(registered_backends().contains(&BackendReference {
        id: "custom-toml".to_string(),
        family: "native".to_string(),
    }));
    with_backend("custom-toml", || {
        assert_eq!(current_backend_id(), Some("custom-toml".to_string()));
    })
    .expect("custom backend should be valid");
    assert_eq!(current_backend_id(), None);
}
