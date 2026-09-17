use ast_merge::ThreeWayMergeOutcome;
use bash_merge::{BashDialect, typed};
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
                LanguagePackProvider::new("bash.test".into(), language.into()).unwrap(),
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
                        backend_id: Some("bash.test".into()),
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

    fn inputs(&self, sources: [&str; 3]) -> [ParsedResult; 3] {
        [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs].map(|role| {
            let index = match role {
                SourceRole::Base => 0,
                SourceRole::Ours => 1,
                _ => 2,
            };
            self.parse(sources[index], role)
        })
    }
}

#[test]
fn normalized_view_preserves_native_identity_topology_and_exact_bytes() {
    let parser = Parser::new("bash");
    let parsed = parser.parse("# é\nx=1\nf() { echo '雪'; }\n", SourceRole::Source);
    let nodes = parsed.normalized_nodes().unwrap();
    for node in &nodes {
        let original = parsed.document.node(&node.id).unwrap();
        assert_eq!(node.span, original.span);
        assert_eq!(node.kind, original.native_type);
        assert_eq!(
            node.source_fragment.as_bytes(),
            parsed.source.slice(node.span.range.clone()).unwrap()
        );
        assert_eq!(
            node.child_ids,
            original.children.iter().map(|edge| edge.node_id.clone()).collect::<Vec<_>>()
        );
    }
    let owners = typed::owners(&parsed).unwrap();
    assert_eq!(
        owners.owners.iter().map(|owner| owner.path.as_str()).collect::<Vec<_>>(),
        ["/variable:x", "/function:f"]
    );
    let mut mismatched = parsed.clone();
    mismatched.source = parser.parse("other=2\n", SourceRole::Source).source;
    assert!(mismatched.normalized_nodes().is_err());
}

#[test]
fn typed_bash_preserves_existing_family_semantics_and_verifies_noops() {
    let parser = Parser::new("bash");
    for texts in [
        ["x=1\nf() { :; }\n", "x=2\nf() { :; }\n", "x=1\nf() { echo theirs; }\n"],
        ["# é\nx=1\n"; 3],
        ["x=1\n", "x=1\n", "x=2\n"],
        [
            "test_expect_success 'title' 'echo one'\n",
            "test_expect_success 'title' 'echo two'\n",
            "test_expect_success 'title' 'echo three'\n",
        ],
        ["f() { :; }\n", "f() { echo ours; }\n", "f() { echo theirs; }\n"],
    ] {
        let legacy =
            bash_merge::merge_bash_three_way(texts[0], texts[1], texts[2], BashDialect::Bash);
        let [base, ours, theirs] = parser.inputs(texts);
        let mut calls = 0;
        let execution = typed::merge3(&base, &ours, &theirs, |text| {
            calls += 1;
            Ok(parser.parse(text, SourceRole::Output))
        })
        .unwrap();
        assert_eq!(execution.evidence.result, legacy);
        if legacy.outcome == ThreeWayMergeOutcome::Clean {
            assert_eq!(calls, 1);
            let parsed = execution.output_parse.unwrap();
            let output = legacy.output.unwrap();
            assert_eq!(parsed.source.bytes(), output.as_bytes());
            assert!(!execution.evidence.source_segments.is_empty());
            for segment in execution.evidence.source_segments {
                let source = match segment.revision {
                    ast_merge::SourceRevision::Base => &base.source,
                    ast_merge::SourceRevision::Ours => &ours.source,
                    ast_merge::SourceRevision::Theirs => &theirs.source,
                };
                assert_eq!(
                    source.slice(segment.source_range.clone()).unwrap(),
                    &output.as_bytes()
                        [segment.output_range.start_byte..segment.output_range.end_byte]
                );
                assert_eq!(source.range_digest(segment.source_range).unwrap(), segment.sha256);
            }
        } else {
            assert_eq!(calls, 0);
            assert!(execution.output_parse.is_none());
        }
    }
}

#[test]
fn unsupported_or_ambiguous_bash_remains_rejected() {
    let parser = Parser::new("bash");
    for source in [
        "echo arbitrary\n",
        "f() { :; }\nf() { :; }\n",
        "test_expect_success \"dynamic $title\" 'echo one'\n",
        "f() {",
    ] {
        let parsed = parser.parse(source, SourceRole::Source);
        assert!(typed::owners(&parsed).is_err(), "{source}");
    }
    let json = Parser::new("json").parse("{}", SourceRole::Source);
    assert!(typed::owners(&json).is_err());
}

#[test]
fn output_verification_cannot_reuse_inputs_change_bytes_or_switch_backend() {
    let parser = Parser::new("bash");
    let [base, ours, theirs] = parser.inputs(["x=1\n"; 3]);
    for mode in ["input", "bytes", "backend", "identity", "error"] {
        let result = typed::merge3(&base, &ours, &theirs, |text| {
            if mode == "error" {
                return Err("unavailable".into());
            }
            if mode == "input" {
                return Ok(ours.clone());
            }
            let mut output =
                parser.parse(if mode == "bytes" { "x=2\n" } else { text }, SourceRole::Output);
            if mode == "backend" {
                output.backend.id = "other".into();
            }
            if mode == "identity" {
                let mut input = source_input(
                    "Ours".into(),
                    SourceRole::Output,
                    SourceEncoding::Utf8,
                    text.as_bytes().to_vec(),
                )
                .unwrap();
                // Source identity mismatch with the parse must also fail closed.
                input.descriptor.role = SourceRole::Output;
                output.source = SourceDocument::validate(input, 10000).unwrap();
            }
            Ok(output)
        });
        assert!(result.is_err(), "{mode}");
    }
    assert!(typed::merge3(&ours, &base, &theirs, |_| panic!()).is_err());
}

#[test]
fn failed_changed_output_never_exposes_clean_result_or_source_evidence() {
    let parser = Parser::new("bash");
    let [base, ours, theirs] = parser.inputs(["x=1\ny=1\n", "x=2\ny=1\n", "x=1\ny=2\n"]);
    let execution =
        typed::merge3(&base, &ours, &theirs, |_| Err("parser unavailable".into())).unwrap();
    assert_eq!(execution.evidence.result.outcome, ThreeWayMergeOutcome::Error);
    assert!(execution.evidence.result.output.is_none());
    assert!(execution.evidence.source_segments.is_empty());
    assert!(execution.output_parse.is_none());
}
