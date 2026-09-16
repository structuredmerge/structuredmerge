use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::LayoutGap;

/// Exact source gaps for the conservative native-owner renderer. Unlike the
/// line-based blank-run augmenter, these retain *all* bytes outside AST owners.
/// They do not classify comments or authorize transferring gaps after deletion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceLayoutGap {
    pub id: String,
    pub range: tree_haver::ByteRange,
    pub source_sha256: String,
    pub before_owner_id: Option<String>,
    pub after_owner_id: Option<String>,
    /// The following owner emits its leading gap; the final owner emits the
    /// suffix. An ownerless document has no invented owner/controller.
    pub controller_owner_id: Option<String>,
}

/// One slot before each owner and one final suffix, including empty slots.
/// Callers may omit empty slots from reports, but must not regenerate their
/// nonempty counterparts from whitespace counts or parser hints.
pub fn source_layout_gaps(
    document: &crate::SourcePreservingOwnerDocument,
) -> Result<Vec<SourceLayoutGap>, String> {
    use sha2::{Digest, Sha256};
    document.validate("source layout")?;
    let mut gaps = Vec::with_capacity(document.owners.len() + 1);
    let mut cursor = 0;
    let mut before: Option<String> = None;
    for after in document.owners.iter().map(Some).chain(std::iter::once(None)) {
        let end = after.map_or(document.source.len(), |owner| owner.start_byte);
        let after_id = after.map(|owner| owner.id.clone());
        gaps.push(SourceLayoutGap {
            id: format!("source-gap.{}", gaps.len()),
            range: tree_haver::ByteRange { start_byte: cursor, end_byte: end },
            source_sha256: format!(
                "{:x}",
                Sha256::digest(&document.source.as_bytes()[cursor..end])
            ),
            before_owner_id: before.clone(),
            after_owner_id: after_id.clone(),
            controller_owner_id: after_id.clone().or(before.clone()),
        });
        if let Some(owner) = after {
            cursor = owner.end_byte;
            before = after_id;
        }
    }
    Ok(gaps)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutOwner {
    pub owner_id: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutAttachment {
    pub owner_id: String,
    pub leading_gap_id: Option<String>,
    pub trailing_gap_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutAugmentation {
    pub gaps: Vec<LayoutGap>,
    pub attachments: Vec<LayoutAttachment>,
}

pub fn augment_layout(
    lines: &[String],
    owners: &[LayoutOwner],
) -> Result<LayoutAugmentation, String> {
    let mut owners = owners.to_vec();
    for owner in &owners {
        if owner.owner_id.is_empty() {
            return Err("layout owner id must not be empty".to_string());
        }
        if owner.start_line == 0 || owner.end_line < owner.start_line {
            return Err(format!("layout owner {} has an invalid line range", owner.owner_id));
        }
        if owner.end_line > lines.len() {
            return Err(format!("layout owner {} exceeds the source line count", owner.owner_id));
        }
    }
    owners.sort_by_key(|owner| (owner.start_line, owner.end_line, owner.owner_id.clone()));
    for pair in owners.windows(2) {
        if pair[1].start_line <= pair[0].end_line {
            return Err(format!(
                "layout owners {} and {} overlap",
                pair[0].owner_id, pair[1].owner_id
            ));
        }
    }

    let mut gaps = Vec::new();
    for (start_line, end_line) in blank_runs(lines) {
        let before = owners.iter().rev().find(|owner| owner.end_line + 1 == start_line);
        let after = owners.iter().find(|owner| owner.start_line == end_line + 1);
        if before.is_none() && after.is_none() {
            continue;
        }
        let (kind, controller_side) = match (before, after) {
            (None, Some(_)) => ("preamble", "after"),
            (Some(_), None) => ("postlude", "before"),
            (Some(_), Some(_)) => ("interstitial", "after"),
            (None, None) => unreachable!(),
        };
        gaps.push(LayoutGap {
            id: format!("layout-gap:{start_line}-{end_line}"),
            kind: kind.to_string(),
            start_line,
            end_line,
            lines: lines[(start_line - 1)..end_line].to_vec(),
            before_owner_id: before.map(|owner| owner.owner_id.clone()),
            after_owner_id: after.map(|owner| owner.owner_id.clone()),
            controller_side: controller_side.to_string(),
            metadata: HashMap::from([(
                "source".to_string(),
                serde_json::Value::String("layout_augmenter".to_string()),
            )]),
        });
    }

    let attachments = owners
        .iter()
        .map(|owner| LayoutAttachment {
            owner_id: owner.owner_id.clone(),
            leading_gap_id: gaps
                .iter()
                .find(|gap| gap.after_owner_id.as_deref() == Some(&owner.owner_id))
                .map(|gap| gap.id.clone()),
            trailing_gap_id: gaps
                .iter()
                .find(|gap| gap.before_owner_id.as_deref() == Some(&owner.owner_id))
                .map(|gap| gap.id.clone()),
        })
        .collect();
    Ok(LayoutAugmentation { gaps, attachments })
}

fn blank_runs(lines: &[String]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if !lines[index].trim().is_empty() {
            index += 1;
            continue;
        }
        let start = index;
        while index < lines.len() && lines[index].trim().is_empty() {
            index += 1;
        }
        runs.push((start + 1, index));
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_gaps_partition_exact_bytes_without_comment_or_whitespace_inference() {
        let prefix = "\u{feff}# café\r\n";
        let left = "é: one";
        let middle = "  # untouched\r\n\r\n";
        let right = "beta: two";
        let suffix = " # final";
        let source = format!("{prefix}{left}{middle}{right}{suffix}");
        let owner = |id: &str, start: usize, text: &str| crate::SourcePreservingOwner {
            id: id.into(),
            path: id.into(),
            fingerprint: text.into(),
            start_byte: start,
            end_byte: start + text.len(),
            start_line: 1,
            end_line: 1,
        };
        let document = crate::SourcePreservingOwnerDocument {
            source: source.clone(),
            owners: vec![
                owner("left", prefix.len(), left),
                owner("right", prefix.len() + left.len() + middle.len(), right),
            ],
        };
        let gaps = source_layout_gaps(&document).unwrap();
        assert_eq!(gaps.len(), 3);
        for (gap, expected) in gaps.iter().zip([prefix, middle, suffix]) {
            use sha2::{Digest, Sha256};
            assert_eq!(&source[gap.range.start_byte..gap.range.end_byte], expected);
            assert_eq!(gap.source_sha256, format!("{:x}", Sha256::digest(expected.as_bytes())));
        }
        assert_eq!(gaps[0].before_owner_id, None);
        assert_eq!(gaps[0].controller_owner_id.as_deref(), Some("left"));
        assert_eq!(gaps[1].before_owner_id.as_deref(), Some("left"));
        assert_eq!(gaps[1].after_owner_id.as_deref(), Some("right"));
        assert_eq!(gaps[1].controller_owner_id.as_deref(), Some("right"));
        assert_eq!(gaps[2].after_owner_id, None);
        assert_eq!(gaps[2].controller_owner_id.as_deref(), Some("right"));
        let mut invalid = document.clone();
        invalid.owners[1].start_byte = 0;
        assert!(source_layout_gaps(&invalid).is_err());
        invalid = document;
        invalid.owners[0].start_byte += 1; // inside the UTF-8 owner key
        assert!(source_layout_gaps(&invalid).is_err());
    }

    #[test]
    fn ownerless_and_empty_sources_do_not_invent_controllers() {
        for text in ["", "# comment only\r\n"] {
            let document =
                crate::SourcePreservingOwnerDocument { source: text.into(), owners: vec![] };
            let gaps = source_layout_gaps(&document).unwrap();
            assert_eq!(gaps.len(), 1);
            assert_eq!(gaps[0].range.start_byte, 0);
            assert_eq!(gaps[0].range.end_byte, text.len());
            assert!(gaps[0].controller_owner_id.is_none());
            assert!(gaps[0].before_owner_id.is_none());
            assert!(gaps[0].after_owner_id.is_none());
        }
    }

    #[test]
    fn assigns_shared_gaps_to_one_output_controller() {
        let lines =
            ["", "alpha", "", "", "beta", ""].into_iter().map(str::to_string).collect::<Vec<_>>();
        let augmentation = augment_layout(
            &lines,
            &[
                LayoutOwner { owner_id: "alpha".to_string(), start_line: 2, end_line: 2 },
                LayoutOwner { owner_id: "beta".to_string(), start_line: 5, end_line: 5 },
            ],
        )
        .unwrap();

        assert_eq!(
            augmentation.gaps.iter().map(|gap| gap.kind.as_str()).collect::<Vec<_>>(),
            ["preamble", "interstitial", "postlude"]
        );
        let shared = &augmentation.gaps[1];
        assert_eq!(shared.before_owner_id.as_deref(), Some("alpha"));
        assert_eq!(shared.after_owner_id.as_deref(), Some("beta"));
        assert_eq!(shared.controller_owner_id(), Some("beta"));
        assert_eq!(
            augmentation.attachments[0].trailing_gap_id.as_deref(),
            Some(shared.id.as_str())
        );
        assert_eq!(augmentation.attachments[1].leading_gap_id.as_deref(), Some(shared.id.as_str()));

        let removed = std::collections::HashSet::from(["beta"]);
        assert_eq!(shared.effective_controller_owner_id(&removed), Some("alpha"));
    }

    #[test]
    fn rejects_overlapping_or_out_of_range_owners() {
        let lines = vec!["one".to_string(), "two".to_string()];
        let overlap = augment_layout(
            &lines,
            &[
                LayoutOwner { owner_id: "one".to_string(), start_line: 1, end_line: 2 },
                LayoutOwner { owner_id: "two".to_string(), start_line: 2, end_line: 2 },
            ],
        )
        .unwrap_err();
        assert!(overlap.contains("overlap"));

        let outside = augment_layout(
            &lines,
            &[LayoutOwner { owner_id: "outside".to_string(), start_line: 3, end_line: 3 }],
        )
        .unwrap_err();
        assert!(outside.contains("source line count"));
    }
}
