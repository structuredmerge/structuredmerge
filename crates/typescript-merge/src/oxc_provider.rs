//! Native TypeScript/TSX parser provider backed by `oxc_parser`.
//!
//! Emits the same normalized top-level shape the Tree-sitter TypeScript projection
//! produces (`export_statement`/`ambient_declaration` wrappers, `*_declaration`
//! owners with `name` identifier children, `variable_declarator` children) so the
//! shared owner engine in [`crate::typed`] applies unchanged, while spans, comment
//! lists and syntax acceptance come from oxc. Any parse error fails closed.
use std::ops::Range;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Function, Statement, TSModuleDeclarationName,
    VariableDeclarationKind,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
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
pub struct TypeScriptOxcParserProvider {
    descriptor: ParserProviderDescriptor,
}

impl TypeScriptOxcParserProvider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            descriptor: ParserProviderDescriptor {
                id: id.into(),
                family: "typescript".into(),
                runtime: "rust-native".into(),
                package: "typescript-merge".into(),
                package_version: env!("CARGO_PKG_VERSION").into(),
                parser: "oxc_parser".into(),
                parser_version: "0.110".into(),
                grammar: None,
                grammar_version: None,
                languages: vec!["tsx".into(), "typescript".into()],
                dialects: vec!["tsx".into(), "typescript".into()],
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec![
                    "comments".into(),
                    "diagnostics".into(),
                    "native_extensions".into(),
                    "source_fragments".into(),
                ],
                probe_id: "typescript-merge.oxc.parse-result.v1".into(),
                priority: 100,
                metadata: Metadata::new(),
                extensions: vec![],
            },
        }
    }

    fn supports(language: &str, dialect: Option<&str>) -> bool {
        matches!(language, "typescript" | "tsx")
            && dialect.is_none_or(|dialect| matches!(dialect, "typescript" | "tsx"))
    }

    fn parse(
        &self,
        request: ParseRequest,
        context: &ExecutionContext,
    ) -> Result<ParseOutput, ProviderFault> {
        context.check().map_err(service_fault)?;
        if !Self::supports(&request.language, request.dialect.as_deref()) {
            return Err(fault("oxc.unsupported_language", "oxc provider supports TypeScript/TSX"));
        }
        let source = std::str::from_utf8(&request.source.bytes)
            .map_err(|error| fault("source.invalid_utf8", error.to_string()))?;
        if request.source.bytes.len() as u64 > context.max_input_bytes {
            return Err(fault("resource.limit", "TypeScript source exceeds parser input budget"));
        }
        let tsx = request.language == "tsx" || request.dialect.as_deref() == Some("tsx");
        let source_type = if tsx { SourceType::tsx() } else { SourceType::ts() };
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, source_type).parse();
        if parsed.panicked || !parsed.errors.is_empty() {
            let message = parsed
                .errors
                .first()
                .map(|error| error.message.to_string())
                .unwrap_or_else(|| "oxc could not recover from a syntax error".into());
            return Ok(ParseOutput {
                request_id: request.request_id,
                source: request.source.descriptor.clone(),
                ok: false,
                root_id: None,
                nodes: vec![],
                comments: vec![],
                diagnostics: vec![ParseDiagnostic {
                    id: "oxc.parse-error".into(),
                    severity: ParseSeverity::Error,
                    category: "parse_error".into(),
                    code: Some("oxc.syntax-error".into()),
                    message,
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

        let root_id = "oxc:root".to_string();
        let mut nodes = vec![node(
            &root_id,
            "Program",
            "program",
            NodeRole::Structural,
            source,
            0..source.len(),
        )];
        let mut root_children: Vec<(usize, u8, String)> = Vec::new();
        let mut body: Vec<ParseNode> = Vec::new();
        let mut statement_ranges: Vec<(Range<usize>, String)> = Vec::new();

        for (index, statement) in parsed.program.body.iter().enumerate() {
            context.check().map_err(service_fault)?;
            if nodes.len().saturating_add(body.len()) >= context.max_nodes {
                return Err(fault("resource.limit", "TypeScript tree exceeds parser node budget"));
            }
            let span = range(statement.span());
            validate_range(source, &span)?;
            let id = format!("oxc:stmt:{index}");
            let mut emitted =
                self.statement_nodes(source, statement, &id, &root_id, span.clone())?;
            statement_ranges.push((span.clone(), id.clone()));
            root_children.push((span.start, 1, id));
            body.append(&mut emitted);
        }

        let mut comments = Vec::new();
        for (index, comment) in parsed.program.comments.iter().enumerate() {
            context.check().map_err(service_fault)?;
            let comment_range = range(comment.span);
            validate_range(source, &comment_range)?;
            let id = format!("oxc:comment:{index}");
            let parent = statement_ranges
                .iter()
                .find(|(statement, _)| {
                    statement.start <= comment_range.start && comment_range.end <= statement.end
                })
                .map(|(_, statement_id)| statement_id.clone());
            let mut comment_node =
                node(&id, "Comment", "comment", NodeRole::Comment, source, comment_range.clone());
            comment_node.parent_id = Some(parent.clone().unwrap_or_else(|| root_id.clone()));
            let doc = source[comment_range.clone()].starts_with("/**");
            if let Some(parent_id) = &parent {
                let parent_node = body
                    .iter_mut()
                    .find(|candidate| &candidate.id == parent_id)
                    .ok_or_else(|| fault("oxc.invalid_tree", "comment parent is missing"))?;
                let next = parent_node.children.len() as u64;
                parent_node.children.push(edge(&id, next, None));
            } else {
                root_children.push((comment_range.start, 0, id.clone()));
            }
            comments.push(ParseComment {
                node_id: id,
                native_kind: "comment".into(),
                attachment_hint: if doc || comment.attached_to != 0 {
                    AttachmentHint::Leading
                } else {
                    AttachmentHint::Unknown
                },
                metadata: Metadata::new(),
                extra: Metadata::new(),
            });
            body.push(comment_node);
        }

        root_children.sort_by_key(|(start, priority, _)| (*start, *priority));
        for (edge_index, (_, _, child)) in root_children.iter().enumerate() {
            nodes[0].children.push(edge(child, edge_index as u64, None));
        }
        nodes.extend(body);
        if nodes.len() > context.max_nodes {
            return Err(fault("resource.limit", "TypeScript tree exceeds parser node budget"));
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

    /// Emit the normalized nodes for one top-level statement (wrapper, declaration, names).
    fn statement_nodes(
        &self,
        source: &str,
        statement: &Statement<'_>,
        id: &str,
        root_id: &str,
        span: Range<usize>,
    ) -> Result<Vec<ParseNode>, ProviderFault> {
        let mut out = Vec::new();
        if let Some(module) = statement.as_module_declaration() {
            use oxc_ast::ast::ModuleDeclaration as M;
            match module {
                M::ImportDeclaration(_) => {
                    let mut n = node(
                        id,
                        "ImportDeclaration",
                        "import_statement",
                        NodeRole::Structural,
                        source,
                        span,
                    );
                    n.parent_id = Some(root_id.into());
                    out.push(n);
                }
                M::ExportNamedDeclaration(export) if export.declaration.is_some() => {
                    let declaration = export.declaration.as_ref().expect("checked above");
                    let mut wrapper = node(
                        id,
                        "ExportNamedDeclaration",
                        "export_statement",
                        NodeRole::Structural,
                        source,
                        span,
                    );
                    wrapper.parent_id = Some(root_id.into());
                    let inner_id = format!("{id}:decl");
                    let inner_range = range(declaration.span());
                    validate_range(source, &inner_range)?;
                    wrapper.children.push(edge(&inner_id, 0, Some("declaration")));
                    out.push(wrapper);
                    out.extend(self.declaration_nodes(
                        source,
                        declaration,
                        &inner_id,
                        id,
                        inner_range,
                    )?);
                }
                _ => {
                    let mut n = node(
                        id,
                        "ModuleDeclaration",
                        "export_statement",
                        NodeRole::Opaque,
                        source,
                        span,
                    );
                    n.parent_id = Some(root_id.into());
                    n.unsupported_features.push("export_form".into());
                    out.push(n);
                }
            }
            return Ok(out);
        }
        if let Some(declaration) = statement.as_declaration() {
            if declaration_is_ambient(declaration) {
                let mut wrapper = node(
                    id,
                    "ambient",
                    "ambient_declaration",
                    NodeRole::Structural,
                    source,
                    span.clone(),
                );
                wrapper.parent_id = Some(root_id.into());
                let inner_id = format!("{id}:decl");
                let inner_range = range(declaration.span());
                validate_range(source, &inner_range)?;
                wrapper.children.push(edge(&inner_id, 0, Some("declaration")));
                out.push(wrapper);
                out.extend(self.declaration_nodes(
                    source,
                    declaration,
                    &inner_id,
                    id,
                    inner_range,
                )?);
            } else {
                out.extend(self.declaration_nodes(source, declaration, id, root_id, span)?);
            }
            return Ok(out);
        }
        let kind = match statement {
            Statement::ExpressionStatement(_) => "expression_statement",
            _ => "native_statement",
        };
        let mut n = node(id, "Statement", kind, NodeRole::Structural, source, span);
        n.parent_id = Some(root_id.into());
        out.push(n);
        Ok(out)
    }

    fn declaration_nodes(
        &self,
        source: &str,
        declaration: &Declaration<'_>,
        id: &str,
        parent_id: &str,
        span: Range<usize>,
    ) -> Result<Vec<ParseNode>, ProviderFault> {
        let mut out = Vec::new();
        let (native, kind, name): (&str, &str, Option<NameFact>) = match declaration {
            Declaration::FunctionDeclaration(function) => (
                "Function",
                function_kind(function),
                function.id.as_ref().map(|i| (i.name.to_string(), range(i.span), "identifier")),
            ),
            Declaration::ClassDeclaration(class) => (
                "Class",
                "class_declaration",
                class.id.as_ref().map(|i| (i.name.to_string(), range(i.span), "type_identifier")),
            ),
            Declaration::TSInterfaceDeclaration(item) => (
                "TSInterfaceDeclaration",
                "interface_declaration",
                Some((item.id.name.to_string(), range(item.id.span), "type_identifier")),
            ),
            Declaration::TSTypeAliasDeclaration(item) => (
                "TSTypeAliasDeclaration",
                "type_alias_declaration",
                Some((item.id.name.to_string(), range(item.id.span), "type_identifier")),
            ),
            Declaration::TSEnumDeclaration(item) => (
                "TSEnumDeclaration",
                "enum_declaration",
                Some((item.id.name.to_string(), range(item.id.span), "identifier")),
            ),
            Declaration::TSModuleDeclaration(item) => (
                "TSModuleDeclaration",
                "internal_module",
                match &item.id {
                    TSModuleDeclarationName::Identifier(i) => {
                        Some((i.name.to_string(), range(i.span), "identifier"))
                    }
                    _ => None,
                },
            ),
            Declaration::VariableDeclaration(variable) => (
                "VariableDeclaration",
                match variable.kind {
                    VariableDeclarationKind::Var => "variable_declaration",
                    _ => "lexical_declaration",
                },
                None,
            ),
            _ => ("Declaration", "native_declaration", None),
        };
        let mut owner = node(id, native, kind, NodeRole::Structural, source, span.clone());
        owner.parent_id = Some(parent_id.into());
        let mut descendants = Vec::new();
        if let Some((text, name_range, name_kind)) = name {
            validate_range(source, &name_range)?;
            let name_id = format!("{id}:name");
            let mut name_node =
                node(&name_id, "BindingIdentifier", name_kind, NodeRole::Token, source, name_range);
            name_node.parent_id = Some(id.into());
            let _ = text;
            owner.children.push(edge(&name_id, 0, Some("name")));
            descendants.push(name_node);
        }
        if let Declaration::VariableDeclaration(variable) = declaration {
            for (index, declarator) in variable.declarations.iter().enumerate() {
                let declarator_id = format!("{id}:declarator:{index}");
                let declarator_range = range(declarator.span);
                validate_range(source, &declarator_range)?;
                let mut declarator_node = node(
                    &declarator_id,
                    "VariableDeclarator",
                    "variable_declarator",
                    NodeRole::Structural,
                    source,
                    declarator_range,
                );
                declarator_node.parent_id = Some(id.into());
                if let BindingPattern::BindingIdentifier(binding) = &declarator.id {
                    let name_range = range(binding.span);
                    validate_range(source, &name_range)?;
                    let name_id = format!("{declarator_id}:name");
                    let mut name_node = node(
                        &name_id,
                        "BindingIdentifier",
                        "identifier",
                        NodeRole::Token,
                        source,
                        name_range,
                    );
                    name_node.parent_id = Some(declarator_id.clone());
                    declarator_node.children.push(edge(&name_id, 0, Some("name")));
                    descendants.push(name_node);
                }
                owner.children.push(edge(&declarator_id, index as u64, None));
                descendants.push(declarator_node);
            }
        }
        out.push(owner);
        out.extend(descendants);
        Ok(out)
    }
}

/// Name text, name byte range, and normalized name node kind.
type NameFact<'a> = (String, Range<usize>, &'a str);

fn function_kind(function: &Function<'_>) -> &'static str {
    if function.body.is_some() { "function_declaration" } else { "function_signature" }
}

fn declaration_is_ambient(declaration: &Declaration<'_>) -> bool {
    match declaration {
        Declaration::FunctionDeclaration(item) => item.declare,
        Declaration::ClassDeclaration(item) => item.declare,
        Declaration::VariableDeclaration(item) => item.declare,
        Declaration::TSInterfaceDeclaration(item) => item.declare,
        Declaration::TSTypeAliasDeclaration(item) => item.declare,
        Declaration::TSEnumDeclaration(item) => item.declare,
        Declaration::TSModuleDeclaration(item) => item.declare,
        _ => false,
    }
}

impl ParserProvider for TypeScriptOxcParserProvider {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }

    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        let supported = Self::supports(&request.language, request.dialect.as_deref());
        Ok(ParserProbeResult { available: supported, loadable: supported })
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

fn range(span: oxc_span::Span) -> Range<usize> {
    span.start as usize..span.end as usize
}

fn node(
    id: &str,
    native: &str,
    normalized_kind: &str,
    role: NodeRole,
    source: &str,
    range: Range<usize>,
) -> ParseNode {
    ParseNode {
        id: id.into(),
        kind: native.into(),
        native_type: normalized_kind.into(),
        role,
        named: true,
        missing: false,
        has_error: false,
        span: source_span(source, range),
        parent_id: None,
        children: vec![],
        semantic_roles: vec![],
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
        return Err(fault("oxc.invalid_span", "oxc returned a source span outside the input"));
    }
    Ok(())
}

fn fault(code: &str, message: impl Into<String>) -> ProviderFault {
    ProviderFault { code: code.into(), message: message.into() }
}

fn service_fault(error: tree_haver::service::ServiceError) -> ProviderFault {
    match error {
        tree_haver::service::ServiceError::Cancelled => {
            fault("execution.cancelled", "TypeScript parser execution was cancelled")
        }
        tree_haver::service::ServiceError::DeadlineExceeded => {
            fault("execution.deadline_exceeded", "TypeScript parser deadline was exceeded")
        }
        tree_haver::service::ServiceError::LimitExceeded => {
            fault("resource.limit", "TypeScript parser resource limit was exceeded")
        }
        _ => fault("parser.failed", "TypeScript parser service rejected the request"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    use tree_haver::service::{
        ParseOptions, ParseService, ParserRegistry, ParserSelection, TreeHaverParseService,
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

    fn request(source: &str, language: &str) -> ParseRequest {
        ParseRequest {
            schema: tree_haver::service::PARSE_REQUEST_SCHEMA.into(),
            request_id: "oxc-test".into(),
            source: source_input(
                "source".into(),
                SourceRole::Source,
                SourceEncoding::Utf8,
                source.as_bytes().to_vec(),
            )
            .unwrap(),
            language: language.into(),
            dialect: None,
            selection: ParserSelection {
                backend_id: Some("typescript.oxc.test".into()),
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

    fn parse(source: &str) -> tree_haver::service::ParsedResult {
        let registry = ParserRegistry::default();
        registry
            .register(Arc::new(TypeScriptOxcParserProvider::new("typescript.oxc.test")))
            .unwrap();
        TreeHaverParseService::default()
            .parse_batch(
                vec![request(source, "typescript")],
                &registry.snapshot().unwrap(),
                &context(),
            )
            .unwrap()
            .remove(0)
    }

    #[test]
    fn oxc_provider_projects_owners_with_exact_spans_and_comments() {
        let source = "// leading\n/** a docs */\nexport function a<T>(x: T): T { return x } // trail\n\n@dec class C { m(): void {} }\ninterface I { x: number }\ntype U = A | B;\nenum E { A }\nconst k = 1;\n";
        let parsed = parse(source);
        assert_eq!(parsed.backend.parser, "oxc_parser");
        let analysis = crate::typed::analysis(&parsed).unwrap();
        let paths: Vec<_> =
            analysis.document.owners.iter().map(|owner| owner.path.as_str()).collect();
        assert_eq!(
            paths,
            ["/function:a", "/class:C", "/interface:I", "/type_alias:U", "/enum:E", "/variables:k"]
        );
        let first = &analysis.document.owners[0];
        let text = &source[first.start_byte..first.end_byte];
        // The leading file comment stays root-scoped; JSDoc and the same-line trailing comment attach.
        assert!(
            text.starts_with("/** a docs */\nexport function a<T>") && text.ends_with("// trail"),
            "{text:?}"
        );
        assert!(text.contains("export function a<T>"));
    }

    #[test]
    fn oxc_provider_fails_closed_on_syntax_errors() {
        let parsed = parse("function (");
        assert!(!parsed.document.output().ok);
        assert!(crate::typed::analysis(&parsed).is_err());
    }
}
