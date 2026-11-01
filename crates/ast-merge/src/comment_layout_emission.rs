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
