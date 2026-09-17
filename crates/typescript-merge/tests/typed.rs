use ast_merge::ThreeWayMergeOutcome;
use std::sync::{Arc, atomic::AtomicBool};
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*, source::*};
use typescript_merge::{TypeScriptDialect, typed};

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
                LanguagePackProvider::new("typescript.test".into(), language.into()).unwrap(),
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
                        backend_id: Some("typescript.test".into()),
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

const BASE: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 1; }\nfunction g() { return 1; }\n";
const OURS: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 2; }\nfunction g() { return 1; }\n";
const THEIRS: &str =
    "import { x } from 'x';\n// é\nexport function f() { return 1; }\nfunction g() { return 2; }\n";

#[test]
fn analysis_retains_supported_owner_kinds_and_complete_wrapper_spans() {
    let source = "import { x } from 'x';\n// é\nexport class C {}\nenum E { A }\nexport function f() {}\ndeclare function signature(): void;\ninterface I {}\ndeclare namespace N {}\ntype T = string;\nconst v = 1;\n";
    for language in ["typescript", "tsx"] {
        let parser = Parser::new(language);
        let parsed = parser.parse(source, SourceRole::Source);
        let analysis = typed::analysis(&parsed).unwrap_or_else(|error| {
            let nodes = parsed.normalized_nodes().unwrap();
            let index = tree_haver::NormalizedTreeIndex::new(&nodes).unwrap();
            let root = index.root(parsed.document.output().root_id.as_deref().unwrap()).unwrap();
            panic!(
                "{language}: {error}: {:?}",
                index
                    .children(root)
                    .iter()
                    .map(|n| (&n.kind, &n.source_fragment))
                    .collect::<Vec<_>>()
            );
        });
        assert_eq!(
            analysis.document.owners.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            [
                "/class:C",
                "/enum:E",
                "/function:f",
                "/function_signature:signature",
                "/interface:I",
                "/internal_module:N",
                "/type_alias:T",
                "/variables:v"
            ]
        );
        for owner in &analysis.document.owners {
            let ids = &analysis.owner_node_ids[&owner.id];
            assert_eq!(ids.len(), 1);
            let node = parsed.document.output().nodes.iter().find(|n| n.id == ids[0]).unwrap();
            assert_eq!(node.span.range.start_byte, owner.start_byte);
            assert_eq!(node.span.range.end_byte, owner.end_byte);
            assert_eq!(
                parsed.source.slice(node.span.range.clone()).unwrap(),
                owner.fingerprint.as_bytes()
            );
        }
        assert_eq!(analysis.document.owners[0].fingerprint, "export class C {}");
        assert_eq!(analysis.document.owners[3].fingerprint, "declare function signature(): void;");
        // The retained policy uses the native name-field fragment even for a
        // binding pattern. This is opaque identity, not per-binding ownership.
        let pattern = parser.parse("const { a } = value;\n", SourceRole::Source);
        assert_eq!(typed::owners(&pattern).unwrap().owners[0].id, "/variables:{ a }");
        let mut wrong = parsed.clone();
        wrong.source = parser.parse(BASE, SourceRole::Source).source;
        assert!(typed::analysis(&wrong).is_err());
    }
}

#[test]
fn typed_merges_match_legacy_typescript_and_tsx_decisions() {
    for (language, dialect) in
        [("typescript", TypeScriptDialect::TypeScript), ("tsx", TypeScriptDialect::Tsx)]
    {
        let parser = Parser::new(language);
        let added = format!("{BASE}function added() {{}}\n");
        let conflict = OURS.replace("return 2", "return 3");
        let mut cases = vec![
            [BASE, OURS, THEIRS],
            [BASE; 3],
            [BASE, BASE, OURS],
            [BASE, OURS, added.as_str()],
            [BASE, OURS, conflict.as_str()],
            [
                "const a = 1;\ninterface I { x: string }\n",
                "const a = 2;\ninterface I { x: string }\n",
                "const a = 1;\ninterface I { x: number }\n",
            ],
        ];
        if language == "tsx" {
            cases.push([
                "function View() { return <div>one</div>; }\nfunction f() { return 1; }\n",
                "function View() { return <div>two</div>; }\nfunction f() { return 1; }\n",
                "function View() { return <div>one</div>; }\nfunction f() { return 2; }\n",
            ]);
        }
        for texts in cases {
            let legacy =
                typescript_merge::merge_typescript_three_way(texts[0], texts[1], texts[2], dialect);
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
}

#[test]
fn unsupported_wrappers_ambiguous_owners_and_wrong_parsers_fail_closed() {
    let parser = Parser::new("typescript");
    // Preserve the existing single-wrapper/single-owner boundary, rather than
    // interpreting nested exports or overload sets in host code.
    for source in [
        "import { x } from 'x';\n",
        "export const v = 1;\n",
        "const a = 1, b = 2;\n",
        "export default 1;\n",
        "namespace N {}\n",
        "function f() {}\nfunction f() {}\n",
        "function f( {",
        "function View() { return <div/>; }\n",
    ] {
        assert!(typed::analysis(&parser.parse(source, SourceRole::Source)).is_err(), "{source}");
    }
    assert!(
        typed::analysis(&Parser::new("rust").parse("fn f() {}\n", SourceRole::Source)).is_err()
    );
    let [base, ours, theirs] = parser.inputs([BASE, OURS, THEIRS]);
    assert!(typed::merge3(&ours, &base, &theirs, |_| panic!("wrong roles")).is_err());
    let tsx_base = Parser::new("tsx").parse(BASE, SourceRole::Base);
    assert!(typed::merge3(&tsx_base, &ours, &theirs, |_| panic!("mixed dialect parsers")).is_err());
}

#[test]
fn output_identity_bytes_backend_and_service_failure_cannot_claim_success() {
    let parser = Parser::new("typescript");
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
