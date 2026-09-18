//! Compiler-emitted metadata, never runtime checkout/package discovery.
include!(concat!(env!("OUT_DIR"), "/smorg_build.rs"));

pub fn value() -> serde_json::Value {
    serde_json::json!({
        "schema": "structuredmerge.cli-build/v1",
        "target": TARGET, "host": HOST, "cargo_profile": PROFILE,
        "cargo_opt_level": OPT_LEVEL, "cargo_debug": DEBUG,
        "cargo_feature_flags": CARGO_FEATURE_FLAGS, "target_features": TARGET_FEATURES,
        "source": {"revision": SOURCE_REVISION, "state": SOURCE_STATE,
            "origin": if SOURCE_REVISION.is_some() { "build-environment" } else { "unspecified" },
            "verified": false},
        "provenance_verified": false,
    })
}
