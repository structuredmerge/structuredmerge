//! Git rendering over validated parser facts, without parser discovery or a host
//! transport. Clean output uses the JSON family's verified edit evidence. Marker
//! output is a review artifact, not reparsed JSON or a resolved merge.
use std::collections::HashMap;

use ast_merge::{
    ConflictLabels, MergeConflict, RenderFragment, SourceRenderResult, SourceRevision,
    ThreeWayMergeOutcome, ThreeWayMergeResult, localized_conflict_render_plan, render_source_plan,
};
use json_merge::{JsonDialect, typed::JsonMergeExecution};
use serde::Serialize;
use tree_haver::{service::ParsedResult, source::SourceDescriptor};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConflictRenderOptions {
    pub marker_size: usize,
    pub labels: ConflictLabels,
}

impl Default for ConflictRenderOptions {
    fn default() -> Self {
        Self { marker_size: 7, labels: ConflictLabels::default() }
    }
}

impl ConflictRenderOptions {
    pub fn validate(&self) -> Result<(), String> {
        // Bound caller-controlled marker allocation and reject label injection.
        // These are protocol limits, not source classification heuristics.
        if !(1..=1024).contains(&self.marker_size) {
            return Err("Git conflict marker size must be between 1 and 1024".into());
        }
        for label in [&self.labels.base, &self.labels.ours, &self.labels.theirs] {
            if label.is_empty() || label.len() > 1024 || label.chars().any(char::is_control) {
                return Err(
                    "Git conflict labels must be nonempty single-line text (max 1024 bytes)".into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConflictRenderEvidence {
    /// In base/ours/theirs order; binds line provenance to exact input identities.
    pub sources: Vec<SourceDescriptor>,
    pub options: ConflictRenderOptions,
    pub rendered: SourceRenderResult,
}

impl ConflictRenderEvidence {
    /// Replay against trusted native conflict decisions and validated inputs.
    /// This checks transport integrity, not the semantic authority of foreign
    /// conflict decisions. In particular it does not claim output reparsing.
    pub fn validate(
        &self,
        inputs: [&ParsedResult; 3],
        conflicts: &[MergeConflict],
        options: &ConflictRenderOptions,
    ) -> Result<(), String> {
        if *self != render_conflicts(inputs, conflicts, options)? {
            return Err("Git conflict render differs from its input-bound replay".into());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct GitMergeExecution {
    pub merge: JsonMergeExecution<ThreeWayMergeResult<String>>,
    pub conflict_render: Option<ConflictRenderEvidence>,
    /// Unrenderable conflicts remain conflicts; never fall back to whole-file
    /// markers or pretend that the unchanged ours source is a merged output.
    pub conflict_render_error: Option<String>,
}

pub fn merge3(
    base: &ParsedResult,
    ours: &ParsedResult,
    theirs: &ParsedResult,
    dialect: JsonDialect,
    options: &ConflictRenderOptions,
    parse_output: impl FnMut(&str) -> Result<ParsedResult, String>,
) -> Result<GitMergeExecution, String> {
    // Validate even on a clean result, so unsupported options cannot be silently
    // accepted depending on the source contents.
    options.validate()?;
    let merge = json_merge::typed::merge3_with_evidence(base, ours, theirs, dialect, parse_output)?;
    let (conflict_render, conflict_render_error) =
        if merge.result.outcome == ThreeWayMergeOutcome::Conflict {
            match render_conflicts([base, ours, theirs], &merge.result.conflicts, options) {
                Ok(render) => (Some(render), None),
                Err(error) => (None, Some(error)),
            }
        } else {
            (None, None)
        };
    Ok(GitMergeExecution { merge, conflict_render, conflict_render_error })
}

fn render_conflicts(
    inputs: [&ParsedResult; 3],
    conflicts: &[MergeConflict],
    options: &ConflictRenderOptions,
) -> Result<ConflictRenderEvidence, String> {
    use tree_haver::source::SourceRole;
    options.validate()?;
    let mut sources = HashMap::new();
    let mut identities = std::collections::HashSet::new();
    for ((role, revision), parsed) in [
        (SourceRole::Base, SourceRevision::Base),
        (SourceRole::Ours, SourceRevision::Ours),
        (SourceRole::Theirs, SourceRevision::Theirs),
    ]
    .into_iter()
    .zip(inputs)
    {
        let descriptor = parsed.source.descriptor();
        if descriptor.role != role
            || !identities.insert(&descriptor.source_id)
            || !parsed.document.output().ok
            || parsed.document.output().source != *descriptor
        {
            return Err(
                "Git conflict render requires distinct, validated base/ours/theirs inputs".into()
            );
        }
        sources.insert(
            revision,
            std::str::from_utf8(parsed.source.bytes()).map_err(|e| e.to_string())?.to_string(),
        );
    }
    let mut plan = localized_conflict_render_plan(sources, conflicts, options.marker_size)
        .map_err(|error| error.to_string())?;
    for fragment in &mut plan.fragments {
        if let RenderFragment::Conflict(fragment) = fragment {
            fragment.labels = options.labels.clone();
        }
    }
    let rendered = render_source_plan(&plan).map_err(|error| error.to_string())?;
    Ok(ConflictRenderEvidence {
        sources: inputs.map(|parsed| parsed.source.descriptor().clone()).to_vec(),
        options: options.clone(),
        rendered,
    })
}
