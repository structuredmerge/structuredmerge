use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;

/// Source-preserving replacement of a one-based inclusive line range.
///
/// Untouched source chunks remain byte-exact. Separator re-emission and
/// junction capping follow the language-neutral ast-merge structural splice
/// contract shared with Ruby.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplicePlan {
    source: String,
    replacement: String,
    replace_start_line: usize,
    replace_end_line: usize,
    preserve_removed_trailing_blank_lines: bool,
}

impl SplicePlan {
    pub fn new(
        source: impl Into<String>,
        replacement: impl Into<String>,
        replace_start_line: usize,
        replace_end_line: usize,
    ) -> Result<Self, String> {
        let plan = Self {
            source: source.into(),
            replacement: replacement.into(),
            replace_start_line,
            replace_end_line,
            preserve_removed_trailing_blank_lines: true,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn preserve_removed_trailing_blank_lines(mut self, preserve: bool) -> Self {
        self.preserve_removed_trailing_blank_lines = preserve;
        self
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
    pub fn replace_start_line(&self) -> usize {
        self.replace_start_line
    }
    pub fn replace_end_line(&self) -> usize {
        self.replace_end_line
    }
    pub fn line_range(&self) -> RangeInclusive<usize> {
        self.replace_start_line..=self.replace_end_line
    }

    pub fn before_content(&self) -> String {
        let chunks = line_chunks(&self.source);
        chunks[..self.replace_start_line - 1].concat()
    }

    pub fn removed_content(&self) -> String {
        let chunks = line_chunks(&self.source);
        chunks[self.replace_start_line - 1..self.replace_end_line].concat()
    }

    pub fn after_content(&self) -> String {
        let chunks = line_chunks(&self.source);
        chunks[self.replace_end_line..].concat()
    }

    pub fn merged_content(&self) -> String {
        let before = self.before_content();
        let removed = self.removed_content();
        let after = self.after_content();
        let preserved = self.preserved_trailing_blank_chunks(&removed, &after);
        let mut head = before.clone();
        head.push_str(&self.replacement);
        for chunk in &preserved {
            head.push_str(chunk);
        }
        cap_junction(&head, &after, preserved.len(), &before, &self.replacement)
    }

    pub fn changed(&self) -> bool {
        self.merged_content() != self.source
    }

    pub fn apply_to(&self, alternate_source: &str) -> Result<String, String> {
        if alternate_source == self.source {
            return Ok(self.merged_content());
        }
        Ok(Self::new(
            alternate_source,
            self.replacement.clone(),
            self.replace_start_line,
            self.replace_end_line,
        )?
        .merged_content())
    }

    fn validate(&self) -> Result<(), String> {
        if self.replace_start_line == 0 {
            return Err("replace_start_line must be >= 1".into());
        }
        if self.replace_end_line < self.replace_start_line {
            return Err("replace_end_line must be >= replace_start_line".into());
        }
        let line_count = line_chunks(&self.source).len();
        if self.replace_end_line > line_count {
            return Err(format!(
                "replace_end_line {} exceeds source line count {line_count}",
                self.replace_end_line
            ));
        }
        Ok(())
    }

    fn preserved_trailing_blank_chunks<'a>(&self, removed: &'a str, after: &str) -> Vec<&'a str> {
        if !self.preserve_removed_trailing_blank_lines
            || self.replacement.is_empty()
            || after.is_empty()
        {
            return vec![];
        }
        let removed_chunks = trailing_blank_chunks(removed);
        let replacement_count = trailing_blank_chunks(&self.replacement).len();
        if removed_chunks.len() <= replacement_count || after.starts_with('\n') {
            return vec![];
        }
        removed_chunks[replacement_count..].to_vec()
    }
}

fn line_chunks(source: &str) -> Vec<&str> {
    source.split_inclusive('\n').collect()
}

fn trailing_blank_chunks(source: &str) -> Vec<&str> {
    let chunks = line_chunks(source);
    let start =
        chunks.iter().rposition(|chunk| !chunk.trim().is_empty()).map_or(0, |index| index + 1);
    chunks[start..].to_vec()
}

fn leading_blank_chunk_count(source: &str) -> usize {
    line_chunks(source).iter().take_while(|chunk| chunk.trim().is_empty()).count()
}

fn cap_junction(
    head: &str,
    tail: &str,
    preserved_count: usize,
    before: &str,
    replacement: &str,
) -> String {
    let mut head_chunks = line_chunks(head);
    let trailing_count = trailing_blank_chunks(head).len();
    if tail.is_empty() || trailing_count == 0 {
        return format!("{head}{tail}");
    }
    let leading_count = leading_blank_chunk_count(tail);
    let allowed = trailing_blank_chunks(before)
        .len()
        .max(trailing_blank_chunks(replacement).len())
        .max(preserved_count)
        .max(leading_count);
    let excess = (trailing_count + leading_count).saturating_sub(allowed);
    if excess == 0 {
        return format!("{head}{tail}");
    }
    head_chunks.truncate(head_chunks.len() - excess);
    format!("{}{tail}", head_chunks.concat())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryEdge {
    Leading,
    Trailing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuralBoundary {
    pub edge: BoundaryEdge,
    pub owner_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RemovedOwnerAttachments {
    pub owner_id: Option<String>,
    pub leading_region_id: Option<String>,
    pub inline_region_id: Option<String>,
    #[serde(default)]
    pub orphan_region_ids: Vec<String>,
    pub trailing_region_id: Option<String>,
    pub leading_gap_id: Option<String>,
    pub trailing_gap_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RehomeFragmentKind {
    Leading,
    Ambiguous,
    Trailing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RehomePlan {
    pub source_owner_id: Option<String>,
    pub target_boundary: StructuralBoundary,
    pub kind: RehomeFragmentKind,
    pub comment_region_ids: Vec<String>,
    pub layout_gap_ids: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RehomedCommentAttachment {
    pub owner_id: Option<String>,
    pub leading_region_id: Option<String>,
    pub inline_region_id: Option<String>,
    pub trailing_region_id: Option<String>,
    pub orphan_region_ids: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RehomedLayoutAttachment {
    pub owner_id: Option<String>,
    pub leading_gap_id: Option<String>,
    pub trailing_gap_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RehomePlan {
    pub fn target_owner_id(&self) -> Option<&str> {
        self.target_boundary.owner_id.as_deref()
    }

    pub fn edge(&self) -> BoundaryEdge {
        self.target_boundary.edge
    }

    pub fn is_empty(&self) -> bool {
        self.comment_region_ids.is_empty() && self.layout_gap_ids.is_empty()
    }

    pub fn comment_attachment(&self) -> RehomedCommentAttachment {
        let primary = self.comment_region_ids.first().cloned();
        let orphan_region_ids = self.comment_region_ids.iter().skip(1).cloned().collect();
        let (leading_region_id, trailing_region_id) = match self.edge() {
            BoundaryEdge::Leading => (None, primary),
            BoundaryEdge::Trailing => (primary, None),
        };
        RehomedCommentAttachment {
            owner_id: self.target_boundary.owner_id.clone(),
            leading_region_id,
            inline_region_id: None,
            trailing_region_id,
            orphan_region_ids,
            metadata: self.attachment_metadata(),
        }
    }

    pub fn layout_attachment(&self) -> RehomedLayoutAttachment {
        let primary = self.layout_gap_ids.first().cloned();
        let (leading_gap_id, trailing_gap_id) = match self.edge() {
            BoundaryEdge::Leading => (None, primary),
            BoundaryEdge::Trailing => (primary, None),
        };
        RehomedLayoutAttachment {
            owner_id: self.target_boundary.owner_id.clone(),
            leading_gap_id,
            trailing_gap_id,
            metadata: self.attachment_metadata(),
        }
    }

    fn attachment_metadata(&self) -> HashMap<String, serde_json::Value> {
        self.metadata.clone()
    }
}

/// The ordered source lines and physical line numbers retained from a removed owner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RemovedOwnerPreservedLines {
    pub lines: Vec<String>,
    pub line_numbers: Vec<usize>,
}

/// Materialize Ruby ast-merge's removed-owner preservation rule from classified
/// line segments. A trailing layout gap is retained only when its effective
/// controller, after removing this owner, is the gap's after owner.
pub fn preserve_removed_owner_lines(
    leading_segment_lines: &[String],
    leading_segment_line_numbers: &[usize],
    inline_lines: &[String],
    inline_line_numbers: &[usize],
    trailing_region: Option<&crate::CommentRegion>,
    trailing_gap: Option<&crate::LayoutGap>,
    removed_owner_id: &str,
) -> RemovedOwnerPreservedLines {
    let mut preserved = RemovedOwnerPreservedLines {
        lines: leading_segment_lines.to_vec(),
        line_numbers: leading_segment_line_numbers.to_vec(),
    };
    preserved.lines.extend(inline_lines.iter().cloned());
    preserved.line_numbers.extend(inline_line_numbers.iter().copied());
    if let Some(region) = trailing_region {
        preserved.lines.extend(region.text().split_terminator('\n').map(str::to_owned));
        if let (Some(start), Some(end)) = (region.start_line(), region.end_line()) {
            preserved.line_numbers.extend(start..=end);
        }
    }
    if let Some(gap) = trailing_gap {
        let removed = HashSet::from([removed_owner_id]);
        if gap.after_owner_id.as_deref().is_some_and(|after_owner_id| {
            gap.effective_controller_owner_id(&removed) == Some(after_owner_id)
        }) {
            preserved.lines.extend(gap.lines.iter().cloned());
            preserved.line_numbers.extend(gap.start_line..=gap.end_line);
        }
    }
    preserved
}

/// A contiguous source removal plus passive fragment-promotion decisions.
///
/// This mirrors Ruby RemovePlan: it does not mutate attachments, but it records
/// which removed-owner regions and gaps can be reattached at surviving edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovePlan {
    splice_plan: SplicePlan,
    pub leading_boundary: Option<StructuralBoundary>,
    pub trailing_boundary: Option<StructuralBoundary>,
    pub removed_attachments: Vec<RemovedOwnerAttachments>,
    pub removed_owner_ids: Vec<String>,
    pub retained_owner_ids: Vec<String>,
    pub rehome_plans: Vec<RehomePlan>,
}

impl RemovePlan {
    pub fn new(
        source: impl Into<String>,
        remove_start_line: usize,
        remove_end_line: usize,
        leading_boundary: Option<StructuralBoundary>,
        trailing_boundary: Option<StructuralBoundary>,
        removed_attachments: Vec<RemovedOwnerAttachments>,
    ) -> Result<Self, String> {
        if leading_boundary.as_ref().is_some_and(|boundary| boundary.edge != BoundaryEdge::Leading)
        {
            return Err("leading_boundary must use edge leading".into());
        }
        if trailing_boundary
            .as_ref()
            .is_some_and(|boundary| boundary.edge != BoundaryEdge::Trailing)
        {
            return Err("trailing_boundary must use edge trailing".into());
        }
        let splice_plan = SplicePlan::new(source, "", remove_start_line, remove_end_line)?;
        let removed_owner_ids = unique_ids(
            removed_attachments.iter().filter_map(|attachment| attachment.owner_id.clone()),
        );
        let retained_owner_ids = unique_ids(
            leading_boundary
                .iter()
                .chain(trailing_boundary.iter())
                .filter_map(|boundary| boundary.owner_id.clone()),
        );
        let mut plan = Self {
            splice_plan,
            leading_boundary,
            trailing_boundary,
            removed_attachments,
            removed_owner_ids,
            retained_owner_ids,
            rehome_plans: vec![],
        };
        plan.rehome_plans = plan
            .removed_attachments
            .iter()
            .flat_map(|attachment| plan.rehome_plans_for(attachment))
            .collect();
        Ok(plan)
    }

    pub fn source(&self) -> &str {
        self.splice_plan.source()
    }
    pub fn remove_start_line(&self) -> usize {
        self.splice_plan.replace_start_line()
    }
    pub fn remove_end_line(&self) -> usize {
        self.splice_plan.replace_end_line()
    }
    pub fn before_content(&self) -> String {
        self.splice_plan.before_content()
    }
    pub fn removed_content(&self) -> String {
        self.splice_plan.removed_content()
    }
    pub fn after_content(&self) -> String {
        self.splice_plan.after_content()
    }
    pub fn merged_content(&self) -> String {
        self.splice_plan.merged_content()
    }
    pub fn changed(&self) -> bool {
        self.splice_plan.changed()
    }
    pub fn apply_to(&self, alternate_source: &str) -> Result<String, String> {
        self.splice_plan.apply_to(alternate_source)
    }
    /// Override owner inventories with caller-provided identities, matching
    /// Ruby RemovePlan's explicit removed_owners/retained_owners options.
    pub fn with_owner_overrides(
        mut self,
        removed_owner_ids: Option<Vec<String>>,
        retained_owner_ids: Option<Vec<String>>,
    ) -> Self {
        if let Some(owner_ids) = removed_owner_ids {
            self.removed_owner_ids = unique_ids(owner_ids);
        }
        if let Some(owner_ids) = retained_owner_ids {
            self.retained_owner_ids = unique_ids(owner_ids);
        }
        self
    }

    pub fn promoted_comment_region_ids(&self) -> Vec<String> {
        self.rehome_plans.iter().flat_map(|plan| plan.comment_region_ids.iter().cloned()).collect()
    }
    pub fn promoted_layout_gap_ids(&self) -> Vec<String> {
        self.rehome_plans.iter().flat_map(|plan| plan.layout_gap_ids.iter().cloned()).collect()
    }

    fn rehome_plans_for(&self, attachment: &RemovedOwnerAttachments) -> Vec<RehomePlan> {
        let leading_target =
            self.leading_boundary.clone().or_else(|| self.trailing_boundary.clone());
        let trailing_target =
            self.trailing_boundary.clone().or_else(|| self.leading_boundary.clone());
        let leading_comments = attachment.leading_region_id.iter().cloned().collect::<Vec<_>>();
        let leading_gaps = attachment.leading_gap_id.iter().cloned().collect::<Vec<_>>();
        let mut ambiguous_comments =
            attachment.inline_region_id.iter().cloned().collect::<Vec<_>>();
        ambiguous_comments.extend(attachment.orphan_region_ids.iter().cloned());
        let trailing_comments = attachment.trailing_region_id.iter().cloned().collect::<Vec<_>>();
        let trailing_gaps = attachment.trailing_gap_id.iter().cloned().collect::<Vec<_>>();
        [
            (leading_target, RehomeFragmentKind::Leading, leading_comments, leading_gaps),
            (
                self.leading_boundary.clone().or_else(|| self.trailing_boundary.clone()),
                RehomeFragmentKind::Ambiguous,
                ambiguous_comments,
                vec![],
            ),
            (trailing_target, RehomeFragmentKind::Trailing, trailing_comments, trailing_gaps),
        ]
        .into_iter()
        .filter_map(|(target_boundary, kind, comment_region_ids, layout_gap_ids)| {
            let target_boundary = target_boundary?;
            (comment_region_ids.len() + layout_gap_ids.len() > 0).then_some(RehomePlan {
                source_owner_id: attachment.owner_id.clone(),
                target_boundary,
                kind,
                comment_region_ids,
                layout_gap_ids,
                metadata: HashMap::from([
                    (
                        "source".to_string(),
                        serde_json::Value::String("structural_edit_remove_plan".to_string()),
                    ),
                    (
                        "kind".to_string(),
                        serde_json::Value::String(
                            match kind {
                                RehomeFragmentKind::Leading => "leading",
                                RehomeFragmentKind::Ambiguous => "ambiguous",
                                RehomeFragmentKind::Trailing => "trailing",
                            }
                            .to_string(),
                        ),
                    ),
                ]),
            })
        })
        .collect()
    }
}

fn unique_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    ids.into_iter().filter(|id| seen.insert(id.clone())).collect()
}
