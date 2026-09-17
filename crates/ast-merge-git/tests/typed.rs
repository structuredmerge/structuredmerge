use ast_merge::{ConflictLabels, ThreeWayMergeOutcome};
use ast_merge_git::typed::{self, ConflictRenderOptions};
use json_merge::JsonDialect;
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
                LanguagePackProvider::new("git.test".into(), language.into()).unwrap(),
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
                        backend_id: Some("git.test".into()),
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
fn clean_git_merge_retains_verified_family_edits_for_each_dialect() {
    for (dialect, language, sources) in [
        (
            JsonDialect::Json,
            "json",
            ["{\"a\":0,\"b\":0}\n", "{\"a\":1,\"b\":0}\n", "{\"a\":0,\"b\":2}\n"],
        ),
        (
            JsonDialect::Jsonc,
            "json5",
            ["{// c\n\"a\":0,\"b\":0}\n", "{// c\n\"a\":1,\"b\":0}\n", "{// c\n\"a\":0,\"b\":2}\n"],
        ),
        (JsonDialect::Json5, "json5", ["{a:0,b:0}\n", "{a:1,b:0}\n", "{a:0,b:2}\n"]),
    ] {
        let parser = Parser::new(language);
        let [base, ours, theirs] = parser.inputs(sources);
        let mut reparses = 0;
        let result = typed::merge3(&base, &ours, &theirs, dialect, &Default::default(), |text| {
            reparses += 1;
            Ok(parser.parse(text, SourceRole::Output))
        })
        .unwrap();
        assert_eq!(result.merge.result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(reparses, 1);
        let output = result.merge.result.output.as_ref().unwrap();
        assert!(output.contains("1") && output.contains("2"));
        result.merge.render.unwrap().validate(&ours.source, output).unwrap();
        assert!(result.conflict_render.is_none());
        assert!(result.conflict_render_error.is_none());
    }
}

#[test]
fn conflict_review_output_has_custom_markers_labels_and_replayable_provenance() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs([
        "{\r\n  \"é\": 0,\r\n  \"x\": 1\r\n}\r\n",
        "{\r\n  \"é\": 0,\r\n  \"x\": 2\r\n}\r\n",
        "{\r\n  \"é\": 0,\r\n  \"x\": 3\r\n}\r\n",
    ]);
    let options = ConflictRenderOptions {
        marker_size: 9,
        labels: ConflictLabels {
            base: "ancestor".into(),
            ours: "local-é".into(),
            theirs: "remote".into(),
        },
    };
    let result = typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &options, |_| {
        panic!("markers must not be reparsed as JSON")
    })
    .unwrap();
    assert_eq!(result.merge.result.outcome, ThreeWayMergeOutcome::Conflict);
    assert!(result.merge.result.output.is_none());
    assert!(result.merge.render.is_none());
    assert!(result.conflict_render_error.is_none());
    let evidence = result.conflict_render.unwrap();
    assert!(evidence.rendered.content.starts_with("{\r\n  \"é\": 0,\r\n"));
    assert!(evidence.rendered.content.ends_with("}\r\n"));
    assert!(evidence.rendered.content.contains("<<<<<<<<< local-é\n"));
    assert!(evidence.rendered.content.contains("||||||||| ancestor\n"));
    assert!(evidence.rendered.content.contains(">>>>>>>>> remote\n"));
    assert_eq!(evidence.rendered.verification_input.conflict_count, 1);
    assert!(!evidence.rendered.synthesized_fragments.is_empty());
    assert!(!evidence.rendered.verification_input.source_fragments.is_empty());
    let inputs = [&base, &ours, &theirs];
    evidence.validate(inputs, &result.merge.result.conflicts, &options).unwrap();
    let mut changed = evidence.clone();
    changed.rendered.content.push(' ');
    assert!(changed.validate(inputs, &result.merge.result.conflicts, &options).is_err());
    let mut changed = evidence.clone();
    changed.sources[0].source_id.push('!');
    assert!(changed.validate(inputs, &result.merge.result.conflicts, &options).is_err());
    let mut changed = evidence.clone();
    changed.rendered.line_records[0].original_line = Some(999);
    assert!(changed.validate(inputs, &result.merge.result.conflicts, &options).is_err());
    assert!(
        evidence
            .validate([&ours, &base, &theirs], &result.merge.result.conflicts, &options)
            .is_err()
    );
    assert!(
        evidence.validate(inputs, &result.merge.result.conflicts, &Default::default()).is_err()
    );
}

#[test]
fn overlapping_conflict_lines_fail_closed_without_whole_file_markers() {
    let parser = Parser::new("json");
    let [base, ours, theirs] =
        parser.inputs(["{\"x\":0,\"y\":0}", "{\"x\":1,\"y\":1}", "{\"x\":2,\"y\":2}"]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            panic!("unexpected output parse")
        })
        .unwrap();
    assert_eq!(result.merge.result.outcome, ThreeWayMergeOutcome::Conflict);
    assert_eq!(result.merge.result.conflicts.len(), 2);
    assert!(result.conflict_render.is_none());
    assert!(result.conflict_render_error.unwrap().contains("overlaps"));
}

#[test]
fn absent_alternatives_render_empty_review_sides_without_inventing_source_regions() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs(["{\"x\":0}", "{}", "{\"x\":2}"]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            panic!("unexpected output parse")
        })
        .unwrap();
    assert_eq!(result.merge.result.outcome, ThreeWayMergeOutcome::Conflict);
    assert!(!result.merge.result.conflicts.is_empty());
    let evidence = result.conflict_render.unwrap();
    assert!(evidence.rendered.content.starts_with("{}\n<<<<<<< ours\n||||||| base\n"));
    assert_eq!(evidence.rendered.conflicts[0].metadata["placement"], "end_of_ours_absent_owner");
    assert_eq!(evidence.rendered.conflicts[0].output_start_line, 2);
    assert_eq!(
        evidence.rendered.conflicts[0].output_end_line,
        evidence.rendered.content.lines().count()
    );
    assert!(
        !evidence
            .rendered
            .line_records
            .iter()
            .any(|line| line.conflict_side == Some(ast_merge::SourceRevision::Ours)
                && line.fragment_kind == ast_merge::RenderFragmentKind::Source)
    );
    evidence
        .validate([&base, &ours, &theirs], &result.merge.result.conflicts, &Default::default())
        .unwrap();
    assert!(result.conflict_render_error.is_none());
    assert!(result.merge.result.output.is_none());
}

#[test]
fn missing_final_newlines_are_recorded_as_synthesized_boundaries() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs(["{\"x\":0}", "{\"x\":1}", "{\"x\":2}"]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            panic!("unexpected output parse")
        })
        .unwrap();
    let evidence = result.conflict_render.unwrap();
    assert_eq!(
        evidence
            .rendered
            .synthesized_fragments
            .iter()
            .filter(|fragment| fragment.reason == "conflict_line_boundary")
            .count(),
        3
    );
    evidence
        .validate([&base, &ours, &theirs], &result.merge.result.conflicts, &Default::default())
        .unwrap();
}

#[test]
fn multiple_deleted_owners_have_distinct_appended_review_blocks() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs(["{\"x\":0,\"y\":0}", "{}", "{\"x\":1,\"y\":2}"]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            panic!("no merged output parse")
        })
        .unwrap();
    assert_eq!(result.merge.result.conflicts.len(), 2);
    assert!(result.merge.result.output.is_none());
    let evidence = result.conflict_render.unwrap();
    assert!(evidence.rendered.content.starts_with("{}\n<<<<<<< ours\n||||||| base\n"));
    assert_eq!(evidence.rendered.conflicts.len(), 2);
    assert_ne!(
        evidence.rendered.conflicts[0].conflict_id,
        evidence.rendered.conflicts[1].conflict_id
    );
    assert!(
        evidence.rendered.conflicts[0].output_end_line
            < evidence.rendered.conflicts[1].output_start_line
    );
    assert_eq!(
        evidence.rendered.conflicts[1].output_end_line,
        evidence.rendered.content.lines().count()
    );
    evidence
        .validate([&base, &ours, &theirs], &result.merge.result.conflicts, &Default::default())
        .unwrap();
}

#[test]
fn invalid_options_are_rejected_even_for_identical_inputs() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs(["{}"; 3]);
    for options in [
        ConflictRenderOptions { marker_size: 0, ..Default::default() },
        ConflictRenderOptions { marker_size: 1025, ..Default::default() },
        ConflictRenderOptions {
            labels: ConflictLabels { ours: "injected\nmarker".into(), ..Default::default() },
            ..Default::default()
        },
    ] {
        assert!(
            typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &options, |_| panic!(
                "invalid options must fail before parsing output"
            ))
            .is_err()
        );
    }
}

#[test]
fn rejected_clean_output_verification_never_becomes_a_git_success() {
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs(["{}"; 3]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            Err("verification unavailable".into())
        })
        .unwrap();
    assert_eq!(result.merge.result.outcome, ThreeWayMergeOutcome::Error);
    assert!(result.merge.result.output.is_none());
    assert!(result.merge.render.is_none());
    assert!(result.conflict_render.is_none());
}

#[test]
fn marker_review_keeps_ours_outside_conflicts_not_unmerged_theirs_edits() {
    // Existing renderer semantics: this is a conflict review artifact, not a
    // partially resolved merge. Do not claim every independent edit was applied.
    let parser = Parser::new("json");
    let [base, ours, theirs] = parser.inputs([
        "{\n\"x\":0,\n\"other\":0\n}\n",
        "{\n\"x\":1,\n\"other\":0\n}\n",
        "{\n\"x\":2,\n\"other\":9\n}\n",
    ]);
    let result =
        typed::merge3(&base, &ours, &theirs, JsonDialect::Json, &Default::default(), |_| {
            panic!("unexpected output parse")
        })
        .unwrap();
    let rendered = result.conflict_render.unwrap().rendered;
    assert!(rendered.content.ends_with("\"other\":0\n}\n"));
    assert!(!rendered.content.contains("\"other\":9"));
}
