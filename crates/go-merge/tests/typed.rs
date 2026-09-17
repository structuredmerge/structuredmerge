use ast_merge::ThreeWayMergeOutcome;
use go_merge::{GoDialect, typed};
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
                LanguagePackProvider::new("go.test".into(), language.into()).unwrap(),
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
                        backend_id: Some("go.test".into()),
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
    "package main\n\nfunc left() int { return 1 }\n\nfunc right() int { return 1 }\n";
const OURS: &str =
    "package main\n\nfunc left() int { return 2 }\n\nfunc right() int { return 1 }\n";
const THEIRS: &str =
    "package main\n\nfunc left() int { return 1 }\n\nfunc right() int { return 2 }\n";

#[test]
fn directional_planner_checks_native_owners_even_when_there_are_no_additions() {
    let parser = Parser::new("go");
    let incoming = parser.parse(BASE, SourceRole::Incoming);
    let current = parser.parse(OURS, SourceRole::Current);
    let mut incoming_owners = go_merge::directional::owners(&incoming).unwrap();
    let current_owners = go_merge::directional::owners(&current).unwrap();
    assert!(
        go_merge::directional::plan_insertions(
            &incoming,
            &incoming_owners,
            &current,
            &current_owners
        )
        .unwrap()
        .is_empty()
    );
    incoming_owners.owners.clear();
    assert!(
        go_merge::directional::plan_insertions(
            &incoming,
            &incoming_owners,
            &current,
            &current_owners
        )
        .is_err()
    );
}

#[test]
fn analysis_preserves_native_function_references_and_unowned_header_bytes() {
    let parser = Parser::new("go");
    let parsed = parser.parse(
        "package main\nimport \"fmt\"\n// é\nfunc f() { fmt.Println(\"雪\") }\n",
        SourceRole::Source,
    );
    let analysis = typed::analysis(&parsed).unwrap();
    analysis.validate(&parsed).unwrap();
    assert_eq!(analysis.document.owners.len(), 1);
    let owner = &analysis.document.owners[0];
    assert_eq!(owner.path, "/function:f");
    let ids = &analysis.owner_node_ids[&owner.id];
    assert_eq!(ids.len(), 1);
    let node = parsed.document.node(&ids[0]).unwrap();
    assert_eq!(node.native_type, "function_declaration");
    assert_eq!(node.span.range.start_byte, owner.start_byte);
    assert_eq!(node.span.range.end_byte, owner.end_byte);
    assert_eq!(owner.fingerprint.as_bytes(), parsed.source.slice(node.span.range.clone()).unwrap());
    let gaps = analysis.layout_gaps().unwrap();
    assert_eq!(
        parsed.source.slice(gaps[0].range.clone()).unwrap(),
        "package main\nimport \"fmt\"\n// é\n".as_bytes()
    );
    let mut wrong = parsed.clone();
    wrong.source = parser.parse(BASE, SourceRole::Source).source;
    assert!(typed::analysis(&wrong).is_err());
}

#[test]
fn typed_merge_preserves_legacy_decisions_and_freshly_verifies_clean_output() {
    let parser = Parser::new("go");
    for texts in [
        [BASE, OURS, THEIRS],
        [BASE; 3],
        [BASE, BASE, OURS],
        ["package main\n// é\nfunc f() {}"; 3],
        [
            "package main\nfunc f() { println(1) }\n",
            "package main\nfunc f() { println(2) }\n",
            "package main\nfunc f() { println(3) }\n",
        ],
    ] {
        let legacy = go_merge::merge_go_three_way(texts[0], texts[1], texts[2], GoDialect::Go);
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
fn membership_plus_owner_edit_retains_the_family_guard_without_false_proof() {
    let parser = Parser::new("go");
    for texts in [
        [BASE.to_string(), OURS.to_string(), format!("{BASE}\nfunc added() {{}}\n")],
        [BASE.to_string(), OURS.to_string(), "package main\nfunc left() int { return 1 }\n".into()],
        // The guard also applies when one side equals base: do not take the
        // generic whole-source shortcut before checking the family boundary.
        [BASE.to_string(), format!("{OURS}\nfunc added() {{}}\n"), BASE.to_string()],
    ] {
        let legacy = go_merge::merge_go_three_way(&texts[0], &texts[1], &texts[2], GoDialect::Go);
        let [base, ours, theirs] = parser.inputs([&texts[0], &texts[1], &texts[2]]);
        let result =
            typed::merge3(&base, &ours, &theirs, |_| panic!("guard must not render output"))
                .unwrap();
        assert_eq!(result.evidence.result, legacy);
        assert_eq!(result.evidence.result.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(result.evidence.result.conflicts[0].category, "unmanaged_source_change");
        assert!(result.evidence.classification.is_none());
        assert!(result.evidence.source_segments.is_empty());
        assert!(result.output_parse.is_none());
    }
}

#[test]
fn unsupported_syntax_and_wrong_source_roles_fail_closed() {
    let parser = Parser::new("go");
    for text in [
        "package main\n",
        "package main\nvar x = 1\nfunc f() {}\n",
        "package main\nfunc f() {}\nfunc f() {}\n",
        "package main\nfunc f( {\n",
    ] {
        assert!(typed::analysis(&parser.parse(text, SourceRole::Source)).is_err(), "{text}");
    }
    assert!(
        typed::analysis(&Parser::new("bash").parse("f() { :; }\n", SourceRole::Source)).is_err()
    );
    let [base, ours, theirs] = parser.inputs([BASE, OURS, THEIRS]);
    assert!(typed::merge3(&ours, &base, &theirs, |_| panic!("invalid roles")).is_err());
}

#[test]
fn output_verification_rejects_relabeling_wrong_bytes_backend_and_service_failure() {
    let parser = Parser::new("go");
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
