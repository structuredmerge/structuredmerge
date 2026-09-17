use ast_merge::ThreeWayMergeOutcome;
use json_merge::{JsonDialect, typed};
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
                LanguagePackProvider::new("json.test".into(), language.into()).unwrap(),
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
                        backend_id: Some("json.test".into()),
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
}

#[test]
fn nested_directional_merge_preserves_current_and_reparses_once() {
    let parser = Parser::new("json");
    let current = parser.parse(
        "{\r\n  \"nested\": {\"keep\": 9},\r\n  \"items\": [1, 2]\r\n}",
        SourceRole::Current,
    );
    let incoming =
        parser.parse(r#"{"nested":{"keep":1,"add":"é"},"items":[3]}"#, SourceRole::Incoming);
    let mut calls = 0;
    let result = typed::merge2(&incoming, &current, JsonDialect::Json, |output| {
        calls += 1;
        Ok(parser.parse(output, SourceRole::Output))
    })
    .unwrap();
    assert!(result.ok, "{:?}", result.diagnostics);
    assert_eq!(calls, 1);
    let output = result.output.unwrap();
    assert!(output.contains("\"keep\": 9"));
    assert!(output.contains("\"items\": [1, 2]"));
    assert!(output.contains("\r\n"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output).unwrap(),
        serde_json::json!({"nested":{"keep":9,"add":"é"},"items":[1,2]})
    );
}

#[test]
fn three_way_merges_nested_independent_edits_and_reports_real_conflicts() {
    let parser = Parser::new("json");
    let base = parser.parse(r#"{"x":{"a":1,"b":2}}"#, SourceRole::Base);
    let ours = parser.parse(r#"{"x":{"a":3,"b":2}}"#, SourceRole::Ours);
    let theirs = parser.parse(r#"{"x":{"a":1,"b":4}}"#, SourceRole::Theirs);
    let result = typed::merge3(&base, &ours, &theirs, JsonDialect::Json, |output| {
        Ok(parser.parse(output, SourceRole::Output))
    })
    .unwrap();
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result.output.unwrap()).unwrap(),
        serde_json::json!({"x":{"a":3,"b":4}})
    );
    let theirs = parser.parse(r#"{"x":{"a":5,"b":2}}"#, SourceRole::Theirs);
    let conflict = typed::merge3(&base, &ours, &theirs, JsonDialect::Json, |_| {
        panic!("conflict must not render")
    })
    .unwrap();
    assert_eq!(conflict.outcome, ThreeWayMergeOutcome::Conflict);
    assert!(!conflict.conflicts.is_empty());
    assert!(conflict.output.is_none());
}

#[test]
fn dialect_analysis_matches_existing_engine_without_reparsing() {
    for (language, dialect, source) in [
        ("json", JsonDialect::Json, r#"{"nested":{"x":1},"a":[1,true]}"#),
        ("json5", JsonDialect::Jsonc, "{\n // note\n \"x\": 1,\n}\n"),
        ("json5", JsonDialect::Json5, "{\n // note\n x: 'é',\n}\n"),
    ] {
        let parser = Parser::new(language);
        let parsed = parser.parse(source, SourceRole::Source);
        assert_eq!(
            typed::analyze(&parsed, dialect).unwrap(),
            json_merge::parse_json(source, dialect).analysis.unwrap()
        );
    }
    let parser = Parser::new("json5");
    assert!(
        typed::analyze(&parser.parse("{x:1}", SourceRole::Source), JsonDialect::Jsonc).is_err()
    );
    let parser = Parser::new("json");
    assert!(
        typed::analyze(&parser.parse("// no\n{}", SourceRole::Source), JsonDialect::Json).is_err()
    );
    assert!(typed::analyze(&parser.parse("{", SourceRole::Source), JsonDialect::Json).is_err());
}

#[test]
fn commented_dialects_keep_existing_nested_merge_semantics_and_bytes() {
    for (dialect, incoming, current) in [
        (
            JsonDialect::Jsonc,
            "{\"x\":{\"add\":2}}",
            "{\r\n // retained\r\n \"x\":{\"keep\":1}\r\n}",
        ),
        (JsonDialect::Json5, "{x:{add:'é'}}", "{\n // retained\n x:{keep:1}\n}"),
    ] {
        let parser = Parser::new("json5");
        let result = typed::merge2(
            &parser.parse(incoming, SourceRole::Incoming),
            &parser.parse(current, SourceRole::Current),
            dialect,
            |output| Ok(parser.parse(output, SourceRole::Output)),
        )
        .unwrap();
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result, json_merge::merge_json(incoming, current, dialect));
        let output = result.output.unwrap();
        assert!(output.contains("// retained"));
        assert!(output.contains("add"));
        assert_eq!(Some(output), json_merge::merge_json(incoming, current, dialect).output);
    }
}

#[test]
fn even_noop_output_must_be_verified_and_cannot_change_bytes_identity_or_backend() {
    let parser = Parser::new("json");
    let base = parser.parse("{}", SourceRole::Base);
    let ours = parser.parse("{}", SourceRole::Ours);
    let theirs = parser.parse("{}", SourceRole::Theirs);
    for scenario in 0..5 {
        let mut calls = 0;
        let result = typed::merge3(&base, &ours, &theirs, JsonDialect::Json, |output| {
            calls += 1;
            if scenario == 0 {
                return Err("verification unavailable".into());
            }
            let mut parsed = parser.parse(
                if scenario == 1 { "[]" } else { output },
                if scenario == 2 { SourceRole::Source } else { SourceRole::Output },
            );
            if scenario == 3 {
                parsed.backend.id = "swapped".into();
            }
            if scenario == 4 {
                return Ok(ours.clone());
            }
            Ok(parsed)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
        assert!(result.output.is_none());
    }
    let incoming = parser.parse("{}", SourceRole::Incoming);
    let current = parser.parse("{}", SourceRole::Current);
    assert!(
        !typed::merge2(&incoming, &current, JsonDialect::Json, |_| Err("stop".into())).unwrap().ok
    );
    assert!(
        typed::merge2(&current, &incoming, JsonDialect::Json, |_| panic!("wrong roles")).is_err()
    );
}

#[test]
fn render_evidence_covers_retained_bytes_and_actual_replacements_without_donor_claims() {
    let parser = Parser::new("json5");
    let current = parser.parse("// café\r\n{\r\n  x: 1\r\n}\r\n", SourceRole::Current);
    let incoming = parser.parse("{x:2,y:'é'}", SourceRole::Incoming);
    let execution =
        typed::merge2_with_evidence(&incoming, &current, JsonDialect::Json5, |output| {
            Ok(parser.parse(output, SourceRole::Output))
        })
        .unwrap();
    assert!(execution.result.ok, "{:?}", execution.result.diagnostics);
    let output = execution.result.output.unwrap();
    let render = execution.render.unwrap();
    assert_eq!(render.baseline, *current.source.descriptor());
    render.validate(&current.source, &output).unwrap();
    assert!(!render.edits.is_empty());
    assert!(!render.retained.is_empty());
    let mut partitions = render
        .edits
        .iter()
        .map(|edit| edit.output_range.clone())
        .chain(render.retained.iter().map(|region| region.output_range.clone()))
        .collect::<Vec<_>>();
    partitions.sort_by_key(|range| range.start_byte);
    let mut end = 0;
    for range in partitions {
        assert_eq!(range.start_byte, end);
        end = range.end_byte;
    }
    assert_eq!(end, output.len());
    for region in &render.retained {
        assert_eq!(
            current.source.slice(region.source_range.clone()).unwrap(),
            &output.as_bytes()[region.output_range.start_byte..region.output_range.end_byte]
        );
    }
    for edit in &render.edits {
        assert_eq!(
            edit.replacement.as_bytes(),
            &output.as_bytes()[edit.output_range.start_byte..edit.output_range.end_byte]
        );
    }
    for mutation in 0..6 {
        let mut forged = render.clone();
        match mutation {
            0 => forged.retained[0].sha256 = "0".repeat(64),
            1 => forged.retained[0].output_range.end_byte += 1,
            2 => forged.edits[0].replacement.push(' '),
            3 => forged.edits[0].output_range.start_byte += 1,
            4 => forged.baseline.role = SourceRole::Incoming,
            _ => {
                forged.retained.pop();
            }
        }
        assert!(forged.validate(&current.source, &output).is_err());
    }
    assert!(render.validate(&incoming.source, &output).is_err());
    assert!(render.validate(&current.source, &(output + " ")).is_err());
}

#[test]
fn selected_input_retention_names_the_executed_role_and_failure_discards_proof() {
    let parser = Parser::new("json");
    let base = parser.parse(r#"{"x":1}"#, SourceRole::Base);
    let ours = parser.parse(r#"{"x":1}"#, SourceRole::Ours);
    let theirs = parser.parse("{\r\n\"x\":2\r\n}", SourceRole::Theirs);
    let execution =
        typed::merge3_with_evidence(&base, &ours, &theirs, JsonDialect::Json, |output| {
            Ok(parser.parse(output, SourceRole::Output))
        })
        .unwrap();
    let render = execution.render.unwrap();
    assert_eq!(render.baseline.role, SourceRole::Theirs);
    assert!(render.edits.is_empty());
    assert_eq!(render.retained.len(), 1);
    assert_eq!(render.retained[0].source_range.end_byte, theirs.source.bytes().len());
    render.validate(&theirs.source, execution.result.output.as_ref().unwrap()).unwrap();
    let failed = typed::merge3_with_evidence(&base, &ours, &theirs, JsonDialect::Json, |_| {
        Err("parser refused verification".into())
    })
    .unwrap();
    assert!(failed.result.output.is_none());
    assert!(failed.render.is_none());
    let changed_ours = parser.parse(r#"{"x":3}"#, SourceRole::Ours);
    let conflict =
        typed::merge3_with_evidence(&base, &changed_ours, &theirs, JsonDialect::Json, |_| {
            panic!("conflict cannot render")
        })
        .unwrap();
    assert_eq!(conflict.result.outcome, ThreeWayMergeOutcome::Conflict);
    assert!(conflict.render.is_none());
}
