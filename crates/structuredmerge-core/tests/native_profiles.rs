use structuredmerge_core::native_merge_profiles;

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
