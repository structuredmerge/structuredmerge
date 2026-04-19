use ast_merge::MergeResult;
use tree_haver::{AnalysisHandle, ParserAdapter, ParserRequest};

pub const PACKAGE_NAME: &str = "text-merge";
pub const DEFAULT_TEXT_REFINEMENT_THRESHOLD: f64 = 0.7;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextBlock {
    pub index: usize,
    pub normalized: String,
    pub span: TextSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextAnalysis {
    pub normalized_source: String,
    pub blocks: Vec<TextBlock>,
}

impl AnalysisHandle for TextAnalysis {
    fn kind(&self) -> &'static str {
        "text"
    }
}

pub trait TextMerger {
    fn merge(&self, template: &TextAnalysis, destination: &TextAnalysis) -> MergeResult<String>;
}

pub trait TextParserAdapter: ParserAdapter<TextAnalysis> {}

pub fn text_parse_request(source: &str) -> ParserRequest {
    ParserRequest { source: source.to_string(), language: "text".to_string(), dialect: None }
}

pub trait TextAnalyzer {
    fn analyze(&self, source: &str) -> TextAnalysis;
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSimilarity {
    pub score: f64,
    pub threshold: f64,
    pub matched: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRefinementWeights {
    pub content: f64,
    pub length: f64,
    pub position: f64,
}

pub const DEFAULT_TEXT_REFINEMENT_WEIGHTS: TextRefinementWeights =
    TextRefinementWeights { content: 0.7, length: 0.15, position: 0.15 };

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextMatchPhase {
    Exact,
    Refined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextBlockMatch {
    pub template_index: usize,
    pub destination_index: usize,
    pub phase: TextMatchPhase,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextBlockMatchResult {
    pub matched: Vec<TextBlockMatch>,
    pub unmatched_template: Vec<usize>,
    pub unmatched_destination: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextMergeResolution {
    pub output: String,
}

pub trait TextBlockMatcher {
    fn match_blocks(
        &self,
        template: &TextAnalysis,
        destination: &TextAnalysis,
    ) -> TextBlockMatchResult;
}

pub fn normalize_text(source: &str) -> String {
    source
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
        .map(|block| block.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn analyze_text(source: &str) -> TextAnalysis {
    let normalized_source = normalize_text(source);
    let mut cursor = 0usize;
    let blocks = if normalized_source.is_empty() {
        Vec::new()
    } else {
        normalized_source
            .split("\n\n")
            .enumerate()
            .map(|(index, normalized)| {
                let start = cursor;
                let end = start + normalized.len();
                cursor = end + 2;
                TextBlock {
                    index,
                    normalized: normalized.to_string(),
                    span: TextSpan { start, end },
                }
            })
            .collect()
    };

    TextAnalysis { normalized_source, blocks }
}

fn token_set(normalized: &str) -> std::collections::BTreeSet<&str> {
    normalized.split_whitespace().filter(|token| !token.is_empty()).collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();

    let mut previous: Vec<usize> = (0..=left_chars.len()).collect();
    let mut current = vec![0; left_chars.len() + 1];

    for (right_index, right_char) in right_chars.iter().enumerate() {
        current[0] = right_index + 1;

        for (left_index, left_char) in left_chars.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[left_index + 1] = [
                current[left_index] + 1,
                previous[left_index + 1] + 1,
                previous[left_index] + cost,
            ]
            .into_iter()
            .min()
            .unwrap_or(0);
        }

        std::mem::swap(&mut previous, &mut current);
    }

    previous[left_chars.len()]
}

fn string_similarity(left: &str, right: &str) -> f64 {
    if left == right {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let distance = levenshtein_distance(left, right) as f64;
    let max_len = left.chars().count().max(right.chars().count()) as f64;
    1.0 - (distance / max_len)
}

fn length_similarity(left: &str, right: &str) -> f64 {
    if left.len() == right.len() {
        return 1.0;
    }

    let max_len = left.len().max(right.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }

    left.len().min(right.len()) as f64 / max_len
}

fn relative_position(index: usize, total: usize) -> f64 {
    if total > 1 { index as f64 / (total - 1) as f64 } else { 0.5 }
}

fn position_similarity(
    template_index: usize,
    destination_index: usize,
    template_total: usize,
    destination_total: usize,
) -> f64 {
    1.0 - (relative_position(template_index, template_total)
        - relative_position(destination_index, destination_total))
    .abs()
}

fn jaccard(left: &str, right: &str) -> f64 {
    let left_tokens = token_set(left);
    let right_tokens = token_set(right);

    if left_tokens.is_empty() && right_tokens.is_empty() {
        return 1.0;
    }

    let intersection = left_tokens.intersection(&right_tokens).count() as f64;
    let union = left_tokens.union(&right_tokens).count() as f64;

    if union == 0.0 { 1.0 } else { intersection / union }
}

pub fn similarity_score(left_source: &str, right_source: &str) -> f64 {
    let left = analyze_text(left_source);
    let right = analyze_text(right_source);
    let total = left.blocks.len().max(right.blocks.len());

    if total == 0 {
        return 1.0;
    }

    let mut sum = 0.0;
    for index in 0..total {
        if let (Some(left_block), Some(right_block)) =
            (left.blocks.get(index), right.blocks.get(index))
        {
            sum += jaccard(&left_block.normalized, &right_block.normalized);
        }
    }

    sum / total as f64
}

pub fn refined_text_similarity(
    template_block: &TextBlock,
    destination_block: &TextBlock,
    template_total: usize,
    destination_total: usize,
    weights: TextRefinementWeights,
) -> f64 {
    let content = string_similarity(&template_block.normalized, &destination_block.normalized);
    let length = length_similarity(&template_block.normalized, &destination_block.normalized);
    let position = position_similarity(
        template_block.index,
        destination_block.index,
        template_total,
        destination_total,
    );

    weights.content * content + weights.length * length + weights.position * position
}

pub fn is_similar(left_source: &str, right_source: &str, threshold: f64) -> TextSimilarity {
    let score = similarity_score(left_source, right_source);
    TextSimilarity { score, threshold, matched: score >= threshold }
}

pub fn merge_text(template_source: &str, destination_source: &str) -> MergeResult<String> {
    let template = analyze_text(template_source);
    let destination = analyze_text(destination_source);
    let matches = match_text_blocks(template_source, destination_source);
    let matched_template: std::collections::BTreeSet<usize> =
        matches.matched.iter().map(|entry| entry.template_index).collect();
    let mut merged_blocks = Vec::new();

    for destination_block in &destination.blocks {
        merged_blocks.push(destination_block.normalized.clone());
    }

    for (index, template_block) in template.blocks.iter().enumerate() {
        if !matched_template.contains(&index) {
            merged_blocks.push(template_block.normalized.clone());
        }
    }

    MergeResult { ok: true, diagnostics: vec![], output: Some(merged_blocks.join("\n\n")) }
}

pub fn match_text_blocks(template_source: &str, destination_source: &str) -> TextBlockMatchResult {
    let template = analyze_text(template_source);
    let destination = analyze_text(destination_source);
    let mut matched_template = std::collections::BTreeSet::new();
    let mut matched_destination = std::collections::BTreeSet::new();
    let mut matched = Vec::new();

    for (destination_index, destination_block) in destination.blocks.iter().enumerate() {
        if let Some((template_index, _)) =
            template.blocks.iter().enumerate().find(|(candidate_index, template_block)| {
                !matched_template.contains(candidate_index)
                    && template_block.normalized == destination_block.normalized
            })
        {
            matched_template.insert(template_index);
            matched_destination.insert(destination_index);
            matched.push(TextBlockMatch {
                template_index,
                destination_index,
                phase: TextMatchPhase::Exact,
                score: 1.0,
            });
        }
    }

    for (destination_index, destination_block) in destination.blocks.iter().enumerate() {
        if matched_destination.contains(&destination_index) {
            continue;
        }

        let mut best_template_index = None;
        let mut best_score = 0.0;

        for (template_index, template_block) in template.blocks.iter().enumerate() {
            if matched_template.contains(&template_index) {
                continue;
            }

            let score = refined_text_similarity(
                template_block,
                destination_block,
                template.blocks.len(),
                destination.blocks.len(),
                DEFAULT_TEXT_REFINEMENT_WEIGHTS,
            );

            if score >= DEFAULT_TEXT_REFINEMENT_THRESHOLD && score > best_score {
                best_template_index = Some(template_index);
                best_score = score;
            }
        }

        if let Some(template_index) = best_template_index {
            matched_template.insert(template_index);
            matched_destination.insert(destination_index);
            matched.push(TextBlockMatch {
                template_index,
                destination_index,
                phase: TextMatchPhase::Refined,
                score: best_score,
            });
        }
    }

    matched.sort_by_key(|entry| entry.destination_index);

    TextBlockMatchResult {
        matched,
        unmatched_template: (0..template.blocks.len())
            .filter(|index| !matched_template.contains(index))
            .collect(),
        unmatched_destination: (0..destination.blocks.len())
            .filter(|index| !matched_destination.contains(index))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_TEXT_REFINEMENT_WEIGHTS, TextBlock, TextBlockMatch, TextMatchPhase, TextSpan,
        analyze_text, is_similar, match_text_blocks, merge_text, normalize_text,
        refined_text_similarity, similarity_score,
    };

    #[test]
    fn normalizes_and_segments_text_blocks() {
        let source = "  Alpha   beta\r\n\r\nGamma\n   delta\n\n\nEpsilon  \n";

        assert_eq!(normalize_text(source), "Alpha beta\n\nGamma delta\n\nEpsilon");
        assert_eq!(
            analyze_text(source).blocks,
            vec![
                TextBlock {
                    index: 0,
                    normalized: "Alpha beta".to_string(),
                    span: TextSpan { start: 0, end: 10 },
                },
                TextBlock {
                    index: 1,
                    normalized: "Gamma delta".to_string(),
                    span: TextSpan { start: 12, end: 23 },
                },
                TextBlock {
                    index: 2,
                    normalized: "Epsilon".to_string(),
                    span: TextSpan { start: 25, end: 32 },
                },
            ]
        );
    }

    #[test]
    fn scores_text_similarity() {
        assert_eq!(similarity_score("Alpha   beta\n\nGamma", "  Alpha beta  \r\n\r\nGamma  "), 1.0);
        assert_eq!(
            similarity_score("Alpha beta\n\nGamma delta", "Alpha beta\n\nGamma epsilon"),
            0.6666666666666666
        );
        assert_eq!(similarity_score("Alpha beta", "Zeta theta"), 0.0);

        let similarity =
            is_similar("Alpha beta\n\nGamma delta", "Alpha beta\n\nGamma epsilon", 0.6);
        assert!(similarity.matched);
    }

    #[test]
    fn resolves_text_merge() {
        let result = merge_text(
            "Alpha\n\nBeta\n\nAlpha\n\nTemplate only",
            "Beta\n\nAlpha revised\n\nAlpha\n\nDestination only",
        );

        assert!(result.ok);
        assert_eq!(
            result.output,
            Some(
                "Beta\n\nAlpha revised\n\nAlpha\n\nDestination only\n\nAlpha\n\nTemplate only"
                    .to_string()
            )
        );
    }

    #[test]
    fn matches_text_blocks_by_content() {
        let result = match_text_blocks(
            "Alpha\n\nBeta\n\nAlpha\n\nTemplate only",
            "Beta\n\nAlpha\n\nAlpha\n\nDestination only",
        );

        assert_eq!(
            result.matched,
            vec![
                TextBlockMatch {
                    template_index: 1,
                    destination_index: 0,
                    phase: TextMatchPhase::Exact,
                    score: 1.0,
                },
                TextBlockMatch {
                    template_index: 0,
                    destination_index: 1,
                    phase: TextMatchPhase::Exact,
                    score: 1.0,
                },
                TextBlockMatch {
                    template_index: 2,
                    destination_index: 2,
                    phase: TextMatchPhase::Exact,
                    score: 1.0,
                },
            ]
        );
        assert_eq!(result.unmatched_template, vec![3]);
        assert_eq!(result.unmatched_destination, vec![3]);
    }

    #[test]
    fn refines_text_block_matches_by_content() {
        let result = match_text_blocks(
            "Alpha beta gamma\n\nDelta anchor\n\nClosing line",
            "Alpha beta delta\n\nDelta anchor\n\nClosing line",
        );

        assert_eq!(
            result.matched,
            vec![
                TextBlockMatch {
                    template_index: 0,
                    destination_index: 0,
                    phase: TextMatchPhase::Refined,
                    score: 0.825,
                },
                TextBlockMatch {
                    template_index: 1,
                    destination_index: 1,
                    phase: TextMatchPhase::Exact,
                    score: 1.0,
                },
                TextBlockMatch {
                    template_index: 2,
                    destination_index: 2,
                    phase: TextMatchPhase::Exact,
                    score: 1.0,
                },
            ]
        );
        assert!(result.unmatched_template.is_empty());
        assert!(result.unmatched_destination.is_empty());

        let merge_result = merge_text(
            "Alpha beta gamma\n\nDelta anchor\n\nClosing line",
            "Alpha beta delta\n\nDelta anchor\n\nClosing line",
        );
        assert_eq!(
            merge_result.output,
            Some("Alpha beta delta\n\nDelta anchor\n\nClosing line".to_string())
        );
    }

    #[test]
    fn scores_refined_text_similarity() {
        let template = analyze_text("Alpha beta gamma");
        let destination = analyze_text("Alpha beta delta");

        let score = refined_text_similarity(
            &template.blocks[0],
            &destination.blocks[0],
            template.blocks.len(),
            destination.blocks.len(),
            DEFAULT_TEXT_REFINEMENT_WEIGHTS,
        );

        assert_eq!(score, 0.825);
    }
}
