//! Merge declaration filtering followed by TreeHaver parser negotiation.
//! This source-free report is not workflow execution, merge support or authority.

use crate::provider_registry::{MergeProviderRole, MergeProviderSnapshot};
use serde::{Deserialize, Serialize};
use tree_haver::service::{
    ExecutionContext, ParserConstraints, ParserRegistrySnapshot, ParserSelectionRequest,
    SelectionReport, ServiceError, TreeHaverParseService,
};

/// In-process selection inputs; the typed facade owns the portable capability
/// envelope. Parser language is explicit, never guessed from provider names.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MergeSelectionRequest {
    pub provider_id: Option<String>,
    pub family: String,
    pub operation: String,
    pub dialect: Option<String>,
    pub profile: Option<String>,
    pub required_capabilities: Vec<String>,
    pub required_preservation: Vec<String>,
    pub parser: ParserSelectionRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MergeSelectionCandidate {
    pub provider_id: String,
    pub role: MergeProviderRole,
    pub priority: i32,
    pub explicit_match: bool,
    pub rejections: Vec<String>,
    /// Absent when declaration filtering rejects the candidate before probing.
    pub parser_report: Option<SelectionReport>,
    pub selected: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MergeSelectionReport {
    pub requested: MergeSelectionRequest,
    pub provider_generation: u64,
    pub provider_digest: String,
    pub parser_generation: u64,
    pub parser_digest: String,
    pub candidates: Vec<MergeSelectionCandidate>,
    pub selected_provider: Option<String>,
    /// Distinguishes an absent explicit registration from ineligible candidates.
    pub rejections: Vec<String>,
}

/// All candidates use the supplied immutable snapshots. Installing a backend
/// provider cannot change family-only workflow selection. Explicit IDs never
/// substitute, and a provider with incompatible parsers cannot win on priority.
/// No provider callbacks are invoked; only TreeHaver probes parser providers.
pub fn negotiate_merge_provider<P: ?Sized + Send + Sync>(
    request: &MergeSelectionRequest,
    providers: &MergeProviderSnapshot<P>,
    parsers: &ParserRegistrySnapshot,
    parser_service: &TreeHaverParseService,
    context: &ExecutionContext,
) -> Result<MergeSelectionReport, ServiceError> {
    context.check()?;
    validate_request(request)?;
    let mut report = MergeSelectionReport {
        requested: request.clone(),
        provider_generation: providers.generation(),
        provider_digest: providers.digest().into(),
        parser_generation: parsers.generation(),
        parser_digest: parsers.digest().into(),
        candidates: vec![],
        selected_provider: None,
        rejections: vec![],
    };
    if request.provider_id.as_ref().is_some_and(|id| providers.descriptor(id).is_none()) {
        report.rejections.push("unknown_explicit_provider".into());
    }
    for descriptor in providers.inventory().providers {
        context.check()?;
        let mut candidate = MergeSelectionCandidate {
            explicit_match: request.provider_id.as_ref() == Some(&descriptor.provider_id),
            provider_id: descriptor.provider_id.clone(),
            role: descriptor.role,
            priority: descriptor.priority,
            rejections: vec![],
            parser_report: None,
            selected: false,
        };
        if let Some(id) = &request.provider_id {
            if id != &descriptor.provider_id {
                candidate.rejections.push("explicit_provider_mismatch".into());
            }
        } else if descriptor.role != MergeProviderRole::Workflow {
            candidate.rejections.push("not_workflow_provider".into());
        }
        if request.family != descriptor.family {
            candidate.rejections.push("unsupported_family".into());
        }
        if !descriptor.operations.contains(&request.operation) {
            candidate.rejections.push("unsupported_operation".into());
        }
        if request.dialect.as_ref().is_some_and(|dialect| !descriptor.dialects.contains(dialect)) {
            candidate.rejections.push("unsupported_dialect".into());
        }
        if request.profile.as_ref().is_some_and(|profile| !descriptor.profiles.contains(profile)) {
            candidate.rejections.push("unsupported_profile".into());
        }
        for (required, supplied, reason) in [
            (&request.required_capabilities, &descriptor.capabilities, "missing_capability"),
            (
                &request.required_preservation,
                &descriptor.preservation_guarantees,
                "missing_preservation",
            ),
        ] {
            let mut missing: Vec<_> =
                required.iter().filter(|value| !supplied.contains(value)).collect();
            missing.sort();
            for value in missing {
                candidate.rejections.push(format!("{reason}:{value}"));
            }
        }
        let requirements = &descriptor.parser_requirements;
        if !requirements.languages.is_empty()
            && !requirements.languages.contains(&request.parser.language)
        {
            candidate.rejections.push("provider_parser_language_mismatch".into());
        }
        if !requirements.dialects.is_empty()
            && request
                .parser
                .dialect
                .as_ref()
                .is_none_or(|dialect| !requirements.dialects.contains(dialect))
        {
            candidate.rejections.push("provider_parser_dialect_mismatch".into());
        }
        // TreeHaver currently has language preference lists, not a versioned
        // parser-profile registry. Never silently ignore an unprovable profile.
        if !requirements.profiles.is_empty() {
            candidate.rejections.push("unsupported_parser_profile_requirement".into());
        }
        if candidate.rejections.is_empty() {
            let service = parser_service.clone().with_constraints(ParserConstraints {
                allowed_backend_ids: requirements.allowed_backend_ids.clone(),
                forbidden_backend_ids: requirements.forbidden_backend_ids.clone(),
                allowed_backend_families: requirements.allowed_backend_families.clone(),
                forbidden_backend_families: requirements.forbidden_backend_families.clone(),
                required_contracts: requirements.contracts.clone(),
                required_capabilities: requirements.capabilities.clone(),
            })?;
            let parser_report = service.selection_report(&request.parser, parsers, context)?;
            if parser_report.selected_backend.is_none() {
                candidate.rejections.push("no_eligible_parser".into());
            }
            candidate.parser_report = Some(parser_report);
        }
        report.candidates.push(candidate);
    }
    report.candidates.sort_by(|left, right| {
        right
            .explicit_match
            .cmp(&left.explicit_match)
            .then_with(|| right.priority.cmp(&left.priority))
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
    if let Some(winner) =
        report.candidates.iter_mut().find(|candidate| candidate.rejections.is_empty())
    {
        winner.selected = true;
        report.selected_provider = Some(winner.provider_id.clone());
    } else {
        report.rejections.push("no_eligible_provider".into());
    }
    context.check()?;
    Ok(report)
}

fn validate_request(request: &MergeSelectionRequest) -> Result<(), ServiceError> {
    fn valid(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 256
            && !value.chars().any(|c| c.is_whitespace() || c.is_control())
    }
    if !valid(&request.family)
        || !valid(&request.parser.language)
        || !matches!(request.operation.as_str(), "analyze" | "diff2" | "merge2" | "merge3")
        || [
            &request.provider_id,
            &request.dialect,
            &request.profile,
            &request.parser.dialect,
            &request.parser.selection.backend_id,
        ]
        .iter()
        .any(|value| value.as_ref().is_some_and(|value| !valid(value)))
    {
        return Err(ServiceError::InvalidRequest);
    }
    for values in [
        &request.required_capabilities,
        &request.required_preservation,
        &request.parser.selection.preference,
        &request.parser.selection.required_capabilities,
    ] {
        let mut seen = std::collections::BTreeSet::new();
        if values.len() > 256 || values.iter().any(|value| !valid(value) || !seen.insert(value)) {
            return Err(ServiceError::InvalidRequest);
        }
    }
    if !request.parser.selection.required_capabilities.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(ServiceError::InvalidRequest);
    }
    Ok(())
}
