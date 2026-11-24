//! The Rust tree-sitter projection presents one stable node shape across
//! tree-sitter-rust revisions: outer attributes and doc comments precede their
//! item as siblings, and line comments end on their own line.
use tree_haver::{ParserRequest, parse_normalized_with_language_pack};

fn top_level(source: &str) -> Vec<(String, usize, usize, String)> {
    let parsed = parse_normalized_with_language_pack(&ParserRequest {
        source: source.to_string(),
        language: "rust".to_string(),
        dialect: Some("rust".to_string()),
    });
    assert!(parsed.ok, "{:?}", parsed.diagnostics);
    let root = parsed.nodes.iter().find(|node| node.id == parsed.root_id).unwrap();
    root.child_ids
        .iter()
        .map(|id| parsed.nodes.iter().find(|node| &node.id == id).unwrap())
        .map(|node| {
            (
                node.kind.clone(),
                node.span.start_point.row,
                node.span.end_point.row,
                node.source_fragment.clone(),
            )
        })
        .collect()
}

#[test]
fn outer_docs_and_attributes_precede_their_item() {
    let shape = top_level("/// doc\n#[test]\nfn f() {}\n");

    assert_eq!(
        shape,
        vec![
            ("line_comment".to_string(), 0, 0, "/// doc".to_string()),
            ("attribute_item".to_string(), 1, 1, "#[test]".to_string()),
            ("function_item".to_string(), 2, 2, "fn f() {}".to_string()),
        ]
    );
}

#[test]
fn inner_doc_and_block_doc_comments_end_on_their_own_line() {
    let shape = top_level("//! module\n/** block doc */\nstruct S;\n");

    assert_eq!(
        shape,
        vec![
            ("line_comment".to_string(), 0, 0, "//! module".to_string()),
            ("block_comment".to_string(), 1, 1, "/** block doc */".to_string()),
            ("struct_item".to_string(), 2, 2, "struct S;".to_string()),
        ]
    );
}
