use serde_json::{Value, json};

pub const PACKAGE_NAME: &str = "ast-crispr";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    pub code: String,
    pub message: String,
}

type Predicate = fn(usize, usize) -> bool;

#[derive(Clone)]
struct LimitConstraint {
    description: String,
    value: usize,
    predicate: Predicate,
}

#[derive(Clone)]
pub struct Limit {
    constraints: Vec<LimitConstraint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchProfile {
    pub start_boundary: String,
    pub end_boundary: String,
    pub payload_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionProfile {
    pub owner_scope: String,
    pub owner_selector: String,
    pub selector_kind: String,
    pub selection_intent: String,
    pub comment_region: Option<String>,
    pub include_trailing_gap: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestinationProfile {
    pub resolution_kind: String,
    pub resolution_source: String,
    pub anchor_boundary: String,
    pub used_if_missing: bool,
}

struct ProfileDescriptor {
    family: &'static str,
}

impl Limit {
    pub fn new(spec: Option<&Value>) -> Result<Self, Error> {
        let default_spec = json!({"exactly": 1});
        let active_spec = match spec {
            Some(Value::Null) | None => &default_spec,
            Some(value) => value,
        };
        let constraints = normalize_limit(active_spec)?;
        Ok(Self { constraints })
    }

    pub fn allows(&self, count: usize) -> bool {
        self.constraints.iter().all(|constraint| (constraint.predicate)(count, constraint.value))
    }

    pub fn describe(&self) -> String {
        self.constraints
            .iter()
            .map(|constraint| constraint.description.as_str())
            .collect::<Vec<_>>()
            .join(" and ")
    }
}

impl MatchProfile {
    pub fn new(start_boundary: &str, end_boundary: &str, payload_kind: &str) -> Self {
        Self {
            start_boundary: start_boundary.to_string(),
            end_boundary: end_boundary.to_string(),
            payload_kind: payload_kind.to_string(),
        }
    }

    pub fn report(&self) -> Value {
        let (start_family, known_start_boundary) =
            descriptor_family(start_boundary_descriptor(&self.start_boundary));
        let (end_family, known_end_boundary) =
            descriptor_family(end_boundary_descriptor(&self.end_boundary));
        let (payload_family, known_payload_kind) =
            descriptor_family(payload_kind_descriptor(&self.payload_kind));
        json!({
            "start_boundary": self.start_boundary,
            "start_boundary_family": start_family,
            "known_start_boundary": known_start_boundary,
            "end_boundary": self.end_boundary,
            "end_boundary_family": end_family,
            "known_end_boundary": known_end_boundary,
            "payload_kind": self.payload_kind,
            "payload_family": payload_family,
            "known_payload_kind": known_payload_kind,
            "comment_anchored": start_family == "comment_anchor" || payload_family == "comment_owned",
            "trailing_gap_extended": end_family == "gap_extension"
        })
    }
}

impl SelectionProfile {
    pub fn new(
        owner_scope: &str,
        owner_selector: &str,
        selector_kind: &str,
        selection_intent: &str,
        comment_region: Option<&str>,
        include_trailing_gap: bool,
    ) -> Self {
        Self {
            owner_scope: defaulted(owner_scope, "shared_default"),
            owner_selector: defaulted(owner_selector, "line_bound_statements"),
            selector_kind: defaulted(selector_kind, "owner_filter"),
            selection_intent: defaulted(selection_intent, "predicate_filter"),
            comment_region: comment_region.map(str::to_string),
            include_trailing_gap,
        }
    }

    pub fn report(&self) -> Value {
        let (owner_selector_family, known_owner_selector) =
            descriptor_family(owner_selector_descriptor(&self.owner_selector));
        let (selector_kind_family, known_selector_kind) =
            descriptor_family(selector_kind_descriptor(&self.selector_kind));
        let (selection_intent_family, known_selection_intent) =
            descriptor_family(selection_intent_descriptor(&self.selection_intent));
        let (comment_region_family, known_comment_region) = match self.comment_region.as_deref() {
            Some(comment_region) => descriptor_family(comment_region_descriptor(comment_region)),
            None => ("none", false),
        };
        json!({
            "owner_scope": self.owner_scope,
            "owner_selector": self.owner_selector,
            "owner_selector_family": owner_selector_family,
            "known_owner_selector": known_owner_selector,
            "selector_kind": self.selector_kind,
            "selector_kind_family": selector_kind_family,
            "known_selector_kind": known_selector_kind,
            "selection_intent": self.selection_intent,
            "selection_intent_family": selection_intent_family,
            "known_selection_intent": known_selection_intent,
            "comment_region": self.comment_region,
            "comment_region_family": comment_region_family,
            "known_comment_region": known_comment_region,
            "comment_anchored": selector_kind_family == "comment_anchor" || selection_intent_family == "comment" || known_comment_region,
            "include_trailing_gap": self.include_trailing_gap
        })
    }
}

impl DestinationProfile {
    pub fn new(
        resolution_kind: &str,
        resolution_source: &str,
        anchor_boundary: &str,
        used_if_missing: bool,
    ) -> Self {
        Self {
            resolution_kind: defaulted(resolution_kind, "append_fallback"),
            resolution_source: defaulted(resolution_source, "none"),
            anchor_boundary: defaulted(anchor_boundary, "none"),
            used_if_missing,
        }
    }

    pub fn report(&self) -> Value {
        let (resolution_family, known_resolution_kind) =
            descriptor_family(resolution_kind_descriptor(&self.resolution_kind));
        let (resolution_source_family, known_resolution_source) =
            descriptor_family(resolution_source_descriptor(&self.resolution_source));
        let (anchor_boundary_family, known_anchor_boundary) =
            descriptor_family(anchor_boundary_descriptor(&self.anchor_boundary));
        json!({
            "resolution_kind": self.resolution_kind,
            "resolution_family": resolution_family,
            "known_resolution_kind": known_resolution_kind,
            "resolution_source": self.resolution_source,
            "resolution_source_family": resolution_source_family,
            "known_resolution_source": known_resolution_source,
            "anchor_boundary": self.anchor_boundary,
            "anchor_boundary_family": anchor_boundary_family,
            "known_anchor_boundary": known_anchor_boundary,
            "used_if_missing": self.used_if_missing,
            "append_fallback": self.resolution_kind == "append_fallback",
            "anchored": resolution_family == "anchored"
        })
    }
}

fn defaulted(value: &str, fallback: &str) -> String {
    if value.is_empty() { fallback } else { value }.to_string()
}

fn descriptor_family(descriptor: Option<ProfileDescriptor>) -> (&'static str, bool) {
    match descriptor {
        Some(descriptor) => (descriptor.family, true),
        None => ("unknown", false),
    }
}

fn start_boundary_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "owner_start" => Some(ProfileDescriptor { family: "structural_owner" }),
        "comment_region_start" => Some(ProfileDescriptor { family: "comment_anchor" }),
        _ => None,
    }
}

fn end_boundary_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "owner_end" => Some(ProfileDescriptor { family: "structural_owner" }),
        "owner_end_plus_trailing_gap" => Some(ProfileDescriptor { family: "gap_extension" }),
        _ => None,
    }
}

fn payload_kind_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "structural_owner_body" => Some(ProfileDescriptor { family: "owner_body" }),
        "comment_owned_body" => Some(ProfileDescriptor { family: "comment_owned" }),
        "section_branch" => Some(ProfileDescriptor { family: "section_branch" }),
        _ => None,
    }
}

fn owner_selector_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "line_bound_statements" => Some(ProfileDescriptor { family: "line_oriented" }),
        "heading_sections" => Some(ProfileDescriptor { family: "section" }),
        _ => None,
    }
}

fn selector_kind_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "owner_filter" => Some(ProfileDescriptor { family: "owner_filter" }),
        "comment_region_owner" => Some(ProfileDescriptor { family: "comment_anchor" }),
        "heading_section" => Some(ProfileDescriptor { family: "section_branch" }),
        _ => None,
    }
}

fn selection_intent_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "predicate_filter" => Some(ProfileDescriptor { family: "predicate" }),
        "comment_region_filter" => Some(ProfileDescriptor { family: "comment" }),
        "section_heading" => Some(ProfileDescriptor { family: "section" }),
        _ => None,
    }
}

fn comment_region_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "leading" => Some(ProfileDescriptor { family: "leading" }),
        "trailing" => Some(ProfileDescriptor { family: "trailing" }),
        "inline" => Some(ProfileDescriptor { family: "inline" }),
        _ => None,
    }
}

fn resolution_kind_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "append_fallback" => Some(ProfileDescriptor { family: "append" }),
        "anchor_after_statement" => Some(ProfileDescriptor { family: "anchored" }),
        _ => None,
    }
}

fn resolution_source_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "none" => Some(ProfileDescriptor { family: "implicit" }),
        "callable" => Some(ProfileDescriptor { family: "callable" }),
        "selector" => Some(ProfileDescriptor { family: "selector" }),
        _ => None,
    }
}

fn anchor_boundary_descriptor(value: &str) -> Option<ProfileDescriptor> {
    match value {
        "none" => Some(ProfileDescriptor { family: "none" }),
        "statement_end_plus_following_gap" => {
            Some(ProfileDescriptor { family: "gap_preserving_statement" })
        }
        _ => None,
    }
}

fn normalize_limit(spec: &Value) -> Result<Vec<LimitConstraint>, Error> {
    match spec {
        Value::Object(map) => normalize_limit_map(spec, map),
        Value::Array(entries) => {
            let mut constraints = Vec::new();
            for entry in entries {
                constraints.extend(normalize_limit(entry)?);
            }
            Ok(constraints)
        }
        Value::String(expression) => Ok(vec![constraint_for_operator(expression)?]),
        _ => Err(Error {
            code: "ast_crispr_limit_unsupported".to_string(),
            message: "Unsupported ast-crispr limit specification".to_string(),
        }),
    }
}

fn normalize_limit_map(
    spec: &Value,
    map: &serde_json::Map<String, Value>,
) -> Result<Vec<LimitConstraint>, Error> {
    let mut constraints = Vec::new();
    if let Some(value) = map.get("exactly").and_then(Value::as_u64) {
        constraints.push(LimitConstraint {
            description: format!("== {value}"),
            value: value as usize,
            predicate: |count, value| count == value,
        });
    }
    if let Some(value) = map.get("at_most").and_then(Value::as_u64) {
        constraints.push(LimitConstraint {
            description: format!("<= {value}"),
            value: value as usize,
            predicate: |count, value| count <= value,
        });
    }
    if let Some(value) = map.get("at_least").and_then(Value::as_u64) {
        constraints.push(LimitConstraint {
            description: format!(">= {value}"),
            value: value as usize,
            predicate: |count, value| count >= value,
        });
    }
    if map.get("none_or_one").and_then(Value::as_bool).unwrap_or(false) {
        constraints.push(LimitConstraint {
            description: "<= 1".to_string(),
            value: 1,
            predicate: |count, value| count <= value,
        });
    }
    if constraints.is_empty() {
        return Err(Error {
            code: "ast_crispr_limit_empty".to_string(),
            message: format!("ast-crispr limit must define at least one constraint: {spec}"),
        });
    }
    Ok(constraints)
}

fn constraint_for_operator(expression: &str) -> Result<LimitConstraint, Error> {
    let trimmed = expression.trim();
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some(rest) = trimmed.strip_prefix(operator) {
            let value = rest.trim().parse::<usize>().map_err(|_| Error {
                code: "ast_crispr_limit_invalid_expression".to_string(),
                message: "Invalid ast-crispr limit expression".to_string(),
            })?;
            let predicate: Predicate = match operator {
                "==" => |count, value| count == value,
                "!=" => |count, value| count != value,
                "<=" => |count, value| count <= value,
                ">=" => |count, value| count >= value,
                "<" => |count, value| count < value,
                ">" => |count, value| count > value,
                _ => unreachable!(),
            };
            return Ok(LimitConstraint {
                description: format!("{operator} {value}"),
                value,
                predicate,
            });
        }
    }
    Err(Error {
        code: "ast_crispr_limit_invalid_expression".to_string(),
        message: "Invalid ast-crispr limit expression".to_string(),
    })
}

pub fn ast_merge_contract_anchor() -> &'static str {
    std::any::type_name::<ast_merge::StructuredEditCrisprExampleParityReport>()
}

pub fn boundary_report() -> Value {
    json!({
        "package": PACKAGE_NAME,
        "layer": "structural_edit_tool",
        "status": "active_thin_package",
        "base_contract_package": "ast-merge",
        "relationship": {
            "ast_merge": [
                "owns portable structured-edit envelope contracts",
                "owns transport, report, replay, review, and provider handoff vocabulary",
                "remains the substrate for provider-neutral fixtures"
            ],
            "ast_crispr": [
                "owns ergonomic structural-edit selectors, profiles, and operation helpers",
                "wraps ast-merge contracts instead of forking them",
                "may grow compatibility helpers for old ast-crispr concepts after fixture-backed review"
            ],
            "provider_packages": [
                "own parser-specific execution and metadata projection",
                "may expose provider adapters consumed by ast-crispr",
                "keep raw parser details behind normalized tree metadata or semantic sidecars"
            ],
            "ast_template": [
                "orchestrates template and directory workflows",
                "invokes structural edits through ast-merge or ast-crispr registries/envelopes",
                "does not own parser-specific selectors"
            ]
        },
        "implementations": [
            {
                "language": "go",
                "package_name": "astcrispr",
                "import": "github.com/structuredmerge/structuredmerge-go/astcrispr"
            },
            {
                "language": "ruby",
                "package_name": "ast-crispr",
                "require": "ast/crispr"
            },
            {
                "language": "rust",
                "package_name": "ast-crispr",
                "crate": "ast_crispr"
            },
            {
                "language": "typescript",
                "package_name": "@structuredmerge/ast-crispr",
                "import": "@structuredmerge/ast-crispr"
            }
        ],
        "initial_exports": [
            "package identity",
            "boundary report",
            "ast-merge structured-edit contract anchor",
            "limit helpers",
            "match profile helpers",
            "selection profile helpers",
            "destination profile helpers"
        ],
        "future_exports": [
            "operation profile helpers",
            "replace/delete/insert/move helpers",
            "batch operation helpers"
        ],
        "metadata": {
            "source": "legacy_crispr_reference",
            "decision": "Keep ast-merge as the base contract layer and revive ast-crispr as a separate thin package in every implementation."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn boundary_report_matches_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("fixtures")
            .join("diagnostics")
            .join("slice-916-ast-crispr-package-boundary")
            .join("ast-crispr-package-boundary.json");
        let source = fs::read_to_string(fixture_path).expect("read fixture");
        let fixture: Value = serde_json::from_str(&source).expect("parse fixture");

        assert_eq!(boundary_report(), fixture["boundary"]);
        assert!(ast_merge_contract_anchor().ends_with("StructuredEditCrisprExampleParityReport"));
    }

    #[test]
    fn limit_helpers_match_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("fixtures")
            .join("diagnostics")
            .join("slice-917-ast-crispr-limit-helpers")
            .join("ast-crispr-limit-helpers.json");
        let source = fs::read_to_string(fixture_path).expect("read fixture");
        let fixture: Value = serde_json::from_str(&source).expect("parse fixture");

        for test_case in fixture["cases"].as_array().expect("cases") {
            let limit = Limit::new(test_case.get("spec")).expect("limit");
            assert_eq!(limit.describe(), test_case["expected_description"]);
            for expectation in test_case["expectations"].as_array().expect("expectations") {
                let count = expectation["count"].as_u64().expect("count") as usize;
                assert_eq!(limit.allows(count), expectation["allowed"]);
            }
        }

        for test_case in fixture["invalid_cases"].as_array().expect("invalid cases") {
            let result = Limit::new(test_case.get("spec"));
            assert!(result.is_err());
            assert_eq!(result.err().expect("invalid limit").code, test_case["expected_error"]);
        }
    }

    #[test]
    fn match_profile_helpers_match_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("fixtures")
            .join("diagnostics")
            .join("slice-918-ast-crispr-match-profile-helpers")
            .join("ast-crispr-match-profile-helpers.json");
        let source = fs::read_to_string(fixture_path).expect("read fixture");
        let fixture: Value = serde_json::from_str(&source).expect("parse fixture");

        for test_case in fixture["cases"].as_array().expect("cases") {
            let profile_fixture = &test_case["profile"];
            let profile = MatchProfile::new(
                profile_fixture["start_boundary"].as_str().expect("start boundary"),
                profile_fixture["end_boundary"].as_str().expect("end boundary"),
                profile_fixture["payload_kind"].as_str().expect("payload kind"),
            );
            assert_eq!(profile.report(), test_case["expected"]);
        }
    }

    #[test]
    fn selection_profile_helpers_match_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("fixtures")
            .join("diagnostics")
            .join("slice-919-ast-crispr-selection-profile-helpers")
            .join("ast-crispr-selection-profile-helpers.json");
        let source = fs::read_to_string(fixture_path).expect("read fixture");
        let fixture: Value = serde_json::from_str(&source).expect("parse fixture");

        for test_case in fixture["cases"].as_array().expect("cases") {
            let profile_fixture = &test_case["profile"];
            let profile = SelectionProfile::new(
                profile_fixture["owner_scope"].as_str().expect("owner scope"),
                profile_fixture["owner_selector"].as_str().expect("owner selector"),
                profile_fixture["selector_kind"].as_str().expect("selector kind"),
                profile_fixture["selection_intent"].as_str().expect("selection intent"),
                profile_fixture["comment_region"].as_str(),
                profile_fixture["include_trailing_gap"].as_bool().expect("include trailing gap"),
            );
            assert_eq!(profile.report(), test_case["expected"]);
        }
    }

    #[test]
    fn destination_profile_helpers_match_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("fixtures")
            .join("diagnostics")
            .join("slice-920-ast-crispr-destination-profile-helpers")
            .join("ast-crispr-destination-profile-helpers.json");
        let source = fs::read_to_string(fixture_path).expect("read fixture");
        let fixture: Value = serde_json::from_str(&source).expect("parse fixture");

        for test_case in fixture["cases"].as_array().expect("cases") {
            let profile_fixture = &test_case["profile"];
            let profile = DestinationProfile::new(
                profile_fixture["resolution_kind"].as_str().expect("resolution kind"),
                profile_fixture["resolution_source"].as_str().expect("resolution source"),
                profile_fixture["anchor_boundary"].as_str().expect("anchor boundary"),
                profile_fixture["used_if_missing"].as_bool().expect("used if missing"),
            );
            assert_eq!(profile.report(), test_case["expected"]);
        }
    }
}
