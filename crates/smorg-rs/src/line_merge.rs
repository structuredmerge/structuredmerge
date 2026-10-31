//! Conservative line-based three-way merge for files without a structural
//! provider. Unlike a block/template merge it uses the ancestor, never rewrites
//! or re-joins lines, and reports overlapping edits as conflict hunks.

const MAX_DP_CELLS: usize = 40_000_000;

pub struct LineMerge {
    pub output: String,
    pub conflicted: bool,
}

/// Returns `None` when the inputs are too large for the quadratic diff; the
/// caller should then fall back to a whole-file conflict.
pub fn merge3_lines(base: &str, ours: &str, theirs: &str, marker_size: usize) -> Option<LineMerge> {
    if ours == theirs {
        return Some(LineMerge { output: ours.to_string(), conflicted: false });
    }
    if ours == base {
        return Some(LineMerge { output: theirs.to_string(), conflicted: false });
    }
    if theirs == base {
        return Some(LineMerge { output: ours.to_string(), conflicted: false });
    }
    let b: Vec<&str> = base.split_inclusive('\n').collect();
    let o: Vec<&str> = ours.split_inclusive('\n').collect();
    let t: Vec<&str> = theirs.split_inclusive('\n').collect();
    let mo = matches(&b, &o)?;
    let mt = matches(&b, &t)?;

    // Sync points: base lines kept by both sides, as (base, ours, theirs).
    let mut sync: Vec<(usize, usize, usize)> = Vec::new();
    for (i, (x, y)) in mo.iter().zip(mt.iter()).enumerate() {
        if let (Some(x), Some(y)) = (x, y) {
            sync.push((i, *x, *y));
        }
    }
    sync.push((b.len(), o.len(), t.len()));

    let marker = marker_size.max(1);
    let mut out = String::new();
    let mut conflicted = false;
    let (mut pb, mut po, mut pt) = (0usize, 0usize, 0usize);
    for (sb, so, st) in sync {
        let (cb, co, ct) = (&b[pb..sb], &o[po..so], &t[pt..st]);
        if co == ct || ct == cb {
            co.iter().for_each(|l| out.push_str(l));
        } else if co == cb {
            ct.iter().for_each(|l| out.push_str(l));
        } else {
            conflicted = true;
            push_marker(&mut out, '<', marker, " ours");
            co.iter().for_each(|l| out.push_str(l));
            ensure_newline(&mut out);
            push_marker(&mut out, '|', marker, " base");
            cb.iter().for_each(|l| out.push_str(l));
            ensure_newline(&mut out);
            push_marker(&mut out, '=', marker, "");
            ct.iter().for_each(|l| out.push_str(l));
            ensure_newline(&mut out);
            push_marker(&mut out, '>', marker, " theirs");
        }
        if sb < b.len() {
            out.push_str(o[so]);
            pb = sb + 1;
            po = so + 1;
            pt = st + 1;
        }
    }
    Some(LineMerge { output: out, conflicted })
}

fn push_marker(out: &mut String, ch: char, size: usize, label: &str) {
    out.extend(std::iter::repeat_n(ch, size));
    out.push_str(label);
    out.push('\n');
}

fn ensure_newline(out: &mut String) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

/// For each base line, the index of its LCS partner in `other`.
fn matches(base: &[&str], other: &[&str]) -> Option<Vec<Option<usize>>> {
    let mut result = vec![None; base.len()];
    let mut start = 0;
    while start < base.len() && start < other.len() && base[start] == other[start] {
        result[start] = Some(start);
        start += 1;
    }
    let (mut be, mut oe) = (base.len(), other.len());
    while be > start && oe > start && base[be - 1] == other[oe - 1] {
        be -= 1;
        oe -= 1;
        result[be] = Some(oe);
    }
    let (n, m) = (be - start, oe - start);
    if n == 0 || m == 0 {
        return Some(result);
    }
    if (n + 1).saturating_mul(m + 1) > MAX_DP_CELLS {
        return None;
    }
    let width = m + 1;
    let mut dp = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * width + j] = if base[start + i] == other[start + j] {
                dp[(i + 1) * width + j + 1] + 1
            } else {
                dp[(i + 1) * width + j].max(dp[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if base[start + i] == other[start + j] {
            result[start + i] = Some(start + j);
            i += 1;
            j += 1;
        } else if dp[(i + 1) * width + j] >= dp[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(b: &str, o: &str, t: &str) -> LineMerge {
        merge3_lines(b, o, t, 7).expect("merge")
    }

    #[test]
    fn disjoint_edits_merge_cleanly_and_keep_lines() {
        let r = merge(
            "#!/bin/sh\n# tags\n\ncmd a\ncmd b\n",
            "#!/bin/sh\n# tags2\n\ncmd a\ncmd b\n",
            "#!/bin/sh\n# tags\n\ncmd a\ncmd b2\n",
        );
        assert!(!r.conflicted);
        assert_eq!(r.output, "#!/bin/sh\n# tags2\n\ncmd a\ncmd b2\n");
    }

    #[test]
    fn overlapping_edits_conflict_with_markers() {
        let r = merge("a\nb\nc\n", "a\nX\nc\n", "a\nY\nc\n");
        assert!(r.conflicted);
        assert_eq!(
            r.output,
            "a\n<<<<<<< ours\nX\n||||||| base\nb\n=======\nY\n>>>>>>> theirs\nc\n"
        );
    }

    #[test]
    fn identical_edits_and_missing_final_newline_are_preserved() {
        let r = merge("a\nb", "a\nB", "a\nB");
        assert!(!r.conflicted);
        assert_eq!(r.output, "a\nB");
    }

    #[test]
    fn never_collapses_newlines() {
        let r = merge("# c\nx\n", "# c\nx\ny\n", "# c2\nx\n");
        assert!(!r.conflicted);
        assert_eq!(r.output, "# c2\nx\ny\n");
    }
}
