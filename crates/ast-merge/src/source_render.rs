use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRevision {
    Base,
    Ours,
    Theirs,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAlternativeState {
    Present,
    Absent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnedSourceRegion {
    pub node_id: String,
    pub region_kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

impl OwnedSourceRegion {
    pub fn validate(&self) -> Result<(), SourceRenderError> {
        if self.start_byte > self.end_byte {
            return Err(SourceRenderError::new("owned source byte range is reversed"));
        }
        if self.start_line == 0 || self.end_line < self.start_line {
            return Err(SourceRenderError::new("owned source line range is invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictAlternative {
    pub revision: SourceRevision,
    pub state: ConflictAlternativeState,
    #[serde(default)]
    pub regions: Vec<OwnedSourceRegion>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceFragment {
    pub revision: SourceRevision,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SynthesizedFragment {
    pub content: String,
    pub reason: String,
    pub producer: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConflictSideFragment {
    Source(SourceFragment),
    Synthesized(SynthesizedFragment),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictLabels {
    pub base: String,
    pub ours: String,
    pub theirs: String,
}

impl Default for ConflictLabels {
    fn default() -> Self {
        Self { base: "base".to_string(), ours: "ours".to_string(), theirs: "theirs".to_string() }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictFragment {
    pub conflict_id: String,
    #[serde(default)]
    pub base: Vec<ConflictSideFragment>,
    #[serde(default)]
    pub ours: Vec<ConflictSideFragment>,
    #[serde(default)]
    pub theirs: Vec<ConflictSideFragment>,
    #[serde(default)]
    pub labels: ConflictLabels,
    #[serde(default = "default_conflict_marker_size")]
    pub marker_size: usize,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_conflict_marker_size() -> usize {
    7
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RenderFragment {
    Source(SourceFragment),
    Synthesized(SynthesizedFragment),
    Conflict(ConflictFragment),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRenderPlan {
    pub sources: HashMap<SourceRevision, String>,
    pub fragments: Vec<RenderFragment>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderFragmentKind {
    Source,
    Synthesized,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RenderLineRecord {
    pub output_line: usize,
    pub fragment_kind: RenderFragmentKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<SourceRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesized_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_side: Option<SourceRevision>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SynthesizedFragmentRecord {
    pub reason: String,
    pub producer: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_side: Option<SourceRevision>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RenderedConflictRecord {
    pub conflict_id: String,
    pub output_start_line: usize,
    pub output_end_line: usize,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceFragmentDigest {
    pub revision: SourceRevision,
    pub start_line: usize,
    pub end_line: usize,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceRenderVerification {
    pub content_sha256: String,
    pub source_fragments: Vec<SourceFragmentDigest>,
    pub synthesized_fragment_count: usize,
    pub conflict_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceRenderResult {
    pub content: String,
    pub line_records: Vec<RenderLineRecord>,
    pub synthesized_fragments: Vec<SynthesizedFragmentRecord>,
    pub conflicts: Vec<RenderedConflictRecord>,
    pub verification_input: SourceRenderVerification,
}

impl SourceRenderResult {
    pub fn conflicted(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

impl SourceRenderPlan {
    pub fn new(
        sources: HashMap<SourceRevision, String>,
        fragments: Vec<RenderFragment>,
    ) -> Result<Self, SourceRenderError> {
        let plan = Self { sources, fragments, metadata: HashMap::new() };
        plan.validate()?;
        Ok(plan)
    }

    fn source_content(&self, fragment: &SourceFragment) -> Result<String, SourceRenderError> {
        let source = self.sources.get(&fragment.revision).ok_or_else(|| {
            SourceRenderError::new(format!(
                "source revision {:?} is not present in the render plan",
                fragment.revision
            ))
        })?;
        let lines = source_lines(source);
        if fragment.start_line == 0 || fragment.end_line < fragment.start_line {
            return Err(SourceRenderError::new(format!(
                "invalid source line range {}..{}",
                fragment.start_line, fragment.end_line
            )));
        }
        if fragment.end_line > lines.len() {
            return Err(SourceRenderError::new(format!(
                "{:?} line {} exceeds source line count {}",
                fragment.revision,
                fragment.end_line,
                lines.len()
            )));
        }
        Ok(lines[(fragment.start_line - 1)..fragment.end_line].concat())
    }

    fn validate(&self) -> Result<(), SourceRenderError> {
        for (index, fragment) in self.fragments.iter().enumerate() {
            match fragment {
                RenderFragment::Source(source) => {
                    let content = self.source_content(source)?;
                    let explicit_boundary = matches!(self.fragments.get(index + 1),
                        Some(RenderFragment::Synthesized(next))
                            if next.content == "\n" && next.reason == "conflict_line_boundary");
                    if index + 1 < self.fragments.len()
                        && !content.ends_with('\n')
                        && !explicit_boundary
                    {
                        return Err(SourceRenderError::new(
                            "every non-final fragment must end at a line boundary",
                        ));
                    }
                }
                RenderFragment::Synthesized(fragment) => {
                    if index + 1 < self.fragments.len() && !fragment.content.ends_with('\n') {
                        return Err(SourceRenderError::new(
                            "every non-final fragment must end at a line boundary",
                        ));
                    }
                }
                RenderFragment::Conflict(conflict) => {
                    if conflict.conflict_id.is_empty() {
                        return Err(SourceRenderError::new("conflict_id must not be empty"));
                    }
                    if conflict.marker_size == 0 {
                        return Err(SourceRenderError::new("marker_size must be at least one"));
                    }
                    for side in [&conflict.base, &conflict.ours, &conflict.theirs] {
                        let mut side_content = String::new();
                        for child in side {
                            let content = match child {
                                ConflictSideFragment::Source(source) => {
                                    self.source_content(source)?
                                }
                                ConflictSideFragment::Synthesized(fragment) => {
                                    fragment.content.clone()
                                }
                            };
                            side_content.push_str(&content);
                        }
                        if !side_content.is_empty() && !side_content.ends_with('\n') {
                            return Err(SourceRenderError::new(
                                "every conflict side must end at a line boundary",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn render_source_plan(
    plan: &SourceRenderPlan,
) -> Result<SourceRenderResult, SourceRenderError> {
    plan.validate()?;
    let mut renderer = SourcePlanRenderer::new(plan);
    for fragment in &plan.fragments {
        renderer.render_fragment(fragment)?;
    }
    Ok(renderer.finish())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceLineSelection {
    pub selection_key: String,
    pub revision: SourceRevision,
}

/// Select source regions in baseline line order, taking structural and attached
/// comment regions from their selected owner revision and each layout gap from
/// its controller owner revision. Root and unmanaged regions stay with baseline.
pub fn source_line_owner_addition_range(
    source: &str,
    dispositions: &[crate::SourceLineDisposition],
    owner_id: &str,
) -> Result<std::ops::Range<usize>, SourceRenderError> {
    let lines = if source.is_empty() {
        Vec::new()
    } else {
        source.split_inclusive("\n").collect::<Vec<_>>()
    };
    if lines.len() != dispositions.len() {
        return Err(SourceRenderError::new("source line facts do not partition the source"));
    }
    let mut selected_lines = std::collections::HashSet::new();
    let mut has_structural_owner = false;
    for (index, disposition) in dispositions.iter().enumerate() {
        if disposition.line_number != index + 1 || disposition.line != lines[index] {
            return Err(SourceRenderError::new(
                "source line facts do not match exact source lines",
            ));
        }
        let selected = match disposition.kind.as_str() {
            "structural_owner" | "comment_owner" => {
                let selected = disposition.owner_id.as_deref() == Some(owner_id);
                has_structural_owner |= selected && disposition.kind == "structural_owner";
                selected
            }
            "layout_gap" => disposition.controller_owner_id.as_deref() == Some(owner_id),
            _ => false,
        };
        if selected {
            selected_lines.insert(disposition.line_number);
        }
    }
    if !has_structural_owner || selected_lines.is_empty() {
        return Err(SourceRenderError::new(format!(
            "owner {owner_id} has no source lines to insert"
        )));
    }
    let start_line = *selected_lines.iter().min().unwrap();
    let end_line = *selected_lines.iter().max().unwrap();
    if (start_line..=end_line).any(|line| !selected_lines.contains(&line)) {
        return Err(SourceRenderError::new(format!(
            "owner {owner_id} insertion lines are not contiguous"
        )));
    }
    let start_byte = lines[..start_line - 1].iter().map(|line| line.len()).sum();
    let end_byte =
        start_byte + lines[start_line - 1..end_line].iter().map(|line| line.len()).sum::<usize>();
    Ok(start_byte..end_byte)
}

pub fn source_line_selections_for_owner_revisions(
    baseline_revision: SourceRevision,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    owner_revisions: &HashMap<String, SourceRevision>,
) -> Result<Vec<SourceLineSelection>, SourceRenderError> {
    source_line_selections_for_owner_revisions_with_deletions(
        baseline_revision,
        dispositions,
        owner_revisions,
        &HashSet::new(),
    )
}

pub fn source_line_selections_for_owner_revisions_with_deletions(
    baseline_revision: SourceRevision,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    owner_revisions: &HashMap<String, SourceRevision>,
    deleted_owner_ids: &HashSet<String>,
) -> Result<Vec<SourceLineSelection>, SourceRenderError> {
    let baseline = dispositions.get(&baseline_revision).ok_or_else(|| {
        SourceRenderError::new(format!("baseline dispositions omit {baseline_revision:?}"))
    })?;
    let available = dispositions
        .iter()
        .map(|(revision, records)| {
            (
                *revision,
                records
                    .iter()
                    .map(|record| record.selection_key.as_str())
                    .collect::<std::collections::HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut seen = std::collections::HashSet::new();
    let mut selections = Vec::new();
    for record in baseline {
        if !seen.insert(record.selection_key.as_str()) {
            continue;
        }
        let revision = match record.kind.as_str() {
            "structural_owner" | "comment_owner" => {
                let owner_id = record.owner_id.as_deref().ok_or_else(|| {
                    SourceRenderError::new(format!(
                        "{} line {} has no owner",
                        record.kind, record.line_number
                    ))
                })?;
                if deleted_owner_ids.contains(owner_id) {
                    None
                } else {
                    Some(*owner_revisions.get(owner_id).ok_or_else(|| {
                        SourceRenderError::new(format!(
                            "owner {owner_id} has no selected source revision"
                        ))
                    })?)
                }
            }
            "layout_gap" => Some(
                record
                    .controller_owner_id
                    .as_ref()
                    .and_then(|owner_id| owner_revisions.get(owner_id))
                    .copied()
                    .unwrap_or(baseline_revision),
            ),
            "root_region" | "unmanaged" => Some(baseline_revision),
            kind => {
                return Err(SourceRenderError::new(format!(
                    "line {} has unsupported disposition kind {kind}",
                    record.line_number
                )));
            }
        };
        let Some(revision) = revision else {
            continue;
        };
        if available.get(&revision).is_some_and(|keys| keys.contains(record.selection_key.as_str()))
        {
            selections.push(SourceLineSelection {
                selection_key: record.selection_key.clone(),
                revision,
            });
        } else if matches!(record.kind.as_str(), "structural_owner" | "unmanaged") {
            return Err(SourceRenderError::new(format!(
                "selected {kind} region {} is absent from {revision:?}",
                record.selection_key,
                kind = record.kind
            )));
        }
    }
    // Owners absent from the baseline are additions. Append their complete
    // comment and structural regions in the selected source order. Include each
    // gap controlled by an added owner, whether it precedes that owner or follows
    // it under the shared attachment policy.
    let baseline_owner_ids = baseline
        .iter()
        .filter(|record| record.kind == "structural_owner")
        .filter_map(|record| record.owner_id.as_deref())
        .collect::<std::collections::HashSet<_>>();
    let additions = owner_revisions
        .iter()
        .filter(|(owner_id, revision)| {
            **revision != baseline_revision && !baseline_owner_ids.contains(owner_id.as_str())
        })
        .map(|(owner_id, revision)| (owner_id.as_str(), *revision))
        .collect::<HashMap<_, _>>();
    let mut selected_keys = selections
        .iter()
        .map(|selection| selection.selection_key.clone())
        .collect::<std::collections::HashSet<_>>();
    for revision in [SourceRevision::Base, SourceRevision::Ours, SourceRevision::Theirs] {
        let Some(records) = dispositions.get(&revision) else {
            continue;
        };
        for record in records {
            let selected_owner = match record.kind.as_str() {
                "structural_owner" | "comment_owner" => {
                    record.owner_id.as_deref().and_then(|owner_id| {
                        additions.get(owner_id).map(|revision| (owner_id, *revision))
                    })
                }
                "layout_gap" => record.controller_owner_id.as_deref().and_then(|owner_id| {
                    additions.get(owner_id).map(|revision| (owner_id, *revision))
                }),
                _ => None,
            };
            if let Some((_owner_id, selected_revision)) = selected_owner
                && selected_revision == revision
                && selected_keys.insert(record.selection_key.clone())
            {
                selections.push(SourceLineSelection {
                    selection_key: record.selection_key.clone(),
                    revision,
                });
            }
        }
    }
    Ok(selections)
}

/// Select owner revisions, then place their additions at the source gaps
/// anchored by retained base owners. This explicit merge3 policy leaves
/// directional callers on the established selector.
pub fn source_line_selections_for_owner_revisions_preserving_addition_gaps(
    baseline_revision: SourceRevision,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    owner_revisions: &HashMap<String, SourceRevision>,
    deleted_owner_ids: &HashSet<String>,
) -> Result<Vec<SourceLineSelection>, SourceRenderError> {
    let selections = source_line_selections_for_owner_revisions_with_deletions(
        baseline_revision,
        dispositions,
        owner_revisions,
        deleted_owner_ids,
    )?;
    position_additions_in_their_source_gaps(
        baseline_revision,
        dispositions,
        owner_revisions,
        deleted_owner_ids,
        selections,
    )
}

/// Compose selected additions inside a bounded base gap. For a shared gap,
/// retain ours-before-theirs owner order, the earlier owner's trailing gap, and
/// the last owner's trailing gap together with each owner's comments.
fn position_additions_in_their_source_gaps(
    baseline_revision: SourceRevision,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    owner_revisions: &HashMap<String, SourceRevision>,
    deleted_owner_ids: &HashSet<String>,
    mut selections: Vec<SourceLineSelection>,
) -> Result<Vec<SourceLineSelection>, SourceRenderError> {
    let base = dispositions.get(&SourceRevision::Base).ok_or_else(|| {
        SourceRenderError::new("base dispositions are required to anchor additions")
    })?;
    let base_owner_ids = base
        .iter()
        .filter(|record| record.kind == "structural_owner")
        .filter_map(|record| record.owner_id.clone())
        .collect::<HashSet<_>>();
    let selected_base_owner_ids = base_owner_ids
        .iter()
        .filter(|owner_id| {
            owner_revisions.contains_key(*owner_id) && !deleted_owner_ids.contains(*owner_id)
        })
        .cloned()
        .collect::<HashSet<_>>();
    let mut additions_by_gap =
        HashMap::<(String, String), HashMap<SourceRevision, Vec<String>>>::new();

    for (owner_id, revision) in owner_revisions {
        if base_owner_ids.contains(owner_id) || deleted_owner_ids.contains(owner_id) {
            continue;
        }
        let Some(records) = dispositions.get(revision) else {
            continue;
        };
        let Some(owner_index) = records.iter().position(|record| {
            record.kind == "structural_owner" && record.owner_id.as_deref() == Some(owner_id)
        }) else {
            continue;
        };
        let previous = records[..owner_index]
            .iter()
            .rev()
            .find(|record| {
                record.kind == "structural_owner"
                    && record
                        .owner_id
                        .as_deref()
                        .is_some_and(|id| selected_base_owner_ids.contains(id))
            })
            .and_then(|record| record.owner_id.clone());
        let following = records[owner_index + 1..]
            .iter()
            .find(|record| {
                record.kind == "structural_owner"
                    && record
                        .owner_id
                        .as_deref()
                        .is_some_and(|id| selected_base_owner_ids.contains(id))
            })
            .and_then(|record| record.owner_id.clone());
        let (Some(previous), Some(following)) = (previous, following) else {
            continue;
        };
        additions_by_gap
            .entry((previous, following))
            .or_default()
            .entry(*revision)
            .or_default()
            .push(owner_id.clone());
    }

    let owner_bounds = |records: &[crate::SourceLineDisposition], owner_id: &str| {
        let mut matching =
            records.iter().filter(|record| record.owner_id.as_deref() == Some(owner_id));
        let first = matching.next()?.line_number;
        let (start, end) = matching.fold((first, first), |(start, end), record| {
            (start.min(record.line_number), end.max(record.line_number))
        });
        Some((start, end))
    };
    let mut positioned = Vec::<(String, SourceRevision, Vec<SourceLineSelection>)>::new();
    let mut replaced_keys = HashSet::<(SourceRevision, String)>::new();

    for ((previous, following), revisions) in additions_by_gap {
        let mut additions = revisions
            .into_iter()
            .flat_map(|(revision, owner_ids)| {
                let records = dispositions.get(&revision);
                owner_ids.into_iter().filter_map(move |owner_id| {
                    let (start, end) = owner_bounds(records?, &owner_id)?;
                    Some((revision, start, end, owner_id))
                })
            })
            .collect::<Vec<_>>();
        additions.sort_by_key(|(revision, start, _, _)| {
            let revision_order = match revision {
                SourceRevision::Ours => 0,
                SourceRevision::Theirs => 1,
                SourceRevision::Base => 2,
            };
            (revision_order, *start)
        });
        if additions.is_empty() {
            continue;
        }

        let mut revision_ranges = HashMap::<SourceRevision, (usize, usize)>::new();
        let mut valid = true;
        for (revision, _, _, _) in &additions {
            if revision_ranges.contains_key(revision) {
                continue;
            }
            let Some(records) = dispositions.get(revision) else {
                valid = false;
                break;
            };
            let Some((previous_start, previous_end)) = owner_bounds(records, &previous) else {
                valid = false;
                break;
            };
            let Some((following_start, _)) = owner_bounds(records, &following) else {
                valid = false;
                break;
            };
            let _ = previous_start;
            let range = (previous_end + 1, following_start.saturating_sub(1));
            if range.0 > range.1 {
                valid = false;
                break;
            }
            let allowed_ids = additions
                .iter()
                .filter(|(candidate_revision, _, _, _)| candidate_revision == revision)
                .map(|(_, _, _, owner_id)| owner_id.as_str())
                .collect::<HashSet<_>>();
            if records
                .iter()
                .filter(|record| (range.0..=range.1).contains(&record.line_number))
                .any(|record| match record.kind.as_str() {
                    "layout_gap" => false,
                    "structural_owner" | "comment_owner" => record
                        .owner_id
                        .as_deref()
                        .is_none_or(|owner_id| !allowed_ids.contains(owner_id)),
                    _ => true,
                })
            {
                valid = false;
                break;
            }
            revision_ranges.insert(*revision, range);
        }
        if !valid {
            continue;
        }

        let mut chunk = Vec::new();
        let mut chunk_keys = HashSet::new();
        let append_gap = |chunk: &mut Vec<SourceLineSelection>,
                          chunk_keys: &mut HashSet<String>,
                          records: &[crate::SourceLineDisposition],
                          revision: SourceRevision,
                          start: usize,
                          end: usize| {
            if start > end {
                return true;
            }
            let gap_records = records
                .iter()
                .filter(|record| (start..=end).contains(&record.line_number))
                .collect::<Vec<_>>();
            if gap_records.iter().any(|record| record.kind != "layout_gap") {
                return false;
            }
            for record in gap_records {
                if chunk_keys.insert(record.selection_key.clone()) {
                    chunk.push(SourceLineSelection {
                        selection_key: record.selection_key.clone(),
                        revision,
                    });
                }
            }
            true
        };

        let (first_revision, first_start, _, _) = &additions[0];
        let first_records = dispositions.get(first_revision).unwrap();
        let first_range = revision_ranges[first_revision];
        if !append_gap(
            &mut chunk,
            &mut chunk_keys,
            first_records,
            *first_revision,
            first_range.0,
            first_start.saturating_sub(1),
        ) {
            continue;
        }

        for (index, (revision, _, end, owner_id)) in additions.iter().enumerate() {
            let records = dispositions.get(revision).unwrap();
            let mut owner_records = records
                .iter()
                .filter(|record| {
                    record.owner_id.as_deref() == Some(owner_id)
                        && matches!(record.kind.as_str(), "structural_owner" | "comment_owner")
                })
                .collect::<Vec<_>>();
            owner_records.sort_by_key(|record| record.line_number);
            if owner_records.is_empty() {
                valid = false;
                break;
            }
            for record in owner_records {
                if chunk_keys.insert(record.selection_key.clone()) {
                    chunk.push(SourceLineSelection {
                        selection_key: record.selection_key.clone(),
                        revision: *revision,
                    });
                }
            }

            let (gap_revision, gap_start, gap_end) =
                if let Some((next_revision, next_start, _, _)) = additions.get(index + 1) {
                    if next_revision == revision {
                        (*revision, end + 1, next_start.saturating_sub(1))
                    } else {
                        let next_base_start = owner_bounds(records, &following).unwrap().0;
                        (*revision, end + 1, next_base_start.saturating_sub(1))
                    }
                } else {
                    let next_base_start = owner_bounds(records, &following).unwrap().0;
                    (*revision, end + 1, next_base_start.saturating_sub(1))
                };
            if !append_gap(
                &mut chunk,
                &mut chunk_keys,
                dispositions.get(&gap_revision).unwrap(),
                gap_revision,
                gap_start,
                gap_end,
            ) {
                valid = false;
                break;
            }
        }
        if !valid || chunk.is_empty() {
            continue;
        }

        for (revision, range) in revision_ranges {
            if let Some(records) = dispositions.get(&revision) {
                replaced_keys.extend(
                    records
                        .iter()
                        .filter(|record| (range.0..=range.1).contains(&record.line_number))
                        .map(|record| (revision, record.selection_key.clone())),
                );
            }
        }
        if let Some(records) = dispositions.get(&baseline_revision) {
            replaced_keys.extend(
                records
                    .iter()
                    .filter(|record| {
                        record.kind == "layout_gap"
                            && record.before_owner_id.as_deref() == Some(previous.as_str())
                            && record.after_owner_id.as_deref() == Some(following.as_str())
                    })
                    .map(|record| (baseline_revision, record.selection_key.clone())),
            );
        }

        let Some(anchor) = base.iter().find(|record| {
            record.kind == "structural_owner"
                && record.owner_id.as_deref() == Some(following.as_str())
        }) else {
            continue;
        };
        let anchor_revision =
            owner_revisions.get(&following).copied().unwrap_or(SourceRevision::Base);
        positioned.push((anchor.selection_key.clone(), anchor_revision, chunk));
    }

    selections.retain(|selection| {
        !replaced_keys.contains(&(selection.revision, selection.selection_key.clone()))
    });
    let mut positioned = positioned
        .into_iter()
        .filter_map(|(anchor_key, anchor_revision, chunk)| {
            selections
                .iter()
                .position(|selection| {
                    selection.selection_key == anchor_key && selection.revision == anchor_revision
                })
                .map(|index| (index, chunk))
        })
        .collect::<Vec<_>>();
    positioned.sort_by_key(|(index, _)| *index);
    let mut offset = 0usize;
    for (index, chunk) in positioned {
        let index = (index + offset).min(selections.len());
        let count = chunk.len();
        selections.splice(index..index, chunk);
        offset += count;
    }
    Ok(selections)
}

/// Convert removed-owner preservation line numbers into ordered
/// selections of complete semantic regions from the revision they came from.
/// A caller supplies the order after applying its removal/rehome boundary plan.
pub fn source_line_selections_for_preserved_lines(
    revision: SourceRevision,
    dispositions: &[crate::SourceLineDisposition],
    line_numbers: &[usize],
) -> Result<Vec<SourceLineSelection>, SourceRenderError> {
    let mut requested = std::collections::HashSet::new();
    let mut previous = 0;
    let mut selections = Vec::new();
    let mut selected_keys = std::collections::HashSet::new();
    for line_number in line_numbers {
        if *line_number <= previous || !requested.insert(*line_number) {
            return Err(SourceRenderError::new(
                "preserved source line numbers must be strictly increasing",
            ));
        }
        previous = *line_number;
        let record = dispositions
            .get(line_number - 1)
            .filter(|record| record.line_number == *line_number)
            .ok_or_else(|| {
                SourceRenderError::new(format!(
                    "preserved source line {line_number} has no line disposition"
                ))
            })?;
        if selected_keys.insert(record.selection_key.as_str()) {
            selections.push(SourceLineSelection {
                selection_key: record.selection_key.clone(),
                revision,
            });
        }
    }
    for selection in &selections {
        for record in
            dispositions.iter().filter(|record| record.selection_key == selection.selection_key)
        {
            if !requested.contains(&record.line_number) {
                return Err(SourceRenderError::new(format!(
                    "preserved selection {} omits line {} from its ownership region",
                    selection.selection_key, record.line_number
                )));
            }
        }
    }
    Ok(selections)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RehomeSourceLineSelections {
    pub source_owner_id: Option<String>,
    pub target_owner_id: Option<String>,
    pub edge: crate::BoundaryEdge,
    pub kind: crate::RehomeFragmentKind,
    pub fragments: Vec<SourceLineRehomeFragment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLineRehomeFragment {
    Source(SourceLineSelection),
    Synthesized(SynthesizedFragment),
}

/// Resolve a structural rehome decision to complete source regions, ordered by
/// source position. Safe same-line inline comments are promoted using Ruby's
/// indentation and raw-comment rule; the caller inserts these at the target edge.
pub fn source_line_fragments_for_rehome_plan(
    revision: SourceRevision,
    dispositions: &[crate::SourceLineDisposition],
    owners: &[crate::SourceLineOwner],
    comment_regions: &[crate::CommentRegion],
    inline_promotion_owner_ids: &std::collections::HashSet<String>,
    rehome: &crate::RehomePlan,
) -> Result<RehomeSourceLineSelections, SourceRenderError> {
    let mut regions = Vec::<(usize, SourceLineRehomeFragment)>::new();
    let mut keys = std::collections::HashSet::new();
    for region_id in &rehome.comment_region_ids {
        let records = dispositions
            .iter()
            .filter(|record| record.comment_region_ids.contains(region_id))
            .collect::<Vec<_>>();
        if records.is_empty() {
            return Err(SourceRenderError::new(format!(
                "rehome comment region {region_id} has no source line disposition"
            )));
        }
        if let Some(owner_line) = records.iter().find(|record| record.kind == "structural_owner") {
            let comment = comment_regions
                .iter()
                .find(|comment| comment.id == *region_id)
                .ok_or_else(|| {
                    SourceRenderError::new(format!(
                        "rehome comment region {region_id} has no comment facts"
                    ))
                })?;
            let owner_id = rehome.source_owner_id.as_deref().ok_or_else(|| {
                SourceRenderError::new(format!(
                    "rehome inline comment region {region_id} has no source owner"
                ))
            })?;
            let owner = owners.iter().find(|owner| owner.id == owner_id).ok_or_else(|| {
                SourceRenderError::new(format!(
                    "rehome inline comment region {region_id} has no owner facts"
                ))
            })?;
            if comment.kind != "inline"
                || comment.owner_id.as_str() != owner_id
                || owner.start_line != owner.end_line
                || owner.start_line != owner_line.line_number
                || !inline_promotion_owner_ids.contains(owner_id)
                || comment.nodes.len() != 1
                || comment.nodes[0].line_number != owner_line.line_number
            {
                return Err(SourceRenderError::new(format!(
                    "rehome comment region {region_id} cannot be promoted from its owner line"
                )));
            }
            let indent = owner_line
                .line
                .chars()
                .take_while(|character| character.is_whitespace())
                .collect::<String>();
            let raw_comment = comment.nodes[0].text.trim_start();
            if raw_comment.is_empty() {
                return Err(SourceRenderError::new(format!(
                    "rehome inline comment region {region_id} has empty source text"
                )));
            }
            // Ruby's emitter materializes promoted comments as logical lines and
            // joins the output with LF, even when the source used CRLF.
            let synthesized = SynthesizedFragment {
                content: format!("{indent}{raw_comment}\n"),
                reason: "removed_owner_inline_comment_promotion".into(),
                producer: "ast_merge.source_render".into(),
                metadata: HashMap::from([
                    (
                        "source_revision".into(),
                        serde_json::json!(format!("{revision:?}").to_lowercase()),
                    ),
                    ("source_owner_id".into(), serde_json::json!(owner_id)),
                    ("source_line".into(), serde_json::json!(owner_line.line_number)),
                    ("comment_region_id".into(), serde_json::json!(region_id)),
                ]),
            };
            regions
                .push((owner_line.line_number, SourceLineRehomeFragment::Synthesized(synthesized)));
            continue;
        }
        push_rehome_region_selection(revision, region_id, &records, &mut regions, &mut keys)?;
    }
    for gap_id in &rehome.layout_gap_ids {
        let records = dispositions
            .iter()
            .filter(|record| record.gap_id.as_deref() == Some(gap_id))
            .collect::<Vec<_>>();
        if records.is_empty() {
            return Err(SourceRenderError::new(format!(
                "rehome layout gap {gap_id} has no source line disposition"
            )));
        }
        push_rehome_region_selection(revision, gap_id, &records, &mut regions, &mut keys)?;
    }
    regions.sort_by_key(|(line_number, _)| *line_number);
    Ok(RehomeSourceLineSelections {
        source_owner_id: rehome.source_owner_id.clone(),
        target_owner_id: rehome.target_owner_id().map(str::to_owned),
        edge: rehome.edge(),
        kind: rehome.kind,
        fragments: regions.into_iter().map(|(_, fragment)| fragment).collect(),
    })
}

fn push_rehome_region_selection(
    revision: SourceRevision,
    region_id: &str,
    records: &[&crate::SourceLineDisposition],
    regions: &mut Vec<(usize, SourceLineRehomeFragment)>,
    keys: &mut std::collections::HashSet<String>,
) -> Result<(), SourceRenderError> {
    let mut line_numbers = records.iter().map(|record| record.line_number).collect::<Vec<_>>();
    line_numbers.sort_unstable();
    line_numbers.dedup();
    for pair in line_numbers.windows(2) {
        if pair[1] != pair[0] + 1 {
            return Err(SourceRenderError::new(format!(
                "rehome region {region_id} has noncontiguous source lines"
            )));
        }
    }
    let first = records[0].selection_key.as_str();
    if records.iter().any(|record| record.selection_key != first) {
        return Err(SourceRenderError::new(format!(
            "rehome region {region_id} maps to multiple source selections"
        )));
    }
    let selection_key = first.to_string();
    if !keys.insert(selection_key.clone()) {
        return Err(SourceRenderError::new(format!(
            "multiple rehome regions map to source selection {selection_key}"
        )));
    }
    let selected_lines =
        records.iter().filter(|record| record.selection_key == selection_key).count();
    if selected_lines != line_numbers.len() {
        return Err(SourceRenderError::new(format!(
            "rehome region {region_id} does not cover its complete source selection"
        )));
    }
    regions.push((
        line_numbers[0],
        SourceLineRehomeFragment::Source(SourceLineSelection { selection_key, revision }),
    ));
    Ok(())
}

pub fn source_render_plan_from_rehome_selections(
    sources: HashMap<SourceRevision, String>,
    dispositions: HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    rehome: &RehomeSourceLineSelections,
) -> Result<SourceRenderPlan, SourceRenderError> {
    let source_selections = rehome
        .fragments
        .iter()
        .filter_map(|fragment| match fragment {
            SourceLineRehomeFragment::Source(selection) => Some(selection.clone()),
            SourceLineRehomeFragment::Synthesized(_) => None,
        })
        .collect::<Vec<_>>();
    let source_plan =
        source_render_plan_from_line_selections(sources.clone(), dispositions, &source_selections)?;
    let mut source_fragments = source_plan.fragments.into_iter();
    let fragments = rehome
        .fragments
        .iter()
        .map(|fragment| match fragment {
            SourceLineRehomeFragment::Source(_) => source_fragments
                .next()
                .ok_or_else(|| SourceRenderError::new("rehome source fragment plan is incomplete")),
            SourceLineRehomeFragment::Synthesized(fragment) => {
                Ok(RenderFragment::Synthesized(fragment.clone()))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if source_fragments.next().is_some() {
        return Err(SourceRenderError::new("rehome source fragment plan has unused regions"));
    }
    SourceRenderPlan::new(sources, fragments)
}

/// Combine an ordered owner-line selection with boundary-anchored fragments
/// promoted by structural removal plans. A trailing boundary targets the
/// following owner's leading region; a leading boundary targets the preceding
/// owner's trailing region. Existing attachment selections for that boundary
/// remain adjacent to the owner, and duplicate source selections fail closed.
pub fn source_render_plan_from_ordered_selections_with_rehomes(
    sources: HashMap<SourceRevision, String>,
    dispositions: HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    comment_regions: HashMap<SourceRevision, Vec<crate::CommentRegion>>,
    selections: &[SourceLineSelection],
    rehomes: &[RehomeSourceLineSelections],
) -> Result<SourceRenderPlan, SourceRenderError> {
    let owner_plan =
        source_render_plan_from_line_selections(sources.clone(), dispositions.clone(), selections)?;
    let mut insertions = HashMap::<usize, Vec<RenderFragment>>::new();
    let mut selected_keys = selections
        .iter()
        .map(|selection| selection.selection_key.clone())
        .collect::<std::collections::HashSet<_>>();

    for rehome in rehomes {
        if rehome.fragments.is_empty() {
            continue;
        }
        let target_owner_id = rehome.target_owner_id.as_deref().ok_or_else(|| {
            SourceRenderError::new("rehome boundary has no surviving target owner")
        })?;
        let owner_index = selections
            .iter()
            .position(|selection| {
                dispositions.get(&selection.revision).is_some_and(|records| {
                    records.iter().any(|record| {
                        record.selection_key == selection.selection_key
                            && record.kind == "structural_owner"
                            && record.owner_id.as_deref() == Some(target_owner_id)
                    })
                })
            })
            .ok_or_else(|| {
                SourceRenderError::new(format!(
                    "rehome target owner {target_owner_id} has no selected source region"
                ))
            })?;

        let insertion_index = match rehome.edge {
            crate::BoundaryEdge::Trailing => (0..=owner_index)
                .find(|index| {
                    selected_region_attaches_to_boundary(
                        &selections[*index],
                        &dispositions,
                        target_owner_id,
                    )
                })
                .unwrap_or(owner_index),
            crate::BoundaryEdge::Leading => (owner_index + 1..selections.len())
                .rev()
                .find(|index| {
                    selected_region_attaches_to_boundary(
                        &selections[*index],
                        &dispositions,
                        target_owner_id,
                    )
                })
                .map_or(owner_index + 1, |index| index + 1),
        };

        let fragment_plan = source_render_plan_from_rehome_selections(
            sources.clone(),
            dispositions.clone(),
            rehome,
        )?;
        for fragment in fragment_plan.fragments {
            if rehome_fragment_already_selected(
                &fragment,
                rehome,
                &dispositions,
                &comment_regions,
                selections,
                target_owner_id,
            )? {
                continue;
            }
            if let RenderFragment::Source(source) = &fragment
                && let Some(key) =
                    source.metadata.get("selection_key").and_then(|value| value.as_str())
                && !selected_keys.insert(key.to_string())
            {
                return Err(SourceRenderError::new(format!(
                    "rehome duplicates selected source region {key}"
                )));
            }
            insertions.entry(insertion_index).or_default().push(fragment);
        }
    }

    let mut fragments = Vec::with_capacity(owner_plan.fragments.len() + rehomes.len());
    for index in 0..=owner_plan.fragments.len() {
        if let Some(promoted) = insertions.remove(&index) {
            fragments.extend(promoted);
        }
        if let Some(owner_fragment) = owner_plan.fragments.get(index) {
            fragments.push(owner_fragment.clone());
        }
    }
    if !insertions.is_empty() {
        return Err(SourceRenderError::new("rehome insertion falls outside the owner render plan"));
    }
    SourceRenderPlan::new(sources, fragments)
}

/// A removed comment can already be present in the selected revision, attached
/// to the same surviving boundary. Ruby ast-merge recognizes this by normalized
/// comment content; use the same semantic region facts here rather than comparing
/// rendered output bytes.
fn rehome_fragment_already_selected(
    fragment: &RenderFragment,
    rehome: &RehomeSourceLineSelections,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    comment_regions: &HashMap<SourceRevision, Vec<crate::CommentRegion>>,
    selections: &[SourceLineSelection],
    target_owner_id: &str,
) -> Result<bool, SourceRenderError> {
    let (revision, region_id) = match fragment {
        RenderFragment::Source(source) => {
            let Some(selection_key) = source.metadata.get("selection_key").and_then(|v| v.as_str())
            else {
                return Ok(false);
            };
            let matching = rehome.fragments.iter().find_map(|candidate| match candidate {
                SourceLineRehomeFragment::Source(selection)
                    if selection.revision == source.revision
                        && selection.selection_key == selection_key =>
                {
                    Some(selection)
                }
                _ => None,
            });
            let Some(selection) = matching else { return Ok(false) };
            let ids = dispositions
                .get(&selection.revision)
                .into_iter()
                .flatten()
                .filter(|record| record.selection_key == selection.selection_key)
                .flat_map(|record| record.comment_region_ids.iter().cloned())
                .collect::<std::collections::HashSet<_>>();
            if ids.len() != 1 {
                return Ok(false);
            }
            (selection.revision, ids.into_iter().next().unwrap())
        }
        RenderFragment::Synthesized(synthesized) => {
            let Some(region_id) =
                synthesized.metadata.get("comment_region_id").and_then(|v| v.as_str())
            else {
                return Ok(false);
            };
            let revision =
                match synthesized.metadata.get("source_revision").and_then(|v| v.as_str()) {
                    Some("base") => SourceRevision::Base,
                    Some("ours") => SourceRevision::Ours,
                    Some("theirs") => SourceRevision::Theirs,
                    _ => return Ok(false),
                };
            (revision, region_id.to_string())
        }
        _ => return Ok(false),
    };
    let Some(source_region) = comment_regions
        .get(&revision)
        .and_then(|regions| regions.iter().find(|region| region.id == region_id))
    else {
        return Ok(false);
    };
    let expected_attachment = match rehome.edge {
        crate::BoundaryEdge::Leading => "trailing",
        crate::BoundaryEdge::Trailing => "leading",
    };
    let target_selection_index = selections.iter().position(|selection| {
        dispositions.get(&selection.revision).is_some_and(|records| {
            records.iter().any(|record| {
                record.selection_key == selection.selection_key
                    && record.kind == "structural_owner"
                    && record.owner_id.as_deref() == Some(target_owner_id)
            })
        })
    });
    let equivalent = selections
        .iter()
        .enumerate()
        .flat_map(|(selection_index, selection)| {
            let Some(records) = dispositions.get(&selection.revision) else { return Vec::new() };
            let ids = records
                .iter()
                .filter(|record| {
                    record.selection_key == selection.selection_key
                        && ((record.owner_id.as_deref() == Some(target_owner_id)
                            && record.comment_kind.as_deref() == Some(expected_attachment))
                            || (record.kind == "root_region"
                                && record.root_region_kind.as_deref() == Some("preamble")
                                && target_selection_index.is_some_and(|target_index| {
                                    selection_index < target_index
                                        && !selections[selection_index + 1..target_index]
                                            .iter()
                                            .any(|between| {
                                                dispositions.get(&between.revision).is_some_and(
                                                    |between_records| {
                                                        between_records.iter().any(|candidate| {
                                                            candidate.selection_key
                                                                == between.selection_key
                                                                && candidate.kind
                                                                    == "structural_owner"
                                                        })
                                                    },
                                                )
                                            })
                                })))
                })
                .flat_map(|record| record.comment_region_ids.iter());
            ids.filter_map(|id| {
                comment_regions
                    .get(&selection.revision)
                    .and_then(|regions| regions.iter().find(|region| region.id == *id))
                    .filter(|region| {
                        region.normalized_content() == source_region.normalized_content()
                    })
                    .map(|_| (selection.revision, id.clone()))
            })
            .collect::<Vec<_>>()
        })
        .collect::<std::collections::HashSet<_>>();
    if equivalent.len() > 1 {
        return Err(SourceRenderError::new(format!(
            "rehome comment {} has multiple equivalent selected boundary regions",
            source_region.id
        )));
    }
    Ok(!equivalent.is_empty())
}

fn selected_region_attaches_to_boundary(
    selection: &SourceLineSelection,
    dispositions: &HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    owner_id: &str,
) -> bool {
    dispositions.get(&selection.revision).is_some_and(|records| {
        records.iter().any(|record| {
            record.selection_key == selection.selection_key
                && ((record.kind == "comment_owner"
                    && record.owner_id.as_deref() == Some(owner_id))
                    || (record.kind == "layout_gap"
                        && record.controller_owner_id.as_deref() == Some(owner_id)))
        })
    })
}

/// Build an exact-byte render plan from ordered semantic line selections.
/// Each selection names one contiguous ownership region in one source revision.
pub fn source_render_plan_from_line_selections(
    sources: HashMap<SourceRevision, String>,
    dispositions: HashMap<SourceRevision, Vec<crate::SourceLineDisposition>>,
    selections: &[SourceLineSelection],
) -> Result<SourceRenderPlan, SourceRenderError> {
    let mut ranges = HashMap::<SourceRevision, HashMap<String, (usize, usize, String)>>::new();
    for (revision, records) in &dispositions {
        let source = sources.get(revision).ok_or_else(|| {
            SourceRenderError::new(format!("source revision {revision:?} has no source text"))
        })?;
        let lines = source_lines(source);
        if lines.len() != records.len() {
            return Err(SourceRenderError::new(format!(
                "{revision:?} has {} line dispositions for {} source lines",
                records.len(),
                lines.len()
            )));
        }
        let mut revision_ranges = HashMap::new();
        for (index, (line, record)) in lines.iter().zip(records).enumerate() {
            let line_number = index + 1;
            if record.line_number != line_number || record.line != *line {
                return Err(SourceRenderError::new(format!(
                    "{revision:?} line disposition {line_number} does not match its source bytes"
                )));
            }
            if record.selection_key.is_empty() {
                return Err(SourceRenderError::new(format!(
                    "{revision:?} line {line_number} has no source selection key"
                )));
            }
            match revision_ranges.entry(record.selection_key.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert((line_number, line_number, record.kind.clone()));
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let range = entry.get_mut();
                    if range.1 + 1 != line_number {
                        return Err(SourceRenderError::new(format!(
                            "{revision:?} source selection key {} is not contiguous",
                            record.selection_key
                        )));
                    }
                    range.1 = line_number;
                }
            }
        }
        ranges.insert(*revision, revision_ranges);
    }

    let mut selected_keys = std::collections::HashSet::new();
    let mut fragments = Vec::with_capacity(selections.len());
    for selection in selections {
        if selection.selection_key.is_empty()
            || !selected_keys.insert(selection.selection_key.as_str())
        {
            return Err(SourceRenderError::new(
                "source line selections have an empty or duplicate key",
            ));
        }
        let (start_line, end_line, disposition_kind) = ranges
            .get(&selection.revision)
            .and_then(|by_key| by_key.get(&selection.selection_key))
            .cloned()
            .ok_or_else(|| {
                SourceRenderError::new(format!(
                    "selection key {} is absent from {:?}",
                    selection.selection_key, selection.revision
                ))
            })?;
        fragments.push(RenderFragment::Source(SourceFragment {
            revision: selection.revision,
            start_line,
            end_line,
            metadata: HashMap::from([
                ("selection_key".to_string(), serde_json::json!(selection.selection_key)),
                ("disposition_kind".to_string(), serde_json::json!(disposition_kind)),
            ]),
        }));
    }
    SourceRenderPlan::new(sources, fragments)
}

pub fn localized_conflict_render_plan(
    sources: HashMap<SourceRevision, String>,
    conflicts: &[crate::MergeConflict],
    marker_size: usize,
) -> Result<SourceRenderPlan, SourceRenderError> {
    if conflicts.is_empty() {
        return Err(SourceRenderError::new(
            "localized conflict rendering requires at least one conflict",
        ));
    }
    if marker_size == 0 {
        return Err(SourceRenderError::new("marker_size must be at least one"));
    }

    let ours_line_count = source_lines(source_for_revision(&sources, SourceRevision::Ours)?).len();
    let mut localized = conflicts
        .iter()
        .map(|conflict| {
            Ok((
                conflict,
                conflict_region(conflict, SourceRevision::Base)?,
                conflict_region(conflict, SourceRevision::Ours)?,
                conflict_region(conflict, SourceRevision::Theirs)?,
            ))
        })
        .collect::<Result<Vec<_>, SourceRenderError>>()?;
    localized.sort_by_key(|(_, _, ours, _)| {
        ours.map_or((ours_line_count + 1, ours_line_count), |region| {
            (region.start_line, region.end_line)
        })
    });

    let mut fragments = Vec::new();
    let mut next_ours_line = 1;
    for (conflict, base, ours, theirs) in localized {
        if base.is_none() && ours.is_none() && theirs.is_none() {
            return Err(SourceRenderError::new("conflict has no present alternative"));
        }
        // A deleted ours owner has no insertion anchor. Append an explicit empty-
        // ours review block, retaining ours verbatim, rather than inventing a span.
        let (start_line, end_line) = ours
            .map_or((ours_line_count + 1, ours_line_count), |region| {
                (region.start_line, region.end_line)
            });
        if start_line < next_ours_line {
            return Err(SourceRenderError::new(format!(
                "conflict {} overlaps a prior ours-owned line region",
                conflict.conflict_id
            )));
        }
        if end_line > ours_line_count {
            return Err(SourceRenderError::new(format!(
                "conflict {} exceeds the ours source line count",
                conflict.conflict_id
            )));
        }
        if next_ours_line < start_line {
            fragments.push(RenderFragment::Source(SourceFragment {
                revision: SourceRevision::Ours,
                start_line: next_ours_line,
                end_line: start_line - 1,
                metadata: HashMap::new(),
            }));
            if ours.is_none()
                && !source_for_revision(&sources, SourceRevision::Ours)?.ends_with('\n')
            {
                fragments.push(RenderFragment::Synthesized(SynthesizedFragment {
                    content: "\n".into(),
                    reason: "conflict_line_boundary".into(),
                    producer: "ast-merge".into(),
                    metadata: HashMap::new(),
                }));
            }
        }
        fragments.push(RenderFragment::Conflict(ConflictFragment {
            conflict_id: conflict.conflict_id.clone(),
            base: base
                .map(|region| source_conflict_side(&sources, SourceRevision::Base, region))
                .transpose()?
                .unwrap_or_default(),
            ours: ours
                .map(|region| source_conflict_side(&sources, SourceRevision::Ours, region))
                .transpose()?
                .unwrap_or_default(),
            theirs: theirs
                .map(|region| source_conflict_side(&sources, SourceRevision::Theirs, region))
                .transpose()?
                .unwrap_or_default(),
            labels: ConflictLabels::default(),
            marker_size,
            metadata: HashMap::from([
                ("category".to_string(), serde_json::json!(conflict.category)),
                ("path".to_string(), serde_json::json!(conflict.path)),
                (
                    "placement".to_string(),
                    serde_json::json!(if ours.is_some() {
                        "ours_owned_region"
                    } else {
                        "end_of_ours_absent_owner"
                    }),
                ),
            ]),
        }));
        next_ours_line = end_line + 1;
    }
    if next_ours_line <= ours_line_count {
        fragments.push(RenderFragment::Source(SourceFragment {
            revision: SourceRevision::Ours,
            start_line: next_ours_line,
            end_line: ours_line_count,
            metadata: HashMap::new(),
        }));
    }
    SourceRenderPlan::new(sources, fragments)
}

fn source_for_revision(
    sources: &HashMap<SourceRevision, String>,
    revision: SourceRevision,
) -> Result<&str, SourceRenderError> {
    sources.get(&revision).map(String::as_str).ok_or_else(|| {
        SourceRenderError::new(format!("source revision {revision:?} is not available"))
    })
}

fn conflict_region(
    conflict: &crate::MergeConflict,
    revision: SourceRevision,
) -> Result<Option<&OwnedSourceRegion>, SourceRenderError> {
    let alternatives = conflict
        .alternatives
        .iter()
        .filter(|alternative| alternative.revision == revision)
        .collect::<Vec<_>>();
    if alternatives.len() != 1 {
        return Err(SourceRenderError::new(format!(
            "conflict {} requires exactly one {revision:?} alternative",
            conflict.conflict_id
        )));
    }
    let alternative = alternatives[0];
    if alternative.state == ConflictAlternativeState::Absent && alternative.regions.is_empty() {
        return Ok(None);
    }
    if alternative.state != ConflictAlternativeState::Present || alternative.regions.len() != 1 {
        return Err(SourceRenderError::new(format!(
            "conflict {} does not have one present {revision:?} source region",
            conflict.conflict_id
        )));
    }
    let region = &alternative.regions[0];
    region.validate()?;
    Ok(Some(region))
}

fn source_conflict_side(
    sources: &HashMap<SourceRevision, String>,
    revision: SourceRevision,
    region: &OwnedSourceRegion,
) -> Result<Vec<ConflictSideFragment>, SourceRenderError> {
    let fragment = SourceFragment {
        revision,
        start_line: region.start_line,
        end_line: region.end_line,
        metadata: HashMap::from([
            ("start_byte".to_string(), serde_json::json!(region.start_byte)),
            ("end_byte".to_string(), serde_json::json!(region.end_byte)),
        ]),
    };
    let lines = source_lines(source_for_revision(sources, revision)?);
    if region.end_line > lines.len() {
        return Err(SourceRenderError::new(format!(
            "{revision:?} conflict region exceeds source line count {}",
            lines.len()
        )));
    }
    let content = lines[(region.start_line - 1)..region.end_line].concat();
    let mut fragments = vec![ConflictSideFragment::Source(fragment)];
    if !content.ends_with('\n') {
        fragments.push(ConflictSideFragment::Synthesized(SynthesizedFragment {
            content: "\n".to_string(),
            reason: "conflict_line_boundary".to_string(),
            producer: "ast-merge".to_string(),
            metadata: HashMap::new(),
        }));
    }
    Ok(fragments)
}

struct SourcePlanRenderer<'a> {
    plan: &'a SourceRenderPlan,
    content: String,
    next_line: usize,
    line_records: Vec<RenderLineRecord>,
    synthesized_fragments: Vec<SynthesizedFragmentRecord>,
    conflicts: Vec<RenderedConflictRecord>,
    source_fragment_digests: Vec<SourceFragmentDigest>,
}

impl<'a> SourcePlanRenderer<'a> {
    fn new(plan: &'a SourceRenderPlan) -> Self {
        Self {
            plan,
            content: String::new(),
            next_line: 1,
            line_records: Vec::new(),
            synthesized_fragments: Vec::new(),
            conflicts: Vec::new(),
            source_fragment_digests: Vec::new(),
        }
    }

    fn render_fragment(&mut self, fragment: &RenderFragment) -> Result<(), SourceRenderError> {
        match fragment {
            RenderFragment::Source(fragment) => self.render_source(fragment, None, None),
            RenderFragment::Synthesized(fragment) => {
                self.render_synthesized(fragment, None, None);
                Ok(())
            }
            RenderFragment::Conflict(fragment) => self.render_conflict(fragment),
        }
    }

    fn render_source(
        &mut self,
        fragment: &SourceFragment,
        conflict_id: Option<&str>,
        conflict_side: Option<SourceRevision>,
    ) -> Result<(), SourceRenderError> {
        let content = self.plan.source_content(fragment)?;
        let first_output_line = self.next_output_line();
        self.content.push_str(&content);
        self.next_line += content.bytes().filter(|byte| *byte == b'\n').count();
        for (index, _) in source_lines(&content).iter().enumerate() {
            self.line_records.push(RenderLineRecord {
                output_line: first_output_line + index,
                fragment_kind: RenderFragmentKind::Source,
                revision: Some(fragment.revision),
                original_line: Some(fragment.start_line + index),
                synthesized_reason: None,
                producer: None,
                conflict_id: conflict_id.map(str::to_string),
                conflict_side,
                metadata: fragment.metadata.clone(),
            });
        }
        self.source_fragment_digests.push(SourceFragmentDigest {
            revision: fragment.revision,
            start_line: fragment.start_line,
            end_line: fragment.end_line,
            sha256: sha256(&content),
        });
        Ok(())
    }

    fn render_synthesized(
        &mut self,
        fragment: &SynthesizedFragment,
        conflict_id: Option<&str>,
        conflict_side: Option<SourceRevision>,
    ) {
        let first_output_line = self.next_output_line();
        self.content.push_str(&fragment.content);
        self.next_line += fragment.content.bytes().filter(|byte| *byte == b'\n').count();
        for (index, _) in source_lines(&fragment.content).iter().enumerate() {
            self.line_records.push(RenderLineRecord {
                output_line: first_output_line + index,
                fragment_kind: RenderFragmentKind::Synthesized,
                revision: None,
                original_line: None,
                synthesized_reason: Some(fragment.reason.clone()),
                producer: Some(fragment.producer.clone()),
                conflict_id: conflict_id.map(str::to_string),
                conflict_side,
                metadata: fragment.metadata.clone(),
            });
        }
        self.synthesized_fragments.push(SynthesizedFragmentRecord {
            reason: fragment.reason.clone(),
            producer: fragment.producer.clone(),
            sha256: sha256(&fragment.content),
            conflict_id: conflict_id.map(str::to_string),
            conflict_side,
            metadata: fragment.metadata.clone(),
        });
    }

    fn render_conflict(&mut self, fragment: &ConflictFragment) -> Result<(), SourceRenderError> {
        let output_start_line = self.next_output_line();
        self.render_marker('<', Some(&fragment.labels.ours), fragment, Some(SourceRevision::Ours));
        self.render_conflict_side(&fragment.ours, fragment, SourceRevision::Ours)?;
        self.render_marker('|', Some(&fragment.labels.base), fragment, Some(SourceRevision::Base));
        self.render_conflict_side(&fragment.base, fragment, SourceRevision::Base)?;
        self.render_marker('=', None, fragment, None);
        self.render_conflict_side(&fragment.theirs, fragment, SourceRevision::Theirs)?;
        self.render_marker(
            '>',
            Some(&fragment.labels.theirs),
            fragment,
            Some(SourceRevision::Theirs),
        );
        self.conflicts.push(RenderedConflictRecord {
            conflict_id: fragment.conflict_id.clone(),
            output_start_line,
            output_end_line: self.next_output_line() - 1,
            metadata: fragment.metadata.clone(),
        });
        Ok(())
    }

    fn render_conflict_side(
        &mut self,
        side: &[ConflictSideFragment],
        conflict: &ConflictFragment,
        role: SourceRevision,
    ) -> Result<(), SourceRenderError> {
        for child in side {
            match child {
                ConflictSideFragment::Source(fragment) => {
                    self.render_source(fragment, Some(&conflict.conflict_id), Some(role))?;
                }
                ConflictSideFragment::Synthesized(fragment) => {
                    self.render_synthesized(fragment, Some(&conflict.conflict_id), Some(role));
                }
            }
        }
        Ok(())
    }

    fn render_marker(
        &mut self,
        character: char,
        label: Option<&str>,
        conflict: &ConflictFragment,
        side: Option<SourceRevision>,
    ) {
        let marker = character.to_string().repeat(conflict.marker_size);
        let content = match label {
            Some(label) => format!("{marker} {label}\n"),
            None => format!("{marker}\n"),
        };
        self.render_synthesized(
            &SynthesizedFragment {
                content,
                reason: "conflict_marker".to_string(),
                producer: "ast-merge".to_string(),
                metadata: HashMap::new(),
            },
            Some(&conflict.conflict_id),
            side,
        );
    }

    fn next_output_line(&self) -> usize {
        self.next_line
    }

    fn finish(self) -> SourceRenderResult {
        let verification_input = SourceRenderVerification {
            content_sha256: sha256(&self.content),
            source_fragments: self.source_fragment_digests,
            synthesized_fragment_count: self.synthesized_fragments.len(),
            conflict_count: self.conflicts.len(),
        };
        SourceRenderResult {
            content: self.content,
            line_records: self.line_records,
            synthesized_fragments: self.synthesized_fragments,
            conflicts: self.conflicts,
            verification_input,
        }
    }
}

fn source_lines(source: &str) -> Vec<&str> {
    source.split_inclusive('\n').collect()
}

fn sha256(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEdit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

impl SourceEdit {
    pub fn replace(start_byte: usize, end_byte: usize, replacement: impl Into<String>) -> Self {
        Self { start_byte, end_byte, replacement: replacement.into() }
    }

    pub fn insert(at_byte: usize, replacement: impl Into<String>) -> Self {
        Self::replace(at_byte, at_byte, replacement)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRenderError {
    message: String,
}

impl SourceRenderError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SourceRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SourceRenderError {}

pub fn apply_source_edits(source: &str, edits: &[SourceEdit]) -> Result<String, SourceRenderError> {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| (edit.start_byte, edit.end_byte));

    let mut previous_end = 0;
    let mut previous_start = None;
    for edit in &ordered {
        if edit.start_byte > edit.end_byte || edit.end_byte > source.len() {
            return Err(SourceRenderError::new(format!(
                "invalid source edit range [{}, {}) for source length {}",
                edit.start_byte,
                edit.end_byte,
                source.len()
            )));
        }
        if !source.is_char_boundary(edit.start_byte) || !source.is_char_boundary(edit.end_byte) {
            return Err(SourceRenderError::new(format!(
                "source edit range [{}, {}) is not on UTF-8 boundaries",
                edit.start_byte, edit.end_byte
            )));
        }
        if edit.start_byte < previous_end {
            return Err(SourceRenderError::new(format!(
                "source edit range [{}, {}) overlaps a prior edit ending at {}",
                edit.start_byte, edit.end_byte, previous_end
            )));
        }
        if previous_start == Some(edit.start_byte) {
            return Err(SourceRenderError::new(format!(
                "multiple source edits start at byte {}; their order is ambiguous",
                edit.start_byte
            )));
        }
        previous_start = Some(edit.start_byte);
        previous_end = edit.end_byte;
    }

    let replacement_bytes = ordered.iter().map(|edit| edit.replacement.len()).sum::<usize>();
    let removed_bytes = ordered.iter().map(|edit| edit.end_byte - edit.start_byte).sum::<usize>();
    let mut output = String::with_capacity(source.len() - removed_bytes + replacement_bytes);
    let mut cursor = 0;
    for edit in ordered {
        output.push_str(&source[cursor..edit.start_byte]);
        output.push_str(&edit.replacement);
        cursor = edit.end_byte;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> HashMap<SourceRevision, String> {
        HashMap::from([
            (SourceRevision::Base, "alpha\nshared\nomega\n".to_string()),
            (SourceRevision::Ours, "alpha ours\nshared\nomega\n".to_string()),
            (SourceRevision::Theirs, "alpha\nshared\nomega theirs\n".to_string()),
        ])
    }

    fn source_fragment(
        revision: SourceRevision,
        start_line: usize,
        end_line: usize,
    ) -> SourceFragment {
        SourceFragment { revision, start_line, end_line, metadata: HashMap::new() }
    }

    fn alternative(
        revision: SourceRevision,
        start_byte: usize,
        end_byte: usize,
        start_line: usize,
        end_line: usize,
    ) -> ConflictAlternative {
        ConflictAlternative {
            revision,
            state: ConflictAlternativeState::Present,
            regions: vec![OwnedSourceRegion {
                node_id: format!("{revision:?}:{start_byte}-{end_byte}"),
                region_kind: "node".to_string(),
                start_byte,
                end_byte,
                start_line,
                end_line,
            }],
        }
    }

    #[test]
    fn renders_exact_source_lines_with_provenance() {
        let plan = SourceRenderPlan::new(
            sources(),
            vec![
                RenderFragment::Source(source_fragment(SourceRevision::Ours, 1, 2)),
                RenderFragment::Source(source_fragment(SourceRevision::Theirs, 3, 3)),
            ],
        )
        .unwrap();
        let result = render_source_plan(&plan).unwrap();

        assert_eq!(result.content, "alpha ours\nshared\nomega theirs\n");
        assert_eq!(result.line_records.len(), 3);
        assert_eq!(result.line_records[0].revision, Some(SourceRevision::Ours));
        assert_eq!(result.line_records[0].original_line, Some(1));
        assert_eq!(result.line_records[2].revision, Some(SourceRevision::Theirs));
        assert!(result.synthesized_fragments.is_empty());
        assert!(!result.conflicted());
    }

    #[test]
    fn renders_localized_conflicts_and_records_synthesis() {
        let conflict = ConflictFragment {
            conflict_id: "owner-shared".to_string(),
            base: vec![ConflictSideFragment::Source(source_fragment(SourceRevision::Base, 2, 2))],
            ours: vec![ConflictSideFragment::Synthesized(SynthesizedFragment {
                content: "shared ours\n".to_string(),
                reason: "family_emission".to_string(),
                producer: "example-provider".to_string(),
                metadata: HashMap::new(),
            })],
            theirs: vec![ConflictSideFragment::Synthesized(SynthesizedFragment {
                content: "shared theirs\n".to_string(),
                reason: "family_emission".to_string(),
                producer: "example-provider".to_string(),
                metadata: HashMap::new(),
            })],
            labels: ConflictLabels::default(),
            marker_size: 7,
            metadata: HashMap::new(),
        };
        let plan = SourceRenderPlan::new(
            sources(),
            vec![
                RenderFragment::Source(source_fragment(SourceRevision::Ours, 1, 1)),
                RenderFragment::Conflict(conflict),
                RenderFragment::Source(source_fragment(SourceRevision::Theirs, 3, 3)),
            ],
        )
        .unwrap();
        let result = render_source_plan(&plan).unwrap();

        assert_eq!(
            result.content,
            concat!(
                "alpha ours\n",
                "<<<<<<< ours\n",
                "shared ours\n",
                "||||||| base\n",
                "shared\n",
                "=======\n",
                "shared theirs\n",
                ">>>>>>> theirs\n",
                "omega theirs\n"
            )
        );
        assert_eq!(result.conflicts[0].output_start_line, 2);
        assert_eq!(result.conflicts[0].output_end_line, 8);
        assert_eq!(
            result
                .synthesized_fragments
                .iter()
                .filter(|fragment| fragment.reason == "conflict_marker")
                .count(),
            4
        );
        assert_eq!(result.verification_input.conflict_count, 1);
    }

    #[test]
    fn builds_localized_conflict_plans_from_exact_owned_regions() {
        let conflict = crate::MergeConflict {
            conflict_id: "owner-shared".to_string(),
            category: "content".to_string(),
            path: "/shared".to_string(),
            fallback_scope: "/shared".to_string(),
            message: "both sides changed shared".to_string(),
            alternatives: vec![
                alternative(SourceRevision::Base, 6, 13, 2, 2),
                alternative(SourceRevision::Ours, 11, 18, 2, 2),
                alternative(SourceRevision::Theirs, 6, 13, 2, 2),
            ],
        };
        let plan = localized_conflict_render_plan(sources(), &[conflict], 7).unwrap();
        let result = render_source_plan(&plan).unwrap();

        assert!(result.content.starts_with("alpha ours\n<<<<<<< ours\nshared\n"));
        assert!(result.content.ends_with(">>>>>>> theirs\nomega\n"));
        assert_eq!(result.conflicts[0].output_start_line, 2);
        assert_eq!(result.conflicts[0].metadata["path"], "/shared");
    }

    #[test]
    fn rejects_incomplete_or_overlapping_conflict_ownership() {
        let incomplete = crate::MergeConflict {
            conflict_id: "incomplete".to_string(),
            category: "content".to_string(),
            path: "/shared".to_string(),
            fallback_scope: "/shared".to_string(),
            message: "missing theirs".to_string(),
            alternatives: vec![
                alternative(SourceRevision::Base, 6, 13, 2, 2),
                alternative(SourceRevision::Ours, 11, 18, 2, 2),
            ],
        };
        let error = localized_conflict_render_plan(sources(), &[incomplete], 7).unwrap_err();
        assert!(error.message().contains("exactly one Theirs alternative"));

        let complete = |conflict_id: &str| crate::MergeConflict {
            conflict_id: conflict_id.to_string(),
            category: "content".to_string(),
            path: "/shared".to_string(),
            fallback_scope: "/shared".to_string(),
            message: "overlap".to_string(),
            alternatives: vec![
                alternative(SourceRevision::Base, 6, 13, 2, 2),
                alternative(SourceRevision::Ours, 11, 18, 2, 2),
                alternative(SourceRevision::Theirs, 6, 13, 2, 2),
            ],
        };
        let error =
            localized_conflict_render_plan(sources(), &[complete("first"), complete("second")], 7)
                .unwrap_err();
        assert!(error.message().contains("overlaps"));
    }

    #[test]
    fn absent_ours_is_an_empty_review_side_not_an_invented_source_region() {
        for ours in ["", "retained 🦀", "retained\r\n"] {
            let mut inputs = sources();
            inputs.insert(SourceRevision::Ours, ours.into());
            let conflict = crate::MergeConflict {
                conflict_id: "deleted".into(),
                category: "delete_modify".into(),
                path: "/shared".into(),
                fallback_scope: "/shared".into(),
                message: "deleted".into(),
                alternatives: vec![
                    alternative(SourceRevision::Base, 6, 13, 2, 2),
                    ConflictAlternative {
                        revision: SourceRevision::Ours,
                        state: ConflictAlternativeState::Absent,
                        regions: vec![],
                    },
                    alternative(SourceRevision::Theirs, 6, 13, 2, 2),
                ],
            };
            let result = render_source_plan(
                &localized_conflict_render_plan(inputs, &[conflict], 9).unwrap(),
            )
            .unwrap();
            let prefix = if ours.is_empty() || ours.ends_with('\n') {
                ours.to_string()
            } else {
                format!("{ours}\n")
            };
            assert!(
                result.content.starts_with(&format!("{prefix}<<<<<<<<< ours\n||||||||| base\n"))
            );
            assert_eq!(result.conflicts[0].metadata["placement"], "end_of_ours_absent_owner");
            assert!(
                !result
                    .line_records
                    .iter()
                    .any(|line| line.conflict_side == Some(SourceRevision::Ours)
                        && line.fragment_kind == RenderFragmentKind::Source)
            );
            assert_eq!(
                result
                    .synthesized_fragments
                    .iter()
                    .filter(|item| item.reason == "conflict_line_boundary")
                    .count(),
                usize::from(!ours.is_empty() && !ours.ends_with('\n'))
            );
        }
    }

    #[test]
    fn absent_base_or_theirs_renders_empty_side_but_invalid_absence_fails() {
        for missing in [SourceRevision::Base, SourceRevision::Theirs] {
            let mut conflict = crate::MergeConflict {
                conflict_id: "missing-side".into(),
                category: "content".into(),
                path: "/shared".into(),
                fallback_scope: "/shared".into(),
                message: "missing".into(),
                alternatives: vec![
                    alternative(SourceRevision::Base, 6, 13, 2, 2),
                    alternative(SourceRevision::Ours, 11, 18, 2, 2),
                    alternative(SourceRevision::Theirs, 6, 13, 2, 2),
                ],
            };
            let side =
                conflict.alternatives.iter_mut().find(|side| side.revision == missing).unwrap();
            side.state = ConflictAlternativeState::Absent;
            // Absence must not conceal contradictory source regions.
            assert!(localized_conflict_render_plan(sources(), &[conflict.clone()], 7).is_err());
            conflict
                .alternatives
                .iter_mut()
                .find(|side| side.revision == missing)
                .unwrap()
                .regions
                .clear();
            let result = render_source_plan(
                &localized_conflict_render_plan(sources(), &[conflict], 7).unwrap(),
            )
            .unwrap();
            assert!(result.content.starts_with("alpha ours\n<<<<<<< ours\nshared\n"));
            assert!(!result.line_records.iter().any(|line| line.conflict_side == Some(missing)
                && line.fragment_kind == RenderFragmentKind::Source));
        }
    }

    #[test]
    fn preserves_missing_final_newline_and_validates_fragment_boundaries() {
        let plan = SourceRenderPlan::new(
            HashMap::from([(SourceRevision::Ours, "first\nlast".to_string())]),
            vec![RenderFragment::Source(source_fragment(SourceRevision::Ours, 1, 2))],
        )
        .unwrap();
        assert_eq!(render_source_plan(&plan).unwrap().content, "first\nlast");

        let outside = SourceRenderPlan::new(
            sources(),
            vec![RenderFragment::Source(source_fragment(SourceRevision::Ours, 3, 4))],
        )
        .unwrap_err();
        assert!(outside.message().contains("exceeds source line count"));

        let boundary = SourceRenderPlan::new(
            HashMap::from([
                (SourceRevision::Ours, "unterminated".to_string()),
                (SourceRevision::Theirs, "next\n".to_string()),
            ]),
            vec![
                RenderFragment::Source(source_fragment(SourceRevision::Ours, 1, 1)),
                RenderFragment::Source(source_fragment(SourceRevision::Theirs, 1, 1)),
            ],
        )
        .unwrap_err();
        assert!(boundary.message().contains("line boundary"));
    }

    #[test]
    fn preserves_untouched_bytes_across_ordered_edits() {
        let source = "{\r\n  \"left\": 1,\r\n  \"right\": 1\r\n}\r\n";
        let left = source.find("1,").unwrap();
        let right = source.rfind('1').unwrap();
        let output = apply_source_edits(
            source,
            &[SourceEdit::replace(right, right + 1, "3"), SourceEdit::replace(left, left + 1, "2")],
        )
        .unwrap();

        assert_eq!(output, "{\r\n  \"left\": 2,\r\n  \"right\": 3\r\n}\r\n");
    }

    #[test]
    fn supports_unambiguous_insertions() {
        assert_eq!(
            apply_source_edits("{}\n", &[SourceEdit::insert(1, "\n  \"key\": 1\n")]).unwrap(),
            "{\n  \"key\": 1\n}\n"
        );
    }

    #[test]
    fn rejects_overlapping_or_ambiguous_edits() {
        let overlap = apply_source_edits(
            "abcdef",
            &[SourceEdit::replace(1, 4, "x"), SourceEdit::replace(3, 5, "y")],
        )
        .unwrap_err();
        assert!(overlap.message().contains("overlaps"));

        let ambiguous = apply_source_edits(
            "abcdef",
            &[SourceEdit::insert(2, "x"), SourceEdit::replace(2, 3, "y")],
        )
        .unwrap_err();
        assert!(ambiguous.message().contains("ambiguous"));
    }

    #[test]
    fn rejects_ranges_that_split_utf8_or_exceed_source() {
        let utf8 = apply_source_edits("aéz", &[SourceEdit::replace(2, 3, "e")]).unwrap_err();
        assert!(utf8.message().contains("UTF-8 boundaries"));

        let outside = apply_source_edits("abc", &[SourceEdit::replace(0, 4, "x")]).unwrap_err();
        assert!(outside.message().contains("source length 3"));
    }
}
