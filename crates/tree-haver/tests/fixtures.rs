use std::{fs, path::PathBuf};

use serde_json::Value;
use tree_haver::{AdapterInfo, FeatureProfile, ParserRequest};

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

fn read_fixture(parts: &[&str]) -> Value {
    let path = fixture_path(parts);
    let source = fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_slice_06_parser_request_fixture() {
    let fixture = read_fixture(&["diagnostics", "slice-06-parser-adapters", "parser-request.json"]);

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
    let fixture =
        read_fixture(&["diagnostics", "slice-19-adapter-policy-support", "adapter-info.json"]);

    let adapter_info = AdapterInfo {
        backend: fixture["adapter_info"]["backend"]
            .as_str()
            .expect("backend should be present")
            .to_string(),
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
    let fixture =
        read_fixture(&["diagnostics", "slice-20-adapter-feature-profile", "feature-profile.json"]);

    let profile = FeatureProfile {
        backend: fixture["feature_profile"]["backend"]
            .as_str()
            .expect("backend should be present")
            .to_string(),
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
