use crate::{
    Diagnostic, DiagnosticCategory, DiagnosticSeverity, ParseResult, ThreeWayMergeOutcome,
    ThreeWayMergeResult,
};

pub fn error_diagnostic(category: DiagnosticCategory, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        category,
        message: message.into(),
        path: None,
        review: None,
    }
}

pub fn parse_error_result<T>(message: impl Into<String>) -> ParseResult<T> {
    ParseResult {
        ok: false,
        diagnostics: vec![error_diagnostic(DiagnosticCategory::ParseError, message)],
        analysis: None,
        policies: vec![],
    }
}

pub fn normalized_parse_error_result<T>(messages: Vec<String>) -> ParseResult<T> {
    ParseResult {
        ok: false,
        diagnostics: messages
            .into_iter()
            .map(|message| error_diagnostic(DiagnosticCategory::ParseError, message))
            .collect(),
        analysis: None,
        policies: vec![],
    }
}

pub fn three_way_parse_error(
    role: &str,
    message: impl Into<String>,
) -> ThreeWayMergeResult<String> {
    ThreeWayMergeResult {
        outcome: ThreeWayMergeOutcome::Error,
        diagnostics: vec![error_diagnostic(
            DiagnosticCategory::ParseError,
            format!("{role} parse error: {}", message.into()),
        )],
        conflicts: vec![],
        output: None,
        policies: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_shared_fail_closed_parse_results() {
        let parse = normalized_parse_error_result::<()>(vec!["invalid syntax".to_string()]);
        assert!(!parse.ok);
        assert_eq!(parse.diagnostics[0].category, DiagnosticCategory::ParseError);

        let merge = three_way_parse_error("theirs", "invalid syntax");
        assert_eq!(merge.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(merge.diagnostics[0].message, "theirs parse error: invalid syntax");
    }
}
