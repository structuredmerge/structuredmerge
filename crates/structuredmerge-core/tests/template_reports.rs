use structuredmerge_core::*;

fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(snapshot(&path));
        } else {
            files.insert(path.clone(), std::fs::read(path).unwrap());
        }
    }
    files
}

fn options() -> TemplateSessionOptions {
    TemplateSessionOptions {
        mode: DirectorySessionMode::Plan,
        template_root: String::new(),
        destination_root: String::new(),
        context: TemplateDestinationContext::default(),
        default_strategy: TemplateStrategy::Merge,
        overrides: vec![],
        replacements: Default::default(),
        allowed_families: None,
        config: None,
    }
}

#[test]
fn configuration_reports_preserve_owner_results_without_filesystem_access() {
    for mode in
        [DirectorySessionMode::Plan, DirectorySessionMode::Apply, DirectorySessionMode::Reapply]
    {
        for root in ["", "/not-a-real-template-root/é"] {
            let mut input = options();
            input.mode = mode;
            input.template_root = root.into();
            input.destination_root = root.into();
            let expected = ast_template::report_template_directory_session_options_request(
                &input.clone().into(),
            );
            assert_eq!(
                serde_json::to_value(report_template_options(input)).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
    }
}

#[test]
fn profile_resolution_preserves_defaults_overrides_and_missing_diagnostics() {
    let profile = DirectorySessionProfile {
        mode: DirectorySessionMode::Apply,
        context: TemplateDestinationContext { project_name: Some("widget".into()) },
        default_strategy: TemplateStrategy::RawCopy,
        overrides: vec![TemplateStrategyOverride {
            path: "README.md".into(),
            strategy: TemplateStrategy::KeepDestination,
        }],
        replacements: [("NAME".into(), "widget".into())].into(),
        allowed_families: Some(vec!["markdown".into()]),
        config: Some(ast_merge::default_template_token_config()),
    };
    let profiles = [("known".into(), profile)].into();
    for name in ["known", "missing"] {
        let mut input = options();
        input.template_root = "/not-read/templates".into();
        input.destination_root = "/not-read/destination".into();
        let expected = ast_template::report_template_directory_session_profile_request(
            &profiles,
            name,
            &input.clone().into(),
        );
        let actual = report_template_profile(TemplateProfileRequest {
            profile_name: name.into(),
            profiles: profiles.clone(),
            options: input,
        });
        assert_eq!(serde_json::to_value(actual).unwrap(), serde_json::to_value(expected).unwrap());
    }
}

#[test]
fn directory_plan_matches_shared_golden_without_applying() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/diagnostics/slice-362-template-directory-session-options-report");
    let golden: serde_json::Value = serde_json::from_str(include_str!("../../../../fixtures/diagnostics/slice-362-template-directory-session-options-report/template-directory-session-options-report.json")).unwrap();
    let mut input = golden["plan_run"]["options"].clone();
    input["template_root"] = root.join("dry-run/template").to_str().unwrap().into();
    input["destination_root"] = root.join("dry-run/destination").to_str().unwrap().into();
    let input: TemplateSessionOptions = serde_json::from_value(input).unwrap();
    let before = snapshot(&root.join("dry-run"));
    let report = plan_template_directory(input.clone()).unwrap();
    assert_eq!(snapshot(&root.join("dry-run")), before);
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        golden["plan_run"]["expected"]["session_report"]
    );
    let mut invalid = input;
    invalid.mode = DirectorySessionMode::Apply;
    assert_eq!(plan_template_directory(invalid).unwrap_err().code, "template.request.invalid");
}
