use ast_merge::ThreeWayMergeOutcome;
use rust_merge::{RustDialect, typed};
use std::sync::{Arc, atomic::AtomicBool};
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*, source::*};

struct Parser {
    snapshot: ParserRegistrySnapshot,
    context: ExecutionContext,
    language: &'static str,
}

impl Parser {
    fn new(language: &'static str) -> Self {
        let registry = ParserRegistry::default();
        registry
            .register(Arc::new(
                LanguagePackProvider::new("rust.test".into(), language.into()).unwrap(),
            ))
            .unwrap();
        Self {
            snapshot: registry.snapshot().unwrap(),
            language,
            context: ExecutionContext {
                cancelled: Arc::new(AtomicBool::new(false)),
                deadline: None,
                max_batch_items: 3,
                max_input_bytes: 100000,
                max_nodes: 10000,
                max_diagnostics: 100,
            },
        }
    }

    fn parse(&self, text: &str, role: SourceRole) -> ParsedResult {
        TreeHaverParseService::default()
            .parse_batch(
                vec![ParseRequest {
                    schema: PARSE_REQUEST_SCHEMA.into(),
                    request_id: format!("{role:?}"),
                    source: source_input(
                        format!("{role:?}"),
                        role,
                        SourceEncoding::Utf8,
                        text.as_bytes().to_vec(),
                    )
                    .unwrap(),
                    language: self.language.into(),
                    dialect: None,
                    selection: ParserSelection {
                        backend_id: Some("rust.test".into()),
                        preference: vec![],
                        required_capabilities: vec![],
                    },
                    options: ParseOptions::default(),
                    metadata: Default::default(),
                    extra: Default::default(),
                }],
                &self.snapshot,
                &self.context,
            )
            .unwrap()
            .remove(0)
    }

    fn inputs(&self, texts: [&str; 3]) -> [ParsedResult; 3] {
        [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs].map(|role| {
            let index = match role {
                SourceRole::Base => 0,
                SourceRole::Ours => 1,
                _ => 2,
            };
            self.parse(texts[index], role)
        })
    }
}

const BASE: &str = "use std::fmt;\n\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
const OURS: &str = "use std::fmt;\n\nfn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
const THEIRS: &str = "use std::fmt;\n\nfn left() -> i32 { 1 }\n\nfn right() -> i32 { 2 }\n";

#[test]
fn native_analysis_retains_all_supported_declaration_kinds_and_exact_references() {
    let parser = Parser::new("rust");
    let source = "use std::fmt;\n// é\nconst C: i32 = 1;\nenum E { A }\nfn f() {}\nmod m {}\nstatic S: i32 = 1;\nstruct T;\ntrait Q {}\ntype A = i32;\nunion U { x: i32 }\n";
    let parsed = parser.parse(source, SourceRole::Source);
    let analysis = typed::analysis(&parsed).unwrap();
    assert_eq!(
        analysis.document.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>(),
        [
            "/const:C",
            "/enum:E",
            "/function:f",
            "/mod:m",
            "/static:S",
            "/struct:T",
            "/trait:Q",
            "/type:A",
            "/union:U"
        ]
    );
    analysis.validate(&parsed).unwrap();
    for owner in &analysis.document.owners {
        let ids = &analysis.owner_node_ids[&owner.id];
        assert_eq!(ids.len(), 1);
        let native = parsed.document.node(&ids[0]).unwrap();
        assert_eq!(native.span.range.start_byte, owner.start_byte);
        assert_eq!(native.span.range.end_byte, owner.end_byte);
        assert_eq!(
            parsed.source.slice(native.span.range.clone()).unwrap(),
            owner.fingerprint.as_bytes()
        );
    }
    let gaps = analysis.layout_gaps().unwrap();
    assert_eq!(
        parsed.source.slice(gaps[0].range.clone()).unwrap(),
        "use std::fmt;\n// é\n".as_bytes()
    );
    let mut wrong = parsed.clone();
    wrong.source = parser.parse(BASE, SourceRole::Source).source;
    assert!(typed::analysis(&wrong).is_err());
}

#[test]
fn typed_rust_preserves_legacy_decisions_and_freshly_verifies_clean_output() {
    let parser = Parser::new("rust");
    for texts in [
        [BASE, OURS, THEIRS],
        [BASE; 3],
        [BASE, BASE, OURS],
        ["// é\nfn f() {}"; 3],
        [
            "const A:i32=1;\nstruct B {x:i32}\n",
            "const A:i32=2;\nstruct B {x:i32}\n",
            "const A:i32=1;\nstruct B {x:i64}\n",
        ],
        [
            "fn f() { println!(\"one\"); }\n",
            "fn f() { println!(\"two\"); }\n",
            "fn f() { println!(\"three\"); }\n",
        ],
    ] {
        let legacy =
            rust_merge::merge_rust_three_way(texts[0], texts[1], texts[2], RustDialect::Rust);
        let [base, ours, theirs] = parser.inputs(texts);
        let mut calls = 0;
        let result = typed::merge3(&base, &ours, &theirs, |text| {
            calls += 1;
            Ok(parser.parse(text, SourceRole::Output))
        })
        .unwrap();
        assert_eq!(result.evidence.result, legacy);
        if legacy.outcome == ThreeWayMergeOutcome::Clean {
            assert_eq!(calls, 1);
            assert_eq!(
                result.output_parse.unwrap().source.bytes(),
                legacy.output.unwrap().as_bytes()
            );
            assert!(!result.evidence.source_segments.is_empty());
        } else {
            assert_eq!(calls, 0);
            assert!(result.output_parse.is_none());
        }
    }
}

#[test]
fn membership_plus_edit_keeps_the_rust_guard_before_generic_shortcuts() {
    let parser = Parser::new("rust");
    for texts in [
        [BASE.to_string(), OURS.to_string(), format!("{BASE}\nfn added() {{}}\n")],
        [BASE.to_string(), OURS.to_string(), "use std::fmt;\nfn left() -> i32 { 1 }\n".into()],
        [BASE.to_string(), format!("{OURS}\nfn added() {{}}\n"), BASE.to_string()],
    ] {
        let legacy =
            rust_merge::merge_rust_three_way(&texts[0], &texts[1], &texts[2], RustDialect::Rust);
        let [base, ours, theirs] = parser.inputs([&texts[0], &texts[1], &texts[2]]);
        let result =
            typed::merge3(&base, &ours, &theirs, |_| panic!("guard must not render")).unwrap();
        assert_eq!(result.evidence.result, legacy);
        assert_eq!(result.evidence.result.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(result.evidence.result.conflicts[0].conflict_id, "rust-unmanaged-source");
        assert!(result.evidence.classification.is_none());
        assert!(result.evidence.source_segments.is_empty());
        assert!(result.output_parse.is_none());
    }
}

#[test]
fn unsupported_native_constructs_and_wrong_roles_fail_closed() {
    let parser = Parser::new("rust");
    for source in [
        "use std::fmt;\n",
        "impl T {}\nfn f() {}\n",
        "#[inline]\nfn f() {}\n",
        "m!();\nfn f() {}\n",
        "fn f() {}\nfn f() {}\n",
        "fn f( {\n",
    ] {
        assert!(typed::analysis(&parser.parse(source, SourceRole::Source)).is_err(), "{source}");
    }
    assert!(
        typed::analysis(&Parser::new("bash").parse("f() { :; }\n", SourceRole::Source)).is_err()
    );
    let [base, ours, theirs] = parser.inputs([BASE, OURS, THEIRS]);
    assert!(typed::merge3(&ours, &base, &theirs, |_| panic!("wrong roles")).is_err());
    let mut different_parser = base.clone();
    different_parser.backend.id = "other".into();
    assert!(typed::merge3(&different_parser, &ours, &theirs, |_| panic!("mixed parsers")).is_err());
}

#[test]
fn output_identity_bytes_backend_and_parse_service_failure_cannot_claim_success() {
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([BASE; 3]);
    assert!(typed::merge3(&base, &ours, &theirs, |_| Ok(ours.clone())).is_err());
    assert!(
        typed::merge3(&base, &ours, &theirs, |_| Ok(parser.parse(OURS, SourceRole::Output)))
            .is_err()
    );
    assert!(
        typed::merge3(&base, &ours, &theirs, |text| {
            let mut output = parser.parse(text, SourceRole::Output);
            output.backend.id = "foreign".into();
            Ok(output)
        })
        .is_err()
    );
    assert!(typed::merge3(&base, &ours, &theirs, |_| Err("service unavailable".into())).is_err());
    let [base, ours, theirs] = parser.inputs([BASE, OURS, THEIRS]);
    let failed =
        typed::merge3(&base, &ours, &theirs, |_| Err("service unavailable".into())).unwrap();
    assert_ne!(failed.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert!(failed.evidence.result.output.is_none());
    assert!(failed.evidence.source_segments.is_empty());
    assert!(failed.output_parse.is_none());
}
