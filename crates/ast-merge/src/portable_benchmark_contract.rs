use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs, io,
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const PORTABLE_BENCHMARK_SCHEMA_MAJOR: u64 = 1;
const HISTORY_MANIFEST: &str = "diagnostics/slice-1021-reviewed-git-history-corpus/manifest.json";
const ENUM_KEYS: &[&str] = &[
    "kinds",
    "operations",
    "partitions",
    "profiles",
    "outcomes",
    "oracle_classes",
    "equivalence_classes",
    "transformations",
    "false_auto_merge_severities",
    "preservation_requirements",
    "unsupported_policies",
    "budget_exceed_actions",
    "llm_hard_gate_forbidden_profiles",
    "quality_dimensions",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableBenchmarkValidationCategory {
    InvalidJson,
    InvalidStructure,
    InvalidSchema,
    InvalidEnum,
    InvalidReference,
    InvalidDigest,
    InvalidEligibility,
    InvalidSafety,
    ProhibitedLlmGate,
    ProhibitedScalarScore,
}

impl fmt::Display for PortableBenchmarkValidationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidJson => "invalid_json",
            Self::InvalidStructure => "invalid_structure",
            Self::InvalidSchema => "invalid_schema",
            Self::InvalidEnum => "invalid_enum",
            Self::InvalidReference => "invalid_reference",
            Self::InvalidDigest => "invalid_digest",
            Self::InvalidEligibility => "invalid_eligibility",
            Self::InvalidSafety => "invalid_safety",
            Self::ProhibitedLlmGate => "prohibited_llm_gate",
            Self::ProhibitedScalarScore => "prohibited_scalar_score",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableBenchmarkValidationError {
    pub category: PortableBenchmarkValidationCategory,
    pub message: String,
}

impl fmt::Display for PortableBenchmarkValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "portable benchmark contract {}: {}", self.category, self.message)
    }
}

impl std::error::Error for PortableBenchmarkValidationError {}

fn error(
    category: PortableBenchmarkValidationCategory,
    message: impl Into<String>,
) -> PortableBenchmarkValidationError {
    PortableBenchmarkValidationError { category, message: message.into() }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkContract {
    pub schema_version: String,
    pub fixture_kind: String,
    pub contract: PortableBenchmarkEnums,
    pub case_schema: PortableBenchmarkCaseSchema,
    pub run_manifest_schema: PortableBenchmarkRunSchema,
    pub case_result_schema: PortableBenchmarkResultSchema,
    pub aggregate_report_schema: PortableBenchmarkAggregateSchema,
    pub cases: Vec<PortableBenchmarkCase>,
    pub run_manifest: PortableBenchmarkRunManifest,
    pub case_results: Vec<PortableBenchmarkCaseResult>,
    pub aggregate_report: PortableBenchmarkAggregateReport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkEnums {
    pub kinds: Vec<String>,
    pub operations: Vec<String>,
    pub partitions: Vec<String>,
    pub profiles: Vec<String>,
    pub outcomes: Vec<String>,
    pub oracle_classes: Vec<String>,
    pub equivalence_classes: Vec<String>,
    pub transformations: Vec<String>,
    pub false_auto_merge_severities: Vec<String>,
    pub preservation_requirements: Vec<String>,
    pub unsupported_policies: Vec<String>,
    pub budget_exceed_actions: Vec<String>,
    pub llm_hard_gate_forbidden_profiles: Vec<String>,
    pub quality_dimensions: Vec<String>,
    pub scalar_score_allowed: bool,
    pub false_auto_merge_compensable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkCaseSchema {
    pub required: Vec<String>,
    pub operation_required_fields: BTreeMap<String, Vec<String>>,
    pub input_policy: PortableBenchmarkInputPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkInputPolicy {
    pub inline_max_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkRunSchema {
    pub required: Vec<String>,
    pub adapter_identity_required: Vec<String>,
    pub fast_selection_required: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkResultSchema {
    pub required: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkAggregateSchema {
    pub required_strata: Vec<String>,
    pub scalar_score_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkOracle {
    pub class: String,
    pub score_eligible: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkEquivalence {
    pub class: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkTransformation {
    #[serde(rename = "type")]
    pub transformation_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkHistoryReference {
    pub slice: u64,
    pub corpus_id: String,
    pub manifest: String,
    pub case_id: String,
    pub merge_commit: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkCase {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub operation: String,
    pub capabilities: Vec<String>,
    pub partition: String,
    pub oracle: PortableBenchmarkOracle,
    pub acceptable_equivalence: Vec<PortableBenchmarkEquivalence>,
    pub preservation_policy: BTreeMap<String, String>,
    pub false_auto_merge_severity: String,
    #[serde(default)]
    pub history_reference: Option<PortableBenchmarkHistoryReference>,
    #[serde(default)]
    pub parent_case_id: Option<String>,
    #[serde(default)]
    pub transformations: Vec<PortableBenchmarkTransformation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkAdapter {
    pub id: String,
    pub source_sha: String,
    pub artifact_sha256: String,
    pub version: String,
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkSelection {
    pub selected_case_ids: Vec<String>,
    pub excluded_case_ids: Vec<String>,
    pub explanation: PortableBenchmarkSelectionExplanation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkSelectionExplanation {
    pub direct_cases: Vec<PortableBenchmarkDirectCases>,
    pub sentinels: Vec<PortableBenchmarkSentinel>,
    pub neighbor_samples: Vec<PortableBenchmarkNeighborSample>,
    pub budget_exceed: PortableBenchmarkBudgetExceed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkDirectCases {
    pub case_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkSentinel {
    pub case_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkNeighborSample {
    pub population: Vec<String>,
    pub selected_case_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkBudgetExceed {
    pub action: String,
    pub regenerate_explanation: bool,
    pub silent_extension: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkRunManifest {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub profile: String,
    pub candidate_adapter: PortableBenchmarkAdapter,
    pub base_adapter: Option<PortableBenchmarkAdapter>,
    pub selection: PortableBenchmarkSelection,
    pub unsupported_policy: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkSafety {
    #[serde(default)]
    pub false_auto_merge: bool,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub compensable: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkDimensions {
    pub safety: PortableBenchmarkSafety,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkCaseResult {
    pub id: String,
    pub run_id: String,
    pub case_id: String,
    pub adapter_id: String,
    pub outcome: String,
    pub score_eligible: bool,
    pub dimensions: PortableBenchmarkDimensions,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkPairedDelta {
    pub case_id: String,
    pub base_result_id: String,
    pub candidate_result_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkAggregateReport {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub run_id: String,
    pub raw_result_ids: Vec<String>,
    pub outcome_counts: BTreeMap<String, usize>,
    pub dimensions: PortableBenchmarkAggregateDimensions,
    pub stratification: BTreeMap<String, Value>,
    pub paired_deltas: Vec<PortableBenchmarkPairedDelta>,
    pub gates: PortableBenchmarkGates,
    pub scalar_score: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkAggregateDimensions {
    pub effectiveness: PortableBenchmarkEligibleDimension,
    pub safety: PortableBenchmarkEligibleDimension,
    pub preservation: PortableBenchmarkEligibleDimension,
    pub coverage: PortableBenchmarkCoverageDimension,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkEligibleDimension {
    pub eligible: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkCoverageDimension {
    pub unsupported_is_quality_failure: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkGates {
    pub safety: PortableBenchmarkAggregateSafetyGate,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableBenchmarkAggregateSafetyGate {
    pub status: String,
    pub eligible_false_auto_merge_count: usize,
    pub non_compensable: bool,
    pub offsets_allowed: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortableBenchmarkSummaryCounts {
    pub adapters: usize,
    pub cases: usize,
    pub results: usize,
    pub selected_cases: usize,
    pub score_eligible_results: usize,
    pub quality_denominator_excluded_results: usize,
    pub eligible_false_auto_merges: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortableBenchmarkFalseAutoMergeSummary {
    pub id: String,
    pub severity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortableBenchmarkSafetyGateSummary {
    pub status: String,
    pub eligible_false_auto_merge_count: usize,
    pub non_compensable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortableBenchmarkCanonicalSummary {
    pub schema: String,
    pub counts: PortableBenchmarkSummaryCounts,
    pub case_ids_by_operation: BTreeMap<String, Vec<String>>,
    pub case_ids_by_partition: BTreeMap<String, Vec<String>>,
    pub score_eligible_result_ids: Vec<String>,
    pub quality_denominator_excluded_result_ids: Vec<String>,
    pub false_auto_merges: Vec<PortableBenchmarkFalseAutoMergeSummary>,
    pub safety_gate: PortableBenchmarkSafetyGateSummary,
    pub selection_reason_categories: Vec<String>,
    pub contract_digest: String,
}

#[derive(Clone, Debug)]
pub struct PortableBenchmarkConsumer {
    pub document: PortableBenchmarkContract,
    pub raw: Value,
    pub contract_digest: String,
}

impl PortableBenchmarkConsumer {
    pub fn parse(source: &[u8]) -> Result<Self, PortableBenchmarkValidationError> {
        let raw: Value = serde_json::from_slice(source).map_err(|cause| {
            error(
                PortableBenchmarkValidationCategory::InvalidJson,
                format!("invalid benchmark contract JSON: {cause}"),
            )
        })?;
        if !raw.is_object() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "contract must be an object",
            ));
        }
        let document = serde_json::from_value(raw.clone()).map_err(|cause| {
            error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                format!("contract has an invalid field type: {cause}"),
            )
        })?;
        let consumer = Self { document, raw, contract_digest: sha256_hex(source) };
        consumer.validate()?;
        Ok(consumer)
    }

    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, PortableBenchmarkLoadError> {
        let source = fs::read(path).map_err(PortableBenchmarkLoadError::Io)?;
        Self::parse(&source).map_err(PortableBenchmarkLoadError::Validation)
    }

    pub fn validate(&self) -> Result<(), PortableBenchmarkValidationError> {
        self.validate_schemas()?;
        self.validate_enums()?;
        self.validate_cases()?;
        self.validate_run()?;
        self.validate_results()?;
        self.validate_report()
    }

    fn validate_schemas(&self) -> Result<(), PortableBenchmarkValidationError> {
        let mut schemas = vec![self.document.schema_version.as_str()];
        schemas.extend(self.document.cases.iter().map(|case| case.schema_version.as_str()));
        schemas.push(&self.document.run_manifest.schema_version);
        schemas.push(&self.document.aggregate_report.schema_version);
        for (index, schema) in schemas.into_iter().enumerate() {
            let Some(major) = schema_major(schema) else {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidSchema,
                    format!("schema version {index} is invalid: {schema:?}"),
                ));
            };
            if major != PORTABLE_BENCHMARK_SCHEMA_MAJOR {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidSchema,
                    format!("unsupported benchmark schema major v{major}"),
                ));
            }
        }
        Ok(())
    }

    fn enum_values(&self, key: &str) -> &[String] {
        let enums = &self.document.contract;
        match key {
            "kinds" => &enums.kinds,
            "operations" => &enums.operations,
            "partitions" => &enums.partitions,
            "profiles" => &enums.profiles,
            "outcomes" => &enums.outcomes,
            "oracle_classes" => &enums.oracle_classes,
            "equivalence_classes" => &enums.equivalence_classes,
            "transformations" => &enums.transformations,
            "false_auto_merge_severities" => &enums.false_auto_merge_severities,
            "preservation_requirements" => &enums.preservation_requirements,
            "unsupported_policies" => &enums.unsupported_policies,
            "budget_exceed_actions" => &enums.budget_exceed_actions,
            "llm_hard_gate_forbidden_profiles" => &enums.llm_hard_gate_forbidden_profiles,
            "quality_dimensions" => &enums.quality_dimensions,
            _ => &[],
        }
    }

    fn membership(
        &self,
        value: &str,
        key: &str,
        label: &str,
    ) -> Result<(), PortableBenchmarkValidationError> {
        if self.enum_values(key).iter().any(|item| item == value) {
            Ok(())
        } else {
            Err(error(
                PortableBenchmarkValidationCategory::InvalidEnum,
                format!("{label} is not a declared {key} member: {value:?}"),
            ))
        }
    }

    fn validate_enums(&self) -> Result<(), PortableBenchmarkValidationError> {
        for key in ENUM_KEYS {
            let values = self.enum_values(key);
            if values.iter().any(String::is_empty) {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidEnum,
                    format!("{key} must contain only non-empty strings"),
                ));
            }
            if has_duplicates(values.iter().map(String::as_str)) {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidEnum,
                    format!("{key} contains duplicate values"),
                ));
            }
        }
        if self.document.contract.scalar_score_allowed {
            return Err(error(
                PortableBenchmarkValidationCategory::ProhibitedScalarScore,
                "scalar scores must be prohibited",
            ));
        }
        if self.document.contract.false_auto_merge_compensable {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidSafety,
                "false auto-merges must be non-compensable",
            ));
        }
        Ok(())
    }

    fn validate_cases(&self) -> Result<(), PortableBenchmarkValidationError> {
        unique_ids(self.document.cases.iter().map(|case| case.id.as_str()), "case")?;
        let raw_cases = array_field(&self.raw, "cases", "contract")?;
        if raw_cases.len() != self.document.cases.len() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "cases must be an array",
            ));
        }
        for (case, raw) in self.document.cases.iter().zip(raw_cases) {
            let label = format!("case {}", case.id);
            required_fields(raw, &self.document.case_schema.required, &label)?;
            self.membership(&case.kind, "kinds", &format!("{label} kind"))?;
            if case.kind != "benchmark_case" {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    format!("{label} kind must be benchmark_case"),
                ));
            }
            self.membership(&case.operation, "operations", &format!("{label} operation"))?;
            self.membership(&case.partition, "partitions", &format!("{label} partition"))?;
            self.membership(&case.oracle.class, "oracle_classes", &format!("{label} oracle"))?;
            self.membership(
                &case.false_auto_merge_severity,
                "false_auto_merge_severities",
                &format!("{label} severity"),
            )?;
            if !case.capabilities.is_sorted()
                || has_duplicates(case.capabilities.iter().map(String::as_str))
            {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    format!("{label} capabilities must be unique and sorted"),
                ));
            }
            for equivalence in &case.acceptable_equivalence {
                self.membership(
                    &equivalence.class,
                    "equivalence_classes",
                    &format!("{label} equivalence"),
                )?;
            }
            for requirement in case.preservation_policy.values() {
                self.membership(
                    requirement,
                    "preservation_requirements",
                    &format!("{label} preservation"),
                )?;
            }
            let fields = self
                .document
                .case_schema
                .operation_required_fields
                .get(&case.operation)
                .ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    format!("{label} operation requirements are missing"),
                )
            })?;
            required_fields(raw, fields, &format!("{label} {}", case.operation))?;
            validate_inline_inputs(
                &Value::Object(raw.clone()),
                &label,
                self.document.case_schema.input_policy.inline_max_bytes,
            )?;
            if case.operation == "history_replay" {
                let history = case.history_reference.as_ref().ok_or_else(|| {
                    error(
                        PortableBenchmarkValidationCategory::InvalidStructure,
                        format!("{label} history reference must be an object"),
                    )
                })?;
                if history.slice != 1021 {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidReference,
                        format!("{label} history reference must target Slice 1021"),
                    ));
                }
                if history.manifest != HISTORY_MANIFEST {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidReference,
                        format!("{label} history manifest must target Slice 1021"),
                    ));
                }
                if raw.get("inputs").is_some() {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidStructure,
                        format!("{label} history reference must not inline source bytes"),
                    ));
                }
            }
            if case.operation == "metamorphic" {
                for transformation in &case.transformations {
                    self.membership(
                        &transformation.transformation_type,
                        "transformations",
                        &format!("{label} transformation"),
                    )?;
                }
            }
        }
        let ids: HashSet<_> = self.document.cases.iter().map(|case| case.id.as_str()).collect();
        for case in &self.document.cases {
            if case.operation == "metamorphic"
                && !case.parent_case_id.as_deref().is_some_and(|id| ids.contains(id))
            {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("case {} parent_case_id is dangling", case.id),
                ));
            }
        }
        Ok(())
    }

    fn adapters(&self) -> impl Iterator<Item = &PortableBenchmarkAdapter> {
        std::iter::once(&self.document.run_manifest.candidate_adapter)
            .chain(self.document.run_manifest.base_adapter.iter())
    }

    fn validate_run(&self) -> Result<(), PortableBenchmarkValidationError> {
        let run = &self.document.run_manifest;
        let raw_run = object_field(&self.raw, "run_manifest", "contract")?;
        required_fields(raw_run, &self.document.run_manifest_schema.required, "run manifest")?;
        if run.kind != "run_manifest" {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "run manifest kind must be run_manifest",
            ));
        }
        self.membership(&run.profile, "profiles", "run profile")?;
        self.membership(&run.unsupported_policy, "unsupported_policies", "unsupported policy")?;
        let adapters: Vec<_> = self.adapters().collect();
        unique_ids(adapters.iter().map(|adapter| adapter.id.as_str()), "adapter")?;
        for adapter in adapters {
            for field in &self.document.run_manifest_schema.adapter_identity_required {
                let value = match field.as_str() {
                    "source_sha" => &adapter.source_sha,
                    "artifact_sha256" => &adapter.artifact_sha256,
                    "version" => &adapter.version,
                    "configuration_sha256" => &adapter.configuration_sha256,
                    _ => "",
                };
                if value.is_empty() {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidStructure,
                        format!("adapter {} is missing required field {field}", adapter.id),
                    ));
                }
            }
        }
        let selected = &run.selection.selected_case_ids;
        let excluded = &run.selection.excluded_case_ids;
        if has_duplicates(selected.iter().map(String::as_str)) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "selected case IDs contain duplicates",
            ));
        }
        if has_duplicates(excluded.iter().map(String::as_str)) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "excluded case IDs contain duplicates",
            ));
        }
        if selected.iter().any(|id| excluded.contains(id)) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "selected and excluded case IDs overlap",
            ));
        }
        let case_ids: HashSet<_> =
            self.document.cases.iter().map(|case| case.id.as_str()).collect();
        for id in selected.iter().chain(excluded) {
            reference(id, &case_ids, "run selection case_id")?;
        }
        let explanation = &run.selection.explanation;
        let raw_explanation = raw_run
            .get("selection")
            .and_then(Value::as_object)
            .and_then(|value| value.get("explanation"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    "run selection.explanation must be an object",
                )
            })?;
        required_fields(
            raw_explanation,
            &self.document.run_manifest_schema.fast_selection_required,
            "selection explanation",
        )?;
        for direct in &explanation.direct_cases {
            for id in &direct.case_ids {
                reference(id, &case_ids, "direct selection case_id")?;
            }
        }
        for sentinel in &explanation.sentinels {
            reference(&sentinel.case_id, &case_ids, "sentinel selection case_id")?;
        }
        for sample in &explanation.neighbor_samples {
            for id in sample.population.iter().chain(&sample.selected_case_ids) {
                reference(id, &case_ids, "neighbor selection case_id")?;
            }
        }
        self.membership(
            &explanation.budget_exceed.action,
            "budget_exceed_actions",
            "budget exceed action",
        )?;
        if !explanation.budget_exceed.regenerate_explanation {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "budget exceed must regenerate the explanation",
            ));
        }
        if explanation.budget_exceed.silent_extension {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "silent budget extension is prohibited",
            ));
        }
        if (matches!(run.profile.as_str(), "micro" | "dev")
            || self.document.contract.llm_hard_gate_forbidden_profiles.contains(&run.profile))
            && self.document.cases.iter().any(|case| {
                selected.contains(&case.id)
                    && case.oracle.class == "llm"
                    && case.oracle.score_eligible
            })
        {
            return Err(error(
                PortableBenchmarkValidationCategory::ProhibitedLlmGate,
                format!("LLM oracle cannot participate in {} hard gates", run.profile),
            ));
        }
        Ok(())
    }

    fn validate_results(&self) -> Result<(), PortableBenchmarkValidationError> {
        unique_ids(self.document.case_results.iter().map(|result| result.id.as_str()), "result")?;
        let raw_results = array_field(&self.raw, "case_results", "contract")?;
        if raw_results.len() != self.document.case_results.len() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "case_results must be an array",
            ));
        }
        let cases: HashMap<_, _> =
            self.document.cases.iter().map(|case| (case.id.as_str(), case)).collect();
        let adapter_ids: HashSet<_> = self.adapters().map(|adapter| adapter.id.as_str()).collect();
        for (result, raw) in self.document.case_results.iter().zip(raw_results) {
            let label = format!("result {}", result.id);
            required_fields(raw, &self.document.case_result_schema.required, &label)?;
            if result.run_id != self.document.run_manifest.id {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("{label} run_id is dangling: {:?}", result.run_id),
                ));
            }
            let case = cases.get(result.case_id.as_str()).ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("{label} case_id is dangling: {:?}", result.case_id),
                )
            })?;
            reference(&result.adapter_id, &adapter_ids, &format!("{label} adapter_id"))?;
            self.membership(&result.outcome, "outcomes", &format!("{label} outcome"))?;
            let expected = case.oracle.score_eligible && !quality_excluded(&result.outcome);
            if result.score_eligible != expected {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidEligibility,
                    format!("{label} score eligibility conflicts with case/outcome"),
                ));
            }
            validate_raw_inline_records(raw, &label)?;
            let safety = &result.dimensions.safety;
            if result.outcome == "false_auto_merge" {
                if !safety.false_auto_merge {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidSafety,
                        format!("{label} must identify a false auto-merge"),
                    ));
                }
                if safety.compensable != Some(false) {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidSafety,
                        format!("{label} false auto-merge cannot be compensable"),
                    ));
                }
                self.membership(
                    &safety.severity,
                    "false_auto_merge_severities",
                    &format!("{label} severity"),
                )?;
            } else if safety.false_auto_merge {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidSafety,
                    format!("{label} safety classification conflicts with outcome"),
                ));
            }
        }
        Ok(())
    }

    fn validate_report(&self) -> Result<(), PortableBenchmarkValidationError> {
        let report = &self.document.aggregate_report;
        if report.kind != "aggregate_report" {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "aggregate report kind must be aggregate_report",
            ));
        }
        if report.run_id != self.document.run_manifest.id {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "aggregate report run_id is dangling",
            ));
        }
        let result_ids: Vec<_> =
            self.document.case_results.iter().map(|result| result.id.as_str()).collect();
        if has_duplicates(report.raw_result_ids.iter().map(String::as_str)) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "aggregate raw result IDs contain duplicates",
            ));
        }
        let report_ids: HashSet<_> = report.raw_result_ids.iter().map(String::as_str).collect();
        if report_ids != result_ids.iter().copied().collect() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidReference,
                "aggregate raw result IDs must exactly cover results",
            ));
        }
        for stratum in &self.document.aggregate_report_schema.required_strata {
            if !report.stratification.contains_key(stratum) {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    format!("report stratification is missing required field {stratum}"),
                ));
            }
        }
        let mut counts: BTreeMap<_, _> =
            self.document.contract.outcomes.iter().map(|outcome| (outcome.clone(), 0)).collect();
        for result in &self.document.case_results {
            *counts.entry(result.outcome.clone()).or_default() += 1;
        }
        if report.outcome_counts != counts {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                "aggregate outcome counts do not match raw results",
            ));
        }
        let eligible =
            self.document.case_results.iter().filter(|result| result.score_eligible).count();
        for (name, count) in [
            ("effectiveness", report.dimensions.effectiveness.eligible),
            ("safety", report.dimensions.safety.eligible),
            ("preservation", report.dimensions.preservation.eligible),
        ] {
            if count != eligible {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidEligibility,
                    format!("aggregate {name} denominator is incorrect"),
                ));
            }
        }
        if self.document.run_manifest.unsupported_policy == "coverage_only"
            && report.dimensions.coverage.unsupported_is_quality_failure
        {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidEligibility,
                "coverage-only unsupported results cannot be quality failures",
            ));
        }
        let by_id: HashMap<_, _> =
            self.document.case_results.iter().map(|result| (result.id.as_str(), result)).collect();
        for pair in &report.paired_deltas {
            let base = by_id.get(pair.base_result_id.as_str()).ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("paired delta {} has dangling base result", pair.case_id),
                )
            })?;
            let candidate = by_id.get(pair.candidate_result_id.as_str()).ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("paired delta {} has dangling candidate result", pair.case_id),
                )
            })?;
            if base.case_id != pair.case_id || candidate.case_id != pair.case_id {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("paired delta {} mixes cases", pair.case_id),
                ));
            }
            if self
                .document
                .run_manifest
                .base_adapter
                .as_ref()
                .is_none_or(|adapter| base.adapter_id != adapter.id)
            {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("paired delta {} base adapter is incorrect", pair.case_id),
                ));
            }
            if candidate.adapter_id != self.document.run_manifest.candidate_adapter.id {
                return Err(error(
                    PortableBenchmarkValidationCategory::InvalidReference,
                    format!("paired delta {} candidate adapter is incorrect", pair.case_id),
                ));
            }
        }
        if self.document.aggregate_report_schema.scalar_score_allowed {
            return Err(error(
                PortableBenchmarkValidationCategory::ProhibitedScalarScore,
                "report schema must prohibit scalar scores",
            ));
        }
        if !report.scalar_score.is_null() {
            return Err(error(
                PortableBenchmarkValidationCategory::ProhibitedScalarScore,
                "aggregate scalar score is prohibited",
            ));
        }
        let false_auto_merges = self.eligible_false_auto_merges();
        let expected_status = if false_auto_merges.is_empty() { "pass" } else { "fail" };
        let gate = &report.gates.safety;
        if gate.status != expected_status {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidSafety,
                "safety gate status does not reflect eligible false auto-merges",
            ));
        }
        if gate.eligible_false_auto_merge_count != false_auto_merges.len() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidSafety,
                "safety gate false-auto-merge count is incorrect",
            ));
        }
        if !gate.non_compensable {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidSafety,
                "safety gate must be non-compensable",
            ));
        }
        if !gate.offsets_allowed.is_empty() {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidSafety,
                "safety gate must prohibit offsets",
            ));
        }
        Ok(())
    }

    pub fn canonical_summary(
        &self,
    ) -> Result<PortableBenchmarkCanonicalSummary, PortableBenchmarkValidationError> {
        self.validate()?;
        let mut eligible_ids: Vec<_> = self
            .document
            .case_results
            .iter()
            .filter(|result| result.score_eligible)
            .map(|result| result.id.clone())
            .collect();
        let mut excluded_ids: Vec<_> = self
            .document
            .case_results
            .iter()
            .filter(|result| quality_excluded(&result.outcome))
            .map(|result| result.id.clone())
            .collect();
        eligible_ids.sort();
        excluded_ids.sort();
        let false_auto_merges = self.eligible_false_auto_merges();
        let explanation = self
            .raw
            .pointer("/run_manifest/selection/explanation")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    "run selection.explanation must be an object",
                )
            })?;
        let mut selection_reason_categories: Vec<_> = explanation.keys().cloned().collect();
        selection_reason_categories.sort();
        Ok(PortableBenchmarkCanonicalSummary {
            schema: self.document.schema_version.clone(),
            counts: PortableBenchmarkSummaryCounts {
                adapters: self.adapters().count(),
                cases: self.document.cases.len(),
                results: self.document.case_results.len(),
                selected_cases: self.document.run_manifest.selection.selected_case_ids.len(),
                score_eligible_results: eligible_ids.len(),
                quality_denominator_excluded_results: excluded_ids.len(),
                eligible_false_auto_merges: false_auto_merges.len(),
            },
            case_ids_by_operation: self
                .grouped_cases(&self.document.contract.operations, |case| &case.operation),
            case_ids_by_partition: self
                .grouped_cases(&self.document.contract.partitions, |case| &case.partition),
            score_eligible_result_ids: eligible_ids,
            quality_denominator_excluded_result_ids: excluded_ids,
            safety_gate: PortableBenchmarkSafetyGateSummary {
                status: if false_auto_merges.is_empty() { "pass".into() } else { "fail".into() },
                eligible_false_auto_merge_count: false_auto_merges.len(),
                non_compensable: true,
            },
            false_auto_merges,
            selection_reason_categories,
            contract_digest: self.contract_digest.clone(),
        })
    }

    fn grouped_cases(
        &self,
        values: &[String],
        field: impl Fn(&PortableBenchmarkCase) -> &String,
    ) -> BTreeMap<String, Vec<String>> {
        values
            .iter()
            .map(|value| {
                let mut ids: Vec<_> = self
                    .document
                    .cases
                    .iter()
                    .filter(|case| field(case) == value)
                    .map(|case| case.id.clone())
                    .collect();
                ids.sort();
                (value.clone(), ids)
            })
            .collect()
    }

    fn eligible_false_auto_merges(&self) -> Vec<PortableBenchmarkFalseAutoMergeSummary> {
        let mut result: Vec<_> = self
            .document
            .case_results
            .iter()
            .filter(|result| result.score_eligible && result.outcome == "false_auto_merge")
            .map(|result| PortableBenchmarkFalseAutoMergeSummary {
                id: result.id.clone(),
                severity: result.dimensions.safety.severity.clone(),
            })
            .collect();
        result.sort_by(|left, right| left.id.cmp(&right.id));
        result
    }
}

#[derive(Debug)]
pub enum PortableBenchmarkLoadError {
    Io(io::Error),
    Validation(PortableBenchmarkValidationError),
}

impl fmt::Display for PortableBenchmarkLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(cause) => write!(f, "failed to read portable benchmark contract: {cause}"),
            Self::Validation(cause) => cause.fmt(f),
        }
    }
}

impl std::error::Error for PortableBenchmarkLoadError {}

fn schema_major(schema: &str) -> Option<u64> {
    let prefix = "structuredmerge.benchmark";
    let rest = schema.strip_prefix(prefix)?;
    let version = if let Some(version) = rest.strip_prefix("/v") {
        version
    } else {
        let (suffix, version) = rest.split_once("/v")?;
        if suffix.len() < 2
            || !suffix.starts_with('.')
            || !suffix[1..].bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        {
            return None;
        }
        version
    };
    let (major, minor) =
        version.split_once('.').map_or((version, None), |(major, minor)| (major, Some(minor)));
    if major.is_empty()
        || !major.bytes().all(|byte| byte.is_ascii_digit())
        || minor.is_some_and(|minor| {
            minor.is_empty() || !minor.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return None;
    }
    major.parse().ok()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn exact_digest(
    digest: &str,
    content: &[u8],
    label: &str,
) -> Result<(), PortableBenchmarkValidationError> {
    if digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(error(
            PortableBenchmarkValidationCategory::InvalidDigest,
            format!("{label} SHA-256 is malformed"),
        ));
    }
    if digest != sha256_hex(content) {
        return Err(error(
            PortableBenchmarkValidationCategory::InvalidDigest,
            format!("{label} SHA-256 does not match exact bytes"),
        ));
    }
    Ok(())
}

fn validate_inline_inputs(
    value: &Value,
    label: &str,
    limit: usize,
) -> Result<(), PortableBenchmarkValidationError> {
    match value {
        Value::Object(object) => {
            if object.get("mode").and_then(Value::as_str) == Some("inline") {
                let bytes = object.get("bytes").and_then(Value::as_str).ok_or_else(|| {
                    error(
                        PortableBenchmarkValidationCategory::InvalidStructure,
                        format!("{label} inline bytes must be a string"),
                    )
                })?;
                if bytes.len() > limit {
                    return Err(error(
                        PortableBenchmarkValidationCategory::InvalidStructure,
                        format!("{label} inline bytes exceed {limit}"),
                    ));
                }
                let digest = object.get("sha256").and_then(Value::as_str).unwrap_or("");
                exact_digest(digest, bytes.as_bytes(), &format!("{label} inline input"))?;
            }
            for child in object.values() {
                validate_inline_inputs(child, label, limit)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                validate_inline_inputs(child, label, limit)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_raw_inline_records(
    result: &Map<String, Value>,
    label: &str,
) -> Result<(), PortableBenchmarkValidationError> {
    let raw = result.get("raw").and_then(Value::as_object).ok_or_else(|| {
        error(
            PortableBenchmarkValidationCategory::InvalidStructure,
            format!("{label}.raw must be an object"),
        )
    })?;
    for (name, value) in raw {
        let Some(record) = value.as_object() else {
            continue;
        };
        let Some(inline_value) = record.get("inline") else {
            continue;
        };
        let inline = inline_value.as_str().ok_or_else(|| {
            error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                format!("{label} raw {name} inline content must be a string"),
            )
        })?;
        if record.get("bytes").and_then(Value::as_u64) != Some(inline.len() as u64) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidDigest,
                format!("{label} raw {name} byte length is not exact"),
            ));
        }
        exact_digest(
            record.get("sha256").and_then(Value::as_str).unwrap_or(""),
            inline.as_bytes(),
            &format!("{label} raw {name}"),
        )?;
    }
    Ok(())
}

fn required_fields(
    object: &Map<String, Value>,
    fields: &[String],
    label: &str,
) -> Result<(), PortableBenchmarkValidationError> {
    for field in fields {
        let mut parts = field.split('.');
        let mut value = parts.next().and_then(|part| object.get(part));
        for part in parts {
            value = value.and_then(|current| current.get(part));
        }
        if value.is_none_or(Value::is_null) {
            return Err(error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                format!("{label} is missing required field {field}"),
            ));
        }
    }
    Ok(())
}

fn object_field<'a>(
    object: &'a Value,
    key: &str,
    label: &str,
) -> Result<&'a Map<String, Value>, PortableBenchmarkValidationError> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        error(
            PortableBenchmarkValidationCategory::InvalidStructure,
            format!("{label}.{key} must be an object"),
        )
    })
}

fn array_field<'a>(
    object: &'a Value,
    key: &str,
    label: &str,
) -> Result<Vec<&'a Map<String, Value>>, PortableBenchmarkValidationError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            error(
                PortableBenchmarkValidationCategory::InvalidStructure,
                format!("{label}.{key} must be an array"),
            )
        })?
        .iter()
        .map(|value| {
            value.as_object().ok_or_else(|| {
                error(
                    PortableBenchmarkValidationCategory::InvalidStructure,
                    format!("{label}.{key} entries must be objects"),
                )
            })
        })
        .collect()
}

fn unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<(), PortableBenchmarkValidationError> {
    let ids: Vec<_> = ids.collect();
    if ids.iter().any(|id| id.is_empty()) {
        return Err(error(
            PortableBenchmarkValidationCategory::InvalidReference,
            format!("{label} IDs must be non-empty strings"),
        ));
    }
    if has_duplicates(ids.iter().copied()) {
        return Err(error(
            PortableBenchmarkValidationCategory::InvalidReference,
            format!("duplicate {label} ID"),
        ));
    }
    Ok(())
}

fn has_duplicates<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.any(|value| !seen.insert(value))
}

fn reference<T: AsRef<str>>(
    value: &str,
    allowed: &HashSet<T>,
    label: &str,
) -> Result<(), PortableBenchmarkValidationError> {
    if allowed.iter().any(|item| item.as_ref() == value) {
        Ok(())
    } else {
        Err(error(
            PortableBenchmarkValidationCategory::InvalidReference,
            format!("{label} is dangling: {value:?}"),
        ))
    }
}

fn quality_excluded(outcome: &str) -> bool {
    matches!(outcome, "unsupported" | "excluded_ambiguous")
}
