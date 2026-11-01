use ast_merge::SplicePlan;
use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn shared_structural_splice_fixture_matches_ruby() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/polyglot/ast-merge/structural_splice.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut plan = SplicePlan::new(
            case["source"].as_str().unwrap(),
            case["replacement"].as_str().unwrap(),
            case["replace_start_line"].as_u64().unwrap() as usize,
            case["replace_end_line"].as_u64().unwrap() as usize,
        )
        .unwrap();
        if let Some(preserve) = case["preserve_removed_trailing_blank_lines"].as_bool() {
            plan = plan.preserve_removed_trailing_blank_lines(preserve);
        }
        assert_eq!(plan.before_content(), case["expected"]["before"].as_str().unwrap());
        assert_eq!(plan.removed_content(), case["expected"]["removed"].as_str().unwrap());
        assert_eq!(plan.after_content(), case["expected"]["after"].as_str().unwrap());
        assert_eq!(plan.merged_content(), case["expected"]["merged"].as_str().unwrap());
        assert_eq!(plan.changed(), case["expected"]["changed"].as_bool().unwrap());
    }
}

#[test]
fn structural_splice_rejects_invalid_line_ranges() {
    assert!(SplicePlan::new("one\n", "", 0, 1).is_err());
    assert!(SplicePlan::new("one\n", "", 2, 1).is_err());
    assert!(SplicePlan::new("one\n", "", 1, 2).is_err());
}
