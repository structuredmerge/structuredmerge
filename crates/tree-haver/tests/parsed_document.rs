use std::collections::BTreeMap;

use serde_json::json;
use tree_haver::parsed::*;
use tree_haver::source::{SourceDocument, SourceEncoding, SourceRole, source_input};
use tree_haver::{ByteRange, NodeRole, SourcePoint, SourceSpan};

fn source() -> SourceDocument {
    SourceDocument::validate(
        source_input(
            "current".into(),
            SourceRole::Current,
            SourceEncoding::Utf8,
            "é\r\n# note\n".as_bytes().to_vec(),
        )
        .unwrap(),
        100,
    )
    .unwrap()
}

fn span(start: usize, end: usize) -> SourceSpan {
    let source = source();
    SourceSpan {
        range: ByteRange { start_byte: start, end_byte: end },
        start_point: source.point(start).unwrap(),
        end_point: source.point(end).unwrap(),
    }
}

fn output() -> ParseOutput {
    let child = ParseNode {
        id: "comment".into(),
        kind: "comment".into(),
        native_type: "native_comment".into(),
        role: NodeRole::Comment,
        named: true,
        missing: false,
        has_error: false,
        span: span(4, 10),
        parent_id: Some("root".into()),
        children: vec![],
        semantic_roles: vec![],
        unsupported_features: vec![],
        extensions: vec![],
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let root = ParseNode {
        id: "root".into(),
        kind: "document".into(),
        native_type: "native_document".into(),
        role: NodeRole::Structural,
        named: true,
        missing: false,
        has_error: false,
        span: span(0, 11),
        parent_id: None,
        children: vec![ChildEdge {
            node_id: "comment".into(),
            index: 0,
            field_name: Some("body".into()),
            extra: BTreeMap::new(),
        }],
        semantic_roles: vec![],
        unsupported_features: vec![],
        extensions: vec![],
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    ParseOutput {
        request_id: "parse-1".into(),
        source: source().descriptor().clone(),
        ok: true,
        root_id: Some("root".into()),
        nodes: vec![root, child],
        comments: vec![ParseComment {
            node_id: "comment".into(),
            native_kind: "line".into(),
            attachment_hint: AttachmentHint::Unknown,
            metadata: BTreeMap::new(),
            extra: BTreeMap::new(),
        }],
        diagnostics: vec![],
        extensions: vec![],
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    }
}

fn validate(output: ParseOutput) -> Result<ParsedDocument, ParseValidationError> {
    ParsedDocument::validate(
        output,
        "parse-1",
        &source(),
        ParseValidationLimits {
            max_nodes: 20,
            max_diagnostics: 20,
            partial_tree_allowed: false,
            comments_supported: true,
        },
    )
}

#[test]
fn accepts_byte_points_and_edge_field_names_without_copying_source_text() {
    let document = validate(output()).unwrap();
    assert_eq!(
        document.node("comment").unwrap().span.start_point,
        SourcePoint { row: 1, column: 0 }
    );
    assert_eq!(document.node("root").unwrap().children[0].field_name.as_deref(), Some("body"));
    assert_eq!(document.output().comments[0].attachment_hint, AttachmentHint::Unknown);
}

#[test]
fn validates_source_request_identity_and_byte_points() {
    let mut bad = output();
    bad.request_id = "another-request".into();
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::IdentityMismatch);
    let mut bad = output();
    bad.source.role = SourceRole::Ours;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::IdentityMismatch);
    let mut bad = output();
    bad.nodes[1].span.start_point.column = 4;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidSpan);
    let mut bad = output();
    bad.nodes[1].span.range.end_byte = 1000;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidSpan);
}

#[test]
fn rejects_duplicate_nodes_broken_edges_and_unreachable_cycles() {
    let mut bad = output();
    bad.nodes.push(bad.nodes[1].clone());
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidNode);
    let mut bad = output();
    bad.nodes[0].children[0].index = 1;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidEdge);
    let mut bad = output();
    bad.nodes[1].parent_id = None;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidEdge);
    let mut bad = output();
    bad.nodes[0].children.clear();
    bad.nodes[1].parent_id = Some("comment".into());
    bad.nodes[1].children = vec![ChildEdge {
        node_id: "comment".into(),
        index: 0,
        field_name: None,
        extra: BTreeMap::new(),
    }];
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::DisconnectedTree);
}

#[test]
fn comments_must_be_accounted_for_exactly_once() {
    let mut bad = output();
    bad.comments.clear();
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidComment);
    let mut bad = output();
    bad.comments.push(bad.comments[0].clone());
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidComment);
}

#[test]
fn forwards_unknown_native_extensions_and_node_fields_without_interpreting_them() {
    let mut original = output();
    original.extensions.push(NativeExtension {
        schema: "example.native/v2".into(),
        namespace: "example".into(),
        capabilities: vec!["future".into()],
        payload: json!({"tokens": [0, 255], "flags": {"raw": true}}),
        extra: BTreeMap::from([("future_header".into(), json!(42))]),
    });
    original.nodes[0].extra.insert("future_node_field".into(), json!({"value": "opaque"}));
    original.nodes[0].children[0].extra.insert("future_edge_field".into(), json!([true, null]));
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: ParseOutput = serde_json::from_slice(&encoded).unwrap();
    let validated = validate(decoded).unwrap();
    assert_eq!(validated.output(), &original);
    assert_eq!(serde_json::to_vec(validated.output()).unwrap(), encoded);
}

#[test]
fn rejects_success_with_errors_and_unadvertised_partial_trees() {
    let mut bad = output();
    bad.nodes[1].has_error = true;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidNode);
    let mut bad = output();
    bad.ok = false;
    assert_eq!(validate(bad.clone()).unwrap_err(), ParseValidationError::InvalidOutcome);
    bad.diagnostics.push(ParseDiagnostic {
        id: "syntax".into(),
        severity: ParseSeverity::Error,
        category: "parse_error".into(),
        code: Some("native.syntax".into()),
        message: "malformed source".into(),
        source_role: SourceRole::Current,
        span: None,
        node_id: None,
        blocking: true,
        metadata: BTreeMap::new(),
        extra: BTreeMap::new(),
    });
    assert_eq!(validate(bad.clone()).unwrap_err(), ParseValidationError::InvalidOutcome);
    bad.nodes.clear();
    bad.comments.clear();
    bad.root_id = None;
    assert!(validate(bad.clone()).is_ok());
    bad.diagnostics[0].source_role = SourceRole::Base;
    assert_eq!(validate(bad).unwrap_err(), ParseValidationError::InvalidDiagnostic);
}
