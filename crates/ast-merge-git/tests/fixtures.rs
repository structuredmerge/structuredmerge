use ast_merge_git::{Merge3Request, merge3};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct Fixture {
    contract: Contract,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct Contract {
    package: String,
    operation: String,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    case_id: String,
    request: Merge3Request,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
struct Expected {
    ok: bool,
    merged_json: Option<Value>,
    conflict_count: usize,
    conflict_categories: Option<Vec<String>>,
    conflict_paths: Option<Vec<String>>,
    reparse_after_render: Option<bool>,
}

#[test]
fn conforms_to_git_merge3_contract_fixture() {
    let fixture: Fixture =
        read_fixture(&["diagnostics", "slice-950-git-merge3-contract", "git-merge3-contract.json"]);
    assert_eq!(fixture.contract.package, "ast-merge-git");
    assert_eq!(fixture.contract.operation, "merge3");

    for case in fixture.cases {
        let result = merge3(&case.request);
        assert_eq!(result.ok, case.expected.ok, "{}", case.case_id);
        assert_eq!(result.conflicts.len(), case.expected.conflict_count, "{}", case.case_id);
        assert_eq!(
            result.reparse_after_render, case.expected.reparse_after_render,
            "{}",
            case.case_id
        );
        if result.ok {
            let merged: Value =
                serde_json::from_str(result.merged_source.as_deref().unwrap_or_default())
                    .expect("merged source should parse");
            assert_eq!(merged, case.expected.merged_json.unwrap(), "{}", case.case_id);
        } else {
            let categories = result
                .conflicts
                .iter()
                .map(|conflict| conflict.category.clone())
                .collect::<Vec<_>>();
            let paths =
                result.conflicts.iter().map(|conflict| conflict.path.clone()).collect::<Vec<_>>();
            assert_eq!(categories, case.expected.conflict_categories.unwrap(), "{}", case.case_id);
            assert_eq!(paths, case.expected.conflict_paths.unwrap(), "{}", case.case_id);
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
