use std::collections::{HashMap, HashSet};

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

/// A comment-region edge that can control a blank gap even when comments
/// separate the gap from the structural owner span.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutCommentBoundary {
    pub owner_id: String,
    /// `leading` anchors the after-owner edge; `trailing` anchors before-owner.
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutAttachment {
    pub owner_id: String,
    pub leading_gap_id: Option<String>,
    pub trailing_gap_id: Option<String>,
}

impl LayoutAttachment {
    pub fn leading_controls_output(&self, gaps: &[LayoutGap]) -> bool {
        self.leading_controls_output_after_removal(gaps, &HashSet::new())
    }

    pub fn trailing_controls_output(&self, gaps: &[LayoutGap]) -> bool {
        self.trailing_controls_output_after_removal(gaps, &HashSet::new())
    }

    pub fn leading_controls_output_after_removal(
        &self,
        gaps: &[LayoutGap],
        removed_owners: &HashSet<&str>,
    ) -> bool {
        self.controls_gap_output(&self.leading_gap_id, gaps, removed_owners)
    }

    pub fn trailing_controls_output_after_removal(
        &self,
        gaps: &[LayoutGap],
        removed_owners: &HashSet<&str>,
    ) -> bool {
        self.controls_gap_output(&self.trailing_gap_id, gaps, removed_owners)
    }

    fn controls_gap_output(
        &self,
        gap_id: &Option<String>,
        gaps: &[LayoutGap],
        removed_owners: &HashSet<&str>,
    ) -> bool {
        gap_id.as_deref().and_then(|id| gaps.iter().find(|gap| gap.id == id)).is_some_and(|gap| {
            gap.controls_output_for_after_removal(&self.owner_id, removed_owners)
        })
    }
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
    augment_layout_with_comment_boundaries(lines, owners, &[])
}

/// Infer blank-line ownership from structural owners and attached comment
/// boundaries. Leading/trailing comment regions act as edges of their owner,
/// so a blank run between comment regions is controlled by the neighboring
/// owner's normal layout policy.
pub fn augment_layout_with_comment_augmentation(
    lines: &[String],
    owners: &[LayoutOwner],
    comments: &crate::CommentAugmentation,
) -> Result<LayoutAugmentation, String> {
    let regions = comments
        .regions
        .iter()
        .map(|region| (region.id.as_str(), region))
        .collect::<HashMap<_, _>>();
    let mut boundaries = Vec::new();
    for attachment in &comments.attachments {
        for (kind, region_id) in [
            ("leading", attachment.leading_region_id.as_ref()),
            ("trailing", attachment.trailing_region_id.as_ref()),
        ] {
            let Some(region_id) = region_id else { continue };
            let Some(region) = regions.get(region_id.as_str()) else {
                return Err(format!("comment attachment refers to missing region {region_id}"));
            };
            let (Some(start_line), Some(end_line)) = (region.start_line(), region.end_line())
            else {
                return Err(format!("comment boundary region {region_id} has no source lines"));
            };
            // Nested comment nodes can be reported by a parser's comment lexer
            // under the following item. They remain inside their structural
            // owner and do not form an external layout boundary.
            if owners
                .iter()
                .any(|owner| start_line >= owner.start_line && end_line <= owner.end_line)
            {
                continue;
            }
            let Some(owner) = owners.iter().find(|owner| owner.owner_id == attachment.owner_id)
            else {
                return Err(format!(
                    "comment attachment refers to missing owner {}",
                    attachment.owner_id
                ));
            };
            let is_external_edge = match kind {
                "leading" => end_line < owner.start_line,
                "trailing" => start_line > owner.end_line,
                _ => false,
            };
            if is_external_edge {
                boundaries.push(LayoutCommentBoundary {
                    owner_id: attachment.owner_id.clone(),
                    kind: kind.to_string(),
                    start_line,
                    end_line,
                });
            }
        }
    }
    augment_layout_with_comment_boundaries(lines, owners, &boundaries)
}

pub fn augment_layout_with_comment_boundaries(
    lines: &[String],
    owners: &[LayoutOwner],
    comment_boundaries: &[LayoutCommentBoundary],
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
    for boundary in comment_boundaries {
        let owner =
            owners.iter().find(|owner| owner.owner_id == boundary.owner_id).ok_or_else(|| {
                format!("layout comment boundary refers to unknown owner {}", boundary.owner_id)
            })?;
        if boundary.start_line == 0
            || boundary.end_line < boundary.start_line
            || boundary.end_line > lines.len()
        {
            return Err(format!(
                "layout comment boundary for {} has an invalid line range",
                boundary.owner_id
            ));
        }
        match boundary.kind.as_str() {
            "leading" if boundary.end_line < owner.start_line => {}
            "trailing" if boundary.start_line > owner.end_line => {}
            "leading" | "trailing" => {
                return Err(format!(
                    "layout comment boundary for {} overlaps its owner",
                    boundary.owner_id
                ));
            }
            _ => {
                return Err(format!(
                    "layout comment boundary for {} has invalid kind",
                    boundary.owner_id
                ));
            }
        }
    }

    let mut gaps = Vec::new();
    for (start_line, end_line) in blank_runs(lines) {
        let structural_before = owners.iter().rev().find(|owner| owner.end_line + 1 == start_line);
        let structural_after = owners.iter().find(|owner| owner.start_line == end_line + 1);
        let comment_before = comment_boundaries
            .iter()
            .rev()
            .filter(|boundary| boundary.kind == "trailing" && boundary.end_line + 1 == start_line)
            .find_map(|boundary| owners.iter().find(|owner| owner.owner_id == boundary.owner_id));
        let comment_after = comment_boundaries
            .iter()
            .filter(|boundary| boundary.kind == "leading" && boundary.start_line == end_line + 1)
            .find_map(|boundary| owners.iter().find(|owner| owner.owner_id == boundary.owner_id));
        // Structural edges control their side. An attached comment can supply
        // the opposite edge, which remains necessary when the structural owner
        // is later deleted and the gap transfers to its surviving neighbor.
        let before = structural_before.or(comment_before);
        let after = structural_after.or(comment_after);
        if before.is_none() && after.is_none() {
            continue;
        }
        let kind = match (before, after) {
            (None, Some(_)) => "preamble",
            (Some(_), None) => "postlude",
            (Some(_), Some(_)) => "interstitial",
            (None, None) => unreachable!(),
        };
        let controller_side =
            if structural_before.is_some() && structural_after.is_none() && comment_after.is_some()
            {
                "before"
            } else if structural_after.is_some() || after.is_some() {
                "after"
            } else {
                "before"
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
