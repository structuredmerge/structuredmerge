use std::collections::HashSet;

use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, Diagnostic, DiagnosticCategory,
    DiagnosticSeverity, FamilyFeatureProfile, MergeConflict, NamedOwnerKind, ParseResult,
    PolicyReference, SourcePreservingOwnerDocument, ThreeWayMergeResult,
    merge_source_preserving_owners, normalized_parse_error_result, parse_error_result,
    three_way_parse_error,
};
use tree_haver::{
    BackendReference, NodeRole, NormalizedTreeIndex, NormalizedTreeNode, ParserRequest,
    kreuzberg_language_pack_backend, language_pack_adapter_info,
    parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "bash-merge";

pub mod directional;
pub mod typed;

const BASH_OWNER_KINDS: &[NamedOwnerKind<'static>] = &[
    NamedOwnerKind { node_kind: "function_definition", path_kind: "function" },
    NamedOwnerKind { node_kind: "variable_assignment", path_kind: "variable" },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BashDialect {
    Bash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BashBackend {
    TreeSitter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BashFunction {
    pub path: String,
    pub name: String,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BashAnalysis {
    pub dialect: BashDialect,
    pub source: String,
    pub functions: Vec<BashFunction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BashFeatureProfile {
    pub family: &'static str,
    pub supported_dialects: Vec<BashDialect>,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BashBackendFeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

fn parse_request(source: &str) -> ParserRequest {
    ParserRequest {
        source: source.to_string(),
        language: "bash".to_string(),
        dialect: Some("bash".to_string()),
    }
}

pub fn bash_feature_profile() -> BashFeatureProfile {
    BashFeatureProfile {
        family: "bash",
        supported_dialects: vec![BashDialect::Bash],
        supported_policies: vec![],
    }
}

pub fn bash_backend_feature_profile(_backend: BashBackend) -> BashBackendFeatureProfile {
    BashBackendFeatureProfile {
        backend: language_pack_adapter_info().backend,
        backend_ref: Some(kreuzberg_language_pack_backend()),
        supports_dialects: true,
        supported_policies: vec![],
    }
}

pub fn bash_plan_context(backend: BashBackend) -> ConformanceFamilyPlanContext {
    let feature_profile = bash_backend_feature_profile(backend);
    ConformanceFamilyPlanContext {
        family_profile: FamilyFeatureProfile {
            family: bash_feature_profile().family.to_string(),
            supported_dialects: vec!["bash".to_string()],
            supported_policies: vec![],
        },
        feature_profile: Some(ConformanceFeatureProfileView {
            backend: feature_profile.backend,
            supports_dialects: feature_profile.supports_dialects,
            supported_policies: feature_profile.supported_policies,
        }),
        merge_engine: None,
    }
}

pub fn bash_backends() -> Vec<BashBackend> {
    vec![BashBackend::TreeSitter]
}

pub fn parse_bash(source: &str, dialect: BashDialect) -> ParseResult<BashAnalysis> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return normalized_parse_error_result(parsed.diagnostics);
    }
    let document = match project_bash_functions(source, &parsed.root_id, &parsed.nodes) {
        Ok(document) => document,
        Err(message) => return parse_error_result(message),
    };
    let functions = document
        .owners
        .iter()
        .map(|owner| BashFunction {
            path: owner.path.clone(),
            name: owner
                .id
                .rsplit_once(':')
                .map_or_else(|| owner.id.clone(), |(_, name)| name.to_string()),
            source: source[owner.start_byte..owner.end_byte].to_string(),
        })
        .collect();
    ParseResult {
        ok: true,
        diagnostics: vec![],
        analysis: Some(BashAnalysis { dialect, source: source.to_string(), functions }),
        policies: vec![],
    }
}

pub fn merge_bash_three_way(
    base_source: &str,
    ours_source: &str,
    theirs_source: &str,
    _dialect: BashDialect,
) -> ThreeWayMergeResult<String> {
    let base = match parse_source_preserving_bash(base_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("base", message),
    };
    let ours = match parse_source_preserving_bash(ours_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("ours", message),
    };
    let theirs = match parse_source_preserving_bash(theirs_source) {
        Ok(document) => document,
        Err(message) => return three_way_parse_error("theirs", message),
    };

    if let Some(conflict) = typed::unmanaged_source_conflict(&base, &ours, &theirs) {
        return conflict;
    }
    merge_source_preserving_owners(base, ours, theirs, parse_source_preserving_bash)
}

pub(crate) fn unmanaged_source_conflict_result() -> ThreeWayMergeResult<String> {
    let message = "Bash owner merge cannot prove ownership of changed unmanaged source";
    let conflict = MergeConflict {
        conflict_id: "bash-unmanaged-source".into(),
        category: "unmanaged_source_change".into(),
        path: "<unmanaged-source>".into(),
        fallback_scope: "full_file".into(),
        message: message.into(),
        alternatives: vec![],
    };
    ThreeWayMergeResult {
        outcome: ast_merge::ThreeWayMergeOutcome::Conflict,
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::MergeConflict,
            message: message.into(),
            path: Some("<unmanaged-source>".into()),
            review: None,
        }],
        conflicts: vec![conflict],
        output: None,
        policies: vec![],
    }
}

fn parse_source_preserving_bash(source: &str) -> Result<SourcePreservingOwnerDocument, String> {
    let parsed = parse_normalized_with_language_pack(&parse_request(source));
    if !parsed.ok {
        return Err(parsed.diagnostics.join("; "));
    }
    if !parsed.source_fragments_available {
        return Err("Bash parser did not retain source fragments".to_string());
    }
    project_bash_functions(source, &parsed.root_id, &parsed.nodes)
}

fn project_bash_functions(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
) -> Result<SourcePreservingOwnerDocument, String> {
    project_bash_analysis(source, root_id, nodes, false)
        .map(|analysis| analysis.document)
        .map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BashAnalysisError {
    Unsupported(String),
    UnsafeSourceRange(String),
}

impl std::fmt::Display for BashAnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) | Self::UnsafeSourceRange(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl From<String> for BashAnalysisError {
    fn from(message: String) -> Self {
        Self::Unsupported(message)
    }
}

fn project_bash_analysis(
    source: &str,
    root_id: &str,
    nodes: &[NormalizedTreeNode],
    reject_unsafe_source_ranges: bool,
) -> Result<ast_merge::native_analysis::NativeOwnerAnalysis, BashAnalysisError> {
    let index = NormalizedTreeIndex::new(nodes)?;
    let root = index.root(root_id)?;
    let mut owners = Vec::new();
    let mut owner_ids = HashSet::new();
    let mut owner_node_ids = std::collections::BTreeMap::new();
    let mut attached_comment_node_ids = std::collections::BTreeMap::new();

    for top_level in index.children(root) {
        if top_level.role == NodeRole::Comment {
            continue;
        }

        let (path, name) = if let Some(owner_kind) =
            BASH_OWNER_KINDS.iter().find(|candidate| candidate.node_kind == top_level.kind)
        {
            let name = named_child_source(&index, top_level)?;
            let path = format!("/{}:{name}", owner_kind.path_kind);
            (path, name)
        } else if let Some(identity) = literal_test_harness_identity(&index, top_level) {
            let encoded = serde_json::to_string(&identity)
                .map_err(|error| format!("failed to encode Bash test identity: {error}"))?;
            (format!("/test_harness_call:{encoded}"), encoded)
        } else {
            return Err(format!("unsupported top-level Bash node {:?}", top_level.kind).into());
        };

        if !owner_ids.insert(path.clone()) {
            return Err(
                format!("Bash document has duplicate top-level owner identity {path:?}").into()
            );
        }
        if top_level.source_fragment.is_empty() {
            return Err(format!("Bash owner {path:?} has no source fragment").into());
        }
        let Some((end_byte, inline_comment_id)) = safe_owner_line_range(source, top_level, nodes)
        else {
            return Err(BashAnalysisError::UnsafeSourceRange(format!(
                "unsafe declaration range [:{}, {:?}]",
                top_level.kind, name
            )));
        };
        if reject_unsafe_source_ranges && inline_comment_id.is_some() {
            return Err(BashAnalysisError::UnsafeSourceRange(format!(
                "unsafe declaration range [:{}, {:?}]",
                top_level.kind, name
            )));
        }

        owner_node_ids.insert(path.clone(), vec![top_level.id.clone()]);
        if let Some(comment_id) = inline_comment_id {
            attached_comment_node_ids.insert(path.clone(), vec![comment_id]);
        }
        let fingerprint = source
            .get(top_level.span.range.start_byte..end_byte)
            .ok_or_else(|| format!("Bash owner {path:?} has an invalid source boundary"))?
            .to_string();
        owners.push(ast_merge::SourcePreservingOwner {
            id: path.clone(),
            path,
            fingerprint,
            start_byte: top_level.span.range.start_byte,
            end_byte,
            start_line: top_level.span.start_point.row + 1,
            end_line: top_level.span.end_point.row + 1,
        });
    }

    if owners.is_empty() {
        return Err("Bash document has no supported top-level named owners".to_string().into());
    }

    Ok(ast_merge::native_analysis::NativeOwnerAnalysis {
        document: SourcePreservingOwnerDocument { source: source.to_string(), owners },
        owner_node_ids,
        attached_comment_node_ids,
    })
}

fn safe_owner_line_range(
    source: &str,
    node: &NormalizedTreeNode,
    nodes: &[NormalizedTreeNode],
) -> Option<(usize, Option<String>)> {
    let bytes = source.as_bytes();
    let start = node.span.range.start_byte;
    let end = node.span.range.end_byte;
    if start > end || end > bytes.len() {
        return None;
    }
    let line_start =
        bytes[..start].iter().rposition(|byte| *byte == b'\n').map_or(0, |index| index + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| end + offset);
    let horizontal_whitespace = |byte: &u8| matches!(byte, b'\t' | b'\r' | b' ');
    if !bytes[line_start..start].iter().all(horizontal_whitespace) {
        return None;
    }
    if bytes[end..line_end].iter().all(horizontal_whitespace) {
        return Some((end, None));
    }

    let mut inline_comments = nodes.iter().filter(|candidate| {
        candidate.role == NodeRole::Comment
            && candidate.span.start_point.row == node.span.end_point.row
            && candidate.span.range.start_byte >= end
            && candidate.span.range.end_byte <= line_end
    });
    let comment = inline_comments.next()?;
    if inline_comments.next().is_some()
        || !bytes[end..comment.span.range.start_byte].iter().all(horizontal_whitespace)
        || !bytes[comment.span.range.end_byte..line_end].iter().all(horizontal_whitespace)
    {
        return None;
    }
    Some((comment.span.range.end_byte, Some(comment.id.clone())))
}

fn named_child_source(
    index: &NormalizedTreeIndex<'_>,
    node: &NormalizedTreeNode,
) -> Result<String, String> {
    let children = index.children(node);
    let candidates = children
        .iter()
        .filter(|child| child.field_name.as_deref() == Some("name"))
        .copied()
        .collect::<Vec<_>>();
    let candidates = if candidates.is_empty() {
        children.iter().filter(|child| child.kind == "word").copied().collect::<Vec<_>>()
    } else {
        candidates
    };
    match candidates.as_slice() {
        [candidate] if !candidate.source_fragment.trim().is_empty() => {
            Ok(candidate.source_fragment.trim().to_string())
        }
        [] => Err(format!("Bash owner {:?} has no stable name", node.kind)),
        _ => Err(format!("Bash owner {:?} has an ambiguous name", node.kind)),
    }
}

fn literal_test_harness_identity(
    index: &NormalizedTreeIndex<'_>,
    node: &NormalizedTreeNode,
) -> Option<Vec<String>> {
    if node.kind != "command" {
        return None;
    }
    let children = index.children(node);
    let command_index = children.iter().position(|child| {
        matches!(child.kind.as_str(), "command_name" | "word")
            && child.source_fragment == "test_expect_success"
    })?;
    if command_index != 0 {
        return None;
    }
    if children.iter().any(|child| child.kind.contains("redirect")) {
        return None;
    }
    let arguments = children.into_iter().skip(command_index + 1).collect::<Vec<_>>();
    if arguments.len() == 2 && literal_test_title(index, arguments[0]) {
        return Some(vec!["test_expect_success".to_string(), arguments[0].source_fragment.clone()]);
    }
    if arguments.len() == 3
        && arguments[0].kind == "word"
        && index.children(arguments[0]).iter().all(|child| !child.named)
        && literal_test_title(index, arguments[1])
    {
        return Some(vec![
            "test_expect_success".to_string(),
            arguments[0].source_fragment.clone(),
            arguments[1].source_fragment.clone(),
        ]);
    }
    None
}

fn literal_test_title(index: &NormalizedTreeIndex<'_>, node: &NormalizedTreeNode) -> bool {
    if node.kind == "raw_string" {
        return true;
    }
    node.kind == "string"
        && index
            .children(node)
            .iter()
            .filter(|child| child.named)
            .all(|child| child.kind == "string_content")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast_merge::{DiagnosticCategory, ThreeWayMergeOutcome};

    #[test]
    fn parses_and_merges_top_level_functions_with_exact_source_preservation() {
        let base = "#!/usr/bin/env bash\nleft() { echo one; }\nright() { echo one; }\n";
        let ours = "#!/usr/bin/env bash\nleft() { echo two; }\nright() { echo one; }\n";
        let theirs = "#!/usr/bin/env bash\nleft() { echo one; }\nright() { echo two; }\n";

        let parsed = parse_bash(base, BashDialect::Bash);
        assert!(parsed.ok, "diagnostics: {:?}", parsed.diagnostics);
        assert_eq!(
            parsed
                .analysis
                .unwrap()
                .functions
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["left", "right"]
        );

        let result = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(
            result.output.as_deref(),
            Some("#!/usr/bin/env bash\nleft() { echo two; }\nright() { echo two; }\n")
        );
    }

    #[test]
    fn preserves_top_level_variable_assignments_as_named_owners() {
        let base = "VALUE=one\nleft() { echo one; }\n";
        let ours = "VALUE=two\nleft() { echo one; }\n";
        let theirs = "VALUE=one\nleft() { echo two; }\n";

        let parsed = parse_bash(base, BashDialect::Bash);
        assert!(parsed.ok, "diagnostics: {:?}", parsed.diagnostics);
        assert_eq!(
            parsed
                .analysis
                .unwrap()
                .functions
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["VALUE", "left"]
        );

        let result = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some("VALUE=two\nleft() { echo two; }\n"));
    }

    #[test]
    fn shared_comment_ownership_fixture_matches_the_bash_typed_merge_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/polyglot/typed-core/bash_merge3_comment_ownership.json"
        ))
        .unwrap();
        let input = &fixture["input"];
        let base = input["base"].as_str().unwrap();
        let ours = input["ours"].as_str().unwrap();
        let theirs = input["theirs"].as_str().unwrap();
        let expected = fixture["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|assertion| assertion["field"] == "output")
            .and_then(|assertion| assertion["value"].as_str())
            .unwrap();

        let result = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(result.output.as_deref(), Some(expected));
    }

    #[test]
    fn preserves_comments_and_blank_line_gaps_between_independent_owner_edits() {
        let base = "# left documentation\nleft() { echo one; }\n\n# right documentation\nright() { echo one; }\n";
        let ours = base.replace("echo one; }\n\n# right", "echo two; }\n\n# right");
        let theirs = base.replace("right() { echo one; }", "right() { echo two; }");

        let result = merge_bash_three_way(base, &ours, &theirs, BashDialect::Bash);

        assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
        assert_eq!(
            result.output.as_deref(),
            Some(
                "# left documentation\nleft() { echo two; }\n\n# right documentation\nright() { echo two; }\n"
            )
        );
    }

    #[test]
    fn preserves_literal_test_harness_calls_as_named_owners() {
        let base = "test_expect_success 'first test' 'echo one'\n";
        let ours = "test_expect_success 'first test' 'echo two'\n";
        let theirs = "test_expect_success 'first test' 'echo three'\n";

        let parsed = parse_bash(base, BashDialect::Bash);
        assert!(parsed.ok, "diagnostics: {:?}", parsed.diagnostics);
        assert_eq!(
            parsed.analysis.unwrap().functions[0].name,
            "[\"test_expect_success\",\"'first test'\"]"
        );

        let result = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(
            result.conflicts[0].path,
            "/test_harness_call:[\"test_expect_success\",\"'first test'\"]"
        );
    }

    #[test]
    fn preserves_literal_test_harness_prerequisites_as_named_owners() {
        let base = "test_expect_success PERL 'first test' 'echo one'\n";
        let ours = "test_expect_success PERL 'first test' 'echo two'\n";
        let theirs = "test_expect_success PERL 'first test' 'echo three'\n";

        let parsed = parse_bash(base, BashDialect::Bash);
        assert!(parsed.ok, "diagnostics: {:?}", parsed.diagnostics);
        assert_eq!(
            parsed.analysis.unwrap().functions[0].name,
            "[\"test_expect_success\",\"PERL\",\"'first test'\"]"
        );

        let result = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);
        assert_eq!(result.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(
            result.conflicts[0].path,
            "/test_harness_call:[\"test_expect_success\",\"PERL\",\"'first test'\"]"
        );
    }

    #[test]
    fn rejects_dynamic_test_harness_titles() {
        let source = "test_expect_success \"dynamic $title\" 'echo one'\n";
        let parsed = parse_bash(source, BashDialect::Bash);
        assert!(!parsed.ok);
        assert!(parsed.diagnostics[0].message.contains("unsupported top-level Bash node"));
    }

    #[test]
    fn does_not_promote_an_argument_named_test_expect_success() {
        let source = "echo test_expect_success 'title' 'echo one'\n";
        let parsed = parse_bash(source, BashDialect::Bash);
        assert!(!parsed.ok);
        assert!(parsed.diagnostics[0].message.contains("unsupported top-level Bash node"));
    }

    #[test]
    fn rejects_conflicting_malformed_and_unowned_bash_changes() {
        let base = "value() { echo one; }\n";
        let ours = "value() { echo two; }\n";
        let theirs = "value() { echo three; }\n";
        let conflict = merge_bash_three_way(base, ours, theirs, BashDialect::Bash);
        assert_eq!(conflict.outcome, ThreeWayMergeOutcome::Conflict);
        assert_eq!(conflict.conflicts[0].path, "/function:value");

        let malformed = merge_bash_three_way(base, ours, "value() {\n", BashDialect::Bash);
        assert_eq!(malformed.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(malformed.diagnostics[0].category, DiagnosticCategory::ParseError);

        let command = merge_bash_three_way(base, ours, "echo unmanaged\n", BashDialect::Bash);
        assert_eq!(command.outcome, ThreeWayMergeOutcome::Error);
        assert_eq!(command.diagnostics[0].category, DiagnosticCategory::ParseError);
    }
}
