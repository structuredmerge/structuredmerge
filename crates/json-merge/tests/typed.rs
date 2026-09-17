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
        self.parse_with_id(text, role, &format!("{role:?}"))
    }
    fn parse_with_id(&self, text: &str, role: SourceRole, source_id: &str) -> ParsedResult {
        TreeHaverParseService::default()
            .parse_batch(
                vec![ParseRequest {
                    schema: PARSE_REQUEST_SCHEMA.into(),
                    request_id: format!("{role:?}"),
                    source: source_input(
                        source_id.into(),
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
fn owner_facts_use_native_spans_for_repeated_fragments_and_escaped_paths() {
    let parser = Parser::new("json");
    let source = "{\r\n \"é\": 0, \"a\": {\"x\":1}, \"b\": {\"x\":1}, \"~/\": [1,1]}";
    let parsed = parser.parse(source, SourceRole::Source);
    let analysis = typed::owner_analysis(&parsed, JsonDialect::Json).unwrap();
    assert_eq!(analysis.source, *parsed.source.descriptor());
    assert_eq!(analysis.family, typed::analyze(&parsed, JsonDialect::Json).unwrap());
    assert_eq!(
        analysis.owners.iter().map(|owner| owner.path.as_str()).collect::<Vec<_>>(),
        ["", "/é", "/a", "/a/x", "/b", "/b/x", "/~0~1", "/~0~1/0", "/~0~1/1"]
    );
    for owner in &analysis.owners {
        assert_eq!(owner.span, parsed.document.node(&owner.node_id).unwrap().span);
        assert_eq!(owner.sha256, parsed.source.range_digest(owner.span.range.clone()).unwrap());
        if let Some(parent_id) = &owner.parent_id {
            let parent = analysis.owners.iter().find(|other| &other.id == parent_id).unwrap();
            assert!(parent.span.range.start_byte <= owner.span.range.start_byte);
            assert!(parent.span.range.end_byte >= owner.span.range.end_byte);
        }
    }
    let first = &analysis.owners[3];
    let second = &analysis.owners[5];
    assert_eq!(first.sha256, second.sha256);
    assert_ne!(first.node_id, second.node_id);
    assert!(first.span.range.end_byte < second.span.range.start_byte);
    assert_eq!(parsed.source.slice(second.span.range.clone()).unwrap(), b"\"x\":1");
    assert_eq!(analysis.owners[6].match_key.as_deref(), Some("~/"));
}

#[test]
fn exact_owner_diff_classifies_nested_changes_without_fragment_relocation() {
    let parser = Parser::new("json");
    let before = parser.parse(r#"{"a":{"x":1},"b":{"x":1},"gone":0}"#, SourceRole::Before);
    let after = parser.parse(r#"{"a":{"x":1},"b":{"x":2},"new":0}"#, SourceRole::After);
    let changes = typed::diff_owner_sources(&before, &after, JsonDialect::Json).unwrap();
    assert_eq!(
        changes.iter().map(|c| (c.path.as_str(), c.classification.as_str())).collect::<Vec<_>>(),
        [
            ("", "edited"),
            ("/b", "edited"),
            ("/b/x", "edited"),
            ("/gone", "deleted"),
            ("/new", "added")
        ]
    );
    let nested = &changes[2];
    assert_eq!(nested.before.as_ref().unwrap().span.range.start_byte, 18);
    assert_eq!(nested.after.as_ref().unwrap().span.range.start_byte, 18);
    assert!(changes[3].after.is_none());
    assert!(changes[4].before.is_none());
    let same = parser.parse(r#"{"a":{"x":1},"b":{"x":1},"gone":0}"#, SourceRole::After);
    assert!(typed::diff_owner_sources(&before, &same, JsonDialect::Json).unwrap().is_empty());
}

#[test]
fn exact_owner_diff_includes_scalar_roots_and_uses_positional_array_identity() {
    let parser = Parser::new("json");
    for (left, right) in [("1", "2"), ("{}", "[]"), ("true", "false")] {
        let changes = typed::diff_owner_sources(
            &parser.parse(left, SourceRole::Before),
            &parser.parse(right, SourceRole::After),
            JsonDialect::Json,
        )
        .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "");
        assert_eq!(changes[0].classification, "edited");
    }
    let changes = typed::diff_owner_sources(
        &parser.parse("[1,2]", SourceRole::Before),
        &parser.parse("[0,1,2]", SourceRole::After),
        JsonDialect::Json,
    )
    .unwrap();
    assert_eq!(
        changes.iter().map(|c| (c.path.as_str(), c.classification.as_str())).collect::<Vec<_>>(),
        [("", "edited"), ("/0", "edited"), ("/1", "edited"), ("/2", "added")]
    );
}

#[test]
fn owner_analysis_rejects_ambiguous_decoded_keys_and_diff_rejects_wrong_roles() {
    let parser = Parser::new("json");
    for source in [r#"{"x":1,"x":2}"#, r#"{"x":1,"\u0078":2}"#, r#"{"a":{"x":1,"x":2}}"#] {
        assert!(
            typed::owner_analysis(&parser.parse(source, SourceRole::Source), JsonDialect::Json)
                .unwrap_err()
                .contains("duplicate JSON owner identity")
        );
    }
    let before = parser.parse("{}", SourceRole::Before);
    let after = parser.parse("{}", SourceRole::After);
    assert!(typed::diff_owner_sources(&after, &before, JsonDialect::Json).is_err());
    assert!(typed::diff_owner_sources(&before, &before, JsonDialect::Json).is_err());
    let same_id = parser.parse_with_id("{}", SourceRole::After, "Before");
    assert!(
        typed::diff_owner_sources(&before, &same_id, JsonDialect::Json)
            .unwrap_err()
            .contains("source IDs must be distinct")
    );
    let parser5 = Parser::new("json5");
    assert!(
        typed::owner_analysis(
            &parser5.parse("{x:1,'x':2}", SourceRole::Source),
            JsonDialect::Json5
        )
        .is_err()
    );
}

#[test]
fn owner_comparison_does_not_claim_to_cover_document_trivia() {
    let parser = Parser::new("json5");
    let before = parser.parse("// before\n{x:1}\n", SourceRole::Before);
    let after = parser.parse("// after\n{x:1}\n\n", SourceRole::After);
    let left = typed::owner_analysis(&before, JsonDialect::Json5).unwrap();
    let right = typed::owner_analysis(&after, JsonDialect::Json5).unwrap();
    assert_ne!(left.source.sha256, right.source.sha256);
    assert_ne!(left.family.comment_regions, right.family.comment_regions);
    assert!(typed::diff_owner_sources(&before, &after, JsonDialect::Json5).unwrap().is_empty());
    let interior = parser.parse("// before\n{x: 1}\n", SourceRole::After);
    let changes = typed::diff_owner_sources(&before, &interior, JsonDialect::Json5).unwrap();
    assert_eq!(changes.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), ["", "/x"]);
}

#[test]
fn comment_provenance_preserves_identical_native_nodes_and_reports_unclaimed_comments() {
    let parser = Parser::new("json5");
    for source in [
        "{/*same*/x:1,/*same*/y:2}",
        "// pre\n{\n x: 1,\n // same\n y: 2\n}\n// post\n",
        "{\n /* multiline\n comment */\n x: 1\n}",
        "{} /* unclaimed */",
    ] {
        let parsed = parser.parse(source, SourceRole::Source);
        let analysis = typed::owner_analysis(&parsed, JsonDialect::Json5).unwrap();
        assert_eq!(analysis.family, typed::analyze(&parsed, JsonDialect::Json5).unwrap());
        for region in &analysis.family.comment_regions {
            let ids = &analysis.comment_region_node_ids[&region.id];
            assert!(!ids.is_empty());
            for id in ids {
                let node = parsed.document.node(id).unwrap();
                assert_eq!(node.role, tree_haver::NodeRole::Comment);
                assert!(
                    parsed.document.output().comments.iter().any(|comment| &comment.node_id == id)
                );
            }
        }
        let observed = analysis
            .comment_region_node_ids
            .values()
            .flatten()
            .chain(&analysis.unclaimed_comment_node_ids)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = parsed
            .document
            .output()
            .comments
            .iter()
            .map(|comment| &comment.node_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(observed, expected);
        if source.starts_with("{/*same*/") {
            assert_eq!(analysis.comment_region_node_ids.values().flatten().count(), 2);
            assert_eq!(observed.len(), 2);
        }
        if source.starts_with("{}") {
            assert_eq!(analysis.unclaimed_comment_node_ids.len(), 1);
        }
    }
}

#[test]
fn layout_gap_evidence_preserves_exact_crlf_and_final_whitespace_bytes() {
    let parser = Parser::new("json5");
    let parsed = parser.parse("\r\n{\"é\":1}\r\n \t\r\n\t", SourceRole::Source);
    let analysis = typed::owner_analysis(&parsed, JsonDialect::Json5).unwrap();
    assert_eq!(analysis.layout_gap_sources.len(), analysis.family.layout_gaps.len());
    assert!(!analysis.layout_gap_sources.is_empty());
    for gap in &analysis.family.layout_gaps {
        let fact = &analysis.layout_gap_sources[&gap.id];
        assert_eq!(fact.span.start_point.row + 1, gap.start_line);
        assert_eq!(fact.sha256, parsed.source.range_digest(fact.span.range.clone()).unwrap());
        let bytes = parsed.source.slice(fact.span.range.clone()).unwrap();
        assert!(bytes.iter().all(u8::is_ascii_whitespace));
    }
    let suffix = analysis
        .layout_gap_sources
        .values()
        .find(|fact| fact.span.range.end_byte == parsed.source.bytes().len())
        .unwrap();
    assert_eq!(parsed.source.slice(suffix.span.range.clone()).unwrap(), b" \t\r\n\t");
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
