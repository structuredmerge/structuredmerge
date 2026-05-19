use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use serde::{Deserialize, Serialize};
use tree_sitter_language_pack::{PackConfig, ProcessConfig, configure, get_parser, process};

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserIdentity {
    pub name: String,
    pub version: String,
    pub implementation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LanguageVersion {
    pub version: String,
    pub dialect: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackendCapability {
    pub backend_ref: BackendReference,
    pub language: String,
    pub parser_identity: ParserIdentity,
    pub language_version: LanguageVersion,
    pub parse_error_behavior: String,
    pub source_span_support: String,
    pub source_fragment_support: String,
    pub render_strategies: Vec<String>,
    pub semantic_role_support: String,
    pub normalized_tree_support: bool,
    pub native_node_access: bool,
    #[serde(default)]
    pub known_node_kinds: Vec<String>,
    #[serde(default)]
    pub known_fields: Vec<String>,
    #[serde(default)]
    pub grammar_inventory: String,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParseErrorNode {
    pub kind: String,
    pub span: SourceSpan,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParseErrorTolerance {
    pub backend_ref: BackendReference,
    pub language: String,
    pub behavior: String,
    pub tolerates_errors: bool,
    pub error_nodes: Vec<ParseErrorNode>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeParserProvider {
    pub id: String,
    pub family: String,
    pub language: String,
    pub operations: Vec<String>,
    pub retains_native_tree: bool,
    pub native_tree_visibility: String,
    pub metadata_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeProviderMetadata {
    pub provider_id: String,
    pub family: String,
    pub host_language: String,
    pub target_language: String,
    pub parser_name: String,
    pub parser_version: String,
    pub language_version: String,
    pub dialect: String,
    pub parse_error_behavior: String,
    pub source_span_support: String,
    pub render_support: String,
    pub semantic_role_support: String,
    pub retains_native_tree: bool,
    pub native_tree_visibility: String,
    pub metadata_policy: String,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedParseResult {
    pub ok: bool,
    pub backend_capability: BackendCapability,
    pub root_id: String,
    pub nodes: Vec<NormalizedTreeNode>,
    pub parse_error_tolerance: ParseErrorTolerance,
    pub source_fragments_available: bool,
    pub diagnostics: Vec<String>,
    pub metadata: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TreeHaverProfile {
    pub profile_id: String,
    pub language: String,
    pub backend_ref: BackendReference,
    pub provider_id: String,
    pub node_roles: Vec<NodeRole>,
    pub normalized_node_fields: Vec<String>,
    pub optional_node_features: Vec<String>,
    pub unsupported_defaults: BTreeMap<String, String>,
    pub capability: BackendCapability,
    pub fixture_slices: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionSupport {
    pub backend_ref: BackendReference,
    pub language: String,
    pub supports_edit_projection: bool,
    pub native_edit_target: String,
    pub normalized_edit_target: String,
    pub supported_operations: Vec<String>,
    pub required_node_fields: Vec<String>,
    pub correlation_keys: Vec<String>,
    pub preserves_source_fragments: bool,
    pub unsupported_reason: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryPathValidation {
    pub path: String,
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackendAvailabilityCheck {
    pub name: String,
    pub status: String,
    pub required: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackendAvailabilityReport {
    pub backend_ref: BackendReference,
    pub status: String,
    pub checks: Vec<BackendAvailabilityCheck>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderDiagnostic {
    pub severity: String,
    pub category: String,
    pub code: String,
    pub message: String,
    pub path: String,
    pub blocking: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderDiagnosticsReport {
    pub provider_id: String,
    pub backend_ref: BackendReference,
    pub language: String,
    pub status: String,
    pub diagnostics: Vec<ProviderDiagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionOperationRequest {
    pub operation: String,
    pub target_node_id: String,
    pub target_node_path: String,
    pub replacement_source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionExecutionRequest {
    pub provider_id: String,
    pub backend_ref: BackendReference,
    pub language: String,
    pub source: String,
    pub operations: Vec<EditProjectionOperationRequest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppliedEditProjectionOperation {
    pub operation: String,
    pub target_node_id: String,
    pub correlation_key: String,
    pub correlation_value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionExecutionResult {
    pub ok: bool,
    pub status: String,
    pub source: String,
    pub applied_operations: Vec<AppliedEditProjectionOperation>,
    pub diagnostics: Vec<ProviderDiagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionProviderOperation {
    pub operation: String,
    pub status: String,
    pub node_scope: String,
    pub correlation_keys: Vec<String>,
    pub fixture_slices: Vec<String>,
    pub formatting_preservation: String,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionProviderMatrixEntry {
    pub provider_id: String,
    pub backend_ref: BackendReference,
    pub language: String,
    pub formatting_preservation: String,
    pub preserves_source_fragments: bool,
    pub operations: Vec<EditProjectionProviderOperation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditProjectionProviderMatrix {
    pub operations: Vec<String>,
    pub providers: Vec<EditProjectionProviderMatrixEntry>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderedSiblingEdge {
    pub parent_id: String,
    pub node_id: String,
    pub previous_sibling_id: Option<String>,
    pub next_sibling_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderedTreePrimitives {
    pub root_id: String,
    pub child_order: BTreeMap<String, Vec<String>>,
    pub sibling_edges: Vec<OrderedSiblingEdge>,
    pub diagnostics: Vec<String>,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourcePoint {
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceSpan {
    pub range: ByteRange,
    pub start_point: SourcePoint,
    pub end_point: SourcePoint,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Structural,
    Token,
    Trivia,
    Comment,
    Delimiter,
    Separator,
    Virtual,
    Error,
    Opaque,
}

pub fn node_roles() -> Vec<NodeRole> {
    vec![
        NodeRole::Structural,
        NodeRole::Token,
        NodeRole::Trivia,
        NodeRole::Comment,
        NodeRole::Delimiter,
        NodeRole::Separator,
        NodeRole::Virtual,
        NodeRole::Error,
        NodeRole::Opaque,
    ]
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedTreeNode {
    pub id: String,
    pub kind: String,
    pub role: NodeRole,
    pub parent_id: Option<String>,
    pub child_ids: Vec<String>,
    pub span: SourceSpan,
    pub field_name: Option<String>,
    pub named: bool,
    pub anonymous: bool,
    pub has_source_text: bool,
    pub source_fragment: String,
    pub backend_kind: Option<String>,
    #[serde(default)]
    pub semantic_roles: Vec<String>,
    #[serde(default)]
    pub backend_roles: Vec<String>,
    #[serde(default)]
    pub unsupported_features: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceFragment {
    pub text: String,
    pub span: SourceSpan,
    pub available: bool,
    pub strategy: String,
    pub byte_length: usize,
    pub diagnostics: Vec<String>,
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

    Ok(source[byte_range.start_byte..byte_range.end_byte].to_string())
}

pub fn extract_source_fragment(source: &str, span: &SourceSpan, strategy: &str) -> SourceFragment {
    match slice_byte_range(source, &span.range) {
        Ok(text) => SourceFragment {
            byte_length: text.len(),
            text,
            span: span.clone(),
            available: true,
            strategy: strategy.to_string(),
            diagnostics: vec![],
        },
        Err(error) => SourceFragment {
            text: String::new(),
            span: span.clone(),
            available: false,
            strategy: strategy.to_string(),
            byte_length: 0,
            diagnostics: vec![error],
        },
    }
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

pub const MAX_LIBRARY_PATH_LENGTH: usize = 4096;

pub fn validate_library_path(path: &str) -> LibraryPathValidation {
    let errors = library_path_errors(path);
    LibraryPathValidation { path: path.to_string(), valid: errors.is_empty(), errors }
}

pub fn library_path_errors(path: &str) -> Vec<String> {
    if path.is_empty() {
        return vec!["path_empty".to_string()];
    }
    let mut errors = Vec::new();
    if path.len() > MAX_LIBRARY_PATH_LENGTH {
        errors.push("path_too_long".to_string());
    }
    if path.contains('\0') {
        errors.push("path_contains_null_byte".to_string());
    }
    if !path.starts_with('/') && !windows_absolute_path(path) {
        errors.push("path_not_absolute".to_string());
    }
    let segments = path.split(['/', '\\']).collect::<Vec<_>>();
    if segments.contains(&"..") {
        errors.push("path_contains_parent_traversal".to_string());
    }
    if segments.contains(&".") {
        errors.push("path_contains_current_directory_traversal".to_string());
    }
    if !has_allowed_library_extension(path) {
        errors.push("path_extension_not_allowed".to_string());
    }
    if !valid_library_filename(filename(path)) {
        errors.push("filename_contains_invalid_characters".to_string());
    }
    errors
}

pub fn safe_language_name(name: &str) -> bool {
    name.len() <= 64
        && name.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
        && name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

pub fn sanitize_language_name(name: &str) -> Option<String> {
    let sanitized = name
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '_')
        .collect::<String>();
    if sanitized.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
        Some(sanitized)
    } else {
        None
    }
}

pub fn safe_symbol_name(symbol: &str) -> bool {
    symbol.len() <= 256
        && symbol.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && symbol.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub fn safe_backend_name(name: &str) -> bool {
    name == "auto" || backend_reference(name).is_some()
}

pub fn build_backend_availability_report(
    backend_ref: BackendReference,
    checks: Vec<BackendAvailabilityCheck>,
) -> BackendAvailabilityReport {
    if checks.is_empty() {
        return BackendAvailabilityReport {
            backend_ref,
            status: "unknown".to_string(),
            checks,
            diagnostics: vec!["backend availability unknown: no checks supplied".to_string()],
        };
    }

    let mut diagnostics = Vec::new();
    let mut status = "available".to_string();
    for check in &checks {
        if check.required && check.status != "available" {
            status = "unavailable".to_string();
            diagnostics.push(format!(
                "backend unavailable: required check {} is {}",
                check.name, check.status
            ));
        }
    }
    BackendAvailabilityReport { backend_ref, status, checks, diagnostics }
}

pub fn build_provider_diagnostics_report(
    provider_id: String,
    backend_ref: BackendReference,
    language: String,
    diagnostics: Vec<ProviderDiagnostic>,
) -> ProviderDiagnosticsReport {
    let mut status = "clean".to_string();
    for diagnostic in &diagnostics {
        if diagnostic.blocking {
            status = "blocked".to_string();
            break;
        }
        if diagnostic.severity == "warning" {
            status = "warning".to_string();
        }
    }
    ProviderDiagnosticsReport { provider_id, backend_ref, language, status, diagnostics }
}

pub fn build_edit_projection_execution_result(
    source: String,
    applied_operations: Vec<AppliedEditProjectionOperation>,
    diagnostics: Vec<ProviderDiagnostic>,
) -> EditProjectionExecutionResult {
    if diagnostics.iter().any(|diagnostic| diagnostic.blocking) {
        return EditProjectionExecutionResult {
            ok: false,
            status: "rejected".to_string(),
            source,
            applied_operations: Vec::new(),
            diagnostics,
        };
    }

    EditProjectionExecutionResult {
        ok: true,
        status: "applied".to_string(),
        source,
        applied_operations,
        diagnostics,
    }
}

pub fn build_edit_projection_provider_matrix(
    operations: Vec<String>,
    providers: Vec<EditProjectionProviderMatrixEntry>,
    diagnostics: Vec<String>,
) -> EditProjectionProviderMatrix {
    EditProjectionProviderMatrix { operations, providers, diagnostics }
}

fn windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn has_allowed_library_extension(path: &str) -> bool {
    path.ends_with(".so")
        || path.ends_with(".dylib")
        || path.ends_with(".dll")
        || path.rsplit_once(".so.").is_some_and(|(_, version)| {
            !version.is_empty() && version.chars().all(|ch| ch.is_ascii_digit())
        })
}

fn filename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn valid_library_filename(filename: &str) -> bool {
    filename.chars().next().is_some_and(|ch| ch.is_ascii_alphanumeric())
        && filename
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
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

    configure(&PackConfig { cache_dir: language_pack_cache_dir(), languages: None, groups: None })
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

    match get_parser(&request.language).and_then(|mut parser| {
        parser.parse(&request.source).ok_or_else(|| {
            tree_sitter_language_pack::Error::ParserSetup("parser returned no tree".to_string())
        })
    }) {
        Ok(tree) => {
            let has_error = tree.root_node().has_error();
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
