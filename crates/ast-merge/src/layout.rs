use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::LayoutGap;

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
