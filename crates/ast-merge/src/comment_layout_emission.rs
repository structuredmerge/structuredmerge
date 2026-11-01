use serde::Deserialize;

/// Inputs for Ruby ast-merge's retained-owner leading-segment decision.
pub struct LeadingSegmentEmissionInput<'a> {
    pub source_lines: &'a [String],
    pub output_lines: &'a [String],
    pub source_region_start_line: usize,
    pub output_owner_start_line: usize,
    pub target_leading_region_start_line: Option<usize>,
    pub previous_owner_trailing_region_matches: bool,
}

/// Select source line numbers before an output owner using Ruby ast-merge's
/// leading-segment blank-prefix rule.
pub fn leading_segment_line_numbers(
    input: &LeadingSegmentEmissionInput<'_>,
) -> Result<Vec<usize>, String> {
    if input.source_region_start_line == 0
        || input.source_region_start_line > input.source_lines.len()
    {
        return Err("source comment region has an invalid start line".to_string());
    }
    if input.output_owner_start_line == 0
        || input.output_owner_start_line > input.output_lines.len() + 1
    {
        return Err("output owner has an invalid start line".to_string());
    }
    if input
        .target_leading_region_start_line
        .is_some_and(|line| line == 0 || line > input.output_lines.len())
    {
        return Err("target leading region has an invalid start line".to_string());
    }
    let desired_blank_count = if input
        .target_leading_region_start_line
        .is_some_and(|line| line < input.output_owner_start_line)
    {
        blank_line_count_before(
            input.output_lines,
            input.target_leading_region_start_line.unwrap(),
        )?
    } else if input.previous_owner_trailing_region_matches {
        0
    } else {
        blank_line_count_before(input.source_lines, input.source_region_start_line)?
    };
    let start_line = input
        .source_region_start_line
        .checked_sub(desired_blank_count)
        .ok_or_else(|| "leading segment starts before the source".to_string())?;
    if start_line >= input.output_owner_start_line {
        return Ok(Vec::new());
    }
    Ok((start_line..input.output_owner_start_line).collect())
}

fn blank_line_count_before(lines: &[String], line_number: usize) -> Result<usize, String> {
    if line_number == 0 || line_number > lines.len() {
        return Err(format!("line {line_number} exceeds source length {}", lines.len()));
    }
    Ok(lines[..line_number - 1].iter().rev().take_while(|line| line.trim().is_empty()).count())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootBoundaryKind {
    Preamble,
    Postlude,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct OwnerLineRange {
    pub start_line: usize,
    pub end_line: usize,
}

/// Select root preamble or postlude lines using Ruby ast-merge's boundary rule.
pub fn root_boundary_line_numbers(
    kind: RootBoundaryKind,
    line_count: usize,
    owners: &[OwnerLineRange],
    comment_region: Option<OwnerLineRange>,
    fallback_to_owner_bounds: bool,
) -> Result<Vec<usize>, String> {
    for owner in owners {
        validate_line_range(*owner, line_count, "root boundary owner")?;
    }
    if let Some(region) = comment_region {
        validate_line_range(region, line_count, "root boundary comment region")?;
    }
    if kind == RootBoundaryKind::Preamble && owners.is_empty() {
        return Ok((1..=line_count).collect());
    }
    if comment_region.is_none() && !fallback_to_owner_bounds {
        return Ok(Vec::new());
    }
    let (start, end) = match (kind, comment_region.is_some()) {
        (RootBoundaryKind::Preamble, true) => (
            1,
            owners
                .iter()
                .map(|owner| owner.start_line)
                .min()
                .map_or(line_count, |line| line.saturating_sub(1)),
        ),
        (RootBoundaryKind::Postlude, true) => (
            owners
                .iter()
                .map(|owner| owner.end_line)
                .max()
                .map_or(comment_region.map_or(1, |region| region.start_line), |line| line + 1),
            line_count,
        ),
        (RootBoundaryKind::Preamble, false) => {
            let Some(first) = owners.iter().map(|owner| owner.start_line).min() else {
                return Ok(Vec::new());
            };
            (1, first.saturating_sub(1))
        }
        (RootBoundaryKind::Postlude, false) => {
            let Some(last) = owners.iter().map(|owner| owner.end_line).max() else {
                return Ok(Vec::new());
            };
            (last.saturating_add(1), line_count)
        }
    };
    if start == 0 || start > end {
        return Ok(Vec::new());
    }
    Ok((start..=end).collect())
}

fn validate_line_range(
    range: OwnerLineRange,
    line_count: usize,
    label: &str,
) -> Result<(), String> {
    if range.start_line == 0 || range.end_line < range.start_line || range.end_line > line_count {
        return Err(format!("{label} has an invalid line range"));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SourceLineOwner {
    pub id: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SourceLineCommentRegion {
    pub id: String,
    pub selection_key: String,
    pub owner_id: Option<String>,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SourceLineLayoutGap {
    pub id: String,
    pub selection_key: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub before_owner_id: Option<String>,
    pub after_owner_id: Option<String>,
    pub controller_side: String,
    pub controller_owner_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct SourceLineDisposition {
    pub line_number: usize,
    pub line: String,
    pub kind: String,
    pub selection_key: String,
    pub owner_id: Option<String>,
    pub comment_region_ids: Vec<String>,
    pub comment_kind: Option<String>,
    pub gap_id: Option<String>,
    pub controller_owner_id: Option<String>,
    pub root_region_kind: Option<String>,
}

/// Compose native comment and layout augmenter output into the shared per-line
/// ownership rubric. Selection keys are semantic: owner/comment kind, root
/// boundary kind, or the gap's adjacent owners and controller.
pub fn source_line_dispositions_from_augmentations(
    source: &str,
    owners: &[crate::LayoutOwner],
    comments: &crate::CommentAugmentation,
    layout: &crate::LayoutAugmentation,
) -> Result<Vec<SourceLineDisposition>, String> {
    let mut attached = std::collections::HashMap::<String, (String, String)>::new();
    for attachment in &comments.attachments {
        for (kind, region_id) in [
            ("leading", attachment.leading_region_id.as_ref()),
            ("inline", attachment.inline_region_id.as_ref()),
            ("trailing", attachment.trailing_region_id.as_ref()),
        ] {
            if let Some(region_id) = region_id
                && attached
                    .insert(region_id.clone(), (attachment.owner_id.clone(), kind.to_string()))
                    .is_some()
            {
                return Err(format!("comment region {region_id} has multiple owner attachments"));
            }
        }
    }

    let mut roots = std::collections::HashMap::<String, (String, String)>::new();
    for (kind, region_id) in [
        ("preamble", comments.preamble_region_id.as_ref()),
        ("postlude", comments.postlude_region_id.as_ref()),
    ] {
        if let Some(region_id) = region_id
            && roots.insert(region_id.clone(), (kind.to_string(), format!("root:{kind}"))).is_some()
        {
            return Err(format!("comment region {region_id} has multiple root assignments"));
        }
    }
    for (index, region_id) in comments.orphan_region_ids.iter().enumerate() {
        if roots
            .insert(region_id.clone(), ("orphan".to_string(), format!("root:orphan:{index}")))
            .is_some()
        {
            return Err(format!("comment region {region_id} has multiple root assignments"));
        }
    }

    let region_ids = comments
        .regions
        .iter()
        .map(|region| region.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if region_ids.len() != comments.regions.len() {
        return Err("comment augmentation contains duplicate region identities".to_string());
    }
    for region_id in attached.keys().chain(roots.keys()) {
        if !region_ids.contains(region_id.as_str()) {
            return Err(format!("comment assignment refers to unknown region {region_id}"));
        }
    }
    if attached.len() + roots.len() != region_ids.len() {
        return Err("comment augmentation has missing or duplicate region assignments".to_string());
    }
    let mut comment_regions = Vec::with_capacity(comments.regions.len());
    for region in &comments.regions {
        if attached.contains_key(&region.id) == roots.contains_key(&region.id) {
            return Err(format!("comment region {} has ambiguous ownership", region.id));
        }
        let (owner_id, kind, selection_key) =
            if let Some((owner_id, kind)) = attached.get(&region.id) {
                (Some(owner_id.clone()), kind.clone(), format!("comment:{owner_id}:{kind}"))
            } else if let Some((kind, selection_key)) = roots.get(&region.id) {
                (None, kind.clone(), selection_key.clone())
            } else {
                return Err(format!(
                    "comment region {} has no attachment or root disposition",
                    region.id
                ));
            };
        let start_line = region
            .start_line()
            .ok_or_else(|| format!("comment region {} has no physical source lines", region.id))?;
        let end_line = region
            .end_line()
            .ok_or_else(|| format!("comment region {} has no physical source lines", region.id))?;
        comment_regions.push(SourceLineCommentRegion {
            id: region.id.clone(),
            selection_key,
            owner_id,
            kind,
            start_line,
            end_line,
        });
    }
    let layout_gaps = layout
        .gaps
        .iter()
        .map(|gap| {
            let before = gap.before_owner_id.as_deref().unwrap_or("root");
            let after = gap.after_owner_id.as_deref().unwrap_or("root");
            SourceLineLayoutGap {
                id: gap.id.clone(),
                selection_key: format!("gap:{}:{before}:{after}", gap.kind),
                kind: gap.kind.clone(),
                start_line: gap.start_line,
                end_line: gap.end_line,
                before_owner_id: gap.before_owner_id.clone(),
                after_owner_id: gap.after_owner_id.clone(),
                controller_side: gap.controller_side.clone(),
                controller_owner_id: gap.controller_owner_id().map(str::to_string),
            }
        })
        .collect::<Vec<_>>();
    let line_owners = owners
        .iter()
        .map(|owner| SourceLineOwner {
            id: owner.owner_id.clone(),
            start_line: owner.start_line,
            end_line: owner.end_line,
        })
        .collect::<Vec<_>>();
    source_line_dispositions(source, &line_owners, &comment_regions, &layout_gaps)
}

/// Assign each physical line exactly one disposition from owner, comment,
/// layout-gap, and root-region facts. Unclaimed lines remain explicitly unmanaged.
pub fn source_line_dispositions(
    source: &str,
    owners: &[SourceLineOwner],
    comment_regions: &[SourceLineCommentRegion],
    layout_gaps: &[SourceLineLayoutGap],
) -> Result<Vec<SourceLineDisposition>, String> {
    let lines = source.split_inclusive('\n').map(str::to_string).collect::<Vec<_>>();
    let line_count = lines.len();
    let mut owner_ids = std::collections::HashSet::new();
    let mut assigned = vec![None::<SourceLineDisposition>; line_count];
    let empty_record = |line_number: usize, line: &str| SourceLineDisposition {
        line_number,
        line: line.to_string(),
        kind: "unmanaged".to_string(),
        selection_key: format!("unmanaged:{line_number}"),
        owner_id: None,
        comment_region_ids: Vec::new(),
        comment_kind: None,
        gap_id: None,
        controller_owner_id: None,
        root_region_kind: None,
    };
    for owner in owners {
        if owner.id.is_empty() || !owner_ids.insert(owner.id.as_str()) {
            return Err("source line owners have an empty or duplicate identity".to_string());
        }
        validate_line_range(
            OwnerLineRange { start_line: owner.start_line, end_line: owner.end_line },
            line_count,
            "source line owner",
        )?;
        for line_number in owner.start_line..=owner.end_line {
            let slot = &mut assigned[line_number - 1];
            if slot.is_some() {
                return Err(format!(
                    "source line {line_number} belongs to multiple structural owners"
                ));
            }
            let mut disposition = empty_record(line_number, &lines[line_number - 1]);
            disposition.kind = "structural_owner".to_string();
            disposition.selection_key = format!("owner:{}", owner.id);
            disposition.owner_id = Some(owner.id.clone());
            *slot = Some(disposition);
        }
    }
    let mut region_ids = std::collections::HashSet::new();
    for region in comment_regions {
        if region.id.is_empty()
            || region.selection_key.is_empty()
            || !region_ids.insert(region.id.as_str())
        {
            return Err("source comment regions have an empty or duplicate identity".to_string());
        }
        validate_line_range(
            OwnerLineRange { start_line: region.start_line, end_line: region.end_line },
            line_count,
            "source comment region",
        )?;
        let root_region = region.owner_id.is_none();
        if let Some(owner_id) = &region.owner_id {
            if !owner_ids.contains(owner_id.as_str()) {
                return Err(format!(
                    "source comment region {} refers to an unknown owner",
                    region.id
                ));
            }
            if !matches!(region.kind.as_str(), "leading" | "inline" | "trailing") {
                return Err(format!(
                    "source comment region {} has an invalid attached kind",
                    region.id
                ));
            }
        } else if !matches!(region.kind.as_str(), "preamble" | "postlude" | "orphan") {
            return Err(format!("source comment region {} has an invalid root kind", region.id));
        }
        for line_number in region.start_line..=region.end_line {
            let slot = &mut assigned[line_number - 1];
            if let Some(disposition) = slot {
                if disposition.kind != "structural_owner"
                    || disposition.owner_id.as_ref() != region.owner_id.as_ref()
                    || root_region
                {
                    return Err(format!(
                        "source line {line_number} has conflicting owner and comment assignments"
                    ));
                }
                if disposition.comment_kind.as_ref().is_some_and(|kind| kind != &region.kind) {
                    return Err(format!("source line {line_number} has conflicting comment kinds"));
                }
                disposition.comment_region_ids.push(region.id.clone());
                disposition.comment_kind = Some(region.kind.clone());
            } else {
                let mut disposition = empty_record(line_number, &lines[line_number - 1]);
                disposition.kind =
                    if root_region { "root_region" } else { "comment_owner" }.to_string();
                disposition.owner_id = region.owner_id.clone();
                disposition.selection_key = region.selection_key.clone();
                disposition.comment_region_ids.push(region.id.clone());
                if root_region {
                    disposition.root_region_kind = Some(region.kind.clone());
                } else {
                    disposition.comment_kind = Some(region.kind.clone());
                }
                *slot = Some(disposition);
            }
        }
    }
    let mut gap_ids = std::collections::HashSet::new();
    for gap in layout_gaps {
        if gap.id.is_empty() || gap.selection_key.is_empty() || !gap_ids.insert(gap.id.as_str()) {
            return Err("source layout gaps have an empty or duplicate identity".to_string());
        }
        validate_line_range(
            OwnerLineRange { start_line: gap.start_line, end_line: gap.end_line },
            line_count,
            "source layout gap",
        )?;
        if !matches!(gap.kind.as_str(), "preamble" | "interstitial" | "postlude")
            || (gap.before_owner_id.is_none() && gap.after_owner_id.is_none())
        {
            return Err(format!("source layout gap {} has invalid ownership edges", gap.id));
        }
        for owner_id in gap.before_owner_id.iter().chain(gap.after_owner_id.iter()) {
            if !owner_ids.contains(owner_id.as_str()) {
                return Err(format!("source layout gap {} refers to an unknown owner", gap.id));
            }
        }
        let expected_controller = match gap.controller_side.as_str() {
            "before" => gap.before_owner_id.as_ref(),
            "after" => gap.after_owner_id.as_ref(),
            _ => {
                return Err(format!("source layout gap {} has an invalid controller side", gap.id));
            }
        };
        if expected_controller.is_none() || expected_controller != gap.controller_owner_id.as_ref()
        {
            return Err(format!("source layout gap {} has an inconsistent controller", gap.id));
        }
        for line_number in gap.start_line..=gap.end_line {
            if !lines[line_number - 1].trim().is_empty() {
                return Err(format!(
                    "source layout gap {} contains nonblank line {line_number}",
                    gap.id
                ));
            }
            let slot = &mut assigned[line_number - 1];
            if slot.is_some() {
                return Err(format!(
                    "source line {line_number} has conflicting layout-gap assignment"
                ));
            }
            let mut disposition = empty_record(line_number, &lines[line_number - 1]);
            disposition.kind = "layout_gap".to_string();
            disposition.selection_key = gap.selection_key.clone();
            disposition.gap_id = Some(gap.id.clone());
            disposition.controller_owner_id = gap.controller_owner_id.clone();
            *slot = Some(disposition);
        }
    }
    Ok(lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            assigned[index].take().unwrap_or_else(|| empty_record(index + 1, line))
        })
        .collect())
}
