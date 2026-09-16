use ast_merge::{
    SourcePreservingOwner, SourcePreservingOwnerDocument,
    owner_merge2::{DirectionalOwnerAction::*, classify_directional_owners},
};
use sha2::{Digest, Sha256};
use tree_haver::source::{SourceDocument, SourceEncoding, SourceRole, source_input};

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
                // Equal fingerprints must not hide differing selected source bytes.
                fingerprint: "equal".into(),
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
fn classifies_all_actions_without_deleting_current_only_owners() {
    let (is, incoming) =
        input(SourceRole::Incoming, "a=1\nb=2\nd=4\n", &[("a", 0, 4), ("b", 4, 8), ("d", 8, 12)]);
    let (cs, current) =
        input(SourceRole::Current, "c=3\nb=2\na=9\n", &[("c", 0, 4), ("b", 4, 8), ("a", 8, 12)]);
    let result = classify_directional_owners(&is, &incoming, &cs, &current).unwrap();
    assert_eq!(
        result.decisions.iter().map(|d| d.action).collect::<Vec<_>>(),
        vec![RetainCurrentOnly, RetainIdenticalCurrent, PreferCurrent, AddIncomingOnly]
    );
    assert_eq!(result.current_owner_order, ["c", "b", "a"]);
    assert_eq!(result.incoming_owner_order, ["a", "b", "d"]);
    assert_eq!(
        result.sources.iter().map(|s| s.role).collect::<Vec<_>>(),
        [SourceRole::Incoming, SourceRole::Current]
    );
    assert!(result.decisions[0].incoming.is_none());
    assert!(result.decisions[3].current.is_none());
    for (index, decision) in result.decisions.iter().enumerate() {
        assert_eq!(decision.id, format!("directional-decision-{index}"));
        for (region, source) in [(&decision.incoming, &is), (&decision.current, &cs)] {
            if let Some(region) = region {
                assert_eq!(region.source_id, source.descriptor().source_id);
                assert_eq!(region.source_role, source.descriptor().role);
                assert_eq!(
                    region.sha256,
                    format!(
                        "{:x}",
                        Sha256::digest(
                            &source.bytes()[region.range.start_byte..region.range.end_byte]
                        )
                    )
                );
            }
        }
    }
    assert_eq!(result, classify_directional_owners(&is, &incoming, &cs, &current).unwrap());
}

#[test]
fn reversing_direction_changes_the_selected_existing_value() {
    let (is, incoming) = input(SourceRole::Incoming, "a=1", &[("a", 0, 3)]);
    let (cs, current) = input(SourceRole::Current, "a=2", &[("a", 0, 3)]);
    assert!(classify_directional_owners(&cs, &current, &is, &incoming).is_err());
    let forward = classify_directional_owners(&is, &incoming, &cs, &current).unwrap();
    let (ri, reversed_incoming) = input(SourceRole::Incoming, "a=2", &[("a", 0, 3)]);
    let (rc, reversed_current) = input(SourceRole::Current, "a=1", &[("a", 0, 3)]);
    let reverse =
        classify_directional_owners(&ri, &reversed_incoming, &rc, &reversed_current).unwrap();
    assert_eq!(forward.decisions[0].action, PreferCurrent);
    assert_eq!(reverse.decisions[0].action, PreferCurrent);
    assert_ne!(forward.decisions[0].current, reverse.decisions[0].current);
}

#[test]
fn keeps_unowned_bom_crlf_comments_and_no_final_newline_observable() {
    let text = "\u{feff}# c\r\né=1\r\n# end";
    let (is, incoming) = input(SourceRole::Incoming, text, &[("é", 8, 14)]);
    let (cs, current) = input(SourceRole::Current, text, &[("é", 8, 14)]);
    let result = classify_directional_owners(&is, &incoming, &cs, &current).unwrap();
    assert_eq!(result.decisions[0].action, RetainIdenticalCurrent);
    for regions in [&result.incoming_layout, &result.current_layout] {
        assert_eq!(regions.len(), 2);
        assert_eq!(
            &text[regions[0].range.start_byte..regions[0].range.end_byte],
            "\u{feff}# c\r\n"
        );
        assert_eq!(&text[regions[1].range.start_byte..regions[1].range.end_byte], "# end");
    }
}

#[test]
fn rejects_invalid_ownership_before_classification() {
    let (is, incoming) = input(SourceRole::Incoming, "é=1\n", &[("a", 0, 5)]);
    let (cs, current) = input(SourceRole::Current, "é=2\n", &[("a", 0, 5)]);
    for range in [(1, 5), (0, 99), (3, 2), (0, 0)] {
        let mut invalid = incoming.clone();
        invalid.owners[0].start_byte = range.0;
        invalid.owners[0].end_byte = range.1;
        assert!(classify_directional_owners(&is, &invalid, &cs, &current).is_err());
    }
    let mut invalid = incoming.clone();
    invalid.source = "wrong".into();
    assert!(classify_directional_owners(&is, &invalid, &cs, &current).is_err());
    let mut invalid = incoming.clone();
    invalid.owners.push(invalid.owners[0].clone());
    assert!(classify_directional_owners(&is, &invalid, &cs, &current).is_err());
    invalid.owners[1].id = "other".into();
    assert!(classify_directional_owners(&is, &invalid, &cs, &current).is_err());
    let mut invalid = incoming.clone();
    invalid.owners[0].path = "/different".into();
    assert!(classify_directional_owners(&is, &invalid, &cs, &current).is_err());
    let same_id = SourceDocument::validate(
        source_input(
            is.descriptor().source_id.clone(),
            SourceRole::Current,
            SourceEncoding::Utf8,
            current.source.as_bytes().to_vec(),
        )
        .unwrap(),
        1000,
    )
    .unwrap();
    assert!(classify_directional_owners(&is, &incoming, &same_id, &current).is_err());
}

#[test]
fn empty_and_layout_only_sources_have_no_fabricated_owners() {
    let (is, incoming) = input(SourceRole::Incoming, "", &[]);
    let (cs, current) = input(SourceRole::Current, "# retained\r\n", &[]);
    let result = classify_directional_owners(&is, &incoming, &cs, &current).unwrap();
    assert!(result.decisions.is_empty());
    assert!(result.incoming_layout.is_empty());
    assert_eq!(result.current_layout.len(), 1);
    assert_eq!(result.current_layout[0].range.end_byte, current.source.len());
}
