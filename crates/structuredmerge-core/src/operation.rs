//! Slice 1025 request transport. Normalization verifies inputs, not provider support.
//! Policies and selection constraints remain attached to the validated request;
//! execution must negotiate them before dispatch, never silently drop them.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    LineEndings, Metadata, NativeExtension, OPERATION_SCHEMA, OperationKind, SourceEncoding,
    SourceMap, SourceRole, source_input,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MergeProviderSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub required_capabilities: Vec<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationParserSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    pub preference: Vec<String>,
    pub required_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_version: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnalyzePolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_depth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_extensions: Option<bool>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DiffPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equivalence: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_preservation_evidence: Option<bool>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DirectionalMergePolicy {
    pub directional_merge: String,
    pub render_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_policy: Option<String>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ThreeWayMergePolicy {
    pub render_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_marker_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
    #[serde(flatten)]
    pub extra: Metadata,
}

/// The operation discriminator determines the policy type, not argument order.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "operation", content = "policy", rename_all = "snake_case")]
pub enum OperationPolicy {
    Analyze(AnalyzePolicy),
    Diff2(DiffPolicy),
    Merge2(DirectionalMergePolicy),
    Merge3(ThreeWayMergePolicy),
}

impl OperationPolicy {
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Analyze(_) => OperationKind::Analyze,
            Self::Diff2(_) => OperationKind::Diff2,
            Self::Merge2(_) => OperationKind::Merge2,
            Self::Merge3(_) => OperationKind::Merge3,
        }
    }

    /// This is a request, not permission to activate an unsupported fallback.
    pub fn fallback_policy(&self) -> &str {
        match self {
            Self::Merge2(policy) => policy.fallback_policy.as_deref().unwrap_or("none"),
            Self::Merge3(policy) => policy.fallback_policy.as_deref().unwrap_or("none"),
            Self::Analyze(_) | Self::Diff2(_) => "none",
        }
    }
}

/// Wire source: exactly one of UTF-8 content, bytes, or an explicitly resolved
/// local reference. Optional layout evidence is verified when supplied and
/// computed from exact bytes otherwise (including the Slice 1025 fixtures).
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationSource {
    pub source_id: String,
    pub role: SourceRole,
    pub byte_length: u64,
    pub sha256: String,
    pub encoding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bom: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_endings: Option<LineEndings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_newline: Option<bool>,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationRequest {
    pub schema: String,
    pub request_id: String,
    #[serde(flatten)]
    pub operation: OperationPolicy,
    pub provider_selection: MergeProviderSelection,
    pub parser_selection: OperationParserSelection,
    #[serde(deserialize_with = "deserialize_sources")]
    pub sources: BTreeMap<SourceRole, OperationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_name: Option<String>,
    pub extensions: Vec<NativeExtension>,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub extra: Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationRequestErrorCode {
    UnsupportedSchema,
    InvalidIdentity,
    InvalidSelection,
    InvalidRoles,
    InvalidSource,
    SourceResolution,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationRequestError {
    pub request_id: String,
    pub source_role: Option<SourceRole>,
    pub code: OperationRequestErrorCode,
    pub message: String,
}

impl fmt::Display for OperationRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} for {:?}: {}", self.code, self.request_id, self.message)
    }
}

impl Error for OperationRequestError {}

/// Immutable verified bytes plus the complete request, including unknown fields.
/// No conversion to a narrower request that could discard policy is provided.
#[derive(Clone, Debug)]
pub struct ValidatedOperationRequest {
    request: OperationRequest,
    sources: SourceMap,
}

impl ValidatedOperationRequest {
    pub fn request(&self) -> &OperationRequest {
        &self.request
    }

    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }
}

impl OperationRequest {
    fn error(
        &self,
        code: OperationRequestErrorCode,
        source_role: Option<SourceRole>,
        message: impl Into<String>,
    ) -> OperationRequestError {
        OperationRequestError {
            request_id: self.request_id.clone(),
            source_role,
            code,
            message: message.into(),
        }
    }

    /// Resolve only references explicitly authorized by the caller. The resolver
    /// must be a bounded local content store, not a URL/path-fetch convenience.
    /// Its returned bytes are untrusted and verified here before use. `limit`
    /// is the declared source byte length, already checked against the budget.
    /// No parser, provider, filesystem, or network access occurs in this layer.
    pub fn validate(
        self,
        max_total_bytes: u64,
        mut resolve: impl FnMut(&OperationSource, u64) -> Result<Vec<u8>, String>,
    ) -> Result<ValidatedOperationRequest, OperationRequestError> {
        use OperationRequestErrorCode as Code;
        if self.schema != OPERATION_SCHEMA {
            return Err(self.error(Code::UnsupportedSchema, None, "unsupported request schema"));
        }
        if self.request_id.is_empty() {
            return Err(self.error(Code::InvalidIdentity, None, "request_id must not be empty"));
        }
        let provider = &self.provider_selection;
        let parser = &self.parser_selection;
        let empty = |value: &Option<String>| value.as_ref().is_some_and(String::is_empty);
        let capabilities_valid = |values: &[String]| {
            values.iter().all(|value| !value.is_empty())
                && values.windows(2).all(|pair| pair[0] < pair[1])
        };
        if (provider.provider_id.is_none() && provider.family.is_none())
            || [
                &provider.provider_id,
                &provider.family,
                &provider.dialect,
                &provider.profile_id,
                &parser.backend,
                &parser.profile_id,
                &parser.language_version,
            ]
            .into_iter()
            .any(empty)
            || parser.preference.iter().any(String::is_empty)
            || !capabilities_valid(&provider.required_capabilities)
            || !capabilities_valid(&parser.required_capabilities)
        {
            return Err(self.error(
                Code::InvalidSelection,
                None,
                "invalid provider/parser selection",
            ));
        }
        let roles = self.operation.kind().source_roles();
        if self.sources.len() != roles.len()
            || !roles.iter().all(|role| self.sources.contains_key(role))
        {
            return Err(self.error(
                Code::InvalidRoles,
                None,
                "operation requires its exact source roles",
            ));
        }
        // Validate the whole descriptor set before any resolver callback.
        let mut remaining = max_total_bytes;
        let mut ids = std::collections::BTreeSet::new();
        for (&role, source) in &self.sources {
            if source.role != role {
                return Err(self.error(
                    Code::InvalidRoles,
                    Some(role),
                    "source key and role differ",
                ));
            }
            if source.source_id.is_empty() || !ids.insert(&source.source_id) {
                return Err(self.error(
                    Code::InvalidSource,
                    Some(role),
                    "empty or duplicate source ID",
                ));
            }
            let content_count = usize::from(source.content.is_some())
                + usize::from(source.bytes.is_some())
                + usize::from(source.reference.is_some());
            if content_count != 1
                || empty(&source.reference)
                || !matches!(source.encoding.as_str(), "utf-8" | "binary")
                || (source.content.is_some() && source.encoding != "utf-8")
                || source.sha256.len() != 64
                || !source
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(self.error(
                    Code::InvalidSource,
                    Some(role),
                    "invalid content, encoding, reference, or digest",
                ));
            }
            remaining = remaining.checked_sub(source.byte_length).ok_or_else(|| {
                self.error(Code::ResourceLimit, Some(role), "source byte budget exceeded")
            })?;
            let inline_length = source
                .content
                .as_ref()
                .map(|text| text.len())
                .or_else(|| source.bytes.as_ref().map(Vec::len));
            if inline_length.is_some_and(|length| length as u64 != source.byte_length) {
                return Err(self.error(
                    Code::InvalidSource,
                    Some(role),
                    "inline byte length differs from descriptor",
                ));
            }
        }
        let mut inputs = Vec::with_capacity(roles.len());
        let mut remaining = max_total_bytes;
        for &role in roles {
            let source = &self.sources[&role];
            let bytes = if let Some(content) = &source.content {
                content.as_bytes().to_vec()
            } else if let Some(bytes) = &source.bytes {
                bytes.clone()
            } else {
                resolve(source, source.byte_length)
                    .map_err(|message| self.error(Code::SourceResolution, Some(role), message))?
            };
            if bytes.len() as u64 > remaining {
                return Err(self.error(
                    Code::ResourceLimit,
                    Some(role),
                    "resolved source exceeds byte budget",
                ));
            }
            let encoding = if source.encoding == "utf-8" {
                SourceEncoding::Utf8
            } else {
                SourceEncoding::Binary
            };
            let input = source_input(source.source_id.clone(), role, encoding, bytes)
                .map_err(|error| self.error(Code::InvalidSource, Some(role), error.to_string()))?;
            let actual = &input.descriptor;
            if actual.byte_length != source.byte_length
                || actual.sha256 != source.sha256
                || source.bom.is_some_and(|bom| bom != actual.bom)
                || source
                    .line_endings
                    .as_ref()
                    .is_some_and(|endings| *endings != actual.line_endings)
                || source.final_newline.is_some_and(|newline| newline != actual.final_newline)
            {
                return Err(self.error(
                    Code::InvalidSource,
                    Some(role),
                    "source evidence does not match exact bytes",
                ));
            }
            remaining -= actual.byte_length;
            inputs.push(input);
        }
        let sources = SourceMap::validate(inputs, max_total_bytes)
            .map_err(|error| self.error(Code::InvalidSource, None, error.to_string()))?;
        Ok(ValidatedOperationRequest { request: self, sources })
    }
}

fn deserialize_sources<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<SourceRole, OperationSource>, D::Error> {
    struct Sources;
    impl<'de> serde::de::Visitor<'de> for Sources {
        type Value = BTreeMap<SourceRole, OperationSource>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with unique source role keys")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut sources = BTreeMap::new();
            while let Some((role, source)) = map.next_entry()? {
                if sources.insert(role, source).is_some() {
                    return Err(serde::de::Error::custom("duplicate source role key"));
                }
            }
            Ok(sources)
        }
    }
    deserializer.deserialize_map(Sources)
}

/// Normalize a complete batch in caller order. IDs must be unique within the
/// batch, and the byte limit covers the entire batch, not each member afresh.
/// A failure returns no partial normalized batch; no operation is dispatched.
pub fn validate_operation_batch(
    requests: Vec<OperationRequest>,
    max_requests: usize,
    max_total_bytes: u64,
    mut resolve: impl FnMut(&OperationSource, u64) -> Result<Vec<u8>, String>,
) -> Result<Vec<ValidatedOperationRequest>, OperationRequestError> {
    use OperationRequestErrorCode as Code;
    if requests.len() > max_requests {
        return Err(requests[max_requests].error(
            Code::ResourceLimit,
            None,
            "request count limit exceeded",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for request in &requests {
        if request.request_id.is_empty() || !ids.insert(&request.request_id) {
            return Err(request.error(
                Code::InvalidIdentity,
                None,
                "empty or duplicate batch request ID",
            ));
        }
    }
    let mut remaining = max_total_bytes;
    let mut results = Vec::with_capacity(requests.len());
    for request in requests {
        let validated = request.validate(remaining, &mut resolve)?;
        for source in validated.request.sources.values() {
            remaining -= source.byte_length;
        }
        results.push(validated);
    }
    Ok(results)
}
