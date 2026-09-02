use ast_merge::detect_freeze_directive_blocks;
use ruby_merge::{RubyDialect, parse_ruby, ruby_discovered_surfaces};

#[test]
fn ruby_magic_and_coverage_comments_remain_language_specific() {
    let source = "# frozen_string_literal: true\n\
# typed: strict\n\
# :nocov:\n\
# Public docs\n\
class Example\n\
end\n";
    let analysis = parse_ruby(source, RubyDialect::Ruby).analysis.unwrap();
    let surfaces = ruby_discovered_surfaces(&analysis);

    assert_eq!(surfaces.len(), 1);
    let entries = surfaces[0].metadata["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["raw"], "# Public docs");

    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let (blocks, diagnostics) =
        detect_freeze_directive_blocks("ruby-directives", &lines, "ast-merge", "hash_comment");
    assert!(blocks.is_empty());
    assert!(diagnostics.is_empty());
}

#[test]
fn generic_freeze_tokens_still_apply_to_ruby_hash_comments() {
    let lines = ["# ast-merge:freeze", "VALUE = 1", "# ast-merge:unfreeze"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let (blocks, diagnostics) =
        detect_freeze_directive_blocks("ruby-freeze", &lines, "ast-merge", "hash_comment");

    assert!(diagnostics.is_empty());
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content_lines, lines);
}
