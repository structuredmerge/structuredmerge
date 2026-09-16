//! Canonical Slice 1028 diagnostic records. Native codes are opaque origin
//! evidence; human messages never select categories or portable codes.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ByteRange, DiagnosticSeverity, Metadata, NativeExtension, OperationKind, SourceMap, SourceRole,
    operation_result::{ResultDiagnostic, ResultSpan},
};

pub const DIAGNOSTIC_SCHEMA: &str = "structuredmerge.diagnostic/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableCategory {
    ParseError,
    DestinationParseError,
    InvalidRequest,
    ConfigurationError,
    SelectionError,
    BackendUnavailable,
    UnsupportedFeature,
    Ambiguity,
    AnalysisError,
    MergeConflict,
    RenderError,
    PreservationError,
    VerificationError,
    FallbackApplied,
    AssumedDefault,
    Cancelled,
    DeadlineExceeded,
    ResourceLimit,
    ReplayRejected,
    InternalError,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLayer {
    Transport,
    Registry,
    Parser,
    Analysis,
    Provider,
    Renderer,
    Verifier,
    Adapter,
    Runner,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DiagnosticOrigin {
    pub layer: DiagnosticLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
    pub native_code: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DiagnosticSourceRef {
    pub source_id: String,
    pub role: SourceRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<ResultSpan>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DiagnosticSubjectRef {
    pub kind: String,
    pub id: String,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PortableDiagnostic {
    pub schema: String,
    pub id: String,
    pub sequence: u64,
    pub severity: DiagnosticSeverity,
    pub category: PortableCategory,
    pub code: String,
    pub message: String,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<OperationKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub source_refs: Vec<DiagnosticSourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_refs: Option<Vec<DiagnosticSubjectRef>>,
    pub cause_ids: Vec<String>,
    pub related_ids: Vec<String>,
    pub origin: DiagnosticOrigin,
    pub data: Metadata,
    pub extensions: Vec<NativeExtension>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

/// A schema-bearing record is always parsed as canonical. Malformed canonical
/// data must never fall back to the permissive migration shape.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DiagnosticRecord {
    Canonical(PortableDiagnostic),
    Migration(ResultDiagnostic),
}

impl<'de> Deserialize<'de> for DiagnosticRecord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Record;
        impl<'de> serde::de::Visitor<'de> for Record {
            type Value = serde_json::Value;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a diagnostic object with unique field names")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut fields = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if fields.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate diagnostic field"));
                    }
                }
                Ok(serde_json::Value::Object(fields))
            }
        }
        let value = deserializer.deserialize_map(Record)?;
        if value.get("schema").is_some() {
            serde_json::from_value(value).map(Self::Canonical).map_err(serde::de::Error::custom)
        } else {
            serde_json::from_value(value).map(Self::Migration).map_err(serde::de::Error::custom)
        }
    }
}

impl DiagnosticRecord {
    pub fn id(&self) -> &str {
        match self {
            Self::Canonical(record) => &record.id,
            Self::Migration(record) => &record.id,
        }
    }

    pub fn blocking(&self) -> bool {
        match self {
            Self::Canonical(record) => record.blocking,
            Self::Migration(record) => record.blocking,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticContractError {
    Schema,
    Identity,
    Code,
    Sequence,
    SemanticOrder,
    Causality,
    RelatedReference,
    SourceReference,
    SubjectReference,
    RequestIdentity,
    UnmappedMigration,
}

impl std::fmt::Display for DiagnosticContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DiagnosticContractError {}

fn portable_code(code: &str) -> bool {
    let segments: Vec<_> = code.split('.').collect();
    segments.len() >= 2
        && segments.iter().all(|segment| {
            segment.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

fn phase(diagnostic: &PortableDiagnostic) -> u8 {
    use PortableCategory as C;
    match diagnostic.category {
        C::InvalidRequest | C::ConfigurationError | C::SelectionError | C::BackendUnavailable => 0,
        C::ParseError | C::DestinationParseError => 1,
        C::AnalysisError => 2,
        C::MergeConflict | C::Ambiguity => 3,
        C::RenderError => 4,
        C::PreservationError => 5,
        C::VerificationError => 6,
        _ => match diagnostic.origin.layer {
            DiagnosticLayer::Transport | DiagnosticLayer::Registry => 0,
            DiagnosticLayer::Parser => 1,
            DiagnosticLayer::Analysis => 2,
            DiagnosticLayer::Provider => 3,
            DiagnosticLayer::Renderer => 4,
            DiagnosticLayer::Verifier => 6,
            DiagnosticLayer::Adapter => 7,
            DiagnosticLayer::Runner => 8,
        },
    }
}

/// Validate an entire result-local diagnostic scope. The owning result supplies
/// subject resolution; arbitrary structural paths cannot validate themselves.
/// Provider execution must additionally prove deterministic ID derivation and
/// discovery order; a single serialized record cannot prove its own provenance.
pub fn validate_diagnostics(
    diagnostics: &[&PortableDiagnostic],
    request_id: &str,
    operation: OperationKind,
    sources: &SourceMap,
    subject_exists: impl Fn(&DiagnosticSubjectRef) -> bool,
) -> Result<(), DiagnosticContractError> {
    use DiagnosticContractError as E;
    let mut ids = BTreeSet::new();
    for diagnostic in diagnostics {
        if diagnostic.id.is_empty() || !ids.insert(diagnostic.id.as_str()) {
            return Err(E::Identity);
        }
    }
    let mut preceding = BTreeSet::new();
    let mut previous_order = None;
    for (sequence, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.schema != DIAGNOSTIC_SCHEMA {
            return Err(E::Schema);
        }
        if diagnostic.sequence != sequence as u64 {
            return Err(E::Sequence);
        }
        if !portable_code(&diagnostic.code) {
            return Err(E::Code);
        }
        if diagnostic.request_id.as_deref().is_some_and(|id| id != request_id)
            || diagnostic.operation.is_some_and(|kind| kind != operation)
        {
            return Err(E::RequestIdentity);
        }
        let mut causes = BTreeSet::new();
        for cause in &diagnostic.cause_ids {
            if !preceding.contains(cause.as_str()) || !causes.insert(cause) {
                return Err(E::Causality);
            }
        }
        let mut related = BTreeSet::new();
        for reference in &diagnostic.related_ids {
            if !ids.contains(reference.as_str()) || !related.insert(reference) {
                return Err(E::RelatedReference);
            }
        }
        if matches!(
            diagnostic.category,
            PortableCategory::ParseError | PortableCategory::DestinationParseError
        ) && diagnostic.source_refs.is_empty()
        {
            return Err(E::SourceReference);
        }
        for reference in &diagnostic.source_refs {
            let document = sources.get(&reference.source_id).map_err(|_| E::SourceReference)?;
            if reference.role != document.descriptor().role
                || !operation.source_roles().contains(&reference.role)
            {
                return Err(E::SourceReference);
            }
            if let Some(span) = &reference.span {
                document
                    .slice(ByteRange {
                        start_byte: span.range.start_byte,
                        end_byte: span.range.end_byte,
                    })
                    .map_err(|_| E::SourceReference)?;
                for (offset, point) in [
                    (span.range.start_byte, &span.start_point),
                    (span.range.end_byte, &span.end_point),
                ] {
                    let actual = document.point(offset).map_err(|_| E::SourceReference)?;
                    if actual.row != point.row || actual.column != point.column {
                        return Err(E::SourceReference);
                    }
                }
            }
        }
        for subject in diagnostic.subject_refs.iter().flatten() {
            if subject.id.is_empty() || subject.kind.is_empty() || !subject_exists(subject) {
                return Err(E::SubjectReference);
            }
        }
        let phase = phase(diagnostic);
        let role_order = if phase == 1 {
            diagnostic
                .source_refs
                .iter()
                .filter_map(|source| {
                    operation.source_roles().iter().position(|role| *role == source.role)
                })
                .min()
                .unwrap_or(0)
        } else {
            0
        };
        let order = (phase, role_order);
        if previous_order.is_some_and(|previous| previous > order) {
            return Err(E::SemanticOrder);
        }
        previous_order = Some(order);
        preceding.insert(diagnostic.id.as_str());
    }
    Ok(())
}

/// Explicit migration for the historical reasons exercised by Slice 1025.
/// Unknown reasons fail closed: neither English text nor punctuation rewriting
/// assigns semantics. The caller supplies actual origin identity, never guessed
/// package/backend versions. The original record remains an opaque extension.
pub fn migrate_diagnostic(
    legacy: ResultDiagnostic,
    sequence: u64,
    request: &crate::operation::ValidatedOperationRequest,
    origin: DiagnosticOrigin,
) -> Result<PortableDiagnostic, DiagnosticContractError> {
    use DiagnosticContractError as E;
    let (category, code) = match (legacy.category.as_str(), legacy.code.as_str()) {
        ("parse-error", "unexpected-eof") => (PortableCategory::ParseError, "parse.unexpected_eof"),
        ("merge-conflict", "edit-edit") => (PortableCategory::MergeConflict, "merge.edit_edit"),
        _ => return Err(E::UnmappedMigration),
    };
    let severity = match legacy.severity.as_str() {
        "info" => DiagnosticSeverity::Info,
        "warning" => DiagnosticSeverity::Warning,
        "error" => DiagnosticSeverity::Error,
        _ => return Err(E::UnmappedMigration),
    };
    let mut source_refs = vec![];
    if let Some(role) = legacy.source_role {
        let source = request.request().sources.get(&role).ok_or(E::SourceReference)?;
        source_refs.push(DiagnosticSourceRef {
            source_id: source.source_id.clone(),
            role,
            span: legacy.span.clone(),
            extra: Metadata::new(),
        });
    } else if legacy.span.is_some() || category == PortableCategory::ParseError {
        return Err(E::SourceReference);
    }
    let payload = serde_json::to_value(&legacy).map_err(|_| E::UnmappedMigration)?;
    Ok(PortableDiagnostic {
        schema: DIAGNOSTIC_SCHEMA.into(),
        id: legacy.id,
        sequence,
        severity,
        category,
        code: code.into(),
        message: legacy.message,
        blocking: legacy.blocking,
        operation: Some(request.request().operation.kind()),
        request_id: Some(request.request().request_id.clone()),
        source_refs,
        subject_refs: None,
        cause_ids: vec![],
        related_ids: vec![],
        origin,
        data: Metadata::new(),
        extensions: vec![NativeExtension {
            schema: "structuredmerge.migration-diagnostic/v1".into(),
            namespace: "structuredmerge.migration".into(),
            capabilities: vec![],
            payload,
            extra: Metadata::new(),
        }],
        metadata: legacy.metadata,
        extra: Metadata::new(),
    })
}
