use std::{fs, path::PathBuf};

use markdown_merge::MarkdownDialect;
use pulldown_cmark_merge::{
    available_markdown_backends, markdown_backend_feature_profile, markdown_plan_context,
    match_markdown_owners, parse_markdown, provider_markdown_feature_profile,
};
use serde_json::Value;

fn fixture_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("..");
    path.push("fixtures");
    for part in parts {
        path.push(part);
    }
    path
}

fn read_fixture(parts: &[&str]) -> Value {
    let source = fs::read_to_string(fixture_path(parts)).expect("fixture should be readable");
    serde_json::from_str(&source).expect("fixture should be valid json")
}

#[test]
fn conforms_to_provider_feature_profile_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-204-markdown-provider-feature-profiles",
        "rust-markdown-provider-feature-profiles.json",
    ]);

    assert_eq!(available_markdown_backends(), vec!["pulldown-cmark".to_string()]);
    assert_eq!(
        serde_json::to_value(markdown_backend_feature_profile()).unwrap(),
        fixture["providers"]["pulldown-cmark"]["feature_profile"]
    );
    assert_eq!(provider_markdown_feature_profile().family, "markdown");
}

#[test]
fn conforms_to_provider_plan_context_fixture() {
    let fixture = read_fixture(&[
        "diagnostics",
        "slice-205-markdown-provider-plan-contexts",
        "rust-markdown-provider-plan-contexts.json",
    ]);

    assert_eq!(
        serde_json::to_value(markdown_plan_context()).unwrap(),
        fixture["providers"]["pulldown-cmark"]
    );
}

#[test]
fn conforms_to_shared_markdown_analysis_and_matching_fixtures() {
    let analysis_fixture =
        read_fixture(&["markdown", "slice-198-analysis", "headings-and-code-fences.json"]);
    let matching_fixture = read_fixture(&["markdown", "slice-199-matching", "path-equality.json"]);

    let analysis = parse_markdown(
        analysis_fixture["source"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    );
    assert!(analysis.ok);
    assert_eq!(analysis.analysis.unwrap().root_kind, markdown_merge::MarkdownRootKind::Document);

    let template = parse_markdown(
        matching_fixture["template"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    )
    .analysis
    .unwrap();
    let destination = parse_markdown(
        matching_fixture["destination"].as_str().unwrap(),
        MarkdownDialect::Markdown,
        None,
    )
    .analysis
    .unwrap();

    let result = match_markdown_owners(template, destination);
    assert_eq!(
        result
            .matched
            .iter()
            .map(|item| vec![item.template_path.clone(), item.destination_path.clone()])
            .collect::<Vec<_>>(),
        matching_fixture["expected"]["matched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                item.as_array()
                    .unwrap()
                    .iter()
                    .map(|part| part.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    );
}
