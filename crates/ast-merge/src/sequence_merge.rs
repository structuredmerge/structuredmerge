use std::collections::{HashMap, HashSet};

use crate::{NodeIdentity, match_node_identities};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SequenceOrderError {
    DuplicateIdentity { sequence_index: usize, node_id: String },
    DuplicateSelection { node_id: String },
    IncompatibleOrder { node_ids: Vec<String> },
}

pub fn merge_sequence_order_constraints(
    sequences: &[Vec<String>],
    selected: &[String],
) -> Result<Vec<String>, SequenceOrderError> {
    let mut selected_set = HashSet::new();
    for node_id in selected {
        if !selected_set.insert(node_id.clone()) {
            return Err(SequenceOrderError::DuplicateSelection { node_id: node_id.clone() });
        }
    }

    let mut rank = HashMap::new();
    let mut next_rank = 0usize;
    let mut edges = HashMap::<String, Vec<String>>::new();
    let mut indegree =
        selected.iter().map(|node_id| (node_id.clone(), 0usize)).collect::<HashMap<_, _>>();

    for (sequence_index, sequence) in sequences.iter().enumerate() {
        let mut seen = HashSet::new();
        for node_id in sequence {
            if !seen.insert(node_id) {
                return Err(SequenceOrderError::DuplicateIdentity {
                    sequence_index,
                    node_id: node_id.clone(),
                });
            }
            if selected_set.contains(node_id) && !rank.contains_key(node_id) {
                rank.insert(node_id.clone(), next_rank);
                next_rank += 1;
            }
        }

        let constrained =
            sequence.iter().filter(|node_id| selected_set.contains(*node_id)).collect::<Vec<_>>();
        for pair in constrained.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let successors = edges.entry(left.clone()).or_default();
            if !successors.contains(right) {
                successors.push(right.clone());
                *indegree.get_mut(right).expect("selected successor") += 1;
            }
        }
    }

    for node_id in selected {
        if !rank.contains_key(node_id) {
            rank.insert(node_id.clone(), next_rank);
            next_rank += 1;
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
        .collect::<Vec<_>>();
    ready.sort_by_key(|node_id| rank[node_id]);
    let mut output = Vec::with_capacity(selected.len());
    while !ready.is_empty() {
        let node_id = ready.remove(0);
        output.push(node_id.clone());
        for successor in edges.get(&node_id).into_iter().flatten() {
            let count = indegree.get_mut(successor).expect("selected successor");
            *count -= 1;
            if *count == 0 {
                ready.push(successor.clone());
            }
        }
        ready.sort_by_key(|candidate| rank[candidate]);
    }

    if output.len() == selected.len() {
        Ok(output)
    } else {
        let mut node_ids = indegree
            .into_iter()
            .filter_map(|(node_id, count)| (count > 0).then_some(node_id))
            .collect::<Vec<_>>();
        node_ids.sort_by_key(|node_id| rank[node_id]);
        Err(SequenceOrderError::IncompatibleOrder { node_ids })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceNode {
    pub node_id: String,
    pub identity: NodeIdentity,
    pub content_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceSide {
    Ours,
    Theirs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceChangeKind {
    Insert,
    Delete,
    Modify,
    Move,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceChange {
    pub side: SequenceSide,
    pub kind: SequenceChangeKind,
    pub base_index: Option<usize>,
    pub side_index: Option<usize>,
    pub node_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceConflictCategory {
    DeleteModify,
    ModifyModify,
    DuplicateInsertion,
    Order,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceConflict {
    pub category: SequenceConflictCategory,
    pub base_index: Option<usize>,
    pub ours_node_ids: Vec<String>,
    pub theirs_node_ids: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceMergeAnalysis {
    pub changes: Vec<SequenceChange>,
    pub conflicts: Vec<SequenceConflict>,
    pub base_alignments: Vec<SequenceBaseAlignment>,
    pub insertion_alignments: Vec<SequenceInsertionAlignment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceBaseAlignment {
    pub base_index: usize,
    pub ours_index: Option<usize>,
    pub theirs_index: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceInsertionAlignment {
    pub ours_index: Option<usize>,
    pub theirs_index: Option<usize>,
}

pub fn analyze_three_way_sequence(
    base: &[SequenceNode],
    ours: &[SequenceNode],
    theirs: &[SequenceNode],
) -> SequenceMergeAnalysis {
    let ours_alignment = align_to_base(base, ours);
    let theirs_alignment = align_to_base(base, theirs);
    let mut changes = Vec::new();
    changes.extend(sequence_changes(SequenceSide::Ours, base, ours, &ours_alignment));
    changes.extend(sequence_changes(SequenceSide::Theirs, base, theirs, &theirs_alignment));

    let mut conflicts = content_conflicts(base, ours, theirs, &ours_alignment, &theirs_alignment);
    let insertion_alignments =
        insertion_alignments(&ours_alignment, &theirs_alignment, ours, theirs);
    conflicts.extend(insertion_conflicts(ours, theirs, &insertion_alignments));
    if matched_base_indices(ours, &ours_alignment)
        != matched_base_indices(theirs, &theirs_alignment)
        && base_order_changed(base, ours, &ours_alignment)
        && base_order_changed(base, theirs, &theirs_alignment)
    {
        conflicts.push(SequenceConflict {
            category: SequenceConflictCategory::Order,
            base_index: None,
            ours_node_ids: matched_base_order(ours, &ours_alignment)
                .into_iter()
                .map(|(_, index)| ours[index].node_id.clone())
                .collect(),
            theirs_node_ids: matched_base_order(theirs, &theirs_alignment)
                .into_iter()
                .map(|(_, index)| theirs[index].node_id.clone())
                .collect(),
            message: "both sides reorder base nodes incompatibly".to_string(),
        });
    }

    let base_alignments = (0..base.len())
        .map(|base_index| SequenceBaseAlignment {
            base_index,
            ours_index: ours_alignment.base_to_side.get(&base_index).copied(),
            theirs_index: theirs_alignment.base_to_side.get(&base_index).copied(),
        })
        .collect();

    SequenceMergeAnalysis { changes, conflicts, base_alignments, insertion_alignments }
}

#[derive(Clone, Debug)]
struct BaseAlignment {
    base_to_side: HashMap<usize, usize>,
    side_to_base: HashMap<usize, usize>,
    inserted_side_indices: Vec<usize>,
}

fn align_to_base(base: &[SequenceNode], side: &[SequenceNode]) -> BaseAlignment {
    let base_identities = base.iter().map(|node| node.identity.clone()).collect::<Vec<_>>();
    let side_identities = side.iter().map(|node| node.identity.clone()).collect::<Vec<_>>();
    let result = match_node_identities(&base_identities, &side_identities);
    let base_to_side = result
        .matched
        .iter()
        .map(|entry| (entry.template_index, entry.destination_index))
        .collect::<HashMap<_, _>>();
    let side_to_base = result
        .matched
        .iter()
        .map(|entry| (entry.destination_index, entry.template_index))
        .collect::<HashMap<_, _>>();
    BaseAlignment {
        base_to_side,
        side_to_base,
        inserted_side_indices: result.unmatched_destination,
    }
}

fn sequence_changes(
    side_name: SequenceSide,
    base: &[SequenceNode],
    side: &[SequenceNode],
    alignment: &BaseAlignment,
) -> Vec<SequenceChange> {
    let moved = reordered_base_indices(base, side, alignment);
    let mut changes = Vec::new();
    for (base_index, base_node) in base.iter().enumerate() {
        if let Some(side_index) = alignment.base_to_side.get(&base_index).copied() {
            if side[side_index].content_hash != base_node.content_hash {
                changes.push(SequenceChange {
                    side: side_name,
                    kind: SequenceChangeKind::Modify,
                    base_index: Some(base_index),
                    side_index: Some(side_index),
                    node_id: side[side_index].node_id.clone(),
                });
            }
            if moved.contains(&base_index) {
                changes.push(SequenceChange {
                    side: side_name,
                    kind: SequenceChangeKind::Move,
                    base_index: Some(base_index),
                    side_index: Some(side_index),
                    node_id: side[side_index].node_id.clone(),
                });
            }
        } else {
            changes.push(SequenceChange {
                side: side_name,
                kind: SequenceChangeKind::Delete,
                base_index: Some(base_index),
                side_index: None,
                node_id: base_node.node_id.clone(),
            });
        }
    }
    changes.extend(alignment.inserted_side_indices.iter().map(|side_index| SequenceChange {
        side: side_name,
        kind: SequenceChangeKind::Insert,
        base_index: None,
        side_index: Some(*side_index),
        node_id: side[*side_index].node_id.clone(),
    }));
    changes
}

fn content_conflicts(
    base: &[SequenceNode],
    ours: &[SequenceNode],
    theirs: &[SequenceNode],
    ours_alignment: &BaseAlignment,
    theirs_alignment: &BaseAlignment,
) -> Vec<SequenceConflict> {
    let mut conflicts = Vec::new();
    for (base_index, base_node) in base.iter().enumerate() {
        let ours_index = ours_alignment.base_to_side.get(&base_index).copied();
        let theirs_index = theirs_alignment.base_to_side.get(&base_index).copied();
        match (ours_index, theirs_index) {
            (None, Some(theirs_index))
                if theirs[theirs_index].content_hash != base_node.content_hash =>
            {
                conflicts.push(delete_modify_conflict(
                    base_index,
                    vec![],
                    vec![theirs[theirs_index].node_id.clone()],
                ));
            }
            (Some(ours_index), None) if ours[ours_index].content_hash != base_node.content_hash => {
                conflicts.push(delete_modify_conflict(
                    base_index,
                    vec![ours[ours_index].node_id.clone()],
                    vec![],
                ));
            }
            (Some(ours_index), Some(theirs_index))
                if ours[ours_index].content_hash != base_node.content_hash
                    && theirs[theirs_index].content_hash != base_node.content_hash
                    && ours[ours_index].content_hash != theirs[theirs_index].content_hash =>
            {
                conflicts.push(SequenceConflict {
                    category: SequenceConflictCategory::ModifyModify,
                    base_index: Some(base_index),
                    ours_node_ids: vec![ours[ours_index].node_id.clone()],
                    theirs_node_ids: vec![theirs[theirs_index].node_id.clone()],
                    message: "both sides modify the same node differently".to_string(),
                });
            }
            _ => {}
        }
    }
    conflicts
}

fn delete_modify_conflict(
    base_index: usize,
    ours_node_ids: Vec<String>,
    theirs_node_ids: Vec<String>,
) -> SequenceConflict {
    SequenceConflict {
        category: SequenceConflictCategory::DeleteModify,
        base_index: Some(base_index),
        ours_node_ids,
        theirs_node_ids,
        message: "one side deletes a node modified by the other side".to_string(),
    }
}

fn insertion_conflicts(
    ours: &[SequenceNode],
    theirs: &[SequenceNode],
    alignments: &[SequenceInsertionAlignment],
) -> Vec<SequenceConflict> {
    alignments
        .iter()
        .filter_map(|entry| match (entry.ours_index, entry.theirs_index) {
            (Some(ours_index), Some(theirs_index))
                if ours[ours_index].content_hash != theirs[theirs_index].content_hash =>
            {
                Some(SequenceConflict {
                    category: SequenceConflictCategory::DuplicateInsertion,
                    base_index: None,
                    ours_node_ids: vec![ours[ours_index].node_id.clone()],
                    theirs_node_ids: vec![theirs[theirs_index].node_id.clone()],
                    message: "both sides insert the same identity with different content"
                        .to_string(),
                })
            }
            _ => None,
        })
        .collect()
}

fn insertion_alignments(
    ours_alignment: &BaseAlignment,
    theirs_alignment: &BaseAlignment,
    ours: &[SequenceNode],
    theirs: &[SequenceNode],
) -> Vec<SequenceInsertionAlignment> {
    let ours_insertions = ours_alignment
        .inserted_side_indices
        .iter()
        .map(|index| ours[*index].identity.clone())
        .collect::<Vec<_>>();
    let theirs_insertions = theirs_alignment
        .inserted_side_indices
        .iter()
        .map(|index| theirs[*index].identity.clone())
        .collect::<Vec<_>>();
    let matched = match_node_identities(&ours_insertions, &theirs_insertions);
    let mut alignments = matched
        .matched
        .iter()
        .map(|entry| SequenceInsertionAlignment {
            ours_index: Some(ours_alignment.inserted_side_indices[entry.template_index]),
            theirs_index: Some(theirs_alignment.inserted_side_indices[entry.destination_index]),
        })
        .collect::<Vec<_>>();
    alignments.extend(matched.unmatched_template.iter().map(|index| SequenceInsertionAlignment {
        ours_index: Some(ours_alignment.inserted_side_indices[*index]),
        theirs_index: None,
    }));
    alignments.extend(matched.unmatched_destination.iter().map(|index| {
        SequenceInsertionAlignment {
            ours_index: None,
            theirs_index: Some(theirs_alignment.inserted_side_indices[*index]),
        }
    }));
    alignments.sort_by_key(|entry| {
        (entry.ours_index.unwrap_or(usize::MAX), entry.theirs_index.unwrap_or(usize::MAX))
    });
    alignments
}

fn matched_base_order(side: &[SequenceNode], alignment: &BaseAlignment) -> Vec<(usize, usize)> {
    side.iter()
        .enumerate()
        .filter_map(|(side_index, _)| {
            alignment
                .side_to_base
                .get(&side_index)
                .copied()
                .map(|base_index| (base_index, side_index))
        })
        .collect()
}

fn matched_base_indices(side: &[SequenceNode], alignment: &BaseAlignment) -> Vec<usize> {
    matched_base_order(side, alignment).into_iter().map(|(base_index, _)| base_index).collect()
}

fn reordered_base_indices(
    base: &[SequenceNode],
    side: &[SequenceNode],
    alignment: &BaseAlignment,
) -> HashSet<usize> {
    let actual = matched_base_order(side, alignment);
    let mut expected = actual.clone();
    expected.sort_by_key(|(base_index, _)| *base_index);
    if actual.iter().map(|entry| entry.0).eq(expected.iter().map(|entry| entry.0)) {
        return HashSet::new();
    }

    let actual_neighbors = neighbors(&actual);
    let expected_neighbors = neighbors(&expected);
    base.iter()
        .enumerate()
        .filter_map(|(base_index, _)| {
            (actual_neighbors.get(&base_index) != expected_neighbors.get(&base_index))
                .then_some(base_index)
        })
        .collect()
}

fn neighbors(order: &[(usize, usize)]) -> HashMap<usize, (Option<usize>, Option<usize>)> {
    order
        .iter()
        .enumerate()
        .map(|(index, (base_index, _))| {
            (
                *base_index,
                (
                    index.checked_sub(1).map(|previous| order[previous].0),
                    order.get(index + 1).map(|next| next.0),
                ),
            )
        })
        .collect()
}

fn base_order_changed(
    base: &[SequenceNode],
    side: &[SequenceNode],
    alignment: &BaseAlignment,
) -> bool {
    !reordered_base_indices(base, side, alignment).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeSignature;

    fn signature_node(id: &str, signature: &str, content_hash: &str) -> SequenceNode {
        SequenceNode {
            node_id: id.to_string(),
            identity: NodeIdentity::new(Some(NodeSignature(vec![signature.to_string()]))),
            content_hash: content_hash.to_string(),
        }
    }

    #[test]
    fn classifies_duplicate_occurrences_without_reusing_the_first_node() {
        let base = [signature_node("b1", "same", "one"), signature_node("b2", "same", "two")];
        let ours = [signature_node("o1", "same", "one"), signature_node("o2", "same", "ours")];
        let theirs = [signature_node("t1", "same", "theirs"), signature_node("t2", "same", "two")];
        let result = analyze_three_way_sequence(&base, &ours, &theirs);

        assert!(result.conflicts.is_empty());
        assert!(
            result
                .changes
                .iter()
                .any(|change| change.node_id == "o2" && change.base_index == Some(1))
        );
        assert!(
            result
                .changes
                .iter()
                .any(|change| change.node_id == "t1" && change.base_index == Some(0))
        );
    }

    #[test]
    fn detects_delete_modify_and_modify_modify_conflicts() {
        let base = [signature_node("base", "item", "base")];
        let deleted: [SequenceNode; 0] = [];
        let edited = [signature_node("edited", "item", "edited")];
        let result = analyze_three_way_sequence(&base, &deleted, &edited);
        assert_eq!(result.conflicts[0].category, SequenceConflictCategory::DeleteModify);

        let ours = [signature_node("ours", "item", "ours")];
        let theirs = [signature_node("theirs", "item", "theirs")];
        let result = analyze_three_way_sequence(&base, &ours, &theirs);
        assert_eq!(result.conflicts[0].category, SequenceConflictCategory::ModifyModify);
    }

    #[test]
    fn detects_incompatible_reorders_but_ignores_insertions_between_base_nodes() {
        let base = [
            signature_node("ba", "a", "a"),
            signature_node("bb", "b", "b"),
            signature_node("bc", "c", "c"),
        ];
        let ours = [
            signature_node("ob", "b", "b"),
            signature_node("oa", "a", "a"),
            signature_node("oc", "c", "c"),
        ];
        let theirs = [
            signature_node("ta", "a", "a"),
            signature_node("tc", "c", "c"),
            signature_node("tb", "b", "b"),
        ];
        let result = analyze_three_way_sequence(&base, &ours, &theirs);
        assert!(
            result
                .conflicts
                .iter()
                .any(|conflict| conflict.category == SequenceConflictCategory::Order)
        );

        let ours_with_insertion = [
            signature_node("oa", "a", "a"),
            signature_node("ox", "x", "x"),
            signature_node("ob", "b", "b"),
            signature_node("oc", "c", "c"),
        ];
        let unchanged = analyze_three_way_sequence(&base, &ours_with_insertion, &base);
        assert!(!unchanged.changes.iter().any(|change| change.kind == SequenceChangeKind::Move));
    }

    #[test]
    fn detects_incompatible_duplicate_insertions() {
        let ours = [signature_node("ours", "new", "ours")];
        let theirs = [signature_node("theirs", "new", "theirs")];
        let result = analyze_three_way_sequence(&[], &ours, &theirs);
        assert_eq!(result.conflicts[0].category, SequenceConflictCategory::DuplicateInsertion);
    }

    #[test]
    fn merges_sequence_constraints_with_stable_first_occurrence_order() {
        let result = merge_sequence_order_constraints(
            &[
                vec!["title".to_string(), "last".to_string()],
                vec!["title".to_string(), "added".to_string(), "last".to_string()],
            ],
            &["title".to_string(), "last".to_string(), "added".to_string()],
        )
        .unwrap();

        assert_eq!(result, ["title", "added", "last"]);
    }

    #[test]
    fn rejects_duplicate_and_cyclic_sequence_constraints() {
        let duplicate = merge_sequence_order_constraints(
            &[vec!["title".to_string(), "title".to_string()]],
            &["title".to_string()],
        );
        assert_eq!(
            duplicate,
            Err(SequenceOrderError::DuplicateIdentity {
                sequence_index: 0,
                node_id: "title".to_string(),
            })
        );

        let cycle = merge_sequence_order_constraints(
            &[
                vec!["title".to_string(), "last".to_string()],
                vec!["last".to_string(), "title".to_string()],
            ],
            &["title".to_string(), "last".to_string()],
        );
        assert_eq!(
            cycle,
            Err(SequenceOrderError::IncompatibleOrder {
                node_ids: vec!["title".to_string(), "last".to_string()],
            })
        );
    }
}
