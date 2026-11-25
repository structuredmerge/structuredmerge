//! tree-sitter-rust 1.27 node shape, consumed natively: an item owns its outer
//! attributes and doc comments (inside an `attributes` child), doc comments end
//! at column 0 of the next row, and the pieces of a comment are not comments.
use tree_haver::{NodeRole, ParserRequest, parse_normalized_with_language_pack};

fn parse(source: &str) -> tree_haver::NormalizedParseResult {
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: "rust".to_string(),
        dialect: Some("rust".to_string()),
    });
    assert!(parsed.ok, "{:?}", parsed.diagnostics);
    parsed
}

#[test]
fn item_owns_its_outer_attributes_and_doc_comments() {
    let parsed = parse("/// doc\n#[test]\nfn f() {}\n");
    let root = parsed.nodes.iter().find(|node| node.id == parsed.root_id).unwrap();
    let item = parsed.nodes.iter().find(|node| node.id == root.child_ids[0]).unwrap();
    assert_eq!(item.kind, "function_item");
    assert_eq!(item.span.start_point.row, 0, "the item starts at its first doc comment");
    let attributes = parsed.nodes.iter().find(|node| node.id == item.child_ids[0]).unwrap();
    assert_eq!(attributes.kind, "attributes");
    let kinds = attributes
        .child_ids
        .iter()
        .map(|id| parsed.nodes.iter().find(|node| &node.id == id).unwrap().kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(kinds, ["line_outer_doc_comment", "attribute_item"]);
}

#[test]
fn doc_comments_end_at_the_next_row_and_report_their_last_line() {
    let parsed = parse("//! module\n/** block doc */\nstruct S;\n");
    let root = parsed.nodes.iter().find(|node| node.id == parsed.root_id).unwrap();
    let inner = parsed.nodes.iter().find(|node| node.id == root.child_ids[0]).unwrap();
    assert_eq!(inner.kind, "line_comment");
    assert_eq!((inner.span.end_point.row, inner.span.end_point.column), (1, 0));
    assert_eq!(inner.span.last_line(), 1);
}

#[test]
fn the_pieces_of_a_comment_are_not_comments() {
    let parsed = parse("//! module\nfn f() {}\n");
    let comments = parsed.nodes.iter().filter(|node| node.role == NodeRole::Comment).count();
    assert_eq!(comments, 1, "only the line_comment itself; its doc_comment payload is a token");
}
