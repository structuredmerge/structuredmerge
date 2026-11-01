use ast_merge::ThreeWayMergeOutcome;
use rust_merge::{RustDialect, typed};
use serde_json::Value;
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
fn rust_source_line_analysis_uses_shared_dispositions_and_following_owner_policy() {
    let parser = Parser::new("rust");
    let parsed = parser
        .parse("//! module docs\n/// function docs\nfn f() {}\n\nfn g() {}\n", SourceRole::Current);
    let analysis = typed::source_line_analysis(&parsed).unwrap();

    let module_docs = analysis.dispositions.iter().find(|line| line.line_number == 1).unwrap();
    assert_eq!(module_docs.kind, "root_region");
    assert_eq!(module_docs.root_region_kind.as_deref(), Some("preamble"));
    assert_eq!(module_docs.owner_id, None);
    let function_docs = analysis.dispositions.iter().find(|line| line.line_number == 2).unwrap();
    assert_eq!(function_docs.kind, "comment_owner");
    assert_eq!(function_docs.comment_kind.as_deref(), Some("leading"));
    assert_eq!(
        function_docs.owner_id.as_deref(),
        Some(analysis.owners.document.owners[0].id.as_str())
    );
    assert_eq!(analysis.dispositions.len(), 5);
    assert!(analysis.dispositions.iter().all(|line| line.kind != "unmanaged"));
}

#[test]
fn merges_a_leading_comment_edit_with_an_independent_owner_edit() {
    let parser = Parser::new("rust");
    let base = "// left documentation\nfn left() -> i32 { 1 }\n\n// right documentation\nfn right() -> i32 { 1 }\n";
    let ours = base.replace("// left documentation", "// left documentation updated");
    let theirs = base.replace("fn right() -> i32 { 1 }", "fn right() -> i32 { 2 }");
    let [base, ours, theirs] = parser.inputs([base, &ours, &theirs]);
    let result =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(
        result.evidence.result.outcome,
        ThreeWayMergeOutcome::Clean,
        "{:#?}",
        result.evidence
    );
    assert!(result.ordered_line_render_applied);
    assert_eq!(
        result.evidence.result.output.as_deref(),
        Some(
            "// left documentation updated\nfn left() -> i32 { 1 }\n\n// right documentation\nfn right() -> i32 { 2 }\n"
        )
    );
}

#[test]
fn clean_merge_selects_root_module_docs_from_the_selected_revision() {
    let parser = Parser::new("rust");
    let base = "//! base module\n/// f docs\nfn f() { 1 }\n";
    let ours = base.replace("base module", "current module");
    let theirs = base.replace("{ 1 }", "{ 2 }");
    let [base, ours, theirs] = parser.inputs([base, &ours, &theirs]);
    let result =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(result.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert!(result.ordered_line_render_applied);
    assert_eq!(
        result.evidence.result.output.as_deref(),
        Some("//! current module\n/// f docs\nfn f() { 2 }\n")
    );
    assert!(!result.evidence.source_segments.is_empty());
    assert_eq!(
        result.output_parse.as_ref().unwrap().source.bytes(),
        result.evidence.result.output.as_ref().unwrap().as_bytes()
    );
}

#[test]
fn clean_merge_selects_independent_owner_additions_from_both_tips() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let [base, ours, theirs] = Parser::new("rust").inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let parser = Parser::new("rust");
    let result =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(result.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert!(result.ordered_line_render_applied);
    assert_eq!(result.evidence.result.output.as_deref(), fixture["expected"]["output"].as_str());
}

#[test]
fn two_tip_additions_in_the_same_internal_gap_follow_native_owner_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions_same_internal_gap.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(
        execution.evidence.result.outcome,
        ThreeWayMergeOutcome::Clean,
        "{:#?}",
        execution.evidence
    );
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn two_tip_additions_in_preamble_and_postlude_gaps_keep_source_positions() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions_file_edges.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(execution.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn same_gap_preamble_additions_follow_ours_then_theirs_with_blank_lines() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions_same_preamble_gap.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(execution.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn keeps_syn_crate_doc_region_when_composing_a_preamble_addition() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_addition_with_crate_docs.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(execution.evidence.result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn composes_two_tip_additions_from_an_empty_or_comment_only_base() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions_empty_base.json"
    ))
    .unwrap();
    let parser = Parser::new("rust");
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let [base, ours, theirs] = parser.inputs([
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
        ]);
        let execution = typed::merge3(&base, &ours, &theirs, |source| {
            Ok(parser.parse(source, SourceRole::Output))
        })
        .unwrap();

        assert_eq!(
            execution.evidence.result.outcome,
            ThreeWayMergeOutcome::Clean,
            "{}: {:#?}",
            case["name"],
            execution.evidence
        );
        assert_eq!(
            execution.evidence.result.output.as_deref(),
            Some(case["expected"]["output"].as_str().unwrap()),
            "{}",
            case["name"]
        );
        assert!(execution.ordered_line_render_applied, "{}", case["name"]);
    }
}

#[test]
fn directional_planning_revalidates_documents_before_noop_selection() {
    let parser = Parser::new("rust");
    let incoming = parser.parse(BASE, SourceRole::Incoming);
    let current = parser.parse(OURS, SourceRole::Current);
    let mut incoming_owners = rust_merge::directional::owners(&incoming).unwrap();
    let current_owners = rust_merge::directional::owners(&current).unwrap();
    assert!(
        rust_merge::directional::plan_insertions(
            &incoming,
            &incoming_owners,
            &current,
            &current_owners
        )
        .unwrap()
        .is_empty()
    );
    incoming_owners.owners[0].fingerprint.push_str(" forged");
    assert!(
        rust_merge::directional::plan_insertions(
            &incoming,
            &incoming_owners,
            &current,
            &current_owners
        )
        .is_err()
    );
    let other = Parser::new("go").parse("package main\n", SourceRole::Incoming);
    assert!(rust_merge::directional::owners(&other).is_err());
}

#[test]
fn directional_insertion_supports_a_named_owner_into_empty_current() {
    let parser = Parser::new("rust");
    let incoming = parser.parse("struct T;\n", SourceRole::Incoming);
    let current = parser.parse("", SourceRole::Current);
    let incoming_owners = rust_merge::directional::owners(&incoming).unwrap();
    let current_owners = rust_merge::directional::owners(&current).unwrap();
    let insertions = rust_merge::directional::plan_insertions(
        &incoming,
        &incoming_owners,
        &current,
        &current_owners,
    )
    .unwrap();
    assert_eq!(insertions.len(), 1);
    assert_eq!(insertions[0].owner_id, "/struct:T");
}

#[test]
fn directional_insertion_keeps_use_items_as_shared_module_facts() {
    let parser = Parser::new("rust");
    let incoming =
        parser.parse("use std::fmt;\n/// new declaration\nfn f() {}\n", SourceRole::Incoming);
    let current = parser.parse("use std::fmt;\n// footer\n", SourceRole::Current);
    let incoming_owners = rust_merge::directional::owners(&incoming).unwrap();
    let current_owners = rust_merge::directional::owners(&current).unwrap();

    assert_eq!(
        incoming_owners.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>(),
        ["/function:f"]
    );
    let insertions = rust_merge::directional::plan_insertions(
        &incoming,
        &incoming_owners,
        &current,
        &current_owners,
    )
    .unwrap();
    assert_eq!(insertions.len(), 1);
    assert_eq!(insertions[0].owner_id, "/function:f");
}

#[test]
fn native_analysis_retains_all_supported_declaration_kinds_and_exact_references() {
    let parser = Parser::new("rust");
    let source = "use std::fmt;\n// é\nconst C: i32 = 1;\nenum E { A }\nfn f() {}\nmod m {}\nstatic S: i32 = 1;\nstruct T;\ntrait Q {}\ntype A = i32;\nunion U { x: i32 }\n";
    let parsed = parser.parse(source, SourceRole::Source);
    let analysis = typed::analysis(&parsed).unwrap();
    assert_eq!(
        analysis.document.owners.iter().map(|owner| owner.id.as_str()).collect::<Vec<_>>(),
        [
            "/use:std::fmt",
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
        assert!(native.span.range.start_byte >= owner.start_byte);
        assert!(native.span.range.end_byte <= owner.end_byte);
        assert_eq!(
            &parsed.source.bytes()[owner.start_byte..owner.end_byte],
            owner.fingerprint.as_bytes()
        );
    }
    let attached = &analysis.attached_comment_node_ids["/const:C"];
    assert_eq!(attached.len(), 1);
    assert_eq!(parsed.document.node(&attached[0]).unwrap().role, tree_haver::NodeRole::Comment);
    let gaps = analysis.layout_gaps().unwrap();
    assert_eq!(parsed.source.slice(gaps[0].range.clone()).unwrap(), b"");
    assert_eq!(parsed.source.slice(gaps[1].range.clone()).unwrap(), "\n".as_bytes());
    let mut wrong = parsed.clone();
    wrong.source = parser.parse(BASE, SourceRole::Source).source;
    assert!(typed::analysis(&wrong).is_err());
}

#[test]
fn native_owner_prefix_projection_matches_shared_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/native_owner_prefix_ownership.json"
    ))
    .unwrap();
    let parser = Parser::new("rust");
    for case in fixture["cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let parsed = parser.parse(source, SourceRole::Source);
        let analysis = typed::analysis(&parsed).unwrap();
        let owner_path = case["owner_path"].as_str().unwrap();
        let owner = analysis.document.owners.iter().find(|owner| owner.id == owner_path).unwrap();
        assert_eq!(
            [owner.start_line, owner.end_line],
            case["owner_line_range"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| line.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
                .as_slice(),
            "{}",
            case["name"].as_str().unwrap()
        );
        assert_eq!(
            &source[owner.start_byte..owner.end_byte],
            case["owner_source"].as_str().unwrap(),
            "{}",
            case["name"].as_str().unwrap()
        );
        let owner_node_kinds = analysis.owner_node_ids[owner_path]
            .iter()
            .map(|id| parsed.document.node(id).unwrap().kind.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            owner_node_kinds,
            case["owner_node_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|kind| kind.as_str().unwrap())
                .collect::<Vec<_>>()
        );
        let comment_texts = analysis
            .attached_comment_node_ids
            .get(owner_path)
            .into_iter()
            .flatten()
            .map(|id| {
                let node = parsed.document.node(id).unwrap();
                std::str::from_utf8(parsed.source.slice(node.span.range.clone()).unwrap())
                    .unwrap()
                    .trim_end_matches('\n')
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            comment_texts,
            case["attached_comment_texts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|text| text.as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        );
        analysis.validate(&parsed).unwrap();
    }
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
fn use_declarations_merge_with_independent_function_edits() {
    let parser = Parser::new("rust");
    let base = "fn f() { 1 }\n";
    let ours = "use std::fmt;\nfn f() { 1 }\n";
    let theirs = "fn f() { 2 }\n";
    let [base, ours, theirs] = parser.inputs([base, ours, theirs]);
    let result =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(
        result.evidence.result.outcome,
        ast_merge::ThreeWayMergeOutcome::Clean,
        "{:?}",
        result.evidence.result.diagnostics
    );
    assert_eq!(result.evidence.result.output.as_deref(), Some("use std::fmt;\nfn f() { 2 }\n"));
}

#[test]
fn unsupported_native_constructs_and_wrong_roles_fail_closed() {
    let parser = Parser::new("rust");
    for source in
        ["impl T {}\nfn f() {}\n", "m!();\nfn f() {}\n", "fn f() {}\nfn f() {}\n", "fn f( {\n"]
    {
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

#[test]
fn two_tip_owner_removals_use_shared_line_selection() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_owner_removals.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(
        execution.evidence.result.outcome,
        ThreeWayMergeOutcome::Clean,
        "{:#?}",
        execution.evidence
    );
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn two_tip_additions_at_distinct_source_gaps_follow_native_owner_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_two_tip_additions_distinct_source_gaps.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let parser = Parser::new("rust");
    let [base, ours, theirs] = parser.inputs([
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
    ]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |source| Ok(parser.parse(source, SourceRole::Output)))
            .unwrap();

    assert_eq!(
        execution.evidence.result.outcome,
        ThreeWayMergeOutcome::Clean,
        "{:#?}",
        execution.evidence
    );
    assert_eq!(
        execution.evidence.result.output.as_deref(),
        Some(fixture["expected"]["output"].as_str().unwrap())
    );
    assert!(execution.ordered_line_render_applied);
}

#[test]
fn deleted_rust_owner_takes_its_inline_trailing_comment_with_it() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_delete_owner_trailing_comment_with_independent_edit.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let result = rust_merge::merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        rust_merge::RustBackend::Native,
    );

    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), fixture["expected"]["output"].as_str());
}

#[test]
fn deleted_rust_owner_takes_its_trailing_block_comment_with_it() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/typed-core/rust_merge3_delete_owner_trailing_block_comment_with_independent_edit.json"
    ))
    .unwrap();
    let input = &fixture["input"];
    let result = rust_merge::merge_rust_three_way_with_backend(
        input["base"].as_str().unwrap(),
        input["ours"].as_str().unwrap(),
        input["theirs"].as_str().unwrap(),
        RustDialect::Rust,
        rust_merge::RustBackend::Native,
    );

    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(result.output.as_deref(), fixture["expected"]["output"].as_str());
}

#[test]
fn edge_addition_composes_with_one_sided_reorder() {
    let result = rust_merge::merge_rust_three_way_with_backend(
        "fn a() {}\nfn b() {}\n",
        "/// ours docs\nfn ours() {}\nfn a() {}\nfn b() {}\n",
        "fn b() {}\nfn a() {}\n",
        rust_merge::RustDialect::Rust,
        rust_merge::RustBackend::Native,
    );
    assert_eq!(result.outcome, ast_merge::ThreeWayMergeOutcome::Clean, "{:?}", result.diagnostics);
    assert_eq!(
        result.output.as_deref(),
        Some("fn b() {}\n/// ours docs\nfn ours() {}\nfn a() {}\n")
    );
}
