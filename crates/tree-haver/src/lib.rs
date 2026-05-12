use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use tree_sitter_language_pack::{
    PackConfig, ProcessConfig, configure, parse_string, process, tree_has_error_nodes,
};

pub const PACKAGE_NAME: &str = "tree-haver";

pub trait AnalysisHandle {
    fn kind(&self) -> &'static str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserRequest {
    pub source: String,
    pub language: String,
    pub dialect: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReference {
    pub id: String,
    pub family: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCategory {
    ParseError,
    UnsupportedFeature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicySurface {
    Fallback,
    Array,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyReference {
    pub surface: PolicySurface,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult<TAnalysis> {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub analysis: Option<TAnalysis>,
    pub policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterInfo {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureProfile {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub supports_dialects: bool,
    pub supported_policies: Vec<PolicyReference>,
}

pub trait ParserAdapter<TAnalysis: AnalysisHandle> {
    fn info(&self) -> AdapterInfo;
    fn parse(&self, request: &ParserRequest) -> ParseResult<TAnalysis>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserDiagnostics {
    pub backend: String,
    pub backend_ref: Option<BackendReference>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRequest {
    pub source: String,
    pub language: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRange {
    pub start_byte: usize,
    pub end_byte: usize,
}

impl ByteRange {
    pub fn is_valid(&self) -> bool {
        self.end_byte >= self.start_byte
    }

    pub fn len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }

    pub fn contains_byte(&self, offset: usize) -> bool {
        self.is_valid() && offset >= self.start_byte && offset < self.end_byte
    }

    pub fn contains_range(&self, other: &ByteRange) -> bool {
        self.is_valid()
            && other.is_valid()
            && other.start_byte >= self.start_byte
            && other.end_byte <= self.end_byte
    }

    pub fn overlaps(&self, other: &ByteRange) -> bool {
        self.is_valid()
            && other.is_valid()
            && self.start_byte < other.end_byte
            && other.start_byte < self.end_byte
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePoint {
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub range: ByteRange,
    pub start_point: SourcePoint,
    pub end_point: SourcePoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteEditSpan {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: SourcePoint,
    pub old_end_point: SourcePoint,
    pub new_end_point: SourcePoint,
}

impl ByteEditSpan {
    pub fn old_range(&self) -> ByteRange {
        ByteRange { start_byte: self.start_byte, end_byte: self.old_end_byte }
    }

    pub fn new_range(&self) -> ByteRange {
        ByteRange { start_byte: self.start_byte, end_byte: self.new_end_byte }
    }

    pub fn byte_delta(&self) -> isize {
        self.new_end_byte as isize - self.old_end_byte as isize
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinaryScalarValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Enum { symbol: String, raw_value: i64 },
    Bytes { encoding: String, value: String },
    Timestamp(String),
    Opaque { format: String, description: String },
    Null,
}

impl BinaryScalarValue {
    pub fn kind(&self) -> &'static str {
        match self {
            BinaryScalarValue::String(_) => "string",
            BinaryScalarValue::Integer(_) => "integer",
            BinaryScalarValue::Float(_) => "float",
            BinaryScalarValue::Boolean(_) => "boolean",
            BinaryScalarValue::Enum { .. } => "enum",
            BinaryScalarValue::Bytes { .. } => "bytes",
            BinaryScalarValue::Timestamp(_) => "timestamp",
            BinaryScalarValue::Opaque { .. } => "opaque",
            BinaryScalarValue::Null => "null",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryRenderPolicy {
    pub schema_path: String,
    pub byte_range: Option<ByteRange>,
    pub operation: String,
    pub disposition: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryDiagnostic {
    pub severity: String,
    pub category: String,
    pub message: String,
    pub schema_path: String,
    pub byte_range: Option<ByteRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryNestedDispatch {
    pub schema_path: String,
    pub family: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryPayloadRegion {
    pub kind: String,
    pub schema_path: String,
    pub byte_range: ByteRange,
    pub expected_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryRawPayload {
    pub encoding: String,
    pub value: String,
    pub byte_length: usize,
    pub regions: Vec<BinaryPayloadRegion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryMergeReport {
    pub format: String,
    pub schema: String,
    pub matched_schema_paths: Vec<String>,
    pub preserved_ranges: Vec<ByteRange>,
    pub rewritten_nodes: Vec<String>,
    pub checksum_updates: Vec<String>,
    pub nested_dispatches: Vec<BinaryNestedDispatch>,
    pub diagnostics: Vec<BinaryDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipArchiveInfo {
    pub format: String,
    pub schema: String,
    pub entry_count: usize,
    pub central_directory_range: ByteRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipArchiveEntry {
    pub path: String,
    pub normalized_path: String,
    pub directory: bool,
    pub compression: String,
    pub compressed_size: usize,
    pub uncompressed_size: usize,
    pub crc32: String,
    pub local_header_range: ByteRange,
    pub data_range: ByteRange,
    pub central_directory_range: ByteRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipMemberDecision {
    pub normalized_path: String,
    pub operation: String,
    pub disposition: String,
    pub nested_family: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipUnsafeEntry {
    pub path: String,
    pub normalized_path: String,
    pub category: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipFamilyReport {
    pub archive: ZipArchiveInfo,
    pub entries: Vec<ZipArchiveEntry>,
    pub member_decisions: Vec<ZipMemberDecision>,
    pub unsafe_entries: Vec<ZipUnsafeEntry>,
    pub merge_report: BinaryMergeReport,
}

pub fn slice_byte_range(source: &str, byte_range: &ByteRange) -> Result<String, String> {
    if !byte_range.is_valid() || byte_range.end_byte > source.len() {
        return Err(format!(
            "invalid byte range [{}, {}) for source length {}",
            byte_range.start_byte,
            byte_range.end_byte,
            source.len()
        ));
    }

    std::str::from_utf8(&source.as_bytes()[byte_range.start_byte..byte_range.end_byte])
        .map(str::to_string)
        .map_err(|error| error.to_string())
}

pub fn byte_offset_for_point(source: &str, point: &SourcePoint) -> Result<usize, String> {
    let mut row = 0;
    let mut column = 0;

    for (offset, value) in source.as_bytes().iter().enumerate() {
        if row == point.row && column == point.column {
            return Ok(offset);
        }
        if *value == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    if row == point.row && column == point.column {
        return Ok(source.len());
    }

    Err(format!("source point ({}, {}) is outside source", point.row, point.column))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessStructureItem {
    pub kind: String,
    pub name: Option<String>,
    pub span: ProcessSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessImportInfo {
    pub source: String,
    pub items: Vec<String>,
    pub span: ProcessSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessDiagnostic {
    pub message: String,
    pub severity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePackAnalysis {
    pub language: String,
    pub dialect: Option<String>,
    pub root_type: String,
    pub has_error: bool,
    pub backend_ref: BackendReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePackProcessAnalysis {
    pub language: String,
    pub structure: Vec<ProcessStructureItem>,
    pub imports: Vec<ProcessImportInfo>,
    pub diagnostics: Vec<ProcessDiagnostic>,
    pub backend_ref: BackendReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KaitaiByteSpan {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KaitaiTreeNode {
    pub kind: String,
    pub schema_path: String,
    pub span: KaitaiByteSpan,
    pub fields: HashMap<String, String>,
    pub children: Vec<KaitaiTreeNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KaitaiTreeAnalysis {
    pub schema: String,
    pub source_byte_length: usize,
    pub root: KaitaiTreeNode,
    pub backend_ref: BackendReference,
    pub diagnostics: Vec<BinaryDiagnostic>,
}

impl AnalysisHandle for LanguagePackAnalysis {
    fn kind(&self) -> &'static str {
        "tree-sitter"
    }
}

impl AnalysisHandle for LanguagePackProcessAnalysis {
    fn kind(&self) -> &'static str {
        "tree-sitter-process"
    }
}

impl AnalysisHandle for KaitaiTreeAnalysis {
    fn kind(&self) -> &'static str {
        "kaitai-tree"
    }
}

pub fn kreuzberg_language_pack_backend() -> BackendReference {
    BackendReference {
        id: "kreuzberg-language-pack".to_string(),
        family: "tree-sitter".to_string(),
    }
}

pub fn pest_backend() -> BackendReference {
    BackendReference { id: "pest".to_string(), family: "peg".to_string() }
}

pub fn kaitai_struct_backend() -> BackendReference {
    BackendReference { id: "kaitai-struct".to_string(), family: "kaitai".to_string() }
}

fn backend_registry() -> &'static Mutex<HashMap<String, BackendReference>> {
    static BACKEND_REGISTRY: OnceLock<Mutex<HashMap<String, BackendReference>>> = OnceLock::new();
    BACKEND_REGISTRY.get_or_init(|| {
        let mut backends = HashMap::new();
        let language_pack = kreuzberg_language_pack_backend();
        let pest = pest_backend();
        let kaitai = kaitai_struct_backend();
        backends.insert(language_pack.id.clone(), language_pack);
        backends.insert(pest.id.clone(), pest);
        backends.insert(kaitai.id.clone(), kaitai);
        Mutex::new(backends)
    })
}

pub fn register_backend(backend: BackendReference) {
    let mut backends =
        backend_registry().lock().expect("backend registry lock should not be poisoned");
    backends.insert(backend.id.clone(), backend);
}

pub fn backend_reference(id: &str) -> Option<BackendReference> {
    backend_registry()
        .lock()
        .expect("backend registry lock should not be poisoned")
        .get(id)
        .cloned()
}

pub fn registered_backends() -> Vec<BackendReference> {
    let mut backends = backend_registry()
        .lock()
        .expect("backend registry lock should not be poisoned")
        .values()
        .cloned()
        .collect::<Vec<_>>();
    backends.sort_by(|left, right| left.id.cmp(&right.id));
    backends
}

pub fn pest_adapter_info() -> AdapterInfo {
    AdapterInfo {
        backend: "pest".to_string(),
        backend_ref: Some(pest_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

pub fn pest_feature_profile() -> FeatureProfile {
    FeatureProfile {
        backend: "pest".to_string(),
        backend_ref: Some(pest_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

pub fn kaitai_adapter_info() -> AdapterInfo {
    AdapterInfo {
        backend: "kaitai-struct".to_string(),
        backend_ref: Some(kaitai_struct_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

pub fn kaitai_feature_profile() -> FeatureProfile {
    FeatureProfile {
        backend: "kaitai-struct".to_string(),
        backend_ref: Some(kaitai_struct_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

thread_local! {
    static CURRENT_BACKEND_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn current_backend_id() -> Option<String> {
    CURRENT_BACKEND_ID.with(|current| current.borrow().clone())
}

pub fn with_backend<T>(backend_id: &str, f: impl FnOnce() -> T) -> Result<T, String> {
    let backend = backend_reference(backend_id)
        .ok_or_else(|| format!("Unknown tree-haver backend {backend_id}."))?;

    Ok(CURRENT_BACKEND_ID.with(|current| {
        let previous_backend = current.replace(Some(backend.id));
        let result = f();
        current.replace(previous_backend);
        result
    }))
}

pub fn language_pack_adapter_info() -> AdapterInfo {
    AdapterInfo {
        backend: "kreuzberg-language-pack".to_string(),
        backend_ref: Some(kreuzberg_language_pack_backend()),
        supports_dialects: false,
        supported_policies: vec![],
    }
}

fn ensure_language_pack_language(language: &str) -> Result<(), String> {
    static INITIALIZED_LANGUAGES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let initialized_languages = INITIALIZED_LANGUAGES.get_or_init(|| Mutex::new(HashSet::new()));
    let mut initialized_languages = initialized_languages
        .lock()
        .map_err(|_| "language-pack initialization lock poisoned".to_string())?;

    if initialized_languages.contains(language) {
        return Ok(());
    }

    configure(&PackConfig {
        cache_dir: language_pack_cache_dir(),
        languages: None,
        groups: None,
    })
    .map_err(|error| error.to_string())?;

    initialized_languages.insert(language.to_string());
    Ok(())
}

fn language_pack_cache_dir() -> Option<PathBuf> {
    std::env::var("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR")
        .ok()
        .or_else(|| std::env::var("TREE_SITTER_LANGUAGE_PACK_CACHE_DIR").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn parse_with_language_pack(request: &ParserRequest) -> ParseResult<LanguagePackAnalysis> {
    if let Err(error) = ensure_language_pack_language(&request.language) {
        return ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::UnsupportedFeature,
                message: error,
                path: None,
            }],
            analysis: None,
            policies: vec![],
        };
    }

    match parse_string(&request.language, request.source.as_bytes()) {
        Ok(tree) => {
            let has_error = tree_has_error_nodes(&tree);
            if has_error {
                ParseResult {
                    ok: false,
                    diagnostics: vec![Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        category: DiagnosticCategory::ParseError,
                        message: format!(
                            "tree-sitter-language-pack reported syntax errors for {}.",
                            request.language
                        ),
                        path: None,
                    }],
                    analysis: None,
                    policies: vec![],
                }
            } else {
                ParseResult {
                    ok: true,
                    diagnostics: vec![],
                    analysis: Some(LanguagePackAnalysis {
                        language: request.language.clone(),
                        dialect: request.dialect.clone(),
                        root_type: tree.root_node().kind().to_string(),
                        has_error: false,
                        backend_ref: kreuzberg_language_pack_backend(),
                    }),
                    policies: vec![],
                }
            }
        }
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::UnsupportedFeature,
                message: error.to_string(),
                path: None,
            }],
            analysis: None,
            policies: vec![],
        },
    }
}

fn structure_kind_name(kind: &tree_sitter_language_pack::StructureKind) -> String {
    match kind {
        tree_sitter_language_pack::StructureKind::Function => "function".to_string(),
        tree_sitter_language_pack::StructureKind::Method => "method".to_string(),
        tree_sitter_language_pack::StructureKind::Class => "class".to_string(),
        tree_sitter_language_pack::StructureKind::Struct => "struct".to_string(),
        tree_sitter_language_pack::StructureKind::Interface => "interface".to_string(),
        tree_sitter_language_pack::StructureKind::Enum => "enum".to_string(),
        tree_sitter_language_pack::StructureKind::Module => "module".to_string(),
        tree_sitter_language_pack::StructureKind::Trait => "trait".to_string(),
        tree_sitter_language_pack::StructureKind::Impl => "impl".to_string(),
        tree_sitter_language_pack::StructureKind::Namespace => "namespace".to_string(),
        tree_sitter_language_pack::StructureKind::Other(other) => other.clone(),
    }
}

fn normalize_typescript_import(item: &tree_sitter_language_pack::ImportInfo) -> ProcessImportInfo {
    let source_match = item
        .source
        .split("from")
        .nth(1)
        .and_then(|tail| tail.split(['"', '\'']).nth(1))
        .or_else(|| item.source.split(['"', '\'']).nth(1))
        .unwrap_or(item.source.as_str())
        .to_string();
    let items = item
        .source
        .split('{')
        .nth(1)
        .and_then(|tail| tail.split('}').next())
        .map(|raw| {
            raw.split(',')
                .map(|part| part.replace("type", "").trim().to_string())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ProcessImportInfo {
        source: source_match,
        items,
        span: ProcessSpan {
            start_byte: item.span.start_byte,
            end_byte: item.span.end_byte,
            start_row: item.span.start_line,
            start_col: item.span.start_column,
            end_row: item.span.end_line,
            end_col: item.span.end_column,
        },
    }
}

fn diagnostic_severity_name(severity: &tree_sitter_language_pack::DiagnosticSeverity) -> String {
    match severity {
        tree_sitter_language_pack::DiagnosticSeverity::Error => "error".to_string(),
        tree_sitter_language_pack::DiagnosticSeverity::Warning => "warning".to_string(),
        tree_sitter_language_pack::DiagnosticSeverity::Info => "info".to_string(),
    }
}

pub fn process_with_language_pack(
    request: &ProcessRequest,
) -> ParseResult<LanguagePackProcessAnalysis> {
    if let Err(error) = ensure_language_pack_language(&request.language) {
        return ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::UnsupportedFeature,
                message: error,
                path: None,
            }],
            analysis: None,
            policies: vec![],
        };
    }

    match process(&request.source, &ProcessConfig::new(&request.language).all()) {
        Ok(result) => ParseResult {
            ok: true,
            diagnostics: vec![],
            analysis: Some(LanguagePackProcessAnalysis {
                language: result.language,
                structure: result
                    .structure
                    .iter()
                    .map(|item| ProcessStructureItem {
                        kind: structure_kind_name(&item.kind),
                        name: item.name.clone(),
                        span: ProcessSpan {
                            start_byte: item.span.start_byte,
                            end_byte: item.span.end_byte,
                            start_row: item.span.start_line,
                            start_col: item.span.start_column,
                            end_row: item.span.end_line,
                            end_col: item.span.end_column,
                        },
                    })
                    .collect(),
                imports: result
                    .imports
                    .iter()
                    .map(|item| {
                        if request.language == "typescript" {
                            normalize_typescript_import(item)
                        } else {
                            ProcessImportInfo {
                                source: item.source.clone(),
                                items: item.items.clone(),
                                span: ProcessSpan {
                                    start_byte: item.span.start_byte,
                                    end_byte: item.span.end_byte,
                                    start_row: item.span.start_line,
                                    start_col: item.span.start_column,
                                    end_row: item.span.end_line,
                                    end_col: item.span.end_column,
                                },
                            }
                        }
                    })
                    .collect(),
                diagnostics: result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| ProcessDiagnostic {
                        message: diagnostic.message.clone(),
                        severity: diagnostic_severity_name(&diagnostic.severity),
                    })
                    .collect(),
                backend_ref: kreuzberg_language_pack_backend(),
            }),
            policies: vec![],
        },
        Err(error) => ParseResult {
            ok: false,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                category: DiagnosticCategory::UnsupportedFeature,
                message: error.to_string(),
                path: None,
            }],
            analysis: None,
            policies: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json_with_language_pack() {
        let result = parse_with_language_pack(&ParserRequest {
            source: "{\"alpha\":1}".to_string(),
            language: "json".to_string(),
            dialect: None,
        });

        assert!(result.ok);
        assert_eq!(
            result.analysis,
            Some(LanguagePackAnalysis {
                language: "json".to_string(),
                dialect: None,
                root_type: "document".to_string(),
                has_error: false,
                backend_ref: kreuzberg_language_pack_backend(),
            })
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn reports_invalid_json_with_language_pack() {
        let result = parse_with_language_pack(&ParserRequest {
            source: "{\"alpha\":1,}".to_string(),
            language: "json".to_string(),
            dialect: None,
        });

        assert!(!result.ok);
        assert!(result.analysis.is_none());
        assert_eq!(
            result.diagnostics[0].message,
            "tree-sitter-language-pack reported syntax errors for json."
        );
    }
}
