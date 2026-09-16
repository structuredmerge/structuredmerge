use structuredmerge_core::{CrisprOperationRequest, report_structural_operations};

#[test]
fn reports_keep_defaults_unknowns_and_batch_order_without_claiming_execution() {
    let default = CrisprOperationRequest {
        operation_kind: "".into(),
        source_requirement: "".into(),
        destination_requirement: "".into(),
        replacement_source: "".into(),
        captures_source_text: true,
        supports_if_missing: false,
    };
    let unknown = CrisprOperationRequest {
        operation_kind: "future".into(),
        source_requirement: "future".into(),
        replacement_source: "future".into(),
        ..default.clone()
    };
    let report = report_structural_operations(vec![unknown, default]);
    assert_eq!(report.operation_count, 2);
    assert_eq!(report.operation_kinds, ["future", "replace"]);
    let unknown = &report.operation_profiles[0];
    assert!(!unknown.known_operation_kind);
    assert_eq!(unknown.operation_family, "unknown");
    assert!(!unknown.known_source_requirement);
    assert!(!unknown.known_replacement_source);
    let default = &report.operation_profiles[1];
    assert!(default.known_operation_kind);
    assert_eq!(default.source_requirement, "required");
    assert!(default.requires_source);
    assert!(!default.supports_destination);
    assert!(default.explicit_replacement);
    assert!(default.captures_source_text);
    assert!(!default.supports_if_missing);
    assert_eq!(report_structural_operations(vec![]).operation_count, 0);
}
