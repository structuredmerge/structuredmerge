use ast_merge::{
    ConformanceFamilyPlanContext, ConformanceFeatureProfileView, FamilyFeatureProfile,
    NamedOwnerKind, NamedOwnerProjectionPolicy, ParseResult, PolicyReference,
    SourcePreservingOwnerDocument, ThreeWayMergeResult, merge_source_preserving_owners,
    normalized_parse_error_result, parse_error_result, project_named_top_level_owners,
    three_way_parse_error,
};
use tree_haver::{
    BackendReference, NormalizedTreeNode, ParserRequest, kreuzberg_language_pack_backend,
    language_pack_adapter_info, parse_normalized_with_language_pack,
};

pub const PACKAGE_NAME: &str = "bash-merge";

const BASH_OWNER_KINDS: &[NamedOwnerKind<'static>] =
    &[NamedOwnerKind { node_kind: "function_definition", path_kind: "function" }];

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
            name: owner.id.strip_prefix("/function:").unwrap_or(&owner.id).to_string(),
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

    merge_source_preserving_owners(base, ours, theirs, parse_source_preserving_bash)
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
    project_named_top_level_owners(
        source,
        root_id,
        nodes,
        NamedOwnerProjectionPolicy {
            family: "Bash",
            owner_kinds: BASH_OWNER_KINDS,
            ignored_kinds: &[],
            wrapper_kinds: &[],
            name_fields: &["name"],
            fallback_name_kinds: &["word"],
            accept_any_named_kind: false,
        },
    )
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
