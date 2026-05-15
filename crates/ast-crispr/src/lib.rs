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
            "limit helpers"
        ],
        "future_exports": [
            "match profile helpers",
            "selection profile helpers",
            "destination profile helpers",
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
}
