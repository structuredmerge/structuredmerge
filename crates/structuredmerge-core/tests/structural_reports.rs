use structuredmerge_core::{CrisprOperationRequest, report_structural_operations};

#[test]
fn match_selection_and_destination_keep_classification_and_optional_values() {
    use structuredmerge_core::*;
    let matched = report_structural_match(CrisprMatchRequest {
        start_boundary: "future".into(),
        end_boundary: "owner_end_plus_trailing_gap".into(),
        payload_kind: "comment_owned_body".into(),
    });
    assert!(!matched.known_start_boundary);
    assert_eq!(matched.start_boundary, "future");
    assert!(matched.trailing_gap_extended && matched.comment_anchored);
    for region in [None, Some("leading"), Some("future")] {
        let selected = report_structural_selection(CrisprSelectionRequest {
            owner_scope: "".into(),
            owner_selector: "".into(),
            selector_kind: "".into(),
            selection_intent: "".into(),
            comment_region: region.map(str::to_owned),
            include_trailing_gap: true,
        });
        assert_eq!(selected.owner_scope, "shared_default");
        assert_eq!(selected.owner_selector, "line_bound_statements");
        assert_eq!(selected.comment_region.as_deref(), region);
        assert_eq!(selected.known_comment_region, region == Some("leading"));
        assert_eq!(selected.comment_anchored, region == Some("leading"));
        assert!(selected.include_trailing_gap);
    }
    let destination = report_structural_destination(CrisprDestinationRequest {
        resolution_kind: "".into(),
        resolution_source: "future".into(),
        anchor_boundary: "".into(),
        used_if_missing: true,
    });
    assert_eq!(destination.resolution_kind, "append_fallback");
    assert!(!destination.known_resolution_source);
    assert!(destination.append_fallback && destination.used_if_missing);
    assert!(!destination.anchored);
}

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
