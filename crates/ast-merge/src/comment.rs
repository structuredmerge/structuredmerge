use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tree_haver::{NodeRole, NormalizedTreeNode};

use crate::{
    CommentAttachment, CommentLine, CommentRegion, LayoutGap, LayoutOwner, augment_layout,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrackedComment {
    pub line: usize,
    pub text: String,
    pub normalized_content: String,
    pub full_line: bool,
    pub indent: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommentAugmentation {
    pub regions: Vec<CommentRegion>,
    pub gaps: Vec<LayoutGap>,
    pub attachments: Vec<CommentAttachment>,
    pub preamble_region_id: Option<String>,
    pub postlude_region_id: Option<String>,
    pub orphan_region_ids: Vec<String>,
}

pub fn augment_normalized_tree_comments(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    style: &str,
    normalize_comment: impl Fn(&str) -> String,
) -> Result<CommentAugmentation, String> {
    let owners = normalized_root_layout_owners(source, root_id, nodes)?;
    augment_normalized_comments_with_owners(source, &owners, nodes, style, normalize_comment)
}

pub fn augment_normalized_comments_with_owners(
    source: &str,
    owners: &[LayoutOwner],
    nodes: &[NormalizedTreeNode],
    style: &str,
    normalize_comment: impl Fn(&str) -> String,
) -> Result<CommentAugmentation, String> {
    let lines = source_lines(source);
    let comments = nodes
        .iter()
        .filter(|node| node.role == NodeRole::Comment)
        .flat_map(|node| tracked_normalized_comment(&lines, node, &normalize_comment))
        .collect::<Vec<_>>();
    augment_comments(&lines, owners, &comments, style)
}

pub fn normalized_root_layout_owners(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
) -> Result<Vec<LayoutOwner>, String> {
    let line_count = source_lines(source).len();
    let root = nodes
        .iter()
        .find(|node| node.id == root_id)
        .ok_or_else(|| "normalized tree omitted its root node".to_string())?;
    let mut owners = nodes
        .iter()
        .filter(|node| {
            node.parent_id.as_deref() == Some(root_id)
                && node.named
                && node.role == NodeRole::Structural
                && node.span.range.start_byte < node.span.range.end_byte
        })
        .map(|node| normalized_layout_owner(node, line_count))
        .collect::<Vec<_>>();
    owners.sort_by_key(|owner| (owner.start_line, owner.end_line, owner.owner_id.clone()));

    if owners.is_empty() || owners.windows(2).any(|pair| pair[1].start_line <= pair[0].end_line) {
        owners = vec![normalized_layout_owner(root, line_count)];
    }
    Ok(owners)
}

pub fn normalized_layout_owners_for_kinds(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    owner_kinds: &[&str],
) -> Result<Vec<LayoutOwner>, String> {
    let line_count = source_lines(source).len();
    let nodes_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let root = nodes_by_id
        .get(root_id)
        .copied()
        .ok_or_else(|| "normalized tree omitted its root node".to_string())?;
    let candidates = nodes
        .iter()
        .filter(|node| {
            owner_kinds.contains(&node.kind.as_str())
                && node.span.range.start_byte < node.span.range.end_byte
        })
        .map(|node| (normalized_node_depth(node, &nodes_by_id), node))
        .collect::<Vec<_>>();
    let Some(minimum_depth) = candidates.iter().map(|(depth, _)| *depth).min() else {
        return Ok(vec![normalized_layout_owner(root, line_count)]);
    };
    let mut owners = candidates
        .into_iter()
        .filter(|(depth, _)| *depth == minimum_depth)
        .map(|(_, node)| normalized_layout_owner(node, line_count))
        .collect::<Vec<_>>();
    owners.sort_by_key(|owner| (owner.start_line, owner.end_line, owner.owner_id.clone()));
    if owners.windows(2).any(|pair| pair[1].start_line <= pair[0].end_line) {
        return Ok(vec![normalized_layout_owner(root, line_count)]);
    }
    Ok(owners)
}

fn normalized_node_depth(
    node: &NormalizedTreeNode,
    nodes: &std::collections::HashMap<&str, &NormalizedTreeNode>,
) -> usize {
    let mut depth = 0;
    let mut parent_id = node.parent_id.as_deref();
    while let Some(id) = parent_id {
        depth += 1;
        parent_id = nodes.get(id).and_then(|parent| parent.parent_id.as_deref());
    }
    depth
}

fn source_lines(source: &str) -> Vec<String> {
    let mut lines = source.split('\n').map(str::to_string).collect::<Vec<_>>();
    if source.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn normalized_layout_owner(node: &NormalizedTreeNode, line_count: usize) -> LayoutOwner {
    let start_line = (node.span.start_point.row + 1).min(line_count.max(1));
    let end_line =
        if node.span.end_point.column == 0 && node.span.end_point.row > node.span.start_point.row {
            node.span.end_point.row
        } else {
            node.span.end_point.row + 1
        };
    LayoutOwner {
        owner_id: node.id.clone(),
        start_line,
        end_line: end_line.max(start_line).min(line_count.max(1)),
    }
}

fn tracked_normalized_comment(
    source_lines: &[String],
    node: &NormalizedTreeNode,
    normalize_comment: &impl Fn(&str) -> String,
) -> Vec<TrackedComment> {
    let start_line = node.span.start_point.row + 1;
    let mut fragments = node.source_fragment.split('\n').collect::<Vec<_>>();
    if node.source_fragment.ends_with('\n') && fragments.last().is_some_and(|text| text.is_empty())
    {
        fragments.pop();
    }
    fragments
        .into_iter()
        .enumerate()
        .map(|(offset, text)| {
            let line = start_line + offset;
            let text = text.trim_end_matches('\r').to_string();
            let indent = if offset == 0 { node.span.start_point.column } else { 0 };
            let full_line = source_lines
                .get(line - 1)
                .and_then(|source_line| source_line.get(..indent))
                .is_none_or(|prefix| prefix.trim().is_empty());
            TrackedComment {
                line,
                normalized_content: normalize_comment(&text),
                text,
                full_line,
                indent: Some(indent),
            }
        })
        .collect()
}

pub fn augment_comments(
    lines: &[String],
    owners: &[LayoutOwner],
    comments: &[TrackedComment],
    style: &str,
) -> Result<CommentAugmentation, String> {
    validate_comments(lines, comments)?;
    let mut owners = owners.to_vec();
    owners.sort_by_key(|owner| (owner.start_line, owner.end_line, owner.owner_id.clone()));
    let layout = augment_layout(lines, &owners)?;
    let mut comments = comments.to_vec();
    comments.sort_by_key(|comment| comment.line);
    let mut claimed = HashSet::new();
    let mut regions = Vec::new();
    let mut attachments = Vec::new();

    for (owner_index, owner) in owners.iter().enumerate() {
        let leading = leading_comment_indices(lines, owner, &comments, &claimed);
        let inline = comments
            .iter()
            .enumerate()
            .filter(|(index, comment)| {
                !comment.full_line
                    && !claimed.contains(index)
                    && (owner.start_line..=owner.end_line).contains(&comment.line)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let trailing = trailing_comment_indices(
            lines,
            owner,
            owners.get(owner_index + 1),
            &comments,
            &claimed,
        );
        let leading_floating = leading.last().is_some_and(|index| {
            ((comments[*index].line + 1)..owner.start_line).any(|line| blank_line(lines, line))
        });

        let leading_region_id = push_region(
            &mut regions,
            "leading",
            owner,
            &leading,
            &comments,
            lines,
            style,
            true,
            leading_floating,
        );
        let inline_region_id = push_region(
            &mut regions,
            "inline",
            owner,
            &inline,
            &comments,
            lines,
            style,
            false,
            false,
        );
        let trailing_region_id = push_region(
            &mut regions,
            "trailing",
            owner,
            &trailing,
            &comments,
            lines,
            style,
            true,
            false,
        );
        claimed.extend(leading.iter().chain(&inline).chain(&trailing).copied());
        let layout_attachment =
            layout.attachments.iter().find(|attachment| attachment.owner_id == owner.owner_id);
        attachments.push(CommentAttachment {
            owner_id: owner.owner_id.clone(),
            leading_region_id,
            inline_region_id,
            trailing_region_id,
            orphan_region_ids: vec![],
            leading_gap_id: layout_attachment.and_then(|entry| entry.leading_gap_id.clone()),
            trailing_gap_id: layout_attachment.and_then(|entry| entry.trailing_gap_id.clone()),
            metadata: Default::default(),
        });
    }

    let last_owner_line = owners.last().map(|owner| owner.end_line);
    let postlude = comments
        .iter()
        .enumerate()
        .filter(|(index, comment)| {
            comment.full_line
                && !claimed.contains(index)
                && last_owner_line.is_some_and(|line| comment.line > line)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let postlude_region_id =
        push_document_region(&mut regions, "postlude", &postlude, &comments, lines, style);
    claimed.extend(postlude);

    let remaining = comments
        .iter()
        .enumerate()
        .filter(|(index, comment)| comment.full_line && !claimed.contains(index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let groups = group_comments_with_blank_lines(lines, &comments, &remaining);
    let first_owner_start = owners.first().map(|owner| owner.start_line);
    let mut preamble_region_id = None;
    let mut orphan_region_ids = Vec::new();
    for group in groups {
        let is_preamble = preamble_region_id.is_none()
            && first_owner_start.is_some_and(|line| comments[*group.last().unwrap()].line < line);
        let kind = if is_preamble { "preamble" } else { "orphan" };
        if let Some(region_id) =
            push_document_region(&mut regions, kind, &group, &comments, lines, style)
        {
            if is_preamble {
                preamble_region_id = Some(region_id);
            } else {
                orphan_region_ids.push(region_id);
            }
        }
    }

    Ok(CommentAugmentation {
        regions,
        gaps: layout.gaps,
        attachments,
        preamble_region_id,
        postlude_region_id,
        orphan_region_ids,
    })
}

fn validate_comments(lines: &[String], comments: &[TrackedComment]) -> Result<(), String> {
    for comment in comments {
        if comment.line == 0 || comment.line > lines.len() {
            return Err(format!("tracked comment line {} is outside the source", comment.line));
        }
    }
    Ok(())
}

fn leading_comment_indices(
    lines: &[String],
    owner: &LayoutOwner,
    comments: &[TrackedComment],
    claimed: &HashSet<usize>,
) -> Vec<usize> {
    let mut selected = Vec::new();
    let mut current = owner.start_line.saturating_sub(1);
    while current > 0 {
        if let Some((index, _)) = comments.iter().enumerate().find(|(index, comment)| {
            comment.full_line && comment.line == current && !claimed.contains(index)
        }) {
            selected.push(index);
            current -= 1;
        } else if blank_line(lines, current) {
            current -= 1;
        } else {
            break;
        }
    }
    selected.reverse();
    if selected.first().is_some_and(|index| comments[*index].line == 1)
        && (1..owner.start_line).any(|line| blank_line(lines, line))
    {
        if let Some(first_gap) = (1..owner.start_line).find(|line| blank_line(lines, *line)) {
            selected.retain(|index| comments[*index].line > first_gap);
        }
    }
    selected
}

fn trailing_comment_indices(
    lines: &[String],
    owner: &LayoutOwner,
    next_owner: Option<&LayoutOwner>,
    comments: &[TrackedComment],
    claimed: &HashSet<usize>,
) -> Vec<usize> {
    let max_line = next_owner.map_or(lines.len(), |next| next.start_line - 1);
    let mut current = owner.end_line + 1;
    if current > max_line || blank_line(lines, current) {
        return vec![];
    }
    let mut selected = Vec::new();
    while current <= max_line {
        let Some((index, comment)) = comments.iter().enumerate().find(|(index, comment)| {
            comment.full_line && comment.line == current && !claimed.contains(index)
        }) else {
            break;
        };
        if let Some(indent) = comment.indent {
            let owner_indent =
                lines[owner.start_line - 1].chars().take_while(|char| char.is_whitespace()).count();
            if indent != owner_indent {
                break;
            }
        }
        selected.push(index);
        current += 1;
        while current <= max_line && blank_line(lines, current) {
            current += 1;
        }
    }
    selected
}

#[allow(clippy::too_many_arguments)]
fn push_region(
    regions: &mut Vec<CommentRegion>,
    kind: &str,
    owner: &LayoutOwner,
    indices: &[usize],
    comments: &[TrackedComment],
    lines: &[String],
    style: &str,
    include_blank_lines: bool,
    floating: bool,
) -> Option<String> {
    push_region_for_owner(
        regions,
        kind,
        &owner.owner_id,
        indices,
        comments,
        lines,
        style,
        include_blank_lines,
        floating,
    )
}

fn push_document_region(
    regions: &mut Vec<CommentRegion>,
    kind: &str,
    indices: &[usize],
    comments: &[TrackedComment],
    lines: &[String],
    style: &str,
) -> Option<String> {
    push_region_for_owner(
        regions,
        kind,
        &format!("document:{kind}"),
        indices,
        comments,
        lines,
        style,
        true,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn push_region_for_owner(
    regions: &mut Vec<CommentRegion>,
    kind: &str,
    owner_id: &str,
    indices: &[usize],
    comments: &[TrackedComment],
    lines: &[String],
    style: &str,
    include_blank_lines: bool,
    floating: bool,
) -> Option<String> {
    if indices.is_empty() {
        return None;
    }
    let mut ordered = indices.to_vec();
    ordered.sort_by_key(|index| comments[*index].line);
    let id = format!(
        "comment-region:{kind}:{}-{}",
        comments[*ordered.first().unwrap()].line,
        comments[*ordered.last().unwrap()].line
    );
    let mut nodes = Vec::new();
    let mut previous_line = None;
    for index in ordered {
        let comment = &comments[index];
        if include_blank_lines {
            if let Some(previous) = previous_line {
                for line in (previous + 1)..comment.line {
                    if blank_line(lines, line) {
                        nodes.push(CommentLine {
                            text: lines[line - 1].clone(),
                            line_number: line,
                            normalized_content: String::new(),
                        });
                    }
                }
            }
        }
        nodes.push(CommentLine {
            text: comment.text.clone(),
            line_number: comment.line,
            normalized_content: comment.normalized_content.clone(),
        });
        previous_line = Some(comment.line);
    }
    regions.push(CommentRegion {
        id: id.clone(),
        kind: kind.to_string(),
        style: style.to_string(),
        owner_id: owner_id.to_string(),
        floating,
        nodes,
    });
    Some(id)
}

fn group_comments_with_blank_lines(
    lines: &[String],
    comments: &[TrackedComment],
    indices: &[usize],
) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for index in indices {
        if let Some(group) = groups.last_mut() {
            let previous_line = comments[*group.last().unwrap()].line;
            if ((previous_line + 1)..comments[*index].line).all(|line| blank_line(lines, line)) {
                group.push(*index);
                continue;
            }
        }
        groups.push(vec![*index]);
    }
    groups
}

fn blank_line(lines: &[String], line: usize) -> bool {
    line > 0 && lines.get(line - 1).is_some_and(|content| content.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(line: usize, text: &str, full_line: bool) -> TrackedComment {
        TrackedComment {
            line,
            text: text.to_string(),
            normalized_content: text.trim_start_matches('#').trim().to_string(),
            full_line,
            indent: Some(0),
        }
    }

    #[test]
    fn separates_document_comments_from_owner_comments_and_reuses_layout_gaps() {
        let lines = ["# file", "", "# alpha", "alpha", "", "# postlude"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let augmentation = augment_comments(
            &lines,
            &[LayoutOwner { owner_id: "alpha".to_string(), start_line: 4, end_line: 4 }],
            &[
                comment(1, "# file", true),
                comment(3, "# alpha", true),
                comment(6, "# postlude", true),
            ],
            "hash_comment",
        )
        .unwrap();

        assert_eq!(augmentation.preamble_region_id.as_deref(), Some("comment-region:preamble:1-1"));
        assert_eq!(augmentation.postlude_region_id.as_deref(), Some("comment-region:postlude:6-6"));
        assert_eq!(
            augmentation.attachments[0].leading_region_id.as_deref(),
            Some("comment-region:leading:3-3")
        );
        assert_eq!(augmentation.attachments[0].leading_gap_id.as_deref(), None);
        assert_eq!(augmentation.attachments[0].trailing_gap_id.as_deref(), Some("layout-gap:5-5"));
    }

    #[test]
    fn attaches_inline_comments_without_claiming_full_line_regions() {
        let lines = vec!["alpha # note".to_string()];
        let augmentation = augment_comments(
            &lines,
            &[LayoutOwner { owner_id: "alpha".to_string(), start_line: 1, end_line: 1 }],
            &[comment(1, "# note", false)],
            "hash_comment",
        )
        .unwrap();

        let attachment = &augmentation.attachments[0];
        assert!(attachment.leading_region_id.is_none());
        assert!(attachment.trailing_region_id.is_none());
        assert_eq!(attachment.inline_region_id.as_deref(), Some("comment-region:inline:1-1"));
    }

    #[test]
    fn projects_normalized_tree_comments_and_root_layout_once() {
        let source = "# alpha\nalpha = 1\n\n# beta\nbeta = 2\n";
        let parsed = tree_haver::parse_normalized_with_language_pack(&tree_haver::ParserRequest {
            source: source.to_string(),
            language: "toml".to_string(),
            dialect: Some("toml".to_string()),
        });
        assert!(parsed.ok);

        let augmentation = augment_normalized_tree_comments(
            source,
            &parsed.root_id,
            &parsed.nodes,
            "hash_comment",
            |text| text.trim_start_matches('#').trim().to_string(),
        )
        .unwrap();

        assert_eq!(augmentation.regions.len(), 2);
        assert_eq!(augmentation.regions[0].normalized_content(), "alpha");
        assert_eq!(augmentation.regions[1].normalized_content(), "beta");
        assert_eq!(augmentation.gaps.len(), 1);
        assert_eq!(augmentation.gaps[0].lines, [""]);
        assert_eq!(augmentation.attachments.len(), 2);
    }
}
