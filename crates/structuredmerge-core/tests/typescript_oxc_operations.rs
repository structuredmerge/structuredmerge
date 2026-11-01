//! Runs the shared TypeScript/TSX typed-core fixtures through the oxc parser provider and
//! compares each operation with the Tree-sitter (language-pack) provider.
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(backend: &str, fixture: &Value) -> Option<OperationRequest> {
    let input = &fixture["input"];
    let dialect = input["dialect"].as_str().unwrap_or("typescript");
    let (operation, roles, keys): (&str, Vec<&str>, Vec<&str>) =
        match fixture["call"].as_str()?.rsplit('_').next()? {
            "analyze" => ("analyze", vec!["source"], vec!["source"]),
            "diff" => ("diff2", vec!["before", "after"], vec!["before", "after"]),
            "merge2" => ("merge2", vec!["incoming", "current"], vec!["incoming", "current"]),
            "merge3" => ("merge3", vec!["base", "ours", "theirs"], vec!["base", "ours", "theirs"]),
            _ => return None,
        };
    let sources: serde_json::Map<_, _> = roles
        .iter()
        .zip(keys)
        .map(|(role, key)| {
            let text = input[key].as_str().unwrap_or_default();
            let source = source_input(
                (*role).into(),
                serde_json::from_value(json!(role)).unwrap(),
                SourceEncoding::Utf8,
                text.as_bytes().to_vec(),
            )
            .unwrap();
            (
                (*role).into(),
                json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8",
                "byte_length":source.descriptor.byte_length,"sha256":source.descriptor.sha256}),
            )
        })
        .collect();
    serde_json::from_value(json!({"schema":OPERATION_SCHEMA,"request_id":"typescript-oxc",
        "operation":operation,"sources":sources,"extensions":[],"metadata":{},
        "provider_selection":{"provider_id":"kernel.typescript","family":"typescript","dialect":dialect,"profile_id":"kernel.typescript.owners.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":backend.replace("{dialect}", dialect),"preference":[],"required_capabilities":[]},
        "policy": match operation { "analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}),
            "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) }
    })).ok()
}

fn setup() -> (ParserRegistrySnapshot, ExecutionContext) {
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(
            LanguagePackProvider::new("typescript.common".into(), "typescript".into()).unwrap(),
        ))
        .unwrap();
    registry
        .register(Arc::new(LanguagePackProvider::new("tsx.common".into(), "tsx".into()).unwrap()))
        .unwrap();
    registry
        .register(Arc::new(typescript_merge::oxc_provider::TypeScriptOxcParserProvider::new(
            "typescript.oxc",
        )))
        .unwrap();
    (
        registry.snapshot().unwrap(),
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 3,
            max_input_bytes: 100000,
            max_nodes: 10000,
            max_diagnostics: 100,
        },
    )
}

fn summarize(request: OperationRequest) -> Value {
    let (snapshot, context) = setup();
    let input = match request.validate(100000, |_, _| panic!()) {
        Ok(input) => input,
        Err(error) => return json!({"invalid_request": format!("{error:?}")}),
    };
    match native_operation::execute_native_operation(&input, &snapshot, &context) {
        Ok(result) => json!({
            "ok": result.ok,
            "output": result.output,
            "diagnostics": result.diagnostics.iter().map(|d| format!("{d:?}").chars().take(0).collect::<String>()).count(),
            "changes": result.changes.iter().map(|c| (c.path.clone(), c.subject_ref.clone())).collect::<Vec<_>>(),
            "conflicts": result.conflicts.len(),
        }),
        Err(error) => json!({"error": format!("{error:?}")}),
    }
}

#[test]
fn oxc_matches_tree_sitter_on_shared_typescript_fixtures() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/polyglot/typed-core");
    let mut compared = 0;
    let mut ok_results = 0;
    let mut failed_closed = 0;
    let mut differences = Vec::new();
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                (name.starts_with("typescript_") || name.starts_with("tsx_"))
                    && name.ends_with(".json")
            })
        })
        .collect();
    names.sort();
    for path in names {
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let (Some(common), Some(oxc)) =
            (request("{dialect}.common", &fixture), request("typescript.oxc", &fixture))
        else {
            continue;
        };
        let expected = summarize(common);
        let actual = summarize(oxc);
        compared += 1;
        if actual["ok"] == true {
            ok_results += 1;
        } else if actual["ok"] == false {
            failed_closed += 1;
        }
        if expected != actual {
            differences.push(format!(
                "{}:\n  tree-sitter {expected}\n  oxc         {actual}",
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }
    assert!(compared >= 20, "only compared {compared} fixtures");
    assert!(
        ok_results >= 10 && failed_closed >= 3,
        "vacuous comparison: {ok_results} ok, {failed_closed} closed"
    );
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
