use ast_merge::{Diagnostic, ParseResult};

pub const PACKAGE_NAME: &str = "tree-haver";

pub trait AnalysisHandle {
    fn kind(&self) -> &'static str;
}

pub trait ParserAdapter<TAnalysis: AnalysisHandle> {
    fn parse(&self, source: &str) -> ParseResult<TAnalysis>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserDiagnostics {
    pub diagnostics: Vec<Diagnostic>,
}
