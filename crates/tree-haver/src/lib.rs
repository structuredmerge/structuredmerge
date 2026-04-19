use ast_merge::{Diagnostic, ParseResult, PolicyReference};

pub const PACKAGE_NAME: &str = "tree-haver";

pub trait AnalysisHandle {
    fn kind(&self) -> &'static str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserRequest {
    pub source: String,
    pub language: String,
    pub dialect: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReference {
    pub id: String,
    pub family: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterInfo {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

pub trait ParserAdapter<TAnalysis: AnalysisHandle> {
    fn info(&self) -> AdapterInfo;
    fn parse(&self, request: &ParserRequest) -> ParseResult<TAnalysis>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserDiagnostics {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub diagnostics: Vec<Diagnostic>,
}
