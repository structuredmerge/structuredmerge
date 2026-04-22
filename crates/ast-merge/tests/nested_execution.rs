use ast_merge::{
    AppliedDelegatedChildOutput, DelegatedChildApplyPlan, DelegatedChildApplyPlanEntry,
    DelegatedChildGroupReviewState, DelegatedChildOperation, DelegatedChildOutputResolutionOptions,
    DelegatedChildSurfaceOutput, Diagnostic, DiagnosticCategory, DiagnosticSeverity,
    DiscoveredSurface, MergeResult, NestedMergeDiscoveryResult, NestedMergeExecutionCallbacks,
    ProjectedChildReviewGroup, ReviewDecision, ReviewDecisionAction, SurfaceOwnerKind,
    SurfaceOwnerRef, execute_delegated_child_apply_plan, execute_nested_merge,
    execute_reviewed_nested_execution, execute_reviewed_nested_merge, reviewed_nested_execution,
};

fn nested_operation(address: &str, family: Option<&str>) -> DelegatedChildOperation {
    let mut metadata = std::collections::HashMap::new();
    if let Some(family) = family {
        metadata.insert("family".to_string(), serde_json::json!(family));
    }

    DelegatedChildOperation {
        operation_id: format!("operation:{address}"),
        parent_operation_id: "parent:merge".to_string(),
        requested_strategy: "delegate_child_surface".to_string(),
        language_chain: vec!["markdown".to_string(), "typescript".to_string()],
        surface: DiscoveredSurface {
            surface_kind: "fenced_code_block".to_string(),
            declared_language: None,
            effective_language: "typescript".to_string(),
            address: address.to_string(),
            parent_address: None,
            span: None,
            owner: SurfaceOwnerRef {
                kind: SurfaceOwnerKind::OwnedRegion,
                address: "/code_fence/0".to_string(),
            },
            reconstruction_strategy: "portable_write".to_string(),
            metadata,
        },
    }
}

#[test]
fn execute_nested_merge_orchestrates_stages() {
    let address = "document[0] > fenced_code_block[/code_fence/0]";
    let nested_outputs = vec![DelegatedChildSurfaceOutput {
        surface_address: address.to_string(),
        output: "export const feature = true;\n".to_string(),
    }];
    let calls = std::cell::RefCell::new(Vec::new());

    let result = execute_nested_merge(
        &nested_outputs,
        &DelegatedChildOutputResolutionOptions {
            default_family: "markdown".to_string(),
            request_id_prefix: "nested_markdown_child".to_string(),
        },
        NestedMergeExecutionCallbacks {
            merge_parent: || {
                calls.borrow_mut().push("merge".to_string());
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("merged-parent".to_string()),
                    policies: vec![],
                }
            },
            discover_operations: |merged_output| {
                calls.borrow_mut().push(format!("discover:{merged_output}"));
                NestedMergeDiscoveryResult {
                    ok: true,
                    diagnostics: vec![],
                    operations: Some(vec![nested_operation(address, Some("typescript"))]),
                }
            },
            apply_resolved_outputs: |merged_output, operations, apply_plan, applied_children| {
                calls.borrow_mut().push(format!("apply:{merged_output}"));
                assert_eq!(operations.len(), 1);
                assert_eq!(apply_plan.entries.len(), 1);
                assert_eq!(apply_plan.entries[0].family, "typescript");
                assert_eq!(
                    applied_children,
                    &[AppliedDelegatedChildOutput {
                        operation_id: format!("operation:{address}"),
                        output: "export const feature = true;\n".to_string(),
                    }]
                );
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("final-parent".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert_eq!(result.output, Some("final-parent".to_string()));
    assert_eq!(
        calls.into_inner(),
        vec![
            "merge".to_string(),
            "discover:merged-parent".to_string(),
            "apply:merged-parent".to_string()
        ]
    );
}

#[test]
fn execute_nested_merge_returns_parent_failure_unchanged() {
    let called = std::cell::Cell::new(false);
    let result = execute_nested_merge::<String, _, _, _>(
        &[],
        &DelegatedChildOutputResolutionOptions {
            default_family: "markdown".to_string(),
            request_id_prefix: "nested".to_string(),
        },
        NestedMergeExecutionCallbacks {
            merge_parent: || MergeResult {
                ok: false,
                diagnostics: vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    category: DiagnosticCategory::ParseError,
                    message: "parent failed".to_string(),
                    path: None,
                    review: None,
                }],
                output: None,
                policies: vec![],
            },
            discover_operations: |_| {
                called.set(true);
                NestedMergeDiscoveryResult {
                    ok: true,
                    diagnostics: vec![],
                    operations: Some(vec![]),
                }
            },
            apply_resolved_outputs: |_, _, _, _| {
                called.set(true);
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("unused".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert!(!result.ok);
    assert!(!called.get());
}

#[test]
fn execute_nested_merge_returns_discovery_failure_and_skips_apply() {
    let applied = std::cell::Cell::new(false);
    let result = execute_nested_merge::<String, _, _, _>(
        &[],
        &DelegatedChildOutputResolutionOptions {
            default_family: "markdown".to_string(),
            request_id_prefix: "nested".to_string(),
        },
        NestedMergeExecutionCallbacks {
            merge_parent: || MergeResult {
                ok: true,
                diagnostics: vec![],
                output: Some("merged-parent".to_string()),
                policies: vec![],
            },
            discover_operations: |_| NestedMergeDiscoveryResult {
                ok: false,
                diagnostics: vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    category: DiagnosticCategory::ConfigurationError,
                    message: "discovery failed".to_string(),
                    path: None,
                    review: None,
                }],
                operations: None,
            },
            apply_resolved_outputs: |_, _, _, _| {
                applied.set(true);
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("unused".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert!(!result.ok);
    assert!(!applied.get());
}

#[test]
fn execute_delegated_child_apply_plan_orchestrates_stages() {
    let address = "document[0] > fenced_code_block[/code_fence/0]";
    let result = execute_delegated_child_apply_plan(
        &DelegatedChildApplyPlan {
            entries: vec![DelegatedChildApplyPlanEntry {
                request_id: "projected_child_group:markdown:fence:typescript".to_string(),
                family: "markdown".to_string(),
                delegated_group: ProjectedChildReviewGroup {
                    delegated_apply_group: "markdown:fence:typescript".to_string(),
                    parent_operation_id: "parent:merge".to_string(),
                    child_operation_id: format!("operation:{address}"),
                    delegated_runtime_surface_path: address.to_string(),
                    case_ids: vec![],
                    delegated_case_ids: vec![],
                },
                decision: ReviewDecision {
                    request_id: "projected_child_group:markdown:fence:typescript".to_string(),
                    action: ReviewDecisionAction::ApplyDelegatedChildGroup,
                    context: None,
                },
            }],
        },
        &[AppliedDelegatedChildOutput {
            operation_id: format!("operation:{address}"),
            output: "child-output\n".to_string(),
        }],
        NestedMergeExecutionCallbacks {
            merge_parent: || MergeResult {
                ok: true,
                diagnostics: vec![],
                output: Some("merged-parent".to_string()),
                policies: vec![],
            },
            discover_operations: |_| NestedMergeDiscoveryResult {
                ok: true,
                diagnostics: vec![],
                operations: Some(vec![nested_operation(address, None)]),
            },
            apply_resolved_outputs: |_, _, apply_plan, applied_children| {
                assert_eq!(apply_plan.entries.len(), 1);
                assert_eq!(applied_children.len(), 1);
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("final-parent".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert_eq!(result.output, Some("final-parent".to_string()));
}

#[test]
fn execute_reviewed_nested_merge_uses_accepted_review_state() {
    let address = "document[0] > fenced_code_block[/code_fence/0]";
    let result = execute_reviewed_nested_merge(
        &DelegatedChildGroupReviewState {
            requests: vec![],
            accepted_groups: vec![ProjectedChildReviewGroup {
                delegated_apply_group: "markdown:fence:typescript".to_string(),
                parent_operation_id: "parent:merge".to_string(),
                child_operation_id: format!("operation:{address}"),
                delegated_runtime_surface_path: address.to_string(),
                case_ids: vec![],
                delegated_case_ids: vec![],
            }],
            applied_decisions: vec![ReviewDecision {
                request_id: "projected_child_group:markdown:fence:typescript".to_string(),
                action: ReviewDecisionAction::ApplyDelegatedChildGroup,
                context: None,
            }],
            diagnostics: vec![],
        },
        "markdown",
        &[AppliedDelegatedChildOutput {
            operation_id: format!("operation:{address}"),
            output: "child-output\n".to_string(),
        }],
        NestedMergeExecutionCallbacks {
            merge_parent: || MergeResult {
                ok: true,
                diagnostics: vec![],
                output: Some("merged-parent".to_string()),
                policies: vec![],
            },
            discover_operations: |_| NestedMergeDiscoveryResult {
                ok: true,
                diagnostics: vec![],
                operations: Some(vec![nested_operation(address, None)]),
            },
            apply_resolved_outputs: |_, _, apply_plan, _| {
                assert_eq!(
                    apply_plan.entries[0].request_id,
                    "projected_child_group:markdown:fence:typescript"
                );
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("final-parent".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert_eq!(result.output, Some("final-parent".to_string()));
}

#[test]
fn execute_reviewed_nested_execution_uses_payload() {
    let address = "document[0] > fenced_code_block[/code_fence/0]";
    let execution = reviewed_nested_execution(
        "markdown",
        &DelegatedChildGroupReviewState {
            requests: vec![],
            accepted_groups: vec![ProjectedChildReviewGroup {
                delegated_apply_group: "markdown:fence:typescript".to_string(),
                parent_operation_id: "parent:merge".to_string(),
                child_operation_id: format!("operation:{address}"),
                delegated_runtime_surface_path: address.to_string(),
                case_ids: vec![],
                delegated_case_ids: vec![],
            }],
            applied_decisions: vec![ReviewDecision {
                request_id: "projected_child_group:markdown:fence:typescript".to_string(),
                action: ReviewDecisionAction::ApplyDelegatedChildGroup,
                context: None,
            }],
            diagnostics: vec![],
        },
        &[AppliedDelegatedChildOutput {
            operation_id: format!("operation:{address}"),
            output: "child-output\n".to_string(),
        }],
    );

    let result = execute_reviewed_nested_execution(
        &execution,
        NestedMergeExecutionCallbacks {
            merge_parent: || MergeResult {
                ok: true,
                diagnostics: vec![],
                output: Some("merged-parent".to_string()),
                policies: vec![],
            },
            discover_operations: |_| NestedMergeDiscoveryResult {
                ok: true,
                diagnostics: vec![],
                operations: Some(vec![nested_operation(address, None)]),
            },
            apply_resolved_outputs: |_, _, apply_plan, applied_children| {
                assert_eq!(
                    apply_plan.entries[0].request_id,
                    "projected_child_group:markdown:fence:typescript"
                );
                assert_eq!(applied_children.len(), 1);
                assert_eq!(applied_children[0].operation_id, format!("operation:{address}"));
                MergeResult {
                    ok: true,
                    diagnostics: vec![],
                    output: Some("final-parent".to_string()),
                    policies: vec![],
                }
            },
        },
    );

    assert_eq!(result.output, Some("final-parent".to_string()));
}
