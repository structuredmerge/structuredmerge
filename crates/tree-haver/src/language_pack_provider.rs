//! Explicit typed TreeHaver provider for the existing Rust language pack.
//! Construction never loads a grammar. Probe/parse may use the language pack's
//! configured cache/download behavior; registration is not default selection.
use crate::{
    ByteRange, NodeRole, SourcePoint, SourceSpan,
    parsed::{
        AttachmentHint, ChildEdge, NativeExtension, ParseComment, ParseDiagnostic, ParseNode,
        ParseOutput, ParseSeverity,
    },
    service::{
        ExecutionContext, PARSE_RESULT_SCHEMA, ParseRequest, ParserProbeRequest, ParserProbeResult,
        ParserProvider, ParserProviderDescriptor, ProviderFault,
    },
    source::{SourceDocument, SourceEncoding},
};

pub struct LanguagePackProvider {
    descriptor: ParserProviderDescriptor,
}

fn fault(code: &str, message: impl Into<String>) -> ProviderFault {
    ProviderFault { code: code.into(), message: message.into() }
}

fn node_flags(payload: serde_json::Value) -> NativeExtension {
    NativeExtension {
        schema: "tree-haver.tree-sitter.node/v1".into(),
        namespace: "tree-sitter".into(),
        capabilities: vec!["node_flags".into()],
        payload,
        extra: Default::default(),
    }
}

impl LanguagePackProvider {
    pub fn new(id: String, language: String) -> Result<Self, ProviderFault> {
        if id.trim().is_empty() || language.trim().is_empty() {
            return Err(fault("request.invalid", "provider ID and language must not be empty"));
        }
        Ok(Self {
            descriptor: ParserProviderDescriptor {
                id: id.clone(),
                family: "tree-sitter".into(),
                runtime: "rust".into(),
                package: "tree-haver".into(),
                package_version: env!("CARGO_PKG_VERSION").into(),
                parser: "tree-sitter-language-pack".into(),
                parser_version: "runtime".into(),
                grammar: Some(language.clone()),
                grammar_version: None,
                languages: vec![language],
                dialects: vec![],
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec![
                    "comments".into(),
                    "diagnostics".into(),
                    "native_extensions".into(),
                    "partial_trees".into(),
                    "source_spans".into(),
                ],
                probe_id: id,
                priority: 0,
                metadata: Default::default(),
                extensions: vec![node_flags(serde_json::json!({}))],
            },
        })
    }

    fn parse(
        &self,
        request: ParseRequest,
        context: &ExecutionContext,
    ) -> Result<ParseOutput, ProviderFault> {
        context.check().map_err(|e| fault("execution.interrupted", format!("{e:?}")))?;
        if request.language != self.descriptor.languages[0]
            || request.dialect.is_some()
            || request.options.tokens
            || request.source.descriptor.encoding != SourceEncoding::Utf8
        {
            return Err(fault(
                "request.unsupported",
                "unsupported language, dialect, encoding or parser option",
            ));
        }
        let source = SourceDocument::validate(request.source.clone(), context.max_input_bytes)
            .map_err(|e| fault("source.invalid", e.to_string()))?;
        let text = std::str::from_utf8(source.bytes())
            .map_err(|e| fault("source.invalid", e.to_string()))?;
        crate::ensure_language_pack_language(&request.language)
            .map_err(|e| fault("parser.unavailable", e))?;
        let mut parser = tree_sitter_language_pack::get_parser(&request.language)
            .map_err(|e| fault("parser.unavailable", e.to_string()))?;
        let tree =
            parser.parse(text).ok_or_else(|| fault("parser.failed", "parser returned no tree"))?;
        context.check().map_err(|e| fault("execution.interrupted", format!("{e:?}")))?;
        let root = tree.root_node();
        let ok = !root.has_error();
        let mut nodes: Vec<ParseNode> = Vec::new();
        let mut comments = Vec::new();
        // Iterative traversal avoids adding a Rust call-stack limit to the tree.
        let mut stack = vec![(root, None::<usize>, None::<String>)];
        while let Some((node, parent, field_name)) = stack.pop() {
            context.check().map_err(|e| fault("execution.interrupted", format!("{e:?}")))?;
            if nodes.len() >= context.max_nodes {
                return Err(fault("resource.limit", "native node projection exceeds budget"));
            }
            let index = nodes.len();
            let id = format!("tslp:{}:{index}", request.language);
            let fragment = text
                .get(node.start_byte()..node.end_byte())
                .ok_or_else(|| fault("parser.invalid_span", "native span cuts source bytes"))?;
            let role = crate::tree_sitter_node_role(&node, fragment);
            let start = node.start_position();
            let end = node.end_position();
            if request.options.comments && role == NodeRole::Comment {
                comments.push(ParseComment {
                    node_id: id.clone(),
                    native_kind: node.kind(),
                    attachment_hint: AttachmentHint::Unknown,
                    metadata: Default::default(),
                    extra: Default::default(),
                });
            }
            if let Some(parent) = parent {
                let edge_index = nodes[parent].children.len() as u64;
                nodes[parent].children.push(ChildEdge {
                    node_id: id.clone(),
                    index: edge_index,
                    field_name,
                    extra: Default::default(),
                });
            }
            nodes.push(ParseNode {
                id,
                kind: node.kind(),
                native_type: node.kind(),
                role,
                named: node.is_named(),
                missing: node.is_missing(),
                has_error: node.has_error(),
                span: SourceSpan {
                    range: ByteRange { start_byte: node.start_byte(), end_byte: node.end_byte() },
                    start_point: SourcePoint { row: start.row, column: start.column },
                    end_point: SourcePoint { row: end.row, column: end.column },
                },
                parent_id: parent.map(|p| nodes[p].id.clone()),
                children: vec![],
                semantic_roles: vec![],
                unsupported_features: vec![],
                extensions: if request.options.native_extensions {
                    vec![node_flags(serde_json::json!({"extra": node.is_extra()}))]
                } else {
                    vec![]
                },
                metadata: Default::default(),
                extra: Default::default(),
            });
            if node.child_count() > context.max_nodes.saturating_sub(nodes.len() + stack.len()) {
                return Err(fault("resource.limit", "native node projection exceeds budget"));
            }
            let mut children = Vec::new();
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    children.push((cursor.node(), Some(index), cursor.field_name()));
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            stack.extend(children.into_iter().rev());
        }
        let diagnostics = if ok {
            vec![]
        } else {
            vec![ParseDiagnostic {
                id: "parse.syntax-error".into(),
                severity: ParseSeverity::Error,
                category: "parse_error".into(),
                code: Some("tree-sitter.syntax-error".into()),
                message: "native parser reported syntax errors".into(),
                source_role: source.descriptor().role,
                span: None,
                node_id: None,
                blocking: true,
                metadata: Default::default(),
                extra: Default::default(),
            }]
        };
        Ok(ParseOutput {
            request_id: request.request_id,
            source: source.descriptor().clone(),
            ok,
            root_id: nodes.first().map(|n| n.id.clone()),
            nodes,
            comments,
            diagnostics,
            extensions: vec![],
            metadata: request.metadata,
            extra: [("request_extra".into(), serde_json::to_value(request.extra).unwrap())].into(),
        })
    }
}

impl ParserProvider for LanguagePackProvider {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }

    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        if request.language != self.descriptor.languages[0] || request.dialect.is_some() {
            return Ok(ParserProbeResult { available: false, loadable: false });
        }
        crate::ensure_language_pack_language(&request.language)
            .map_err(|e| fault("parser.unavailable", e))?;
        tree_sitter_language_pack::get_parser(&request.language)
            .map_err(|e| fault("parser.unavailable", e.to_string()))?;
        Ok(ParserProbeResult { available: true, loadable: true })
    }

    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        if requests.len() > context.max_batch_items {
            return Err(fault("resource.limit", "parse batch exceeds budget"));
        }
        requests.into_iter().map(|request| self.parse(request, context)).collect()
    }
}
