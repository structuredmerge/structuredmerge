use structuredmerge_core::native_merge_profiles;

#[test]
fn common_operation_catalog_lists_scoped_support_without_availability_or_authority() {
    use structuredmerge_core::{OperationKind, OperationProfileCatalog, operation_profile_catalog};
    let catalog = operation_profile_catalog();
    assert_eq!(catalog.schema, "structuredmerge.operation-profile-catalog/v1");
    assert_eq!(catalog, operation_profile_catalog());
    assert_eq!(catalog.profiles.len(), 8);
    assert!(catalog.profiles.windows(2).all(|pair| pair[0].id < pair[1].id));
    for profile in &catalog.profiles {
        assert_eq!(profile.semantic_runtime, "rust");
        assert_eq!(profile.parser_available, None);
        assert!(!profile.approved_as_default);
        assert!(profile.experimental);
        assert!(!profile.syntax_scope.is_empty());
        assert!(profile.syntax_scope.iter().all(|scope| !scope.is_empty()));
        assert!(!profile.limitations.is_empty());
        assert_eq!(profile.parser_contract, "structuredmerge.parse-result/v1");
        let expected = match profile.provider_id.as_str() {
            "kernel.yaml" => {
                vec![OperationKind::Analyze, OperationKind::Diff2, OperationKind::Merge3]
            }
            "kernel.git.json" => vec![OperationKind::Merge3],
            "kernel.bash" | "kernel.go" | "kernel.json" | "kernel.python" | "kernel.rust"
            | "kernel.typescript" => vec![
                OperationKind::Analyze,
                OperationKind::Diff2,
                OperationKind::Merge2,
                OperationKind::Merge3,
            ],
            unexpected => panic!("unreviewed provider {unexpected}"),
        };
        assert_eq!(profile.operations, expected);
        assert_eq!(
            profile.required_native_extension.is_some(),
            matches!(profile.family.as_str(), "python" | "yaml")
        );
    }
    let typescript =
        catalog.profiles.iter().find(|profile| profile.family == "typescript").unwrap();
    assert_eq!(typescript.explicit_dialects, ["typescript", "tsx"]);
    let encoded = serde_json::to_string(&catalog).unwrap();
    assert_eq!(serde_json::from_str::<OperationProfileCatalog>(&encoded).unwrap(), catalog);
    // The old explicit-entry-point listing retains its narrower contract.
    assert_eq!(native_merge_profiles().len(), 2);
}

#[test]
fn native_profile_listing_is_deterministic_and_does_not_claim_default_authority() {
    let profiles = native_merge_profiles();
    assert_eq!(profiles, native_merge_profiles());
    assert_eq!(profiles.len(), 2);
    assert!(profiles.windows(2).all(|pair| pair[0].id < pair[1].id));
    for profile in profiles {
        assert_eq!(profile.operation, "merge3");
        assert_eq!(profile.semantic_runtime, "rust");
        assert_eq!(profile.merge_crate, "ast-merge");
        assert!(profile.experimental);
        assert!(!profile.approved_as_default);
        assert!(!profile.syntax_scope.is_empty());
        assert!(!profile.limitations.is_empty());
        assert!(profile.native_extension.ends_with("/v1"));
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(
            serde_json::from_str::<structuredmerge_core::NativeMergeProfile>(&json).unwrap(),
            profile
        );
    }
}
