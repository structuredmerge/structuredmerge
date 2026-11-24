//! Grammar-revision normalization at the tree-sitter projection boundary.
//!
//! Consumers (owner projection, comment attachment, layout) are written against
//! one node shape. tree-sitter grammars change shape between releases, so the
//! projection normalizes known differences instead of every consumer learning
//! each revision.
//!
//! tree-sitter-rust (shipped by tree-sitter-language-pack 1.27+) differs from
//! the shape earlier releases produced:
//!
//! * outer attributes *and* outer doc comments (`///`, `/** */`) of an item are
//!   children of an `attributes` node *inside* the item, so the item starts at
//!   its first attribute or doc comment; earlier grammars made them preceding
//!   siblings (`attribute_item` / `line_comment` / `block_comment`) and the item
//!   began at its first own token;
//! * line comments that carry doc markers include their terminating newline, so
//!   the span ends at column 0 of the following row;
//! * doc comments expose internal marker and `doc_comment` children.
//!
//! Normalization hoists the `attributes` children to be preceding siblings of the item,
//! renames doc comment kinds to `line_comment` / `block_comment`, trims the
//! trailing newline from line comments, starts items at their first own token
//! and hides comment internals. Other languages are untouched.
use tree_sitter_language_pack::Node;

/// A tree-sitter node with the kind and span consumers should see.
#[derive(Clone)]
pub(crate) struct Projected {
    pub node: Node,
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize),
    pub end_point: (usize, usize),
}

impl Projected {
    pub(crate) fn root(node: Node, language: &str, source: &str) -> Self {
        Self::from_raw(node, language, source)
    }

    fn from_raw(node: Node, language: &str, source: &str) -> Self {
        let raw_start = node.start_position();
        let raw_end = node.end_position();
        let mut projected = Self {
            kind: node.kind().to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: (raw_start.row, raw_start.column),
            end_point: (raw_end.row, raw_end.column),
            node,
        };
        if language == "rust" {
            projected.normalize_rust(source);
        }
        projected
    }

    fn normalize_rust(&mut self, source: &str) {
        match self.kind.as_str() {
            "line_comment" | "line_outer_doc_comment" => {
                self.kind = "line_comment".into();
                self.trim_trailing_newline(source);
            }
            "block_comment" | "block_outer_doc_comment" => {
                self.kind = "block_comment".into();
            }
            _ => {
                let first = self.node.child(0);
                if first.as_ref().is_some_and(|child| child.kind() == "attributes")
                    && let Some(own) = self.node.child(1)
                {
                    let start = own.start_position();
                    self.start_byte = own.start_byte();
                    self.start_point = (start.row, start.column);
                }
            }
        }
    }

    fn trim_trailing_newline(&mut self, source: &str) {
        if self.end_point.1 == 0
            && self.end_point.0 > self.start_point.0
            && self.end_byte > self.start_byte
            && source.as_bytes().get(self.end_byte - 1) == Some(&b'\n')
        {
            self.end_byte -= 1;
            let line_start = source[..self.end_byte].rfind('\n').map_or(0, |index| index + 1);
            self.end_point = (self.end_point.0 - 1, self.end_byte - line_start);
        }
    }

    /// Normalized children in source order with their field names.
    pub(crate) fn children(
        &self,
        language: &str,
        source: &str,
    ) -> Vec<(Projected, Option<String>)> {
        let mut children = Vec::new();
        let mut cursor = self.node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                // The text payload of a comment is already the comment's own text.
                if language == "rust" && child.kind() == "doc_comment" {
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
                // Hoisted below, ahead of the item that owned them.
                if language == "rust" && child.kind() == "attributes" {
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
                if language == "rust"
                    && let Some(attributes) =
                        child.child(0).filter(|first| first.kind() == "attributes")
                {
                    let mut inner = attributes.walk();
                    if inner.goto_first_child() {
                        loop {
                            children.push((
                                Self::from_raw(inner.node(), language, source),
                                inner.field_name(),
                            ));
                            if !inner.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                children.push((Self::from_raw(child, language, source), cursor.field_name()));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        children
    }
}
