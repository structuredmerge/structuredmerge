use bash_merge::{BashDialect, merge_bash_three_way};
use serde_json::Value;

#[test]
fn shared_unmanaged_source_membership_fixture_matches_ruby() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/polyglot/ast-merge/bash_unmanaged_source_membership_conflict.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let result = merge_bash_three_way(
            input["base"].as_str().unwrap(),
            input["ours"].as_str().unwrap(),
            input["theirs"].as_str().unwrap(),
            BashDialect::Bash,
        );
        assert_eq!(
            result.outcome == ast_merge::ThreeWayMergeOutcome::Clean,
            case["expected"]["ok"].as_bool().unwrap(),
            "{}",
            case["name"].as_str().unwrap()
        );
        assert_eq!(
            result.output,
            case["expected"]["output"].as_str().map(str::to_owned),
            "{}",
            case["name"].as_str().unwrap()
        );
        assert_eq!(
            !result.conflicts.is_empty(),
            case["expected"]["conflict"].as_bool().unwrap(),
            "{}",
            case["name"].as_str().unwrap()
        );
    }
}
