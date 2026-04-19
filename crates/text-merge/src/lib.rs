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

#[cfg(test)]
mod tests {
    use super::{analyze_text, normalize_text, TextBlock, TextSpan};

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
}
