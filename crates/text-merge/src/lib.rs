use ast_merge::MergeResult;
use tree_haver::{AnalysisHandle, ParserAdapter};

pub const PACKAGE_NAME: &str = "text-merge";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextAnalysis {
    pub blocks: Vec<String>,
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
