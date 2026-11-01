use serde::Deserialize;

/// Inputs to Ruby ast-merge's retained-owner leading-segment decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct LeadingSegmentEmissionCase {
    pub name: String,
    pub source_lines: Vec<String>,
    pub output_lines: Vec<String>,
    pub source_region_start_line: usize,
    pub output_owner_start_line: usize,
    pub target_leading_region_start_line: Option<usize>,
    pub previous_owner_trailing_region_matches: bool,
    pub expected_line_numbers: Vec<usize>,
}

impl LeadingSegmentEmissionCase {
    /// Select source line numbers before the output owner, preserving the
    /// blank-prefix rule used by Ruby CommentLayoutEmissionSupport.
    pub fn selected_line_numbers(&self) -> Result<Vec<usize>, String> {
        if self.source_region_start_line == 0 || self.output_owner_start_line == 0 {
            return Err("leading segment line numbers are 1-based".to_string());
        }
        let desired_blank_count = if self
            .target_leading_region_start_line
            .is_some_and(|line| line < self.output_owner_start_line)
        {
            let line = self.target_leading_region_start_line.expect("checked above");
            blank_line_count_before(&self.output_lines, line)?
        } else if self.previous_owner_trailing_region_matches {
            0
        } else {
            blank_line_count_before(&self.source_lines, self.source_region_start_line)?
        };
        let start_line = self
            .source_region_start_line
            .checked_sub(desired_blank_count)
            .ok_or_else(|| "leading segment starts before the source".to_string())?;
        if start_line >= self.output_owner_start_line {
            return Ok(Vec::new());
        }
        Ok((start_line..self.output_owner_start_line).collect())
    }
}

fn blank_line_count_before(lines: &[String], line_number: usize) -> Result<usize, String> {
    if line_number > lines.len() {
        return Err(format!("line {line_number} exceeds source length {}", lines.len()));
    }
    Ok(lines[..line_number - 1].iter().rev().take_while(|line| line.trim().is_empty()).count())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootBoundaryOwnerLines {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootBoundaryEmissionCase {
    pub name: String,
    pub kind: String,
    pub lines: Vec<String>,
    pub owners: Vec<RootBoundaryOwnerLines>,
    pub preamble_region: Option<RootBoundaryOwnerLines>,
    pub postlude_region: Option<RootBoundaryOwnerLines>,
    pub fallback_to_owner_bounds: bool,
    pub expected_line_numbers: Vec<usize>,
}

impl RootBoundaryEmissionCase {
    /// Select root-boundary lines using Ruby ast-merge's preamble/postlude rules.
    pub fn selected_line_numbers(&self) -> Result<Vec<usize>, String> {
        if self.kind != "preamble" && self.kind != "postlude" {
            return Err(format!("unknown root boundary kind {}", self.kind));
        }
        for owner in &self.owners {
            if owner.start_line == 0
                || owner.end_line < owner.start_line
                || owner.end_line > self.lines.len()
            {
                return Err("root boundary owner has an invalid line range".to_string());
            }
        }
        let region = if self.kind == "preamble" {
            self.preamble_region.as_ref()
        } else {
            self.postlude_region.as_ref()
        };
        if self.kind == "preamble" && self.owners.is_empty() {
            return Ok((1..=self.lines.len()).collect());
        }
        if region.is_none() && !self.fallback_to_owner_bounds {
            return Ok(Vec::new());
        }
        if region.is_some_and(|region| {
            region.start_line == 0
                || region.end_line < region.start_line
                || region.end_line > self.lines.len()
        }) {
            return Err("root boundary region has an invalid line range".to_string());
        }
        let selected = if region.is_some() {
            match self.kind.as_str() {
                "preamble" => {
                    let end = self
                        .owners
                        .iter()
                        .map(|owner| owner.start_line)
                        .min()
                        .map_or(self.lines.len(), |line| line.saturating_sub(1));
                    1..=end
                }
                "postlude" => {
                    let start = self
                        .owners
                        .iter()
                        .map(|owner| owner.end_line)
                        .max()
                        .map_or(region.map_or(1, |region| region.start_line), |line| line + 1);
                    start..=self.lines.len()
                }
                _ => unreachable!(),
            }
        } else {
            match self.kind.as_str() {
                "preamble" => {
                    let Some(first) = self.owners.iter().map(|owner| owner.start_line).min() else {
                        return Ok(Vec::new());
                    };
                    if first <= 1 {
                        return Ok(Vec::new());
                    }
                    1..=first - 1
                }
                "postlude" => {
                    let Some(last) = self.owners.iter().map(|owner| owner.end_line).max() else {
                        return Ok(Vec::new());
                    };
                    if last >= self.lines.len() {
                        return Ok(Vec::new());
                    }
                    last + 1..=self.lines.len()
                }
                _ => unreachable!(),
            }
        };
        if selected.start() > selected.end() || *selected.start() == 0 {
            return Ok(Vec::new());
        }
        Ok(selected.collect())
    }
}
