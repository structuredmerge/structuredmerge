use std::{collections::HashMap, error::Error, fmt};

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
                    if index + 1 < self.fragments.len() && !content.ends_with('\n') {
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
                        for child in side {
                            let content = match child {
                                ConflictSideFragment::Source(source) => {
                                    self.source_content(source)?
                                }
                                ConflictSideFragment::Synthesized(fragment) => {
                                    fragment.content.clone()
                                }
                            };
                            if !content.is_empty() && !content.ends_with('\n') {
                                return Err(SourceRenderError::new(
                                    "every conflict-side fragment must end at a line boundary",
                                ));
                            }
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
                required_conflict_region(conflict, SourceRevision::Base)?,
                required_conflict_region(conflict, SourceRevision::Ours)?,
                required_conflict_region(conflict, SourceRevision::Theirs)?,
            ))
        })
        .collect::<Result<Vec<_>, SourceRenderError>>()?;
    localized.sort_by_key(|(_, _, ours, _)| (ours.start_line, ours.end_line));

    let mut fragments = Vec::new();
    let mut next_ours_line = 1;
    for (conflict, base, ours, theirs) in localized {
        if ours.start_line < next_ours_line {
            return Err(SourceRenderError::new(format!(
                "conflict {} overlaps a prior ours-owned line region",
                conflict.conflict_id
            )));
        }
        if ours.end_line > ours_line_count {
            return Err(SourceRenderError::new(format!(
                "conflict {} exceeds the ours source line count",
                conflict.conflict_id
            )));
        }
        if next_ours_line < ours.start_line {
            fragments.push(RenderFragment::Source(SourceFragment {
                revision: SourceRevision::Ours,
                start_line: next_ours_line,
                end_line: ours.start_line - 1,
                metadata: HashMap::new(),
            }));
        }
        fragments.push(RenderFragment::Conflict(ConflictFragment {
            conflict_id: conflict.conflict_id.clone(),
            base: vec![source_conflict_side(SourceRevision::Base, base)],
            ours: vec![source_conflict_side(SourceRevision::Ours, ours)],
            theirs: vec![source_conflict_side(SourceRevision::Theirs, theirs)],
            labels: ConflictLabels::default(),
            marker_size,
            metadata: HashMap::from([
                ("category".to_string(), serde_json::json!(conflict.category)),
                ("path".to_string(), serde_json::json!(conflict.path)),
            ]),
        }));
        next_ours_line = ours.end_line + 1;
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

fn required_conflict_region(
    conflict: &crate::MergeConflict,
    revision: SourceRevision,
) -> Result<&OwnedSourceRegion, SourceRenderError> {
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
    if alternative.state != ConflictAlternativeState::Present || alternative.regions.len() != 1 {
        return Err(SourceRenderError::new(format!(
            "conflict {} does not have one present {revision:?} source region",
            conflict.conflict_id
        )));
    }
    let region = &alternative.regions[0];
    region.validate()?;
    Ok(region)
}

fn source_conflict_side(
    revision: SourceRevision,
    region: &OwnedSourceRegion,
) -> ConflictSideFragment {
    ConflictSideFragment::Source(SourceFragment {
        revision,
        start_line: region.start_line,
        end_line: region.end_line,
        metadata: HashMap::from([
            ("start_byte".to_string(), serde_json::json!(region.start_byte)),
            ("end_byte".to_string(), serde_json::json!(region.end_byte)),
        ]),
    })
}

struct SourcePlanRenderer<'a> {
    plan: &'a SourceRenderPlan,
    content: String,
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
            output_end_line: self.line_records.len(),
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
        self.line_records.len() + 1
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
            regions: vec![OwnedSourceRegion { start_byte, end_byte, start_line, end_line }],
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
