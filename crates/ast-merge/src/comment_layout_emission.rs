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
