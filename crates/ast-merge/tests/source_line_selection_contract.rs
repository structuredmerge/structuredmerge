use ast_merge::{
    SourceLineCommentRegion, SourceLineDisposition, SourceLineLayoutGap, SourceLineOwner,
    SourceLineSelection, SourceRevision, render_source_plan, source_line_dispositions,
    source_line_owner_addition_range, source_line_selections_for_owner_revisions,
    source_render_plan_from_line_selections,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct Fixture {
    baseline_revision: String,
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
        "../../../fixtures/polyglot/ast-merge/source_line_selection.json"
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
    let owner_revisions = fixture
        .selections
        .iter()
        .filter_map(|selection| {
            selection
                .selection_key
                .strip_prefix("owner:")
                .map(|owner_id| (owner_id.to_string(), selection.revision))
        })
        .collect::<HashMap<_, _>>();
    let generated_selections = source_line_selections_for_owner_revisions(
        revision(&fixture.baseline_revision),
        &dispositions,
        &owner_revisions,
    )
    .expect("owner choices produce a line selection plan");
    assert_eq!(generated_selections, fixture.selections);
    let plan =
        source_render_plan_from_line_selections(sources, dispositions, &generated_selections)
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

#[derive(Deserialize)]
struct GapArbitrationFixture {
    baseline_revision: String,
    documents: Vec<Document>,
    owner_revisions: Vec<OwnerRevision>,
    deleted_owner_ids: Vec<String>,
    expected: GapArbitrationExpected,
}

#[derive(Deserialize)]
struct GapArbitrationExpected {
    selections: Vec<SourceLineSelection>,
    content: String,
    retained_gap_key: String,
}

#[test]
fn shared_gap_removal_arbitration_prefers_surviving_before_owner_controller() {
    let fixture: GapArbitrationFixture = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/source_line_gap_removal_arbitration.json"
    ))
    .expect("shared gap removal arbitration fixture is valid");
    let mut sources = HashMap::new();
    let mut dispositions = HashMap::new();
    for document in fixture.documents {
        let source_revision = revision(&document.revision);
        let records = source_line_dispositions(
            &document.source,
            &document.owners,
            &document.comment_regions,
            &document.layout_gaps,
        )
        .expect("fixture source assigns each line to one owner or gap");
        sources.insert(source_revision, document.source);
        dispositions.insert(source_revision, records);
    }
    let owner_revisions = fixture
        .owner_revisions
        .iter()
        .map(|entry| (entry.owner_id.clone(), revision(&entry.revision)))
        .collect::<HashMap<_, _>>();
    let deleted_owner_ids = fixture.deleted_owner_ids.into_iter().collect::<HashSet<_>>();
    let selections = ast_merge::source_line_selections_for_owner_revisions_with_deletions(
        revision(&fixture.baseline_revision),
        &dispositions,
        &owner_revisions,
        &deleted_owner_ids,
    )
    .expect("deleted-owner selection resolves the collapsed gap boundary");
    assert_eq!(selections, fixture.expected.selections);
    assert!(
        selections
            .iter()
            .any(|selection| { selection.selection_key == fixture.expected.retained_gap_key })
    );
    let plan = source_render_plan_from_line_selections(sources, dispositions, &selections)
        .expect("gap-arbitration selections form a render plan");
    let rendered = render_source_plan(&plan).expect("selected owner and gap lines render");
    assert_eq!(rendered.content, fixture.expected.content);
}

#[derive(Deserialize)]
struct AdditionFixture {
    baseline_revision: String,
    documents: Vec<AdditionDocument>,
    owner_revisions: Vec<OwnerRevision>,
    expected: AdditionExpected,
}

#[derive(Deserialize)]
struct AdditionDocument {
    revision: String,
    source: String,
    owners: Vec<SourceLineOwner>,
    comment_regions: Vec<SourceLineCommentRegion>,
    layout_gaps: Vec<SourceLineLayoutGap>,
}

#[derive(Deserialize)]
struct OwnerRevision {
    owner_id: String,
    revision: String,
}

#[derive(Deserialize)]
struct AdditionExpected {
    selections: Vec<SourceLineSelection>,
    content: String,
}

#[test]
fn added_owner_selection_carries_its_leading_gap_from_the_selected_source() {
    let fixture: AdditionFixture = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/source_line_addition_selection.json"
    ))
    .expect("shared source line addition fixture is valid");
    let mut sources = HashMap::new();
    let mut dispositions = HashMap::new();
    for document in fixture.documents {
        let source_revision = revision(&document.revision);
        let records = source_line_dispositions(
            &document.source,
            &document.owners,
            &document.comment_regions,
            &document.layout_gaps,
        )
        .expect("added owner line facts are unambiguous");
        if source_revision == SourceRevision::Theirs {
            let range = source_line_owner_addition_range(&document.source, &records, "added")
                .expect("directly adjacent leading gap belongs to added owner");
            assert_eq!(&document.source[range], "\nfn added() {}\n\n");
        }
        sources.insert(source_revision, document.source);
        dispositions.insert(source_revision, records);
    }
    let owner_revisions = fixture
        .owner_revisions
        .iter()
        .map(|entry| (entry.owner_id.clone(), revision(&entry.revision)))
        .collect::<HashMap<_, _>>();
    let selections = source_line_selections_for_owner_revisions(
        revision(&fixture.baseline_revision),
        &dispositions,
        &owner_revisions,
    )
    .expect("owner choices produce an addition line plan");
    assert_eq!(selections, fixture.expected.selections);
    let plan = source_render_plan_from_line_selections(sources, dispositions, &selections)
        .expect("addition source line plan is valid");
    let rendered = render_source_plan(&plan).expect("added owner source lines render");
    assert_eq!(rendered.content, fixture.expected.content);
}

#[derive(Deserialize)]
struct DispositionFixture {
    cases: Vec<DispositionCase>,
}

#[derive(Deserialize)]
struct DispositionCase {
    name: String,
    source: String,
    expected: Vec<SourceLineDisposition>,
}

#[test]
fn attached_comment_source_range_excludes_preceding_owner_postlude_gap() {
    let fixture: DispositionFixture = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/source_line_disposition.json"
    ))
    .expect("shared source line disposition fixture is valid");
    let case = fixture
        .cases
        .into_iter()
        .find(|case| {
            case.name
                == "a blank gap before a leading comment remains controlled by the preceding owner"
        })
        .expect("shared leading-comment gap case exists");
    let range = source_line_owner_addition_range(&case.source, &case.expected, "beta")
        .expect("selected comment and owner lines form one source range");
    assert_eq!(&case.source[range], "# beta docs\nbeta()\n");
}
