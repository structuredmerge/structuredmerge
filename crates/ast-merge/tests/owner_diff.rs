use ast_merge::owner_diff::{OwnerChangeKind, diff_owner_documents};
use ast_merge::{SourcePreservingOwner, SourcePreservingOwnerDocument};
use tree_haver::{
    ByteRange,
    source::{SourceDocument, SourceEncoding, SourceRole, source_input},
};

fn input(
    role: SourceRole,
    text: &str,
    owners: &[(&str, usize, usize)],
) -> (SourceDocument, SourcePreservingOwnerDocument) {
    let source = SourceDocument::validate(
        source_input(format!("{role:?}"), role, SourceEncoding::Utf8, text.as_bytes().to_vec())
            .unwrap(),
        1000,
    )
    .unwrap();
    let document = SourcePreservingOwnerDocument {
        source: text.into(),
        owners: owners
            .iter()
            .map(|(id, start, end)| SourcePreservingOwner {
                id: (*id).into(),
                path: format!("/{id}"),
                fingerprint: "deliberately-identical".into(),
                start_byte: *start,
                end_byte: *end,
                start_line: 1,
                end_line: 1,
            })
            .collect(),
    };
    (source, document)
}

#[test]
fn changes_keep_semantic_roles_exact_bytes_and_stable_order() {
    let (bs, before) = input(SourceRole::Before, "é=1\r\nb=2", &[("a", 0, 6), ("b", 6, 9)]);
    let (as_, after) = input(SourceRole::After, "é=3\r\nc=4", &[("a", 0, 6), ("c", 6, 9)]);
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert_eq!(
        diff.changes.iter().map(|c| c.kind).collect::<Vec<_>>(),
        vec![OwnerChangeKind::Edited, OwnerChangeKind::Deleted, OwnerChangeKind::Added]
    );
    assert_eq!(
        diff.changes.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["change-0", "change-1", "change-2"]
    );
    assert_eq!(
        diff.changes[0].before.as_ref().unwrap().range,
        ByteRange { start_byte: 0, end_byte: 6 }
    );
    assert_eq!(diff.changes[0].before.as_ref().unwrap().source_role, SourceRole::Before);
    assert_eq!(diff.changes[0].after.as_ref().unwrap().source_role, SourceRole::After);
    assert_ne!(
        diff.changes[0].before.as_ref().unwrap().sha256,
        diff.changes[0].after.as_ref().unwrap().sha256
    );
    assert!(diff.changes[1].after.is_none());
    assert!(diff.changes[2].before.is_none());
    assert_eq!(diff, diff_owner_documents(&bs, &before, &as_, &after).unwrap());
}

#[test]
fn repeated_fragments_use_supplied_byte_ranges_not_text_search() {
    let (bs, before) = input(SourceRole::Before, "x\nx\n", &[("first", 0, 2), ("second", 2, 4)]);
    let (as_, after) = input(SourceRole::After, "x\ny\n", &[("first", 0, 2), ("second", 2, 4)]);
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].owner_id, "second");
    assert_eq!(diff.changes[0].before.as_ref().unwrap().range.start_byte, 2);
}

#[test]
fn layout_and_order_changes_are_observable_even_without_owned_byte_edits() {
    let (bs, before) = input(SourceRole::Before, "a\nb\n", &[("a", 0, 2), ("b", 2, 4)]);
    let (as_, after) = input(SourceRole::After, "b\na\n", &[("b", 0, 2), ("a", 2, 4)]);
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert_ne!(diff.before_owner_order, diff.after_owner_order);
    let (as_, after) = input(SourceRole::After, "\u{feff}a\nb\n\r\n", &[("a", 3, 5), ("b", 5, 7)]);
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff.before_layout.is_empty());
    assert_eq!(
        diff.after_layout.iter().map(|r| r.range.clone()).collect::<Vec<_>>(),
        vec![ByteRange { start_byte: 0, end_byte: 3 }, ByteRange { start_byte: 7, end_byte: 9 }]
    );
    assert_ne!(diff.sources[0].sha256, diff.sources[1].sha256);
}

#[test]
fn invalid_or_ambiguous_analysis_fails_before_classification() {
    let (bs, before) = input(SourceRole::Before, "é=1\n", &[("a", 0, 5)]);
    let (as_, after) = input(SourceRole::After, "é=2\n", &[("a", 0, 5)]);
    let mut invalid = before.clone();
    invalid.source = "different".into();
    assert!(diff_owner_documents(&bs, &invalid, &as_, &after).is_err());
    for range in [(1, 5), (0, 99), (3, 2), (0, 0)] {
        let mut invalid = before.clone();
        invalid.owners[0].start_byte = range.0;
        invalid.owners[0].end_byte = range.1;
        assert!(diff_owner_documents(&bs, &invalid, &as_, &after).is_err());
    }
    let mut invalid = before.clone();
    invalid.owners.push(invalid.owners[0].clone());
    assert!(diff_owner_documents(&bs, &invalid, &as_, &after).is_err());
    assert!(diff_owner_documents(&as_, &after, &bs, &before).is_err());
}

#[test]
fn empty_documents_have_no_fabricated_regions() {
    let (bs, before) = input(SourceRole::Before, "", &[]);
    let (as_, after) = input(SourceRole::After, "", &[]);
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff.before_layout.is_empty());
    assert!(diff.after_layout.is_empty());
}

#[test]
fn duplicate_sources_and_overlapping_distinct_owners_are_rejected() {
    let (bs, before) = input(SourceRole::Before, "abc\n", &[("a", 0, 4)]);
    let (_, after) = input(SourceRole::After, "abc\n", &[("a", 0, 4)]);
    let same_id = SourceDocument::validate(
        source_input(
            bs.descriptor().source_id.clone(),
            SourceRole::After,
            SourceEncoding::Utf8,
            after.source.as_bytes().to_vec(),
        )
        .unwrap(),
        1000,
    )
    .unwrap();
    assert!(diff_owner_documents(&bs, &before, &same_id, &after).is_err());
    let (as_, _) = input(SourceRole::After, "abc\n", &[]);
    let mut invalid = before.clone();
    let mut overlapping = invalid.owners[0].clone();
    overlapping.id = "b".into();
    overlapping.start_byte = 2;
    invalid.owners.push(overlapping);
    assert!(diff_owner_documents(&bs, &invalid, &as_, &after).is_err());
}

#[test]
fn changed_subject_path_is_not_hidden_by_equal_bytes() {
    let (bs, before) = input(SourceRole::Before, "a\n", &[("a", 0, 2)]);
    let (as_, mut after) = input(SourceRole::After, "a\n", &[("a", 0, 2)]);
    after.owners[0].path = "/renamed-subject".into();
    let diff = diff_owner_documents(&bs, &before, &as_, &after).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].before_path.as_deref(), Some("/a"));
    assert_eq!(diff.changes[0].after_path.as_deref(), Some("/renamed-subject"));
    assert_eq!(diff.changes[0].kind, OwnerChangeKind::Edited);
    assert_eq!(
        diff.changes[0].before.as_ref().unwrap().sha256,
        diff.changes[0].after.as_ref().unwrap().sha256
    );
}
