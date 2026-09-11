use ast_merge::{
    NamedOwnerProjectionPolicy, ParseResult, SourcePreservingOwnerDocument, ThreeWayMergeResult,
    merge_source_preserving_owners, normalized_parse_error_result, parse_error_result,
    project_named_top_level_owners, three_way_parse_error,
};
use serde::Serialize;
use tree_haver::{ParserRequest, parse_normalized_with_language_pack};

pub const PACKAGE_NAME: &str = "generic-merge";
pub const PROVIDER_ID: &str = "rust.generic.tslp";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenericProviderStatus {
    Experimental,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GenericTslpCapability {
    pub provider_id: &'static str,
    pub package: &'static str,
    pub parser_boundary: &'static str,
    pub parser_backend: &'static str,
    pub status: GenericProviderStatus,
    pub language: String,
    pub dialect: String,
    pub operations: Vec<&'static str>,
    pub constraints: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericOwner {
    pub path: String,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericAnalysis {
    pub language: String,
    pub dialect: String,
    pub source: String,
    pub owners: Vec<GenericOwner>,
}

pub fn generic_tslp_capability(language: &str, dialect: Option<&str>) -> GenericTslpCapability {
    GenericTslpCapability {
        provider_id: PROVIDER_ID,
        package: PACKAGE_NAME,
        parser_boundary: "tree-haver",
        parser_backend: "tree-sitter-language-pack",
        status: GenericProviderStatus::Experimental,
        language: language.to_string(),
        dialect: dialect.unwrap_or(language).to_string(),
        operations: vec!["merge3"],
        constraints: vec![
            "all non-comment top-level nodes have one direct name field",
            "top-level owner identities are unique and retain their order",
            "source outside top-level owners is byte-identical",
            "output reparses through the same TreeHaver provider",
            "unsupported syntax and unavailable grammars fail closed",
        ],
    }
}

pub fn parse_generic_tslp(
    source: &str,
    language: &str,
    dialect: Option<&str>,
) -> ParseResult<GenericAnalysis> {
    let document = match parse_source_preserving_generic(source, language, dialect) {
        Ok(document) => document,
        Err(GenericParseFailure::Parser(diagnostics)) => {
            return normalized_parse_error_result(diagnostics);
        }
        Err(GenericParseFailure::Projection(message)) => return parse_error_result(message),
    };
    let owners = document
        .owners
        .iter()
        .map(|owner| GenericOwner {
            path: owner.path.clone(),
            source: source[owner.start_byte..owner.end_byte].to_string(),
        })
        .collect();
    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(GenericAnalysis {
            language: language.to_string(),
            dialect: dialect.unwrap_or(language).to_string(),
            source: source.to_string(),
            owners,
        }),
        policies: vec![],
    }
}

pub fn merge_generic_tslp_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    language: &str,
    dialect: Option<&str>,
) -> ThreeWayMergeResult<String> {
    let base = match parse_document_for_role(base_source, language, dialect, "base") {
        Ok(document) => document,
        Err(result) => return result,
    };
    let ours = match parse_document_for_role(ours_source, language, dialect, "ours") {
        Ok(document) => document,
        Err(result) => return result,
    };
    let theirs = match parse_document_for_role(theirs_source, language, dialect, "theirs") {
        Ok(document) => document,
        Err(result) => return result,
    };

    merge_source_preserving_owners(base, ours, theirs, |output| {
        parse_source_preserving_generic(output, language, dialect).map_err(
            |failure| match failure {
                GenericParseFailure::Parser(diagnostics) => diagnostics.join("; "),
                GenericParseFailure::Projection(message) => message,
            },
        )
    })
}

fn parse_document_for_role(
    source: &str,
    language: &str,
    dialect: Option<&str>,
    role: &str,
) -> Result<SourcePreservingOwnerDocument, ThreeWayMergeResult<String>> {
    parse_source_preserving_generic(source, language, dialect).map_err(|failure| {
        let message = match failure {
            GenericParseFailure::Parser(diagnostics) => diagnostics.join("; "),
            GenericParseFailure::Projection(message) => message,
        };
        three_way_parse_error(role, message)
    })
}

enum GenericParseFailure {
    Parser(Vec<String>),
    Projection(String),
}

fn parse_source_preserving_generic(
    source: &str,
    language: &str,
    dialect: Option<&str>,
) -> Result<SourcePreservingOwnerDocument, GenericParseFailure> {
    let language = language.trim();
    if language.is_empty() {
        return Err(GenericParseFailure::Projection(
            "generic TreeHaver provider requires a language".to_string(),
        ));
    }
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: language.to_string(),
        dialect: Some(dialect.unwrap_or(language).to_string()),
    });
    if !parsed.ok {
        return Err(GenericParseFailure::Parser(parsed.diagnostics));
    }
    if !parsed.source_fragments_available {
        return Err(GenericParseFailure::Projection(format!(
            "{language} parser did not retain source fragments"
        )));
    }

    project_named_top_level_owners(
        source,
        &parsed.root_id,
        &parsed.nodes,
        NamedOwnerProjectionPolicy {
            family: language,
            owner_kinds: &[],
            ignored_kinds: &[],
            wrapper_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &[],
            accept_any_named_kind: true,
        },
    )
    .map_err(GenericParseFailure::Projection)
}

#[cfg(test)]
mod tests {
    use ast_merge::{DiagnosticCategory, ThreeWayMergeOutcome};

    use super::*;

    #[test]
    fn merges_independent_python_functions_with_exact_source_preservation() {
        let base = "# retained\ndef left():\n    return 1\n\ndef right():\n    return 1\n";
        let ours = "# retained\ndef left():\n    return 2\n\ndef right():\n    return 1\n";
        let theirs = "# retained\ndef left():\n    return 1\n\ndef right():\n    return 2\n";

        let parsed = parse_generic_tslp(base, "python", None);
        assert!(parsed.ok, "diagnostics: {:?}", parsed.diagnostics);
        assert_eq!(
            parsed
                .analysis
                .unwrap()
                .owners
                .iter()
                .map(|owner| owner.path.as_str())
                .collect::<Vec<_>>(),
            vec!["/function_definition:left", "/function_definition:right"]
        );

        let result = merge_generic_tslp_three_way(base, ours, theirs, "python", None);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(
            result.output.as_deref(),
            Some("# retained\ndef left():\n    return 2\n\ndef right():\n    return 2\n")
        );
    }

    #[test]
    fn rejects_unnamed_top_level_syntax_and_membership_changes() {
        let assignment = parse_generic_tslp("answer = 42\n", "python", None);
        assert!(!assignment.ok);
        assert_eq!(assignment.diagnostics[0].category, DiagnosticCategory::ParseError);
        assert!(assignment.diagnostics[0].message.contains("no stable name"));

        let base = "def left():\n    return 1\n";
        let ours = "def left():\n    return 2\n";
        let theirs = "def renamed():\n    return 1\n";
        let renamed = merge_generic_tslp_three_way(base, ours, theirs, "python", None);
        assert_eq!(renamed.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(renamed.diagnostics[0].category, DiagnosticCategory::MergeConflict);
    }

    #[test]
    fn advertises_only_the_experimental_merge3_contract() {
        let capability = generic_tslp_capability("python", Some("python"));

        assert_eq!(capability.provider_id, "rust.generic.tslp");
        assert_eq!(capability.status, GenericProviderStatus::Experimental);
        assert_eq!(capability.operations, vec!["merge3"]);
        assert_eq!(capability.constraints.len(), 5);
    }
}
