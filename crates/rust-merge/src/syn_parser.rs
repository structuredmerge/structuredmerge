//! Native Rust parser provider backed by `syn` item syntax and spans.
//! Comments are lexed from source so shared ownership rules do not depend on a
//! Tree-sitter Rust tree or its syntax acceptance.
use std::ops::Range;
use syn::{Item, spanned::Spanned};
use tree_haver::{
    NodeRole, SourcePoint, SourceSpan,
    parsed::{
        AttachmentHint, ChildEdge, Metadata, ParseComment, ParseDiagnostic, ParseNode, ParseOutput,
        ParseSeverity,
    },
    service::{
        ExecutionContext, PARSE_RESULT_SCHEMA, ParseRequest, ParserProbeRequest, ParserProbeResult,
        ParserProvider, ParserProviderDescriptor, ProviderFault,
    },
};

#[derive(Clone, Debug)]
pub struct RustSynParserProvider {
    descriptor: ParserProviderDescriptor,
}

impl RustSynParserProvider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            descriptor: ParserProviderDescriptor {
                id: id.into(),
                family: "rust".into(),
                runtime: "rust-native".into(),
                package: "rust-merge".into(),
                package_version: env!("CARGO_PKG_VERSION").into(),
                parser: "syn".into(),
                parser_version: "2.x".into(),
                grammar: None,
                grammar_version: None,
                languages: vec!["rust".into()],
                dialects: vec!["rust".into()],
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec![
                    "comments".into(),
                    "diagnostics".into(),
                    "native_extensions".into(),
                    "source_fragments".into(),
                ],
                probe_id: "rust-merge.syn.parse-result.v1".into(),
                priority: 100,
                metadata: Metadata::new(),
                extensions: vec![],
            },
        }
    }

    fn parse(
        &self,
        request: ParseRequest,
        context: &ExecutionContext,
    ) -> Result<ParseOutput, ProviderFault> {
        context.check().map_err(service_fault)?;
        let source = std::str::from_utf8(&request.source.bytes)
            .map_err(|error| fault("source.invalid_utf8", error.to_string()))?;
        if request.source.bytes.len() as u64 > context.max_input_bytes {
            return Err(fault("resource.limit", "Rust source exceeds parser input budget"));
        }
        let file = match syn::parse_file(source) {
            Ok(file) => file,
            Err(error) => {
                return Ok(ParseOutput {
                    request_id: request.request_id,
                    source: request.source.descriptor.clone(),
                    ok: false,
                    root_id: None,
                    nodes: vec![],
                    comments: vec![],
                    diagnostics: vec![ParseDiagnostic {
                        id: "syn.parse-error".into(),
                        severity: ParseSeverity::Error,
                        category: "parse_error".into(),
                        code: Some("syn.syntax-error".into()),
                        message: error.to_string(),
                        source_role: request.source.descriptor.role,
                        span: None,
                        node_id: None,
                        blocking: true,
                        metadata: Metadata::new(),
                        extra: Metadata::new(),
                    }],
                    extensions: vec![],
                    metadata: request.metadata,
                    extra: request.extra,
                });
            }
        };

        let root_id = "syn:root".to_string();
        let root_span = source_span(source, 0..source.len());
        let mut nodes = vec![parse_node(
            root_id.clone(),
            "source_file",
            "syn::File",
            NodeRole::Structural,
            true,
            root_span,
            None,
            vec![],
            vec![],
        )];
        let mut root_children = Vec::<(usize, u8, String)>::new();
        let mut child_nodes = Vec::<(String, ParseNode)>::new();
        let mut comments = Vec::new();
        let mut item_ranges = Vec::new();

        for (index, item) in file.items.iter().enumerate() {
            context.check().map_err(service_fault)?;
            if nodes.len().saturating_add(child_nodes.len()) >= context.max_nodes {
                return Err(fault("resource.limit", "Rust syntax tree exceeds parser node budget"));
            }
            let span = item.span().byte_range();
            validate_range(source, &span)?;
            let item_id = format!("syn:item:{index}");
            let (owner_kind, native_type, name) = item_facts(item, source, span.clone());
            let mut item_child_edges = Vec::new();
            let mut item_descendants = Vec::new();
            if let Some((_name, name_range, name_kind)) = name {
                validate_range(source, &name_range)?;
                let name_id = format!("syn:name:{index}");
                let name_node = parse_node(
                    name_id.clone(),
                    "syn::Ident",
                    name_kind,
                    NodeRole::Token,
                    true,
                    source_span(source, name_range),
                    Some(item_id.clone()),
                    vec![],
                    vec!["name".into()],
                );
                item_child_edges.push(edge(&name_id, 0, Some("name")));
                item_descendants.push((name_id, name_node));
            }
            let item_node = parse_node(
                item_id.clone(),
                native_type,
                owner_kind,
                NodeRole::Structural,
                true,
                source_span(source, span.clone()),
                Some(root_id.clone()),
                item_child_edges,
                vec![],
            );
            root_children.push((span.start, 1, item_id.clone()));
            child_nodes.push((item_id, item_node));
            child_nodes.extend(item_descendants);
            item_ranges.push((span, index));
        }

        for (index, range) in scan_comments(source).into_iter().enumerate() {
            context.check().map_err(service_fault)?;
            let comment_id = format!("syn:comment:{index}");
            let parent = item_ranges
                .iter()
                .find(|(item_range, _)| {
                    item_range.start <= range.start && range.end <= item_range.end
                })
                .map(|(_, item_index)| format!("syn:item:{item_index}"));
            let source_text = &source[range.clone()];
            let inner_doc = source_text.starts_with("//!") || source_text.starts_with("/*!");
            let outer_doc = !inner_doc
                && (source_text.starts_with("///") && !source_text.starts_with("////")
                    || source_text.starts_with("/**") && !source_text.starts_with("/***"));
            let kind = if source_text.starts_with("//") { "line_comment" } else { "block_comment" };
            let comment_node = parse_node(
                comment_id.clone(),
                kind,
                kind,
                NodeRole::Comment,
                true,
                source_span(source, range.clone()),
                parent.clone().or_else(|| Some(root_id.clone())),
                vec![],
                if outer_doc { vec!["doc_comment".into()] } else { vec![] },
            );
            comments.push(ParseComment {
                node_id: comment_id.clone(),
                native_kind: kind.into(),
                attachment_hint: if inner_doc {
                    AttachmentHint::Preamble
                } else if outer_doc {
                    AttachmentHint::Leading
                } else {
                    AttachmentHint::Unknown
                },
                metadata: Metadata::new(),
                extra: Metadata::new(),
            });
            if let Some(parent_id) = parent {
                let parent_node = child_nodes
                    .iter_mut()
                    .find(|(id, _)| id == &parent_id)
                    .ok_or_else(|| fault("syn.invalid_tree", "comment parent item is missing"))?;
                let next = parent_node.1.children.len() as u64;
                parent_node.1.children.push(edge(&comment_id, next, None));
                child_nodes.push((comment_id, comment_node));
            } else {
                root_children.push((range.start, 0, comment_id.clone()));
                child_nodes.push((comment_id, comment_node));
            }
        }

        root_children.sort_by_key(|(start, priority, _)| (*start, *priority));
        for (edge_index, (_, _, child_id)) in root_children.iter().enumerate() {
            nodes[0].children.push(edge(child_id, edge_index as u64, None));
        }
        nodes.extend(child_nodes.into_iter().map(|(_, node)| node));
        if nodes.len() > context.max_nodes {
            return Err(fault("resource.limit", "Rust syntax tree exceeds parser node budget"));
        }

        Ok(ParseOutput {
            request_id: request.request_id,
            source: request.source.descriptor.clone(),
            ok: true,
            root_id: Some(root_id),
            nodes,
            comments,
            diagnostics: vec![],
            extensions: vec![],
            metadata: request.metadata,
            extra: request.extra,
        })
    }
}

impl ParserProvider for RustSynParserProvider {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }

    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        Ok(ParserProbeResult {
            available: request.language == "rust"
                && request.dialect.as_deref().is_none_or(|d| d == "rust"),
            loadable: request.language == "rust"
                && request.dialect.as_deref().is_none_or(|d| d == "rust"),
        })
    }

    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        if requests.len() > context.max_batch_items {
            return Err(fault("resource.limit", "parse batch exceeds parser batch budget"));
        }
        requests.into_iter().map(|request| self.parse(request, context)).collect()
    }
}

fn item_facts(
    item: &Item,
    source: &str,
    item_range: Range<usize>,
) -> (&'static str, &'static str, Option<(String, Range<usize>, &'static str)>) {
    let ident = match item {
        Item::Const(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Fn(item) => Some(&item.sig.ident),
        Item::Mod(item) => Some(&item.ident),
        Item::Static(item) => Some(&item.ident),
        Item::Struct(item) => Some(&item.ident),
        Item::Trait(item) => Some(&item.ident),
        Item::Type(item) => Some(&item.ident),
        Item::Union(item) => Some(&item.ident),
        _ => None,
    };
    let (kind, native_type) = match item {
        Item::Const(_) => ("const_item", "syn::ItemConst"),
        Item::Enum(_) => ("enum_item", "syn::ItemEnum"),
        Item::ExternCrate(_) => ("native_item", "syn::ItemExternCrate"),
        Item::Fn(_) => ("function_item", "syn::ItemFn"),
        Item::ForeignMod(_) => ("native_item", "syn::ItemForeignMod"),
        Item::Impl(_) => ("native_item", "syn::ItemImpl"),
        Item::Macro(_) => ("native_item", "syn::ItemMacro"),
        Item::Mod(_) => ("mod_item", "syn::ItemMod"),
        Item::Static(_) => ("static_item", "syn::ItemStatic"),
        Item::Struct(_) => ("struct_item", "syn::ItemStruct"),
        Item::Trait(_) => ("trait_item", "syn::ItemTrait"),
        Item::TraitAlias(_) => ("native_item", "syn::ItemTraitAlias"),
        Item::Type(_) => ("type_item", "syn::ItemType"),
        Item::Union(_) => ("union_item", "syn::ItemUnion"),
        Item::Use(_) => ("use_declaration", "syn::ItemUse"),
        Item::Verbatim(_) => ("native_item", "syn::ItemVerbatim"),
        _ => ("native_item", "syn::ItemUnknown"),
    };
    let name = if matches!(item, Item::Use(_)) {
        let text = source.get(item_range.clone()).unwrap_or_default();
        Some((super::rust_use_key(text), item_range, "scoped_identifier"))
    } else {
        ident.map(|ident| (ident.to_string(), ident.span().byte_range(), "identifier"))
    };
    (kind, native_type, name)
}

fn parse_node(
    id: String,
    kind: &str,
    native_type: &str,
    role: NodeRole,
    named: bool,
    span: SourceSpan,
    parent_id: Option<String>,
    children: Vec<ChildEdge>,
    semantic_roles: Vec<String>,
) -> ParseNode {
    ParseNode {
        id,
        kind: kind.into(),
        native_type: native_type.into(),
        role,
        named,
        missing: false,
        has_error: false,
        span,
        parent_id,
        children,
        semantic_roles,
        unsupported_features: vec![],
        extensions: vec![],
        metadata: Metadata::new(),
        extra: Metadata::new(),
    }
}

fn edge(id: &str, index: u64, field_name: Option<&str>) -> ChildEdge {
    ChildEdge {
        node_id: id.into(),
        index,
        field_name: field_name.map(str::to_string),
        extra: Metadata::new(),
    }
}

fn source_span(source: &str, range: Range<usize>) -> SourceSpan {
    let point = |offset: usize| {
        let prefix = &source[..offset];
        let row = prefix.bytes().filter(|byte| *byte == b'\n').count();
        let column = prefix.rfind('\n').map_or(offset, |index| offset - index - 1);
        SourcePoint { row, column }
    };
    SourceSpan {
        start_point: point(range.start),
        end_point: point(range.end),
        range: tree_haver::ByteRange { start_byte: range.start, end_byte: range.end },
    }
}

fn validate_range(source: &str, range: &Range<usize>) -> Result<(), ProviderFault> {
    if range.start > range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return Err(fault("syn.invalid_span", "syn returned a source span outside the input"));
    }
    Ok(())
}

fn scan_comments(source: &str) -> Vec<Range<usize>> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if let Some(end) = raw_string_end(bytes, cursor) {
            cursor = end;
        } else if bytes[cursor] == b'"' || prefixed_quoted_start(bytes, cursor).is_some() {
            let quote = if bytes[cursor] == b'"' {
                cursor
            } else {
                prefixed_quoted_start(bytes, cursor).unwrap()
            };
            cursor = quoted_end(bytes, quote, b'"').unwrap_or(bytes.len());
        } else if bytes[cursor] == b'\''
            || (bytes[cursor] == b'b' && bytes.get(cursor + 1) == Some(&b'\''))
        {
            let quote = cursor + usize::from(bytes[cursor] == b'b');
            cursor = char_literal_end(bytes, quote).unwrap_or(cursor + 1);
        } else if bytes.get(cursor..cursor + 2) == Some(b"//") {
            let start = cursor;
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            result.push(start..cursor);
        } else if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            let start = cursor;
            cursor = block_comment_end(bytes, cursor).unwrap_or(bytes.len());
            result.push(start..cursor);
        } else {
            cursor += source[cursor..].chars().next().map_or(1, char::len_utf8);
        }
    }
    result
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote = start;
    if matches!(bytes.get(quote), Some(b'b' | b'c')) {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'r') {
        return None;
    }
    quote += 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let hashes = quote - start - 1 - usize::from(matches!(bytes.get(start), Some(b'b' | b'c')));
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes
                .get(cursor + 1..cursor + 1 + hashes)
                .is_some_and(|tail| tail.iter().all(|byte| *byte == b'#'))
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn prefixed_quoted_start(bytes: &[u8], start: usize) -> Option<usize> {
    if matches!(bytes.get(start), Some(b'b' | b'c')) && bytes.get(start + 1) == Some(&b'"') {
        Some(start + 1)
    } else {
        None
    }
}

fn quoted_end(bytes: &[u8], quote: usize, delimiter: u8) -> Option<usize> {
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor = (cursor + 2).min(bytes.len()),
            byte if byte == delimiter => return Some(cursor + 1),
            b'\n' if delimiter == b'"' => return None,
            _ => cursor += 1,
        }
    }
    None
}

fn char_literal_end(bytes: &[u8], quote: usize) -> Option<usize> {
    let mut cursor = quote + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 2;
        while cursor < bytes.len() && bytes[cursor] != b'\'' && bytes[cursor] != b'\n' {
            cursor += 1;
        }
    } else {
        let first = *bytes.get(cursor)?;
        if first == b'\n' || first == b'\'' {
            return None;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor] & 0b1100_0000 == 0b1000_0000 {
            cursor += 1;
        }
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn block_comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = start + 2;
    while cursor + 1 < bytes.len() {
        match &bytes[cursor..cursor + 2] {
            b"/*" => {
                depth += 1;
                cursor += 2;
            }
            b"*/" => {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

fn fault(code: &str, message: impl Into<String>) -> ProviderFault {
    ProviderFault { code: code.into(), message: message.into() }
}

fn service_fault(error: tree_haver::service::ServiceError) -> ProviderFault {
    match error {
        tree_haver::service::ServiceError::Cancelled => {
            fault("execution.cancelled", "Rust parser execution was cancelled")
        }
        tree_haver::service::ServiceError::DeadlineExceeded => {
            fault("execution.deadline_exceeded", "Rust parser deadline was exceeded")
        }
        tree_haver::service::ServiceError::LimitExceeded => {
            fault("resource.limit", "Rust parser resource limit was exceeded")
        }
        _ => fault("parser.failed", "Rust parser service rejected the request"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    use tree_haver::service::{
        ParseOptions, ParseRequest, ParseService, ParserRegistry, ParserSelection,
        TreeHaverParseService,
    };
    use tree_haver::source::{SourceEncoding, SourceRole, source_input};

    fn context() -> ExecutionContext {
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 8,
            max_input_bytes: 100_000,
            max_nodes: 10_000,
            max_diagnostics: 100,
        }
    }

    fn request(source: &str) -> ParseRequest {
        ParseRequest {
            schema: tree_haver::service::PARSE_REQUEST_SCHEMA.into(),
            request_id: "syn-test".into(),
            source: source_input(
                "source".into(),
                SourceRole::Source,
                SourceEncoding::Utf8,
                source.as_bytes().to_vec(),
            )
            .unwrap(),
            language: "rust".into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: Some("rust.syn.test".into()),
                preference: vec![],
                required_capabilities: vec![],
            },
            options: ParseOptions {
                comments: true,
                tokens: false,
                diagnostics: true,
                native_extensions: false,
            },
            metadata: Metadata::new(),
            extra: Metadata::new(),
        }
    }

    #[test]
    fn syn_provider_accepts_native_syntax_and_emits_source_bound_owner_and_comment_facts() {
        let source = "//! crate docs\n\n/// function docs\npub fn café() { let s = r#\"// not a comment\"#; /* outer /* nested */ done */ }\n";
        let registry = ParserRegistry::default();
        registry.register(Arc::new(RustSynParserProvider::new("rust.syn.test"))).unwrap();
        let parsed = TreeHaverParseService::default()
            .parse_batch(vec![request(source)], &registry.snapshot().unwrap(), &context())
            .unwrap()
            .remove(0);
        assert_eq!(parsed.backend.parser, "syn");
        let nodes = parsed.normalized_nodes().unwrap();
        let root = parsed.document.output().root_id.as_deref().unwrap();
        let analysis = crate::typed::analysis(&parsed).unwrap();
        assert_eq!(analysis.document.owners[0].id, "/function:café");
        let native_function = parsed
            .document
            .output()
            .nodes
            .iter()
            .find(|node| {
                node.parent_id.as_deref() == Some(root) && node.native_type == "function_item"
            })
            .unwrap();
        assert_eq!(native_function.kind, "syn::ItemFn");
        assert!(analysis.attached_comment_node_ids.contains_key("/function:café"));
        let line_analysis = crate::typed::source_line_analysis(&parsed).unwrap();
        let doc_line =
            line_analysis.dispositions.iter().find(|line| line.line_number == 3).unwrap();
        assert_eq!(doc_line.kind, "comment_owner");
        assert_eq!(doc_line.owner_id.as_deref(), Some("/function:café"));
        let code_line =
            line_analysis.dispositions.iter().find(|line| line.line_number == 4).unwrap();
        assert_eq!(code_line.kind, "structural_owner");
        assert_eq!(code_line.owner_id.as_deref(), Some("/function:café"));
        assert!(
            nodes
                .iter()
                .any(|node| node.role == NodeRole::Comment
                    && node.source_fragment.contains("nested"))
        );
        analysis.validate(&parsed).unwrap();
        assert!(nodes.iter().any(|node| node.id == root));
    }

    #[test]
    fn native_parser_reports_syntax_errors_as_blocking_diagnostics() {
        let output = RustSynParserProvider::new("rust.syn.test")
            .parse(request("fn broken("), &context())
            .unwrap();
        assert!(!output.ok);
        assert_eq!(output.diagnostics.len(), 1);
        assert!(output.diagnostics[0].blocking);
        assert_eq!(output.diagnostics[0].category, "parse_error");
    }

    #[test]
    fn native_syntax_outside_the_owner_profile_fails_closed() {
        let source = "unsafe extern \"C\" { fn native_symbol(); }\n";
        let registry = ParserRegistry::default();
        registry.register(Arc::new(RustSynParserProvider::new("rust.syn.test"))).unwrap();
        let parsed = TreeHaverParseService::default()
            .parse_batch(vec![request(source)], &registry.snapshot().unwrap(), &context())
            .unwrap()
            .remove(0);
        assert!(parsed.document.output().ok);
        assert!(
            crate::typed::analysis(&parsed)
                .unwrap_err()
                .contains("unsupported top-level Rust node")
        );
    }

    #[test]
    fn comment_lexer_skips_raw_byte_strings_c_strings_and_lifetimes() {
        let source = concat!(
            "let raw = br###\"/* no */ // no\"###;\n",
            "let text = c\"// no\";\n",
            "fn borrow<'a>(text: &'a str) {}\n",
            "// actual comment\n",
        );
        let comments =
            scan_comments(source).into_iter().map(|range| &source[range]).collect::<Vec<_>>();
        assert_eq!(comments, vec!["// actual comment"]);
    }

    #[test]
    fn native_parser_conforms_to_shared_owner_prefix_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/polyglot/ast-merge/native_owner_prefix_ownership.json"
        ))
        .unwrap();
        let registry = ParserRegistry::default();
        registry.register(Arc::new(RustSynParserProvider::new("rust.syn.test"))).unwrap();
        let snapshot = registry.snapshot().unwrap();
        let service = TreeHaverParseService::default();

        for case in fixture["cases"].as_array().unwrap() {
            let source = case["source"].as_str().unwrap();
            let parsed = service
                .parse_batch(vec![request(source)], &snapshot, &context())
                .unwrap()
                .remove(0);
            let analysis = crate::typed::analysis(&parsed).unwrap();
            let owner_path = case["owner_path"].as_str().unwrap();
            let owner =
                analysis.document.owners.iter().find(|owner| owner.id == owner_path).unwrap();
            assert_eq!(
                vec![owner.start_line, owner.end_line],
                case["owner_line_range"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|line| line.as_u64().unwrap() as usize)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                &source[owner.start_byte..owner.end_byte],
                case["owner_source"].as_str().unwrap(),
                "{}",
                case["name"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn native_parser_accepts_syntax_without_tree_sitter_as_an_acceptance_oracle() {
        let source = "unsafe extern \"C\" { fn native_symbol(); }\n";
        let output =
            RustSynParserProvider::new("rust.syn.test").parse(request(source), &context()).unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert!(
            output.nodes.iter().any(|node| {
                node.kind == "syn::ItemForeignMod" && node.native_type == "native_item"
            }),
            "{:?}",
            output.nodes
        );
    }
}
