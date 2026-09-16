//! Slice 1028 conflict evidence. No marker scanning or implicit resolution.

use crate::{
    ByteRange, NativeExtension, OperationKind, SourceDocument, SourceMap, SourceRole,
    operation_result::{ResultConflict, ResultRange},
    portable_diagnostic::portable_code,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CONFLICT_SCHEMA: &str = "structuredmerge.conflict/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCategory {
    Content,
    DeleteModify,
    AddAdd,
    Move,
    Rename,
    Order,
    Identity,
    Ownership,
    Syntax,
    BinaryOverlap,
    ArchiveEntry,
    ProviderSpecific,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativeState {
    Present,
    Absent,
    Invalid,
    Opaque,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalizationStatus {
    Exact,
    Owner,
    Coarse,
    WholeDocument,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Unresolved,
    Resolved,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    None,
    SelectRole,
    Combine,
    Policy,
    Custom,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictSubject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structural_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_region: Option<ResultRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole_document: Option<bool>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExactConflictRegion {
    pub range: ResultRange,
    pub byte_length: u64,
    pub sha256: String,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictSourceAlternative {
    pub role: SourceRole,
    pub state: AlternativeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub regions: Vec<ExactConflictRegion>,
    pub change_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictClassification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_participated: Option<bool>,
    pub change_ids: Vec<String>,
    pub decision_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictLocalization {
    pub status: LocalizationStatus,
    pub verified: bool,
    pub output_regions: Vec<ExactConflictRegion>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictResolution {
    pub status: ResolutionStatus,
    pub strategy: ResolutionStrategy,
    pub selected_roles: Vec<SourceRole>,
    pub decision_id: Option<String>,
    pub resolver: Option<String>,
    pub reason: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PortableConflict {
    pub schema: String,
    pub id: String,
    pub operation: OperationKind,
    pub category: ConflictCategory,
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub subject: ConflictSubject,
    pub roles: Vec<SourceRole>,
    pub alternatives: Vec<ConflictSourceAlternative>,
    pub classification: ConflictClassification,
    pub localization: ConflictLocalization,
    pub resolution: ConflictResolution,
    pub diagnostic_ids: Vec<String>,
    pub change_ids: Vec<String>,
    pub decision_ids: Vec<String>,
    pub render_fragment_ids: Vec<String>,
    pub extensions: Vec<NativeExtension>,
    pub metadata: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ConflictRecord {
    Canonical(Box<PortableConflict>),
    Migration(Box<ResultConflict>),
}

impl<'de> Deserialize<'de> for ConflictRecord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Record;
        impl<'de> serde::de::Visitor<'de> for Record {
            type Value = serde_json::Value;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a conflict with unique field names")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut fields = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if fields.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate conflict field"));
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

impl ConflictRecord {
    pub fn id(&self) -> &str {
        match self {
            Self::Canonical(c) => &c.id,
            Self::Migration(c) => &c.id,
        }
    }
    pub fn unresolved(&self) -> bool {
        match self {
            Self::Canonical(c) => c.resolution.status == ResolutionStatus::Unresolved,
            Self::Migration(c) => c.resolution == "unresolved",
        }
    }
    pub fn matches_path(&self, path: &str) -> bool {
        match self {
            Self::Canonical(c) => c.subject.structural_path.as_deref() == Some(path),
            Self::Migration(c) => {
                c.path.as_deref() == Some(path) || c.subject_ref.as_deref() == Some(path)
            }
        }
    }
}

/// Evidence supplied by the trusted operation executor, not reconstructed from
/// a conflict's own declarations. Authorization is scoped to this conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionAuthorization {
    pub decision_id: String,
    pub conflict_id: String,
    pub strategy: ResolutionStrategy,
    pub selected_roles: Vec<SourceRole>,
    pub resolver: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConflictEvidence {
    pub decisions: BTreeSet<String>,
    pub render_fragments: BTreeSet<String>,
    pub authorizations: Vec<ResolutionAuthorization>,
}

pub struct ConflictValidationContext<'a> {
    pub operation: OperationKind,
    pub result_ok: bool,
    pub sources: &'a SourceMap,
    pub output: Option<&'a SourceDocument>,
    pub diagnostic_ids: &'a BTreeSet<String>,
    pub change_ids: &'a BTreeSet<String>,
    pub evidence: &'a ConflictEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictContractError {
    Schema,
    Identity,
    Operation,
    Code,
    Subject,
    Roles,
    BaseParticipation,
    Source,
    ExactLocalization,
    ResolutionAuthorization,
    Reference,
    Outcome,
}
impl std::fmt::Display for ConflictContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ConflictContractError {}

fn references(ids: &[String], available: &BTreeSet<String>) -> bool {
    let mut seen = BTreeSet::new();
    ids.iter().all(|id| !id.is_empty() && seen.insert(id) && available.contains(id))
}

fn region_matches(region: &ExactConflictRegion, source: &SourceDocument) -> bool {
    let range = ByteRange { start_byte: region.range.start_byte, end_byte: region.range.end_byte };
    source.slice(range.clone()).is_ok_and(|bytes| bytes.len() as u64 == region.byte_length)
        && source.range_digest(range).is_ok_and(|digest| digest == region.sha256)
}

pub fn validate_conflicts(
    conflicts: &[&PortableConflict],
    context: &ConflictValidationContext<'_>,
) -> Result<(), ConflictContractError> {
    use ConflictContractError as E;
    let mut ids = BTreeSet::new();
    for conflict in conflicts {
        if conflict.schema != CONFLICT_SCHEMA {
            return Err(E::Schema);
        }
        if conflict.id.is_empty() || !ids.insert(&conflict.id) {
            return Err(E::Identity);
        }
        if conflict.operation != context.operation
            || !matches!(context.operation, OperationKind::Merge2 | OperationKind::Merge3)
        {
            return Err(E::Operation);
        }
        if !portable_code(&conflict.code) {
            return Err(E::Code);
        }
        let subject = &conflict.subject;
        if subject.whole_document != Some(true)
            && subject.binary_region.is_none()
            && [
                &subject.structural_path,
                &subject.owner_ref,
                &subject.node_ref,
                &subject.archive_entry,
            ]
            .iter()
            .all(|value| value.as_ref().is_none_or(String::is_empty))
        {
            return Err(E::Subject);
        }
        if conflict.roles != context.operation.source_roles()
            || conflict.alternatives.iter().map(|alternative| alternative.role).collect::<Vec<_>>()
                != conflict.roles
        {
            return Err(E::Roles);
        }
        if context.operation == OperationKind::Merge3
            && conflict.classification.base_participated != Some(true)
        {
            return Err(E::BaseParticipation);
        }
        if context.result_ok && conflict.resolution.status == ResolutionStatus::Unresolved {
            return Err(E::Outcome);
        }
        for (refs, available) in [
            (&conflict.diagnostic_ids, context.diagnostic_ids),
            (&conflict.change_ids, context.change_ids),
            (&conflict.classification.change_ids, context.change_ids),
            (&conflict.decision_ids, &context.evidence.decisions),
            (&conflict.classification.decision_ids, &context.evidence.decisions),
            (&conflict.render_fragment_ids, &context.evidence.render_fragments),
        ] {
            if !references(refs, available) {
                return Err(E::Reference);
            }
        }
        let exact = conflict.localization.status == LocalizationStatus::Exact;
        if exact && !conflict.localization.verified {
            return Err(E::ExactLocalization);
        }
        for alternative in &conflict.alternatives {
            if !references(&alternative.change_ids, context.change_ids) {
                return Err(E::Reference);
            }
            if alternative.state == AlternativeState::Absent {
                if !alternative.regions.is_empty() {
                    return Err(E::Source);
                }
            } else if alternative.state == AlternativeState::Present
                && alternative.source_id.is_none()
            {
                return Err(E::Source);
            }
            if exact
                && alternative.state == AlternativeState::Present
                && alternative.regions.is_empty()
            {
                return Err(E::ExactLocalization);
            }
            if let Some(id) = &alternative.source_id {
                let source = context.sources.get(id).map_err(|_| E::Source)?;
                if source.descriptor().role != alternative.role {
                    return Err(E::Source);
                }
                if !alternative.regions.iter().all(|region| region_matches(region, source)) {
                    return Err(E::Source);
                }
            } else if !alternative.regions.is_empty() {
                return Err(E::Source);
            }
        }
        for region in &conflict.localization.output_regions {
            if !context.output.is_some_and(|output| region_matches(region, output)) {
                return Err(E::Source);
            }
        }
        let resolution = &conflict.resolution;
        match resolution.status {
            ResolutionStatus::Unresolved => {
                if resolution.strategy != ResolutionStrategy::None
                    || !resolution.selected_roles.is_empty()
                    || resolution.decision_id.is_some()
                    || resolution.resolver.is_some()
                    || resolution.reason.is_some()
                {
                    return Err(E::ResolutionAuthorization);
                }
            }
            ResolutionStatus::Resolved => {
                let selected: BTreeSet<_> = resolution.selected_roles.iter().collect();
                if resolution.strategy == ResolutionStrategy::None
                    || resolution.reason.as_ref().is_none_or(String::is_empty)
                    || resolution.resolver.as_ref().is_none_or(String::is_empty)
                    || resolution
                        .decision_id
                        .as_ref()
                        .is_none_or(|id| !context.evidence.decisions.contains(id))
                    || selected.len() != resolution.selected_roles.len()
                    || !resolution.selected_roles.iter().all(|role| conflict.roles.contains(role))
                    || (resolution.strategy == ResolutionStrategy::SelectRole
                        && selected.len() != 1)
                    || !context.evidence.authorizations.iter().any(|authorization| {
                        Some(&authorization.decision_id) == resolution.decision_id.as_ref()
                            && authorization.conflict_id == conflict.id
                            && authorization.strategy == resolution.strategy
                            && authorization.selected_roles == resolution.selected_roles
                            && Some(&authorization.resolver) == resolution.resolver.as_ref()
                    })
                {
                    return Err(E::ResolutionAuthorization);
                }
            }
        }
    }
    Ok(())
}
