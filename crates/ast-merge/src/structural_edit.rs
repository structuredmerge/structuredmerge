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
