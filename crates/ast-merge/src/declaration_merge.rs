use crate::byte_evidence::{SourceByteSegment, append_source_segment, verify_source_byte_segments};
use std::collections::{HashMap, HashSet};
use tree_haver::ByteRange;

use crate::{
    ConflictAlternative, ConflictAlternativeState, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, MergeConflict, OwnedSourceRegion, SourceRevision, ThreeWayMergeOutcome,
    ThreeWayMergeResult,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePreservingOwner {
    pub id: String,
    pub path: String,
    pub fingerprint: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePreservingOwnerDocument {
    pub source: String,
    pub owners: Vec<SourcePreservingOwner>,
}

impl SourcePreservingOwnerDocument {
    pub(crate) fn validate(&self, role: &str) -> Result<(), String> {
        let mut ids = HashSet::new();
        let mut previous_end = 0;
        for owner in &self.owners {
            if owner.id.is_empty() || !ids.insert(owner.id.as_str()) {
                return Err(format!("{role} has an empty or duplicate owner identity"));
            }
            if owner.start_byte >= owner.end_byte
                || owner.end_byte > self.source.len()
                || !self.source.is_char_boundary(owner.start_byte)
                || !self.source.is_char_boundary(owner.end_byte)
            {
                return Err(format!("{role} owner {} has an invalid byte range", owner.path));
            }
            if owner.start_byte < previous_end {
                return Err(format!("{role} owner {} overlaps a prior owner", owner.path));
            }
            if owner.start_line == 0 || owner.end_line < owner.start_line {
                return Err(format!("{role} owner {} has an invalid line range", owner.path));
            }
            previous_end = owner.end_byte;
        }
        Ok(())
    }

    fn owner_ids(&self) -> Vec<&str> {
        self.owners.iter().map(|owner| owner.id.as_str()).collect()
    }

    fn layout_segments(&self) -> Vec<&str> {
        let mut cursor = 0;
        let mut segments = Vec::with_capacity(self.owners.len() + 1);
        for owner in &self.owners {
            segments.push(&self.source[cursor..owner.start_byte]);
            cursor = owner.end_byte;
        }
        segments.push(&self.source[cursor..]);
        segments
    }
}

pub fn merge_source_preserving_owners(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: impl FnOnce(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> ThreeWayMergeResult<String> {
    merge_source_preserving_owners_with_evidence(base, ours, theirs, verify).result
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePreservingMergeEvidence {
    pub result: ThreeWayMergeResult<String>,
    pub source_segments: Vec<SourceByteSegment>,
}

pub fn merge_source_preserving_owners_with_evidence(
    base: SourcePreservingOwnerDocument,
    ours: SourcePreservingOwnerDocument,
    theirs: SourcePreservingOwnerDocument,
    verify: impl FnOnce(&str) -> Result<SourcePreservingOwnerDocument, String>,
) -> SourcePreservingMergeEvidence {
    let mut source_segments = Vec::new();
    let mut result = merge_owners(&base, &ours, &theirs, verify, &mut source_segments);
    if let Some(output) = &result.output {
        let sources = HashMap::from([
            (SourceRevision::Base, base.source.as_bytes()),
            (SourceRevision::Ours, ours.source.as_bytes()),
            (SourceRevision::Theirs, theirs.source.as_bytes()),
        ]);
        if let Err(error) =
            verify_source_byte_segments(output.as_bytes(), &sources, &source_segments)
        {
            result = error_result(
                DiagnosticCategory::ConfigurationError,
                format!("invalid byte evidence: {error:?}"),
            );
        }
    }
    if result.outcome != ThreeWayMergeOutcome::Clean {
        source_segments.clear();
    }
    SourcePreservingMergeEvidence { result, source_segments }
}

fn merge_owners(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
    verify: impl FnOnce(&str) -> Result<SourcePreservingOwnerDocument, String>,
    segments: &mut Vec<SourceByteSegment>,
) -> ThreeWayMergeResult<String> {
    for (role, document) in [("base", &base), ("ours", &ours), ("theirs", &theirs)] {
        if let Err(message) = document.validate(role) {
            return error_result(DiagnosticCategory::Ambiguity, message);
        }
    }

    if ours.source == theirs.source || base.source == theirs.source {
        let mut output = String::new();
        append_source_segment(
            &mut output,
            segments,
            &ours.source,
            SourceRevision::Ours,
            ByteRange { start_byte: 0, end_byte: ours.source.len() },
            None,
        );
        return clean_result(output);
    }
    if base.source == ours.source {
        let mut output = String::new();
        append_source_segment(
            &mut output,
            segments,
            &theirs.source,
            SourceRevision::Theirs,
            ByteRange { start_byte: 0, end_byte: theirs.source.len() },
            None,
        );
        return clean_result(output);
    }

    let base_by_id = owners_by_id(base);
    let ours_by_id = owners_by_id(ours);
    let theirs_by_id = owners_by_id(theirs);
    let mut selected = HashMap::new();
    let mut conflicts = Vec::new();

    for id in all_owner_ids(base, ours, theirs) {
        let base_owner = base_by_id.get(id.as_str()).copied();
        let ours_owner = ours_by_id.get(id.as_str()).copied();
        let theirs_owner = theirs_by_id.get(id.as_str()).copied();
        match (base_owner, ours_owner, theirs_owner) {
            (Some(base_owner), Some(ours_owner), Some(theirs_owner)) => {
                if ours_owner.fingerprint == theirs_owner.fingerprint
                    || base_owner.fingerprint == theirs_owner.fingerprint
                {
                    selected.insert(id, SourceRevision::Ours);
                } else if base_owner.fingerprint == ours_owner.fingerprint {
                    selected.insert(id, SourceRevision::Theirs);
                } else {
                    conflicts.push(owner_conflict(base_owner, ours_owner, theirs_owner));
                }
            }
            (Some(base_owner), Some(ours_owner), None) => {
                if base_owner.fingerprint != ours_owner.fingerprint {
                    conflicts.push(owner_membership_conflict(
                        Some(base_owner),
                        Some(ours_owner),
                        None,
                    ));
                }
            }
            (Some(base_owner), None, Some(theirs_owner)) => {
                if base_owner.fingerprint != theirs_owner.fingerprint {
                    conflicts.push(owner_membership_conflict(
                        Some(base_owner),
                        None,
                        Some(theirs_owner),
                    ));
                }
            }
            (Some(_), None, None) => {}
            (None, Some(ours_owner), Some(theirs_owner)) => {
                if ours_owner.fingerprint == theirs_owner.fingerprint {
                    selected.insert(id, SourceRevision::Ours);
                } else {
                    conflicts.push(owner_membership_conflict(
                        None,
                        Some(ours_owner),
                        Some(theirs_owner),
                    ));
                }
            }
            (None, Some(_), None) => {
                selected.insert(id, SourceRevision::Ours);
            }
            (None, None, Some(_)) => {
                selected.insert(id, SourceRevision::Theirs);
            }
            (None, None, None) => unreachable!("owner id came from at least one document"),
        }
    }
    if !conflicts.is_empty() {
        return conflict_result(conflicts);
    }

    let selected_ids = selected.keys().cloned().collect::<HashSet<_>>();
    let ours_ids = ours_by_id.keys().map(|id| (*id).to_string()).collect::<HashSet<_>>();
    let theirs_ids = theirs_by_id.keys().map(|id| (*id).to_string()).collect::<HashSet<_>>();
    let (baseline, baseline_revision) = if selected_ids == ours_ids {
        (&ours, SourceRevision::Ours)
    } else if selected_ids == theirs_ids {
        (&theirs, SourceRevision::Theirs)
    } else {
        return error_result(
            DiagnosticCategory::UnsupportedFeature,
            "source-preserving declaration merge cannot prove a combined owner membership layout",
        );
    };

    let stable_ids = base
        .owner_ids()
        .into_iter()
        .filter(|id| ours_by_id.contains_key(*id) && theirs_by_id.contains_key(*id))
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let membership_changed =
        base.owner_ids() != ours.owner_ids() || base.owner_ids() != theirs.owner_ids();
    let layout_matches = if membership_changed && stable_ids.is_empty() {
        false
    } else if membership_changed {
        let base_layout = layout_segments_for_ids(base, &stable_ids);
        let ours_layout = layout_segments_for_ids(ours, &stable_ids);
        let theirs_layout = layout_segments_for_ids(theirs, &stable_ids);
        base_layout == ours_layout && base_layout == theirs_layout
    } else {
        base.layout_segments() == ours.layout_segments()
            && base.layout_segments() == theirs.layout_segments()
    };
    if !layout_matches {
        return error_result(
            DiagnosticCategory::UnsupportedFeature,
            "source-preserving declaration merge cannot prove ownership of changed inter-owner layout",
        );
    }

    let mut output = String::new();
    let mut cursor = 0;
    let mut expected = HashMap::new();
    for baseline_owner in &baseline.owners {
        let revision = selected[baseline_owner.id.as_str()];
        let selected_owner = owner_for_revision(
            baseline_owner.id.as_str(),
            revision,
            &base_by_id,
            &ours_by_id,
            &theirs_by_id,
        );
        expected.insert(baseline_owner.id.as_str(), selected_owner.fingerprint.as_str());
        append_source_segment(
            &mut output,
            segments,
            &baseline.source,
            baseline_revision,
            ByteRange { start_byte: cursor, end_byte: baseline_owner.start_byte },
            None,
        );
        let selected_source = match revision {
            SourceRevision::Base => &base.source,
            SourceRevision::Ours => &ours.source,
            SourceRevision::Theirs => &theirs.source,
        };
        append_source_segment(
            &mut output,
            segments,
            selected_source,
            revision,
            ByteRange { start_byte: selected_owner.start_byte, end_byte: selected_owner.end_byte },
            Some(selected_owner.id.clone()),
        );
        cursor = baseline_owner.end_byte;
    }

    append_source_segment(
        &mut output,
        segments,
        &baseline.source,
        baseline_revision,
        ByteRange { start_byte: cursor, end_byte: baseline.source.len() },
        None,
    );

    let rendered = match verify(&output) {
        Ok(document) => document,
        Err(message) => {
            return error_result(
                DiagnosticCategory::ConfigurationError,
                format!("source-preserving declaration render did not reparse: {message}"),
            );
        }
    };
    if let Err(message) = rendered.validate("output") {
        return error_result(DiagnosticCategory::ConfigurationError, message);
    }
    if rendered.owner_ids() != baseline.owner_ids()
        || rendered
            .owners
            .iter()
            .any(|owner| expected.get(owner.id.as_str()) != Some(&owner.fingerprint.as_str()))
    {
        return error_result(
            DiagnosticCategory::ConfigurationError,
            "source-preserving declaration render changed the planned owner structure",
        );
    }
    if rendered.layout_segments() != baseline.layout_segments() {
        return error_result(
            DiagnosticCategory::ConfigurationError,
            "source-preserving declaration render changed inter-owner layout",
        );
    }

    clean_result(output)
}

fn owners_by_id(document: &SourcePreservingOwnerDocument) -> HashMap<&str, &SourcePreservingOwner> {
    document.owners.iter().map(|owner| (owner.id.as_str(), owner)).collect()
}

fn all_owner_ids(
    base: &SourcePreservingOwnerDocument,
    ours: &SourcePreservingOwnerDocument,
    theirs: &SourcePreservingOwnerDocument,
) -> Vec<String> {
    let mut ids = Vec::new();
    for owner in base.owners.iter().chain(ours.owners.iter()).chain(theirs.owners.iter()) {
        if !ids.iter().any(|id| id == &owner.id) {
            ids.push(owner.id.clone());
        }
    }
    ids
}

fn owner_for_revision<'a>(
    id: &str,
    revision: SourceRevision,
    base: &HashMap<&'a str, &'a SourcePreservingOwner>,
    ours: &HashMap<&'a str, &'a SourcePreservingOwner>,
    theirs: &HashMap<&'a str, &'a SourcePreservingOwner>,
) -> &'a SourcePreservingOwner {
    match revision {
        SourceRevision::Base => base[id],
        SourceRevision::Ours => ours[id],
        SourceRevision::Theirs => theirs[id],
    }
}

fn layout_segments_for_ids(
    document: &SourcePreservingOwnerDocument,
    ids: &HashSet<String>,
) -> Vec<String> {
    let mut cursor = 0;
    let mut segments = Vec::new();
    for owner in &document.owners {
        if ids.contains(&owner.id) {
            segments.push(document.source[cursor..owner.start_byte].to_string());
            cursor = owner.end_byte;
        } else {
            cursor = owner.end_byte;
        }
    }
    segments.push(document.source[cursor..].to_string());
    segments
}

fn owner_conflict(
    base: &SourcePreservingOwner,
    ours: &SourcePreservingOwner,
    theirs: &SourcePreservingOwner,
) -> MergeConflict {
    owner_membership_conflict(Some(base), Some(ours), Some(theirs))
}

fn owner_membership_conflict(
    base: Option<&SourcePreservingOwner>,
    ours: Option<&SourcePreservingOwner>,
    theirs: Option<&SourcePreservingOwner>,
) -> MergeConflict {
    let owner = ours.or(theirs).or(base).expect("a conflict has an owner");
    MergeConflict {
        conflict_id: format!("declaration:{}", owner.id),
        category: "edit_edit".to_string(),
        path: owner.path.clone(),
        fallback_scope: "owner".to_string(),
        message: format!("{} changed incompatibly on both sides", owner.path),
        alternatives: vec![
            owner_alternative_optional(SourceRevision::Base, base),
            owner_alternative_optional(SourceRevision::Ours, ours),
            owner_alternative_optional(SourceRevision::Theirs, theirs),
        ],
    }
}

fn owner_alternative_optional(
    revision: SourceRevision,
    owner: Option<&SourcePreservingOwner>,
) -> ConflictAlternative {
    ConflictAlternative {
        revision,
        state: owner
            .map(|_| ConflictAlternativeState::Present)
            .unwrap_or(ConflictAlternativeState::Absent),
        regions: owner
            .map(|owner| {
                vec![OwnedSourceRegion {
                    node_id: owner.id.clone(),
                    region_kind: "declaration".to_string(),
                    start_byte: owner.start_byte,
                    end_byte: owner.end_byte,
                    start_line: owner.start_line,
                    end_line: owner.end_line,
                }]
            })
            .unwrap_or_default(),
    }
}

fn diagnostic(category: DiagnosticCategory, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category,
        message: message.into(),
        path: None,
        review: None,
    }
}

fn clean_result(output: String) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Clean,
        diagnostics: vec![],
        conflicts: vec![],
        output: Some(output),
        policies: vec![],
    }
}

fn conflict_result(conflicts: Vec<MergeConflict>) -> ThreeWayMergeResult<String> {
    let diagnostics = conflicts
        .iter()
        .map(|conflict| {
            let mut entry = diagnostic(DiagnosticCategory::MergeConflict, &conflict.message);
            entry.path = Some(conflict.path.clone());
            entry
        })
        .collect();
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Conflict,
        diagnostics,
        conflicts,
        output: None,
        policies: vec![],
    }
}

fn error_result(
    category: DiagnosticCategory,
    message: impl Into<String>,
) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Error,
        diagnostics: vec![diagnostic(category, message)],
        conflicts: vec![],
        output: None,
        policies: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(
        id: &str,
        fingerprint: &str,
        start: usize,
        end: usize,
        line: usize,
    ) -> SourcePreservingOwner {
        SourcePreservingOwner {
            id: id.to_string(),
            path: format!("/function:{id}"),
            fingerprint: fingerprint.to_string(),
            start_byte: start,
            end_byte: end,
            start_line: line,
            end_line: line,
        }
    }

    fn document(source: &str, left: &str, right: &str) -> SourcePreservingOwnerDocument {
        SourcePreservingOwnerDocument {
            source: source.to_string(),
            owners: vec![owner("left", left, 0, 3, 1), owner("right", right, 5, 8, 3)],
        }
    }

    #[test]
    fn merges_independent_owner_edits_without_changing_layout() {
        let base = document("one\n\ntwo\n", "one", "two");
        let ours = document("ONE\n\ntwo\n", "ONE", "two");
        let theirs = document("one\n\nTWO\n", "one", "TWO");

        let result = merge_source_preserving_owners(base, ours, theirs, |source| {
            Ok(document(source, "ONE", "TWO"))
        });

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some("ONE\n\nTWO\n"));
    }

    #[test]
    fn records_exact_owner_and_gap_origins_and_discards_failed_render_evidence() {
        let run = |reject: bool| {
            merge_source_preserving_owners_with_evidence(
                document("one\n\ntwo\n", "one", "two"),
                document("ONE\n\ntwo\n", "ONE", "two"),
                document("one\n\nTWO\n", "one", "TWO"),
                |source| {
                    if reject { Err("rejected".into()) } else { Ok(document(source, "ONE", "TWO")) }
                },
            )
        };
        let clean = run(false);
        assert_eq!(clean.result.output.as_deref(), Some("ONE\n\nTWO\n"));
        assert_eq!(clean.source_segments.len(), 4);
        assert_eq!(clean.source_segments[0].owner_id.as_deref(), Some("left"));
        assert_eq!(clean.source_segments[1].owner_id, None);
        assert_eq!(clean.source_segments[2].revision, SourceRevision::Theirs);
        assert_eq!(clean.source_segments[2].output_range, ByteRange { start_byte: 5, end_byte: 8 });
        let failed = run(true);
        assert_eq!(failed.result.outcome, ThreeWayMergeOutcome::Error);
        assert!(failed.source_segments.is_empty());
        assert!(failed.result.output.is_none());
    }

    #[test]
    fn whole_source_selection_and_empty_outputs_have_complete_partitions() {
        for text in ["", "\u{feff}é\r\n"] {
            let doc = || SourcePreservingOwnerDocument { source: text.into(), owners: vec![] };
            let result = merge_source_preserving_owners_with_evidence(doc(), doc(), doc(), |_| {
                panic!("already parsed source")
            });
            assert_eq!(result.result.output.as_deref(), Some(text));
            assert_eq!(result.source_segments.len(), usize::from(!text.is_empty()));
            if !text.is_empty() {
                assert_eq!(result.source_segments[0].revision, SourceRevision::Ours);
                assert_eq!(result.source_segments[0].output_range.end_byte, text.len());
            }
        }
    }

    #[test]
    fn merges_an_independent_edit_with_a_one_sided_owner_deletion() {
        let base = document("one\n\ntwo\n", "one", "two");
        let ours = document("ONE\n\ntwo\n", "ONE", "two");
        let theirs = SourcePreservingOwnerDocument {
            source: "one\n".to_string(),
            owners: vec![owner("left", "one", 0, 3, 1)],
        };

        let result = merge_source_preserving_owners(base, ours, theirs, |source| {
            Ok(SourcePreservingOwnerDocument {
                source: source.to_string(),
                owners: vec![owner("left", "ONE", 0, 3, 1)],
            })
        });

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some("ONE\n"));
    }

    #[test]
    fn merges_an_independent_edit_with_a_one_sided_owner_addition() {
        let base = SourcePreservingOwnerDocument {
            source: "one\n".to_string(),
            owners: vec![owner("left", "one", 0, 3, 1)],
        };
        let ours = SourcePreservingOwnerDocument {
            source: "ONE\n".to_string(),
            owners: vec![owner("left", "ONE", 0, 3, 1)],
        };
        let theirs = SourcePreservingOwnerDocument {
            source: "one\n\nthree\n".to_string(),
            owners: vec![owner("left", "one", 0, 3, 1), owner("third", "three", 5, 10, 3)],
        };

        let result = merge_source_preserving_owners(base, ours, theirs, |source| {
            Ok(SourcePreservingOwnerDocument {
                source: source.to_string(),
                owners: vec![owner("left", "ONE", 0, 3, 1), owner("third", "three", 5, 10, 3)],
            })
        });

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some("ONE\n\nthree\n"));
    }

    #[test]
    fn rejects_changed_inter_owner_layout() {
        let base = document("one\n\ntwo\n", "one", "two");
        let ours = document("ONE\n\ntwo\n", "ONE", "two");
        let theirs = document("one\n\n\nTWO\n", "one", "TWO");

        let result = merge_source_preserving_owners(base, ours, theirs, |_| unreachable!());

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::UnsupportedFeature);
    }

    #[test]
    fn reports_incompatible_owner_edits_as_a_local_conflict() {
        let base = document("one\n\ntwo\n", "one", "two");
        let ours = document("ONE\n\ntwo\n", "ONE", "two");
        let theirs = document("TWO\n\ntwo\n", "TWO", "two");

        let result = merge_source_preserving_owners(base, ours, theirs, |_| unreachable!());

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(result.conflicts[0].path, "/function:left");
        assert_eq!(result.conflicts[0].fallback_scope, "owner");
    }

    #[test]
    fn rejects_owner_membership_changes() {
        let base = document("one\n\ntwo\n", "one", "two");
        let ours = SourcePreservingOwnerDocument {
            source: "ONE\n\ntwo\n\nfour\n".to_string(),
            owners: vec![
                owner("left", "ONE", 0, 3, 1),
                owner("right", "two", 5, 8, 3),
                owner("fourth", "four", 10, 14, 5),
            ],
        };
        let mut theirs = document("one\n\nTWO\n", "one", "TWO");
        theirs.owners.pop();
        theirs.source = "one\n\nthree\n".to_string();
        theirs.owners.push(owner("third", "three", 5, 10, 3));

        let result = merge_source_preserving_owners(base, ours, theirs, |_| unreachable!());

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::UnsupportedFeature);
    }
}
