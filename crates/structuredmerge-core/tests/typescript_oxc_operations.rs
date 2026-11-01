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
            "owners": result.analysis.as_ref().and_then(|a| a.extra.get("owners")).map(|owners| {
                owners.as_array().map(|list| list.iter().map(|o| o["id"].clone()).collect::<Vec<_>>())
            }),
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

#[test]
fn documented_owner_deletion_with_independent_edit_merges_cleanly_on_both_backends() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../fixtures/polyglot/typed-core/typescript_merge3_delete_documented_owner_with_independent_edit.json",
    );
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let expected = fixture["rust_host_expected"]["output"].as_str().unwrap();
    let call = json!({"call": "typescript_common_merge3", "input": {
        "dialect": "typescript",
        "base": fixture["input"]["base"], "ours": fixture["input"]["ours"],
        "theirs": fixture["input"]["theirs"]}});
    for backend in ["{dialect}.common", "typescript.oxc"] {
        let (snapshot, context) = setup();
        let input = request(backend, &call).unwrap().validate(100000, |_, _| panic!()).unwrap();
        let result =
            native_operation::execute_native_operation(&input, &snapshot, &context).unwrap();
        assert!(result.ok, "{backend}: {:?}", result.diagnostics);
        assert_eq!(result.output.as_deref(), Some(expected), "{backend}");
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert!(!result.verification.retained_source_regions.as_ref().unwrap().is_empty());
        result.validate_against(&input).unwrap();
    }
}

#[test]
fn owner_deletion_recovery_selects_whole_lines_and_keeps_conflicts_closed() {
    // (name, base, ours, theirs, expected output; None = must fail closed)
    let cases: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        (
            "undocumented",
            "function a() {}\n\nfunction b() { return 1 }\n",
            "function b() { return 1 }\n",
            "function a() {}\n\nfunction b() { return 2 }\n",
            Some("function b() { return 2 }\n"),
        ),
        (
            "trailing comment leaves with its owner",
            "function a() {} // note\n\nfunction b() { return 1 }\n",
            "function b() { return 1 }\n",
            "function a() {} // note\n\nfunction b() { return 2 }\n",
            Some("function b() { return 2 }\n"),
        ),
        (
            "delete with independent addition",
            "/** a */\nfunction a() {}\n\nfunction b() { return 1 }\n",
            "function b() { return 1 }\n",
            "/** a */\nfunction a() {}\n\nfunction b() { return 1 }\n\nfunction c() {}\n",
            Some("function b() { return 1 }\n\nfunction c() {}\n"),
        ),
        (
            "delete the last owner",
            "function b() { return 1 }\n\n/** a */\nfunction a() {}\n",
            "function b() { return 1 }\n",
            "function b() { return 2 }\n\n/** a */\nfunction a() {}\n",
            Some("function b() { return 2 }\n"),
        ),
        (
            "file header stays",
            "// header\n\n/** a */\nfunction a() {}\n\nfunction b() { return 1 }\n",
            "// header\n\nfunction b() { return 1 }\n",
            "// header\n\n/** a */\nfunction a() {}\n\nfunction b() { return 2 }\n",
            Some("// header\n\nfunction b() { return 2 }\n"),
        ),
        (
            "delete versus edit of the same owner",
            "/** a */\nfunction a() { 1 }\n\nfunction b() {}\n",
            "function b() {}\n",
            "/** a */\nfunction a() { 2 }\n\nfunction b() {}\n",
            None,
        ),
    ];
    for (name, base, ours, theirs, expected) in cases {
        let call = json!({"call": "typescript_common_merge3", "input": {
            "dialect": "typescript", "base": base, "ours": ours, "theirs": theirs}});
        for backend in ["{dialect}.common", "typescript.oxc"] {
            let (snapshot, context) = setup();
            let input = request(backend, &call).unwrap().validate(100000, |_, _| panic!()).unwrap();
            let result =
                native_operation::execute_native_operation(&input, &snapshot, &context).unwrap();
            assert_eq!(
                result.ok,
                expected.is_some(),
                "{name} [{backend}]: {:?}",
                result.diagnostics
            );
            assert_eq!(result.output.as_deref(), expected, "{name} [{backend}]");
        }
    }
}

#[test]
fn oxc_matches_tree_sitter_on_tsx_declaration_files_and_member_decorators() {
    // (name, dialect, source)
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "tsx jsx",
            "tsx",
            "import React from 'react';\nexport function App() { return <div className=\"a\">hi</div>; }\nconst x = <A b=\"c\" />;\n",
        ),
        (
            "tsx generic arrow",
            "tsx",
            "export const f = <T,>(x: T) => x;\nexport function G() { return <></>; }\n",
        ),
        (
            "declaration file",
            "typescript",
            "declare module 'x' {\n  export function f(): void;\n}\ndeclare function g(a: number): string;\ndeclare const k: number;\nexport declare class C { m(): void }\ndeclare namespace N { const y: number }\n",
        ),
        (
            "member decorators",
            "typescript",
            "@Component({ selector: 'x' })\nexport class A {\n  @Input() name: string;\n  @HostListener('click') onClick() {}\n}\n\n@dec class B {}\n",
        ),
        (
            "overloads and abstract",
            "typescript",
            "function f(a: string): void;\nfunction f(a: number): void;\nfunction f(a: any) {}\nabstract class Q<T> { abstract m(): T }\nexport default class D {}\n",
        ),
        (
            "type imports and exports",
            "typescript",
            "import type { A } from 'a';\nexport { x } from 'y';\nexport * from 'z';\nexport type { B } from 'b';\nexport interface I { a: number }\nconst enum E { A = 1 }\ndeclare global { interface Window { x: number } }\n",
        ),
        ("non-ascii", "typescript", "// é\nexport function café() { return 'ü'; }\nconst ñ = 1;\n"),
        ("crlf", "typescript", "export function a() {}\r\n\r\nexport function b() {}\r\n"),
        ("var and multi-declarator", "typescript", "var a = 1;\nlet b = 2, c = 3;\nconst d = 4;\n"),
        ("empty and comment only", "typescript", "// only a comment\n"),
    ];
    let mut differences = Vec::new();
    let mut analyzed_ok = Vec::new();
    for (name, dialect, source) in cases {
        for (operation, input) in [
            ("analyze", json!({"dialect": dialect, "source": source})),
            (
                "diff",
                json!({"dialect": dialect, "before": source, "after": format!("{source}// changed\n")}),
            ),
            (
                "merge3",
                json!({"dialect": dialect, "base": source, "ours": source, "theirs": format!("{source}\nconst appended = 1;\n")}),
            ),
        ] {
            let call = json!({"call": format!("typescript_common_{operation}"), "input": input});
            let tree_sitter = summarize(request("{dialect}.common", &call).unwrap());
            let oxc = summarize(request("typescript.oxc", &call).unwrap());
            if operation == "analyze" && oxc["ok"] == true {
                analyzed_ok.push(name);
            }
            if tree_sitter != oxc {
                differences.push(format!(
                    "{name} / {operation}:\n  tree-sitter {tree_sitter}\n  oxc         {oxc}"
                ));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences.join("\n")
    );
    // Both backends accept exactly these; every other case fails closed identically (unsupported
    // declare/overload/export-only/var/multi-declarator/comment-only forms stay unowned).
    assert_eq!(
        analyzed_ok,
        ["tsx jsx", "member decorators", "non-ascii", "crlf"],
        "supported-source set changed"
    );
}
