use std::collections::{HashMap, HashSet};

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
    fn validate(&self, role: &str) -> Result<(), String> {
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
    for (role, document) in [("base", &base), ("ours", &ours), ("theirs", &theirs)] {
        if let Err(message) = document.validate(role) {
            return error_result(DiagnosticCategory::Ambiguity, message);
        }
    }

    if ours.source == theirs.source || base.source == theirs.source {
        return clean_result(ours.source);
    }
    if base.source == ours.source {
        return clean_result(theirs.source);
    }

    if base.owner_ids() != ours.owner_ids() || base.owner_ids() != theirs.owner_ids() {
        return error_result(
            DiagnosticCategory::UnsupportedFeature,
            "source-preserving declaration merge requires identical ordered owner identities",
        );
    }
    if base.layout_segments() != ours.layout_segments()
        || base.layout_segments() != theirs.layout_segments()
    {
        return error_result(
            DiagnosticCategory::UnsupportedFeature,
            "source-preserving declaration merge cannot prove ownership of changed inter-owner layout",
        );
    }

    let base_by_id = owners_by_id(&base);
    let theirs_by_id = owners_by_id(&theirs);
    let mut replacements = Vec::new();
    let mut conflicts = Vec::new();
    let mut expected = HashMap::new();
    for ours_owner in &ours.owners {
        let base_owner = base_by_id[ours_owner.id.as_str()];
        let theirs_owner = theirs_by_id[ours_owner.id.as_str()];
        let selected = if ours_owner.fingerprint == theirs_owner.fingerprint
            || base_owner.fingerprint == theirs_owner.fingerprint
        {
            ours_owner
        } else if base_owner.fingerprint == ours_owner.fingerprint {
            replacements.push((ours_owner, theirs_owner));
            theirs_owner
        } else {
            conflicts.push(owner_conflict(base_owner, ours_owner, theirs_owner));
            ours_owner
        };
        expected.insert(selected.id.as_str(), selected.fingerprint.as_str());
    }
    if !conflicts.is_empty() {
        return conflict_result(conflicts);
    }

    replacements.sort_by_key(|(owner, _)| std::cmp::Reverse(owner.start_byte));
    let mut output = ours.source.clone();
    for (ours_owner, theirs_owner) in replacements {
        output.replace_range(
            ours_owner.start_byte..ours_owner.end_byte,
            &theirs.source[theirs_owner.start_byte..theirs_owner.end_byte],
        );
    }

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
    if rendered.owner_ids() != ours.owner_ids()
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
    if rendered.layout_segments() != ours.layout_segments() {
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

fn owner_conflict(
    base: &SourcePreservingOwner,
    ours: &SourcePreservingOwner,
    theirs: &SourcePreservingOwner,
) -> MergeConflict {
    MergeConflict {
        conflict_id: format!("declaration:{}", ours.id),
        category: "edit_edit".to_string(),
        path: ours.path.clone(),
        fallback_scope: "owner".to_string(),
        message: format!("{} changed incompatibly on both sides", ours.path),
        alternatives: vec![
            owner_alternative(SourceRevision::Base, base),
            owner_alternative(SourceRevision::Ours, ours),
            owner_alternative(SourceRevision::Theirs, theirs),
        ],
    }
}

fn owner_alternative(
    revision: SourceRevision,
    owner: &SourcePreservingOwner,
) -> ConflictAlternative {
    ConflictAlternative {
        revision,
        state: ConflictAlternativeState::Present,
        regions: vec![OwnedSourceRegion {
            node_id: owner.id.clone(),
            region_kind: "declaration".to_string(),
            start_byte: owner.start_byte,
            end_byte: owner.end_byte,
            start_line: owner.start_line,
            end_line: owner.end_line,
        }],
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
        let ours = document("ONE\n\ntwo\n", "ONE", "two");
        let mut theirs = document("one\n\nTWO\n", "one", "TWO");
        theirs.owners.pop();

        let result = merge_source_preserving_owners(base, ours, theirs, |_| unreachable!());

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(result.diagnostics[0].category, DiagnosticCategory::UnsupportedFeature);
    }
}
