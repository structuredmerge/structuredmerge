use ast_merge::{
    SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap, SourceLineOwner,
    SourceLineSelection, SourceRevision, render_source_plan, source_line_dispositions,
    source_render_plan_from_line_selections,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Fixture {
    documents: Vec<Document>,
    selections: Vec<SourceLineSelection>,
    expected: Expected,
}

#[derive(Deserialize)]
struct Document {
    revision: String,
    source: String,
    owners: Vec<SourceLineOwner>,
    comment_regions: Vec<SourceLineCommentRegion>,
    layout_gaps: Vec<SourceLineLayoutGap>,
}

#[derive(Deserialize)]
struct Expected {
    content: String,
    line_records: Vec<ExpectedLineRecord>,
}

#[derive(Deserialize)]
struct ExpectedLineRecord {
    output_line: usize,
    revision: String,
    original_line: usize,
    selection_key: String,
    disposition_kind: String,
}

fn revision(name: &str) -> SourceRevision {
    match name {
        "base" => SourceRevision::Base,
        "ours" => SourceRevision::Ours,
        "theirs" => SourceRevision::Theirs,
        _ => panic!("unknown source revision {name}"),
    }
}

#[test]
fn shared_source_line_selection_fixture_renders_exact_bytes_and_provenance() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../fixtures/polyglot/ast-merge/source_line_selection.json"
    ))
    .expect("shared source line selection fixture is valid");
    let mut sources = HashMap::new();
    let mut dispositions = HashMap::<SourceRevision, Vec<SourceLineDisposition>>::new();
    for document in fixture.documents {
        let source_revision = revision(&document.revision);
        let line_dispositions = source_line_dispositions(
            &document.source,
            &document.owners,
            &document.comment_regions,
            &document.layout_gaps,
        )
        .expect("source document line ownership is unambiguous");
        sources.insert(source_revision, document.source);
        dispositions.insert(source_revision, line_dispositions);
    }
    let plan = source_render_plan_from_line_selections(sources, dispositions, &fixture.selections)
        .expect("line selection plan is valid");
    let rendered = render_source_plan(&plan).expect("selected source lines render");
    assert_eq!(rendered.content, fixture.expected.content);
    let actual_records = rendered
        .line_records
        .iter()
        .map(|record| {
            serde_json::json!({
                "output_line": record.output_line,
                "revision": format!("{:?}", record.revision.unwrap()).to_lowercase(),
                "original_line": record.original_line.unwrap(),
                "selection_key": record.metadata["selection_key"],
                "disposition_kind": record.metadata["disposition_kind"]
            })
        })
        .collect::<Vec<_>>();
    let expected_records = fixture
        .expected
        .line_records
        .iter()
        .map(|record| {
            serde_json::json!({
                "output_line": record.output_line,
                "revision": record.revision,
                "original_line": record.original_line,
                "selection_key": record.selection_key,
                "disposition_kind": record.disposition_kind
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_records, expected_records);
}
