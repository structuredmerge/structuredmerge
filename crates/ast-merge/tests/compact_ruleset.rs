use std::{fs, path::Path};

use ast_merge::parse_compact_ruleset;

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
