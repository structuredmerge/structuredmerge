use ast_merge::DiagnosticCategory;
use ast_merge_git::{Merge3Request, merge_comment_delta, merge3};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct Contract {
    package: String,
    operation: String,
}

#[derive(Debug, Deserialize)]
struct CommentDeltaFixture {
    contract: Contract,
    owner: CommentDeltaOwner,
    cases: Vec<CommentDeltaCase>,
}

#[derive(Debug, Deserialize)]
struct CommentDeltaOwner {
    path: String,
}

#[derive(Debug, Deserialize)]
struct CommentDeltaCase {
    case_id: String,
    base_comment: Option<String>,
    ours_comment: Option<String>,
    theirs_comment: Option<String>,
    expected: CommentDeltaExpected,
}

#[derive(Debug, Deserialize)]
struct CommentDeltaExpected {
    ok: bool,
    merged_comment: Option<String>,
    conflict_count: usize,
    conflict_categories: Option<Vec<String>>,
    comment_owner_path: Option<String>,
}

fn json_request(base: &str, ours: &str, theirs: &str, dialect: &str) -> Merge3Request {
    Merge3Request {
        base_source: base.to_string(),
        ours_source: ours.to_string(),
        theirs_source: theirs.to_string(),
        path_name: Some(format!("fixture.{dialect}")),
        language: Some(dialect.to_string()),
        dialect: Some(dialect.to_string()),
        profile_id: Some("source_preserving".to_string()),
        fallback_policy: Some("none".to_string()),
        conflict_marker_size: Some(7),
        render_policy: Some("source_preserving_edits".to_string()),
    }
}

#[test]
fn delegates_json_to_the_source_preserving_substrate() {
    let result = merge3(&json_request(
        "{\n  \"shared\": true\n}\n",
        "{\n  \"shared\": true,\n  \"ours\": 1\n}\n",
        "{\n  \"shared\": true,\n  \"theirs\": 2\n}\n",
        "json",
    ));

    assert!(result.ok);
    assert_eq!(
        result.merged_source.as_deref(),
        Some("{\n  \"shared\": true,\n  \"ours\": 1,\n  \"theirs\": 2\n}\n")
    );
    assert_eq!(result.render_report.strategy, "source_preserving_edits");
    assert_eq!(result.render_report.backend_id, "tree-sitter-language-pack");
    assert_eq!(result.render_report.parser_identity, "tree-haver");
}

#[test]
fn preserves_jsonc_comments_through_the_same_substrate() {
    let result = merge3(&json_request(
        "{\n  // retained\n  \"shared\": true\n}\n",
        "{\n  // retained\n  \"shared\": true,\n  \"ours\": 1\n}\n",
        "{\n  // retained\n  \"shared\": true,\n  \"theirs\": 2\n}\n",
        "jsonc",
    ));

    assert!(result.ok);
    assert!(result.merged_source.unwrap().contains("// retained"));
}

#[test]
fn propagates_structural_conflicts_without_guessing_source_ranges() {
    let result = merge3(&json_request(
        "{\"enabled\":true}",
        "{\"enabled\":false}",
        "{\"enabled\":\"yes\"}",
        "json",
    ));

    assert!(!result.ok);
    assert_eq!(result.conflicts.len(), 1);
    assert_eq!(result.conflicts[0].category, "modify_modify");
    assert_eq!(result.conflicts[0].path, "/enabled");
    assert!(result.conflicted_source.is_none());
    assert!(result.owned_regions.is_empty());
}

#[test]
fn fails_closed_on_tree_haver_parse_errors() {
    let result =
        merge3(&json_request("{\"ok\":true}\n", "{\"ok\": tru\n", "{\"ok\":false}\n", "json"));

    assert!(!result.ok);
    assert!(result.conflicts.is_empty());
    assert_eq!(result.diagnostics[0].category, DiagnosticCategory::ParseError);
    assert!(result.diagnostics[0].message.starts_with("ours parse error:"));
}

#[test]
fn conforms_to_git_comment_delta_semantics_fixture() {
    let fixture: CommentDeltaFixture = read_fixture(&[
        "diagnostics",
        "slice-953-git-comment-delta-semantics",
        "git-comment-delta-semantics.json",
    ]);
    assert_eq!(fixture.contract.package, "ast-merge-git");
    assert_eq!(fixture.contract.operation, "comment_delta_semantics");

    for case in fixture.cases {
        let result = merge_comment_delta(
            case.base_comment.as_deref(),
            case.ours_comment.as_deref(),
            case.theirs_comment.as_deref(),
            &fixture.owner.path,
        );
        assert_eq!(result.ok, case.expected.ok, "{}", case.case_id);
        assert_eq!(result.conflicts.len(), case.expected.conflict_count, "{}", case.case_id);
        assert_eq!(result.merged_comment, case.expected.merged_comment, "{}", case.case_id);
        if let Some(expected_categories) = case.expected.conflict_categories {
            let categories = result
                .conflicts
                .iter()
                .map(|conflict| conflict.category.clone())
                .collect::<Vec<_>>();
            assert_eq!(categories, expected_categories, "{}", case.case_id);
        }
        if let Some(expected_owner_path) = case.expected.comment_owner_path {
            assert_eq!(fixture.owner.path, expected_owner_path, "{}", case.case_id);
        }
    }
}

fn read_fixture<T: for<'de> Deserialize<'de>>(parts: &[&str]) -> T {
    let path = fixture_path(parts);
    let source = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read fixture {}: {error}", path.display());
    });
    serde_json::from_str(&source).expect("fixture should deserialize")
}

fn fixture_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../../fixtures");
    for part in parts {
        path.push(part);
    }
    path
}
