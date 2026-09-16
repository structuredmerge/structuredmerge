use ast_merge::directional_render::{
    DirectionalInsertion, render_directional_owners, verify_directional_segments,
};
use ast_merge::{
    SourcePreservingOwner, SourcePreservingOwnerDocument,
    owner_merge2::{DirectionalOwnerAction::*, classify_directional_owners},
};
use sha2::{Digest, Sha256};
use tree_haver::ByteRange;
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

#[test]
fn directional_render_retains_current_bytes_and_explicit_incoming_layout() {
    let (is, incoming) = input(SourceRole::Incoming, "a=1\nb=2\n", &[("a", 0, 3), ("b", 4, 7)]);
    let (cs, current) =
        input(SourceRole::Current, "\u{feff}a=9\r\n# local\r\nz=0", &[("a", 3, 6), ("z", 17, 20)]);
    let insertions = [DirectionalInsertion {
        owner_id: "b".into(),
        before_current_owner_id: Some("z".into()),
        current_offset: 6,
        source_range: ByteRange { start_byte: 3, end_byte: 7 },
    }];
    let expected = "\u{feff}a=9\nb=2\r\n# local\r\nz=0";
    // Unit seam only: native reparse integration is a separate execution gate.
    let result = render_directional_owners(&is, &incoming, &cs, &current, &insertions, |out| {
        assert_eq!(out, expected);
        Ok(input(SourceRole::Output, out, &[("a", 3, 6), ("b", 7, 10), ("z", 21, 24)]).1)
    })
    .unwrap();
    assert_eq!(result.output, expected);
    assert_eq!(result.owner_order, ["a", "b", "z"]);
    assert_eq!(
        result.segments.iter().map(|s| s.source_role).collect::<Vec<_>>(),
        [SourceRole::Current, SourceRole::Incoming, SourceRole::Current]
    );
    assert_eq!(
        verify_directional_segments(expected.as_bytes(), &is, &cs, &result.segments),
        Ok(())
    );
    let retained: Vec<_> = result
        .segments
        .iter()
        .filter(|s| s.source_role == SourceRole::Current)
        .flat_map(|s| {
            result.output.as_bytes()[s.output_range.start_byte..s.output_range.end_byte]
                .iter()
                .copied()
        })
        .collect();
    assert_eq!(retained, current.source.as_bytes());
    let original = result.segments;
    for field in 0..7 {
        let mut changed = original.clone();
        match field {
            0 => changed[1].source_role = SourceRole::Theirs,
            1 => changed[1].source_id = "wrong".into(),
            2 => changed[1].sha256 = "wrong".into(),
            3 => changed[1].output_range.start_byte += 1,
            4 => changed[2].source_range.start_byte += 1,
            5 => changed[1].source_range.end_byte = usize::MAX,
            _ => changed[1].id = changed[0].id.clone(),
        }
        assert!(verify_directional_segments(expected.as_bytes(), &is, &cs, &changed).is_err());
    }
    assert!(
        verify_directional_segments(expected.replace("b=2", "b=3").as_bytes(), &is, &cs, &original)
            .is_err()
    );
    assert!(verify_directional_segments(b"", &is, &cs, &[]).is_err());
}

#[test]
fn incomplete_or_ambiguous_insertion_plans_fail_before_verification() {
    let (is, incoming) = input(SourceRole::Incoming, "a=1\nb=2\n", &[("a", 0, 3), ("b", 4, 7)]);
    let (cs, current) = input(SourceRole::Current, "a=9\n", &[("a", 0, 3)]);
    let valid = DirectionalInsertion {
        owner_id: "b".into(),
        before_current_owner_id: None,
        current_offset: current.source.len(),
        source_range: ByteRange { start_byte: 4, end_byte: 8 },
    };
    for insertions in [
        vec![],
        vec![valid.clone(), valid.clone()],
        vec![DirectionalInsertion { owner_id: "a".into(), ..valid.clone() }],
        vec![DirectionalInsertion {
            before_current_owner_id: Some("missing".into()),
            ..valid.clone()
        }],
        vec![DirectionalInsertion { current_offset: 1, ..valid.clone() }],
        vec![DirectionalInsertion { current_offset: usize::MAX, ..valid.clone() }],
        vec![DirectionalInsertion {
            source_range: ByteRange { start_byte: 0, end_byte: 8 },
            ..valid.clone()
        }],
        vec![DirectionalInsertion {
            source_range: ByteRange { start_byte: 4, end_byte: 6 },
            ..valid.clone()
        }],
        vec![DirectionalInsertion {
            source_range: ByteRange { start_byte: 4, end_byte: 99 },
            ..valid.clone()
        }],
    ] {
        assert!(
            render_directional_owners(&is, &incoming, &cs, &current, &insertions, |_| panic!(
                "invalid plan reached verification"
            ))
            .is_err()
        );
    }
}

#[test]
fn output_must_reparse_with_exact_selected_owners_not_just_equal_fingerprints() {
    let (is, incoming) = input(SourceRole::Incoming, "a=1\nb=2\n", &[("a", 0, 3), ("b", 4, 7)]);
    let (cs, current) = input(SourceRole::Current, "a=9\n", &[("a", 0, 3)]);
    let insertions = [DirectionalInsertion {
        owner_id: "b".into(),
        before_current_owner_id: None,
        current_offset: current.source.len(),
        source_range: ByteRange { start_byte: 4, end_byte: 8 },
    }];
    for failure in 0..6 {
        let result = render_directional_owners(&is, &incoming, &cs, &current, &insertions, |out| {
            assert_eq!(out, "a=9\nb=2\n");
            if failure == 0 {
                return Err("native output parse rejected".into());
            }
            let mut doc = input(SourceRole::Output, out, &[("a", 0, 3), ("b", 4, 7)]).1;
            match failure {
                1 => doc.source = "other bytes".into(),
                2 => {
                    doc.owners.pop();
                }
                3 => doc.owners[0].fingerprint = "wrong".into(),
                4 => doc.owners[0].end_byte = 2,
                _ => doc.owners[0].path = "/wrong".into(),
            }
            Ok(doc)
        });
        assert!(result.is_err());
    }
}

#[test]
fn exact_current_and_empty_outputs_still_invoke_verification() {
    for text in ["", "\u{feff}# comment\r\n"] {
        let (is, incoming) = input(SourceRole::Incoming, "", &[]);
        let (cs, current) = input(SourceRole::Current, text, &[]);
        let mut called = false;
        let result = render_directional_owners(&is, &incoming, &cs, &current, &[], |out| {
            called = true;
            assert_eq!(out, text);
            Ok(current.clone())
        })
        .unwrap();
        assert!(called);
        assert_eq!(result.output, text);
        assert_eq!(result.segments.len(), usize::from(!text.is_empty()));
    }
}

#[test]
fn ownerless_current_accepts_explicit_order_and_rejects_overlapping_layout() {
    let (is, incoming) = input(SourceRole::Incoming, "b=2\nc=3\n", &[("b", 0, 3), ("c", 4, 7)]);
    let (cs, current) = input(SourceRole::Current, "", &[]);
    let b = DirectionalInsertion {
        owner_id: "b".into(),
        before_current_owner_id: None,
        current_offset: current.source.len(),
        source_range: ByteRange { start_byte: 0, end_byte: 4 },
    };
    let c = DirectionalInsertion {
        owner_id: "c".into(),
        before_current_owner_id: None,
        current_offset: current.source.len(),
        source_range: ByteRange { start_byte: 4, end_byte: 8 },
    };
    let result =
        render_directional_owners(&is, &incoming, &cs, &current, &[c.clone(), b.clone()], |out| {
            assert_eq!(out, "c=3\nb=2\n");
            Ok(input(SourceRole::Output, out, &[("c", 0, 3), ("b", 4, 7)]).1)
        })
        .unwrap();
    assert_eq!(result.owner_order, ["c", "b"]);
    assert_eq!(result.classification.incoming_owner_order, ["b", "c"]);
    let overlap =
        DirectionalInsertion { source_range: ByteRange { start_byte: 3, end_byte: 8 }, ..c };
    assert!(
        render_directional_owners(&is, &incoming, &cs, &current, &[b, overlap], |_| panic!(
            "overlap reached verification"
        ))
        .is_err()
    );
}
