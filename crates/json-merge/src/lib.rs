use ast_merge::MergeResult;
use tree_haver::{AnalysisHandle, ParserAdapter};

pub const PACKAGE_NAME: &str = "json-merge";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonAnalysis {
    pub allows_comments: bool,
}

impl AnalysisHandle for JsonAnalysis {
    fn kind(&self) -> &'static str {
        "json"
    }
}

pub trait JsonMerger {
    fn merge(&self, template: &JsonAnalysis, destination: &JsonAnalysis) -> MergeResult<String>;
}

pub trait JsonParserAdapter: ParserAdapter<JsonAnalysis> {}
