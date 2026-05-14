use std::{fs, path::PathBuf};

use serde_json::Value;
use tree_haver::{
    AdapterInfo, BackendCapability, BackendReference, BinaryDiagnostic, BinaryMergeReport,
    BinaryNestedDispatch, BinaryPayloadRegion, BinaryRawPayload, BinaryRenderPolicy,
    BinaryScalarValue, ByteEditSpan, ByteRange, FeatureProfile, KaitaiByteSpan, KaitaiTreeAnalysis,
    KaitaiTreeNode, NativeParserProvider, NodeRole, NormalizedParseResult, NormalizedTreeNode,
    ParseErrorTolerance, ParserRequest, PolicyReference, PolicySurface, ProcessRequest,
    SourcePoint, SourceSpan, TreeHaverProfile, ZipUnsafeEntry, byte_offset_for_point,
    current_backend_id, extract_source_fragment, kaitai_adapter_info, kaitai_feature_profile,
    kaitai_struct_backend, node_roles, pest_adapter_info, pest_backend, pest_feature_profile,
    process_with_language_pack, register_backend, registered_backends, slice_byte_range,
    with_backend,
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

fn read_manifest() -> Value {
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
    let entries = manifest["families"]["diagnostics"]
        .as_array()
        .expect("diagnostics family should be present");
    let entry = entries
        .iter()
        .find(|entry| entry["role"].as_str() == Some(role))
        .expect("diagnostics fixture entry should be present");
    let segments = entry["path"]
        .as_array()
        .expect("fixture path should be an array")
        .iter()
        .map(|segment| segment.as_str().expect("path segment should be a string").to_string())
        .collect::<Vec<_>>();

    path_buf_from_segments(&segments)
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
            PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            },
            PolicyReference {
                surface: PolicySurface::Fallback,
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
                        PolicySurface::Fallback => "fallback",
                        PolicySurface::Array => "array",
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
            PolicyReference {
                surface: PolicySurface::Array,
                name: "destination_wins_array".to_string(),
            },
            PolicyReference {
                surface: PolicySurface::Fallback,
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
                        PolicySurface::Fallback => "fallback",
                        PolicySurface::Array => "array",
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
        schema: fixture["analysis"]["schema"].as_str().unwrap().to_string(),
        source_byte_length: fixture["analysis"]["source_byte_length"].as_u64().unwrap() as usize,
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
        diagnostics: vec![BinaryDiagnostic {
            severity: fixture["analysis"]["diagnostics"][0]["severity"]
                .as_str()
                .unwrap()
                .to_string(),
            category: fixture["analysis"]["diagnostics"][0]["category"]
                .as_str()
                .unwrap()
                .to_string(),
            message: fixture["analysis"]["diagnostics"][0]["message"].as_str().unwrap().to_string(),
            schema_path: fixture["analysis"]["diagnostics"][0]["schema_path"]
                .as_str()
                .unwrap()
                .to_string(),
            byte_range: Some(ByteRange {
                start_byte: fixture["analysis"]["diagnostics"][0]["byte_range"]["start_byte"]
                    .as_u64()
                    .unwrap() as usize,
                end_byte: fixture["analysis"]["diagnostics"][0]["byte_range"]["end_byte"]
                    .as_u64()
                    .unwrap() as usize,
            }),
        }],
    };

    assert_eq!(tree_haver::AnalysisHandle::kind(&analysis), "kaitai-tree");
    assert_eq!(
        analysis.source_byte_length,
        fixture["analysis"]["source_byte_length"].as_u64().unwrap() as usize
    );
    assert_eq!(
        analysis.diagnostics[0].schema_path,
        fixture["analysis"]["diagnostics"][0]["schema_path"].as_str().unwrap()
    );
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
    let edit_span = ByteEditSpan {
        start_byte: fixture["edit_span"]["start_byte"].as_u64().unwrap() as usize,
        old_end_byte: fixture["edit_span"]["old_end_byte"].as_u64().unwrap() as usize,
        new_end_byte: fixture["edit_span"]["new_end_byte"].as_u64().unwrap() as usize,
        start_point: source_point_from_fixture(&fixture["edit_span"]["start_point"]),
        old_end_point: source_point_from_fixture(&fixture["edit_span"]["old_end_point"]),
        new_end_point: source_point_from_fixture(&fixture["edit_span"]["new_end_point"]),
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
    assert_eq!(
        edit_span.old_range().len(),
        fixture["expected"]["old_edit_length"].as_u64().unwrap() as usize
    );
    assert_eq!(
        edit_span.new_range().len(),
        fixture["expected"]["new_edit_length"].as_u64().unwrap() as usize
    );
    assert_eq!(
        edit_span.byte_delta(),
        fixture["expected"]["edit_delta"].as_i64().unwrap() as isize
    );
    assert_eq!(
        slice_byte_range(source, &edit_span.old_range()).unwrap(),
        fixture["expected"]["old_edit_slice"].as_str().unwrap()
    );
}

#[test]
fn conforms_to_slice_782_normalized_tree_node_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-782-normalized-tree-node",
        "normalized-tree-node.json",
    ]));
    let roles = node_roles();
    let role_fixture = fixture["node_roles"].as_array().unwrap();
    assert_eq!(roles.len(), role_fixture.len());
    for (role, expected) in roles.iter().zip(role_fixture) {
        assert_eq!(serde_json::to_value(role).unwrap(), *expected);
    }

    let node: NormalizedTreeNode =
        serde_json::from_value(fixture["node"].clone()).expect("node should deserialize");
    let child: NormalizedTreeNode =
        serde_json::from_value(fixture["child"].clone()).expect("child should deserialize");

    assert_eq!(node.role, NodeRole::Structural);
    assert_eq!(node.child_ids[1], child.id);
    assert_eq!(child.parent_id.as_deref(), Some(node.id.as_str()));
    assert_eq!(child.field_name.as_deref(), Some("declaration"));
    assert!(child.has_source_text);
}

#[test]
fn conforms_to_slice_786_progressive_node_metadata_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-786-progressive-node-metadata",
        "progressive-node-metadata.json",
    ]));
    let enhanced: NormalizedTreeNode = serde_json::from_value(fixture["enhanced_node"].clone())
        .expect("enhanced node should deserialize");
    let limited: NormalizedTreeNode = serde_json::from_value(fixture["limited_node"].clone())
        .expect("limited node should deserialize");

    assert_eq!(enhanced.backend_kind.as_deref(), Some("FuncDecl"));
    assert_eq!(enhanced.semantic_roles[0], "declaration");
    assert_eq!(enhanced.metadata["go_dst"]["node_path"], "decls[0]");
    assert!(!limited.has_source_text);
    assert_eq!(limited.unsupported_features[1], "source_fragment");
    assert_eq!(limited.metadata["psych"]["location_support"], "line_column_only");
}

#[test]
fn conforms_to_slice_787_native_parser_adapter_contract_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-787-native-parser-adapter-contract",
        "native-parser-adapter-contract.json",
    ]));
    let provider: NativeParserProvider =
        serde_json::from_value(fixture["provider"].clone()).expect("provider should deserialize");
    let result: NormalizedParseResult = serde_json::from_value(fixture["parse_result"].clone())
        .expect("parse result should deserialize");

    assert_eq!(provider.id, "go-dst");
    assert!(provider.retains_native_tree);
    assert_eq!(provider.native_tree_visibility, "provider_internal");
    assert_eq!(result.root_id, result.nodes[0].id);
    assert_eq!(result.nodes[1].semantic_roles[1], "function");
    assert_eq!(result.metadata["go_dst"]["native_tree_visibility"], "provider_internal");
    assert!(result.source_fragments_available);
}

#[test]
fn conforms_to_slice_788_tree_haver_profile_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-788-tree-haver-profile",
        "tree-haver-profile.json",
    ]));
    let profile: TreeHaverProfile =
        serde_json::from_value(fixture["profile"].clone()).expect("profile should deserialize");

    assert_eq!(profile.profile_id, "go-dst-normalized-tree-v1");
    assert_eq!(profile.backend_ref.id, "go-dst");
    assert_eq!(profile.node_roles[0], NodeRole::Structural);
    assert_eq!(profile.normalized_node_fields.last().map(String::as_str), Some("metadata"));
    assert_eq!(profile.unsupported_defaults["field_name"], "null");
    assert_eq!(profile.capability.parser_identity.name, "github.com/dave/dst");
    assert_eq!(profile.fixture_slices[0], "slice-782-normalized-tree-node");
}

#[test]
fn conforms_to_slice_783_backend_capability_report_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-783-backend-capability-report",
        "backend-capability-report.json",
    ]));
    let capability: BackendCapability = serde_json::from_value(fixture["capability"].clone())
        .expect("capability should deserialize");

    assert_eq!(capability.backend_ref.id, "go-dst");
    assert_eq!(capability.backend_ref.family, "native");
    assert_eq!(capability.language, "go");
    assert_eq!(capability.parser_identity.name, "github.com/dave/dst");
    assert_eq!(capability.parse_error_behavior, "diagnostic_and_partial_tree");
    assert_eq!(capability.render_strategies[0], "source_fragment_reuse");
    assert!(capability.normalized_tree_support);
    assert!(capability.native_node_access);
}

#[test]
fn conforms_to_slice_784_source_fragment_extraction_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-784-source-fragment-extraction",
        "source-fragment-extraction.json",
    ]));
    let span: SourceSpan =
        serde_json::from_value(fixture["span"].clone()).expect("span should deserialize");
    let fragment = extract_source_fragment(
        fixture["source"].as_str().unwrap(),
        &span,
        fixture["strategy"].as_str().unwrap(),
    );

    assert_eq!(fragment.text, fixture["fragment"]["text"].as_str().unwrap());
    assert_eq!(fragment.available, fixture["fragment"]["available"].as_bool().unwrap());
    assert_eq!(fragment.strategy, fixture["fragment"]["strategy"].as_str().unwrap());
    assert_eq!(fragment.byte_length, fixture["fragment"]["byte_length"].as_u64().unwrap() as usize);
    assert_eq!(
        fragment.diagnostics.len(),
        fixture["fragment"]["diagnostics"].as_array().unwrap().len()
    );
}

#[test]
fn conforms_to_slice_785_parse_error_tolerance_fixture() {
    let fixture = read_fixture_from_path(fixture_path(&[
        "diagnostics",
        "slice-785-parse-error-tolerance",
        "parse-error-tolerance.json",
    ]));
    let tolerance: ParseErrorTolerance =
        serde_json::from_value(fixture["parse_error_tolerance"].clone())
            .expect("tolerance should deserialize");

    assert_eq!(tolerance.backend_ref.id, "tree-sitter-go");
    assert_eq!(tolerance.behavior, "diagnostic_and_partial_tree");
    assert!(tolerance.tolerates_errors);
    assert_eq!(tolerance.error_nodes[0].span.range.start_byte, 27);
    assert_eq!(tolerance.diagnostics[0], "partial tree contains parser error nodes");
}

fn source_point_from_fixture(value: &Value) -> SourcePoint {
    SourcePoint {
        row: value["row"].as_u64().unwrap() as usize,
        column: value["column"].as_u64().unwrap() as usize,
    }
}

#[test]
fn conforms_to_slice_723_binary_core_contract_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("binary_core_contract"));

    let raw_payload = BinaryRawPayload {
        encoding: fixture["raw_payload"]["encoding"].as_str().unwrap().to_string(),
        value: fixture["raw_payload"]["value"].as_str().unwrap().to_string(),
        byte_length: fixture["raw_payload"]["byte_length"].as_u64().unwrap() as usize,
        regions: fixture["raw_payload"]["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|region| BinaryPayloadRegion {
                kind: region["kind"].as_str().unwrap().to_string(),
                schema_path: region["schema_path"].as_str().unwrap().to_string(),
                byte_range: ByteRange {
                    start_byte: region["byte_range"]["start_byte"].as_u64().unwrap() as usize,
                    end_byte: region["byte_range"]["end_byte"].as_u64().unwrap() as usize,
                },
                expected_hex: region["expected_hex"].as_str().unwrap().to_string(),
            })
            .collect(),
    };
    let checksum_region = &raw_payload.regions[3];
    let checksum_start = checksum_region.byte_range.start_byte * 2;
    let checksum_end = checksum_region.byte_range.end_byte * 2;
    assert_eq!(raw_payload.encoding, "hex");
    assert_eq!(raw_payload.value.len() / 2, raw_payload.byte_length);
    assert_eq!(raw_payload.regions[0].kind, "header");
    assert_eq!(raw_payload.regions[0].byte_range.len(), 8);
    assert_eq!(&raw_payload.value[checksum_start..checksum_end], checksum_region.expected_hex);

    let scalar_values = vec![
        BinaryScalarValue::String(
            fixture["scalar_values"][0]["value"].as_str().unwrap().to_string(),
        ),
        BinaryScalarValue::Integer(fixture["scalar_values"][1]["value"].as_i64().unwrap()),
        BinaryScalarValue::Float(fixture["scalar_values"][2]["value"].as_f64().unwrap()),
        BinaryScalarValue::Boolean(fixture["scalar_values"][3]["value"].as_bool().unwrap()),
        BinaryScalarValue::Enum {
            symbol: fixture["scalar_values"][4]["symbol"].as_str().unwrap().to_string(),
            raw_value: fixture["scalar_values"][4]["raw_value"].as_i64().unwrap(),
        },
        BinaryScalarValue::Bytes {
            encoding: fixture["scalar_values"][5]["encoding"].as_str().unwrap().to_string(),
            value: fixture["scalar_values"][5]["value"].as_str().unwrap().to_string(),
        },
        BinaryScalarValue::Timestamp(
            fixture["scalar_values"][6]["value"].as_str().unwrap().to_string(),
        ),
        BinaryScalarValue::Opaque {
            format: fixture["scalar_values"][7]["format"].as_str().unwrap().to_string(),
            description: fixture["scalar_values"][7]["description"].as_str().unwrap().to_string(),
        },
        BinaryScalarValue::Null,
    ];
    assert_eq!(scalar_values.len(), 9);
    assert_eq!(scalar_values[0].kind(), "string");
    assert_eq!(scalar_values[8].kind(), "null");

    let policies = fixture["render_policies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| BinaryRenderPolicy {
            schema_path: item["schema_path"].as_str().unwrap().to_string(),
            byte_range: Some(ByteRange {
                start_byte: item["byte_range"]["start_byte"].as_u64().unwrap() as usize,
                end_byte: item["byte_range"]["end_byte"].as_u64().unwrap() as usize,
            }),
            operation: item["operation"].as_str().unwrap().to_string(),
            disposition: item["disposition"].as_str().unwrap().to_string(),
            reason: item["reason"].as_str().unwrap().to_string(),
        })
        .collect::<Vec<_>>();
    assert_eq!(policies[0].operation, "preserve");
    assert_eq!(policies[1].disposition, "requires_renderer");
    assert_eq!(policies[2].disposition, "unsafe");

    let report = BinaryMergeReport {
        format: fixture["merge_report"]["format"].as_str().unwrap().to_string(),
        schema: fixture["merge_report"]["schema"].as_str().unwrap().to_string(),
        matched_schema_paths: vec!["/chunks/0".to_string(), "/chunks/1".to_string()],
        preserved_ranges: vec![ByteRange {
            start_byte: fixture["merge_report"]["preserved_ranges"][0]["start_byte"]
                .as_u64()
                .unwrap() as usize,
            end_byte: fixture["merge_report"]["preserved_ranges"][0]["end_byte"].as_u64().unwrap()
                as usize,
        }],
        rewritten_nodes: vec!["/chunks/1".to_string()],
        checksum_updates: vec!["/chunks/1/crc".to_string()],
        nested_dispatches: vec![BinaryNestedDispatch {
            schema_path: fixture["merge_report"]["nested_dispatches"][0]["schema_path"]
                .as_str()
                .unwrap()
                .to_string(),
            family: fixture["merge_report"]["nested_dispatches"][0]["family"]
                .as_str()
                .unwrap()
                .to_string(),
            status: fixture["merge_report"]["nested_dispatches"][0]["status"]
                .as_str()
                .unwrap()
                .to_string(),
        }],
        diagnostics: vec![BinaryDiagnostic {
            severity: fixture["merge_report"]["diagnostics"][0]["severity"]
                .as_str()
                .unwrap()
                .to_string(),
            category: fixture["merge_report"]["diagnostics"][0]["category"]
                .as_str()
                .unwrap()
                .to_string(),
            message: fixture["merge_report"]["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .to_string(),
            schema_path: fixture["merge_report"]["diagnostics"][0]["schema_path"]
                .as_str()
                .unwrap()
                .to_string(),
            byte_range: Some(ByteRange {
                start_byte: fixture["merge_report"]["diagnostics"][0]["byte_range"]["start_byte"]
                    .as_u64()
                    .unwrap() as usize,
                end_byte: fixture["merge_report"]["diagnostics"][0]["byte_range"]["end_byte"]
                    .as_u64()
                    .unwrap() as usize,
            }),
        }],
    };
    assert_eq!(report.format, "png");
    assert_eq!(report.preserved_ranges[0].len(), 25);
    assert_eq!(report.nested_dispatches[0].family, "text");
    assert_eq!(report.diagnostics[0].category, "unsupported_checksum_rewrite");
}

#[test]
fn conforms_to_slice_729_zip_unsafe_entries_fixture() {
    let fixture = read_fixture_from_path(diagnostics_fixture_path("zip_family_contract"));
    let unsafe_entries = fixture["unsafe_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| ZipUnsafeEntry {
            path: entry["path"].as_str().unwrap().to_string(),
            normalized_path: entry["normalized_path"].as_str().unwrap().to_string(),
            category: entry["category"].as_str().unwrap().to_string(),
            reason: entry["reason"].as_str().unwrap().to_string(),
        })
        .collect::<Vec<_>>();

    assert_eq!(unsafe_entries[0].category, "path_traversal");
    assert_eq!(unsafe_entries[1].normalized_path, "config/settings.yml");
    assert_eq!(unsafe_entries[2].category, "encrypted_member");
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
