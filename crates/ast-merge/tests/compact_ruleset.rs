use std::{fs, path::Path};

use ast_merge::{CompactRulesetProfile, compact_ruleset_feature_profile, parse_compact_ruleset};

#[test]
fn parses_compact_ruleset_fixtures() {
    let ruleset_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/rulesets");
    let mut count = 0;
    visit_smrules(&ruleset_root, &mut |path| {
        let source = fs::read_to_string(path).expect("ruleset fixture should be readable");
        let result = parse_compact_ruleset(&source);
        assert!(result.ok, "expected {} to parse: {:?}", path.display(), result.diagnostics);
        assert!(
            result.analysis.as_ref().is_some_and(|analysis| !analysis.directives.is_empty()),
            "expected {} to produce directives",
            path.display()
        );
        count += 1;
    });
    assert!(count > 0, "expected compact ruleset fixtures");
}

#[test]
fn rejects_compact_ruleset_edges() {
    let cases = [
        (
            "missing-required",
            "format json\nowners line_bound_statements\nmatch stable_path\nread native_read_portable_write\n",
        ),
        (
            "repeated-format",
            "format json\nformat yaml\nowners line_bound_statements\nmatch stable_path\nread native_read_portable_write\nattach layout_only\n",
        ),
        (
            "unknown-read",
            "format json\nowners line_bound_statements\nmatch stable_path\nread imaginary\nattach layout_only\n",
        ),
        (
            "unknown-directive",
            "format json\nowners line_bound_statements\nmatch stable_path\nread native_read_portable_write\nattach layout_only\nmystery value\n",
        ),
    ];
    for (name, source) in cases {
        let result = parse_compact_ruleset(source);
        assert!(!result.ok, "expected {name} to fail");
        assert!(!result.diagnostics.is_empty(), "expected {name} to produce diagnostics");
    }
}

#[test]
fn derives_compact_ruleset_feature_profile_fixture() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../fixtures/diagnostics/slice-781-compact-ruleset-profile/module-profile.json",
    );
    let fixture: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture_path).expect("profile fixture should be readable"),
    )
    .expect("profile fixture should parse");

    let ruleset_path_parts =
        fixture["ruleset_path"].as_array().expect("ruleset path should be an array");
    let mut ruleset_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    for part in ruleset_path_parts {
        ruleset_path.push(part.as_str().expect("ruleset path part should be a string"));
    }

    let source = fs::read_to_string(ruleset_path).expect("ruleset fixture should be readable");
    let result = parse_compact_ruleset(&source);
    assert!(result.ok, "expected profile ruleset to parse: {:?}", result.diagnostics);

    let expected: CompactRulesetProfile =
        serde_json::from_value(fixture["profile"].clone()).expect("profile should deserialize");
    let actual = compact_ruleset_feature_profile(
        result.analysis.as_ref().expect("profile ruleset should produce analysis"),
    );
    assert_eq!(actual, expected);
}

fn visit_smrules(root: &Path, visit: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(root).expect("ruleset directory should be readable") {
        let entry = entry.expect("ruleset directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            visit_smrules(&path, visit);
        } else if path.extension().is_some_and(|extension| extension == "smrules") {
            visit(&path);
        }
    }
}
