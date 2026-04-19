use ast_merge::MergeResult;
use tree_haver::{AnalysisHandle, ParserAdapter};

pub const PACKAGE_NAME: &str = "text-merge";

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

pub trait TextAnalyzer {
    fn analyze(&self, source: &str) -> TextAnalysis;
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSimilarity {
    pub score: f64,
    pub threshold: f64,
    pub matched: bool,
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

    TextAnalysis {
        normalized_source,
        blocks,
    }
}

fn token_set(normalized: &str) -> std::collections::BTreeSet<&str> {
    normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect()
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
        if let (Some(left_block), Some(right_block)) = (left.blocks.get(index), right.blocks.get(index)) {
            sum += jaccard(&left_block.normalized, &right_block.normalized);
        }
    }

    sum / total as f64
}

pub fn is_similar(left_source: &str, right_source: &str, threshold: f64) -> TextSimilarity {
    let score = similarity_score(left_source, right_source);
    TextSimilarity {
        score,
        threshold,
        matched: score >= threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_text, is_similar, normalize_text, similarity_score, TextBlock, TextSpan};

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
        assert_eq!(
            similarity_score("Alpha   beta\n\nGamma", "  Alpha beta  \r\n\r\nGamma  "),
            1.0
        );
        assert_eq!(
            similarity_score("Alpha beta\n\nGamma delta", "Alpha beta\n\nGamma epsilon"),
            0.6666666666666666
        );
        assert_eq!(similarity_score("Alpha beta", "Zeta theta"), 0.0);

        let similarity = is_similar("Alpha beta\n\nGamma delta", "Alpha beta\n\nGamma epsilon", 0.6);
        assert!(similarity.matched);
    }
}
