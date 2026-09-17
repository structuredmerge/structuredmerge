use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};
use structuredmerge_core::*;
use tree_haver::{language_pack_provider::LanguagePackProvider, service::*};

fn request(operation: &str, dialect: &str, texts: &[&str]) -> OperationRequest {
    let roles: &[&str] = match operation {
        "analyze" => &["source"],
        "diff2" => &["before", "after"],
        "merge2" => &["incoming", "current"],
        _ => &["base", "ours", "theirs"],
    };
    let mut sources = serde_json::Map::new();
    for (role, text) in roles.iter().zip(texts) {
        let input = source_input(
            role.to_string(),
            serde_json::from_value(json!(role)).unwrap(),
            SourceEncoding::Utf8,
            text.as_bytes().to_vec(),
        )
        .unwrap();
        sources.insert(role.to_string(), json!({"source_id":role,"role":role,"content":text,"encoding":"utf-8","byte_length":input.descriptor.byte_length,"sha256":input.descriptor.sha256}));
    }
    serde_json::from_value(json!({"schema": OPERATION_SCHEMA,"request_id":"json-common",
        "operation":operation,"provider_selection":{"provider_id":"kernel.json","family":"json","dialect":dialect,"profile_id":"kernel.json.nested.v1","required_capabilities":[operation]},
        "parser_selection":{"backend":"json.common","preference":[],"required_capabilities":[]},"sources":sources,
        "policy":match operation { "analyze" => json!({}), "diff2" => json!({"comparison_profile":"exact-source-owners","equivalence":["exact-source"]}), "merge2" => json!({"directional_merge":"template-into-current","render_policy":"source-preserving"}), _ => json!({"render_policy":"source-preserving"}) },
        "extensions":[],"metadata":{}})).unwrap()
}
fn run(request: OperationRequest) -> (OperationResult, ValidatedOperationRequest) {
    let language = if request.provider_selection.dialect.as_deref() == Some("json") {
        "json"
    } else {
        "json5"
    };
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(
            LanguagePackProvider::new("json.common".into(), language.into()).unwrap(),
        ))
        .unwrap();
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
    };
    let validated = request.validate(100000, |_, _| panic!()).unwrap();
    let result = native_operation::execute_native_operation(
        &validated,
        &registry.snapshot().unwrap(),
        &context,
    )
    .unwrap();
    result.validate_against(&validated).unwrap();
    (result, validated)
}

fn git_request(dialect: &str, texts: &[&str]) -> OperationRequest {
    let mut input = request("merge3", dialect, texts);
    input.provider_selection.provider_id = Some("kernel.git.json".into());
    input.provider_selection.profile_id = Some("kernel.git.json.v1".into());
    input
}

#[test]
fn common_git_profile_has_verified_clean_output_for_all_json_dialects() {
    for (dialect, texts) in [
        ("json", ["{\"x\":0,\"y\":0}", "{\"x\":1,\"y\":0}", "{\"x\":0,\"y\":2}"]),
        ("jsonc", ["{/*c*/\"x\":0,\"y\":0}", "{/*c*/\"x\":1,\"y\":0}", "{/*c*/\"x\":0,\"y\":2}"]),
        ("json5", ["{x:0,y:0}", "{x:1,y:0}", "{x:0,y:2}"]),
    ] {
        let (result, validated) = run(git_request(dialect, &texts));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.provider.provider_id.as_deref(), Some("kernel.git.json"));
        assert_eq!(result.verification.output_reparsed, Some(true));
        assert_eq!(result.verification.base_participated, Some(true));
        assert!(result.output.as_ref().unwrap().contains('2'));
        let mut tampered = result.clone();
        tampered.output.as_mut().unwrap().push(' ');
        assert!(tampered.validate_against(&validated).is_err());
    }
}

#[test]
fn common_git_conflict_evidence_rejects_tampering_and_retains_unknown_fields() {
    let mut input = git_request("json", &["{\n\"x\":0\n}\n", "{\n\"x\":1\n}\n", "{\n\"x\":2\n}\n"]);
    let OperationPolicy::Merge3(policy) = &mut input.operation else { panic!() };
    policy.conflict_marker_size = Some(9);
    policy.labels = Some([("ours".into(), "local-é".into())].into());
    let (result, validated) = run(input);
    assert!(!result.ok);
    assert!(result.output.is_none());
    assert_eq!(result.conflicts.len(), 1);
    assert!(result.conflicted_output.as_ref().unwrap().contains("<<<<<<<<< local-é"));
    assert_eq!(result.verification.output_reparsed, None);
    assert_eq!(result.render_report["outside_conflicts"], "ours-not-partially-merged");
    for mutation in ["bytes", "source", "provenance", "classification", "reparse", "omission"] {
        let mut changed = serde_json::to_value(&result).unwrap();
        match mutation {
            "bytes" => changed["conflicted_output"] = json!("wrong"),
            "source" => {
                changed["render_report"]["evidence"]["sources"][0]["sha256"] = json!("0".repeat(64))
            }
            "provenance" => {
                changed["render_report"]["evidence"]["rendered"]["line_records"][0]["original_line"] =
                    json!(999)
            }
            "classification" => {
                changed["conflicts"][0]["classification"]["native_conflict"]["category"] =
                    json!("fabricated")
            }
            "reparse" => changed["verification"]["output_reparsed"] = json!(true),
            _ => {
                changed["render_report"] = json!({});
                changed.as_object_mut().unwrap().remove("conflicted_output");
            }
        }
        let changed: OperationResult = serde_json::from_value(changed).unwrap();
        assert!(changed.validate_against(&validated).is_err(), "{mutation}");
    }
    let mut compatible = result.clone();
    compatible.render_report.insert("future_passive_fact".into(), json!({"a":1}));
    compatible.validate_against(&validated).unwrap();
}

#[test]
fn common_git_unrenderable_conflicts_remain_unresolved_without_output() {
    let texts = ["{\"x\":0,\"y\":0}", "{\"x\":1,\"y\":1}", "{\"x\":2,\"y\":2}"];
    let (result, _) = run(git_request("json", &texts));
    assert!(!result.ok);
    assert!(!result.conflicts.is_empty());
    assert!(result.output.is_none() && result.conflicted_output.is_none());
    assert_eq!(result.render_report["strategy"], "git-unrendered-conflict");
    assert!(result.render_report["render_error"].is_string());
}

#[test]
fn common_git_deleted_owner_has_explicit_review_placement_not_merged_output() {
    let (result, _) = run(git_request("json", &["{\"x\":0}", "{}", "{\"x\":2}"]));
    assert!(!result.ok);
    assert_eq!(result.conflicts.len(), 1);
    assert!(result.output.is_none());
    assert_eq!(
        result.conflicted_output.as_deref(),
        Some("{}\n<<<<<<< ours\n||||||| base\n{\"x\":0}\n=======\n{\"x\":2}\n>>>>>>> theirs\n")
    );
    assert_eq!(result.render_report["strategy"], "git-absent-owner-conflict-review");
    assert!(result.render_report["render_error"].is_null());
    assert_eq!(result.verification.output_reparsed, None);
}

#[test]
fn common_git_rejects_unsupported_operations_options_and_parser_failures() {
    for operation in ["analyze", "diff2", "merge2"] {
        let mut input = request(operation, "json", &["{}", "{}"]);
        input.provider_selection.provider_id = Some("kernel.git.json".into());
        input.provider_selection.profile_id = Some("kernel.git.json.v1".into());
        let (result, _) = run(input);
        assert!(!result.ok);
        assert!(result.conflicted_output.is_none());
    }
    for policy in [
        json!({"render_policy":"source-preserving","labels":{"unknown":"x"}}),
        json!({"render_policy":"source-preserving","labels":{"ours":"x\ny"}}),
        json!({"render_policy":"source-preserving","conflict_marker_size":1025}),
        json!({"render_policy":"source-preserving","fallback_policy":"text"}),
    ] {
        let input = git_request("json", &["{}", "{}", "{}"]);
        let mut value = serde_json::to_value(input).unwrap();
        value["policy"] = policy;
        let (result, _) = run(serde_json::from_value(value).unwrap());
        assert!(!result.ok);
        assert!(result.render_report.is_empty());
    }
    let (result, _) = run(git_request("json", &["{}", "{", "{}"]));
    assert!(!result.ok);
    assert!(result.conflicts.is_empty());
}

#[test]
fn common_json_analysis_has_resolvable_owners_comments_layout_and_native_evidence() {
    let (result, _) = run(request(
        "analyze",
        "json5",
        &["// pre\r\n{\r\n a:{x:'é'},\r\n\r\n // next\r\n b:[1,2]\r\n}\r\n// post\r\n"],
    ));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.output.is_none());
    assert!(result.diff.is_none());
    let analysis = result.analysis.unwrap().extra;
    let owners = analysis["owners"].as_array().unwrap();
    let ids =
        owners.iter().map(|o| o["id"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
    assert!(ids.contains("json:/a/x"));
    assert!(ids.contains("json:/b/1"));
    let nodes = analysis["parse_result"]["parsed"]["nodes"].as_array().unwrap();
    for owner in owners {
        let node = nodes.iter().find(|node| node["id"] == owner["node_id"]).unwrap();
        assert_eq!(owner["span"], node["span"]);
        for node_id in owner["node_ids"].as_array().unwrap() {
            assert!(nodes.iter().any(|node| &node["id"] == node_id));
        }
    }
    for comment in analysis["comment_regions"].as_array().unwrap() {
        for group_id in comment["family_region_ids"].as_array().unwrap() {
            assert!(
                analysis["family_comment_regions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|group| &group["id"] == group_id
                        && ids.contains(group["owner_id"].as_str().unwrap()))
            );
        }
        assert!(ids.contains(comment["owner_id"].as_str().unwrap()));
        assert_eq!(comment["node_ids"].as_array().unwrap().len(), 1);
        let node = nodes.iter().find(|node| node["id"] == comment["node_ids"][0]).unwrap();
        assert_eq!(node["span"], comment["span"]);
    }
    assert_eq!(analysis["comment_regions"].as_array().unwrap().len(), 3);
    for decision in analysis["ownership"].as_array().unwrap() {
        assert!(ids.contains(decision["selected_owner_ref"].as_str().unwrap()));
    }
    assert!(!analysis["layout_gaps"].as_array().unwrap().is_empty());
    for gap in analysis["layout_gaps"].as_array().unwrap() {
        let side = gap["controller_side"].as_str().unwrap();
        let controller = gap[format!("{side}_owner_id")].as_str().unwrap();
        assert!(ids.contains(controller));
        let decisions = analysis["ownership"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|decision| decision["subject_ref"] == gap["id"])
            .collect::<Vec<_>>();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0]["selected_owner_ref"], controller);
    }
}

#[test]
fn common_json_analysis_supports_nested_arrays_scalar_roots_and_explicit_enrichment() {
    for (dialect, source) in [
        ("json", "42"),
        ("json", "[1,{\"x\":[2]}]"),
        ("jsonc", "{\n // note\n \"x\":1,\n}"),
        ("json5", "{x:'é'}"),
    ] {
        let mut input = request("analyze", dialect, &[source]);
        let OperationPolicy::Analyze(policy) = &mut input.operation else { panic!() };
        policy.comments = Some(true);
        policy.ownership = Some(true);
        policy.native_extensions = Some(true);
        let (result, _) = run(input);
        assert!(result.ok, "{dialect} {source}: {:?}", result.diagnostics);
        let analysis = result.analysis.unwrap();
        assert!(!analysis.extra["owners"].as_array().unwrap().is_empty());
        assert!(
            analysis.extra["parse_result"]["parsed"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| !node["extensions"].as_array().unwrap().is_empty())
        );
    }
    for source in ["{", r#"{"x":1,"x":2}"#, "// forbidden\n{}"] {
        let (result, _) = run(request("analyze", "json", &[source]));
        assert!(!result.ok);
        assert!(result.analysis.is_none());
    }
}

#[test]
fn common_json_analysis_reports_unclaimed_comments_and_never_merges_comment_bytes_with_code() {
    for (source, count, unresolved) in [("{/*same*/x:1,/*same*/y:2}", 2, 0), ("{} /*tail*/", 1, 1)]
    {
        let (result, _) = run(request("analyze", "json5", &[source]));
        assert!(result.ok);
        let analysis = result.analysis.unwrap().extra;
        assert_eq!(analysis["comment_regions"].as_array().unwrap().len(), count);
        assert_eq!(
            analysis["metadata"]["unresolved_comment_node_ids"].as_array().unwrap().len(),
            unresolved
        );
        if unresolved == 1 {
            assert_eq!(analysis["comment_regions"][0]["attachment_resolved"], false);
            assert_eq!(analysis["ownership"][0]["confidence"], "unresolved");
            assert_eq!(analysis["diagnostics"][0]["code"], "json.comment_attachment_unresolved");
            assert_eq!(analysis["diagnostics"][0]["blocking"], false);
        }
        for region in analysis["comment_regions"].as_array().unwrap() {
            let start = region["span"]["range"]["start_byte"].as_u64().unwrap() as usize;
            let end = region["span"]["range"]["end_byte"].as_u64().unwrap() as usize;
            assert!(matches!(&source[start..end], "/*same*/" | "/*tail*/"));
        }
    }
}

#[test]
fn common_json_analysis_declares_the_existing_shared_gap_fallback() {
    let (result, _) = run(request("analyze", "json", &["{\n \"a\":1,\n\n \"b\":2\n}"]));
    assert!(result.ok);
    let analysis = result.analysis.unwrap();
    let gap = &analysis.extra["layout_gaps"][0];
    assert_eq!(gap["before_owner_id"], "json:/a");
    assert_eq!(gap["after_owner_id"], "json:/b");
    assert_eq!(gap["controller_side"], "after");
    assert_eq!(gap["fallback_controller_side"], "before");
}

#[test]
fn common_json_analysis_rejects_tampered_decisions_and_unsupported_policies() {
    let (result, request) = run(request("analyze", "json5", &["// note\n{x:1}\n\n"]));
    assert!(result.ok);
    for mutation in 0..6 {
        let mut forged = result.clone();
        let analysis = &mut forged.analysis.as_mut().unwrap().extra;
        match mutation {
            0 => analysis.get_mut("owners").unwrap()[1]["node_id"] = json!("missing"),
            1 => {
                analysis.get_mut("comment_regions").unwrap()[0]["source_sha256"] =
                    json!("0".repeat(64))
            }
            2 => analysis.get_mut("ownership").unwrap()[0]["selected_owner_ref"] = json!("missing"),
            3 => analysis.get_mut("attachments").unwrap()[0]["owner_id"] = json!("missing"),
            4 => analysis.get_mut("layout_gaps").unwrap()[0]["controller_side"] = json!("after"),
            _ => {
                analysis.insert("comment_regions".into(), json!([]));
            }
        }
        assert!(forged.validate_against(&request).is_err(), "mutation {mutation}");
    }
    let mut future = result.clone();
    future.analysis.as_mut().unwrap().extra.get_mut("owners").unwrap()[0]["future"] = json!(true);
    future.validate_against(&request).unwrap();
    let mut unsupported = request.request().clone();
    let OperationPolicy::Analyze(policy) = &mut unsupported.operation else { panic!() };
    policy.tokens = Some(true);
    let (result, _) = run(unsupported);
    assert!(!result.ok);
    assert!(!result.extra.contains_key("input_parses"));
}

#[test]
fn common_json_diff_compares_complete_bytes_and_native_nested_subjects() {
    let (result, _) = run(request(
        "diff2",
        "json5",
        &["// old\r\n{a:{x:1},b:{x:1},gone:0}", "// new\r\n{a:{x:1},b:{x:2},added:0}"],
    ));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.output.is_none());
    assert!(result.render_report.is_empty());
    assert_ne!(result.verification.output_reparsed, Some(true));
    assert_eq!(
        result
            .changes
            .iter()
            .map(|c| (c.path.as_deref(), c.classification.as_str()))
            .collect::<Vec<_>>(),
        [
            (None, "edited"),
            (Some(""), "edited"),
            (Some("/added"), "added"),
            (Some("/b"), "edited"),
            (Some("/b/x"), "edited"),
            (Some("/gone"), "deleted")
        ]
    );
    let nested = &result.changes[4];
    assert_eq!(nested.role_states["before"]["owner"]["path"], "/b/x");
    assert_eq!(nested.role_states["before"]["source"]["role"], "before");
    assert_eq!(result.diff.unwrap().extra["document_bytes_compared"], true);
}

#[test]
fn common_json_diff_covers_trivia_only_edits_noops_scalars_and_positional_arrays() {
    for (left, right, count) in [
        ("// old\n{}", "// new\n{}", 1),
        ("{}", "{}\r\n", 1),
        ("1", "2", 2),
        ("[1,2]", "[0,1,2]", 5),
        ("{x:1}", "{x:1}", 0),
    ] {
        let (result, _) = run(request("diff2", "json5", &[left, right]));
        assert!(result.ok, "{:?}", result.diagnostics);
        assert_eq!(result.changes.len(), count, "{left:?} -> {right:?}");
        assert_eq!(
            result.verification.consumed_source_roles,
            Some(vec![SourceRole::Before, SourceRole::After])
        );
    }
}

#[test]
fn common_json_diff_rejects_ambiguous_keys_and_unsupported_equivalence() {
    let (result, _) = run(request("diff2", "json", &[r#"{"x":1,"x":2}"#, "{}"]));
    assert!(!result.ok);
    assert!(result.diff.is_none());
    let mut unsupported = request("diff2", "json", &["{}", "{}"]);
    let OperationPolicy::Diff2(policy) = &mut unsupported.operation else { panic!() };
    policy.equivalence = Some(vec!["ignore-whitespace".into()]);
    let (result, _) = run(unsupported);
    assert!(!result.ok);
    assert!(!result.extra.contains_key("input_parses"));
}

#[test]
fn common_json_diff_recomputes_evidence_and_preserves_compatible_unknown_fields() {
    let (result, request) = run(request("diff2", "json", &[r#"{"x":1}"#, r#"{"x":2}"#]));
    assert!(result.ok);
    for mutation in 0..8 {
        let mut forged = result.clone();
        match mutation {
            0 => forged.changes[1].classification = "added".into(),
            1 => {
                forged.changes[1].role_states.get_mut("before").unwrap()["owner"]["sha256"] =
                    json!("0".repeat(64))
            }
            2 => {
                forged.changes.remove(0);
                forged.diff.as_mut().unwrap().change_ids.remove(0);
            }
            3 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["source"]["role"] =
                    json!("after")
            }
            4 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["backend"]["languages"] =
                    json!(["yaml"])
            }
            5 => {
                forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["nodes"][0]["span"]["range"]
                    ["end_byte"] = json!(99999)
            }
            6 => forged
                .diff
                .as_mut()
                .unwrap()
                .extra
                .insert("document_bytes_compared".into(), json!(false))
                .map(|_| ())
                .unwrap(),
            _ => forged.verification.output_reparsed = Some(true),
        }
        assert!(forged.validate_against(&request).is_err(), "mutation {mutation}");
    }
    let mut extended = result.clone();
    extended.changes[0].extra.insert("future".into(), json!({"passive":true}));
    extended.changes[1].role_states.get_mut("before").unwrap()["owner"]["future"] =
        json!(["retained"]);
    extended.diff.as_mut().unwrap().extra.insert("future".into(), json!(true));
    extended.validate_against(&request).unwrap();
    let roundtrip: OperationResult =
        serde_json::from_value(serde_json::to_value(&extended).unwrap()).unwrap();
    assert_eq!(roundtrip, extended);
}

#[test]
fn common_directional_nested_json_has_replayed_source_evidence() {
    let (result, request) = run(request(
        "merge2",
        "json5",
        &["{x:{added:'é'},arr:[3]}", "// note\r\n{\r\n x:{keep:1},\r\n arr:[1,2]\r\n}"],
    ));
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(result.output.as_ref().unwrap().contains("// note\r\n"));
    assert!(result.output.as_ref().unwrap().contains("added"));
    assert_eq!(result.verification.directional_roles_preserved, Some(true));
    assert_eq!(result.verification.base_participated, None);
    let proof: json_merge::render_evidence::JsonRenderEvidence =
        serde_json::from_value(result.render_report["evidence"].clone()).unwrap();
    proof
        .validate(request.sources().get("current").unwrap(), result.output.as_ref().unwrap())
        .unwrap();
    assert!(!result.verification.retained_source_regions.unwrap().is_empty());
}

#[test]
fn common_json_merge_rejects_forged_input_and_output_selection_evidence() {
    for operation in ["merge2", "merge3"] {
        let texts = if operation == "merge2" {
            vec!["{\"add\":1}", "{\"keep\":2}"]
        } else {
            vec!["{\"x\":0}", "{\"x\":0}", "{\"x\":1}"]
        };
        let (result, request) = run(request(operation, "json", &texts));
        assert!(result.ok);
        for mutation in 0..8 {
            let mut forged = result.clone();
            match mutation {
                0 => {
                    forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["source"]["sha256"] =
                        json!("0".repeat(64))
                }
                1 => {
                    forged.extra.get_mut("input_parses").unwrap()[0]["parsed"]["request_id"] =
                        json!("another-request")
                }
                2 => {
                    forged.extra.get_mut("input_parses").unwrap().as_array_mut().unwrap().pop();
                }
                3 => {
                    forged.extra.get_mut("output_parse").unwrap()["parsed"]["request_id"] =
                        json!("another-request")
                }
                4 => {
                    forged.extra.get_mut("output_parse").unwrap()["selection"]["requested"]["backend_id"] =
                        json!("wrong")
                }
                5 => {
                    forged.extra.get_mut("output_parse").unwrap()["selection"]["candidates"] =
                        json!([])
                }
                6 => {
                    forged.extra.get_mut("output_parse").unwrap()["selection"]["candidates"][0]["available"] =
                        json!(false)
                }
                _ => {
                    forged.extra.get_mut("output_parse").unwrap()["selection"]["candidates"][0]["loadable"] =
                        json!(false)
                }
            }
            assert!(forged.validate_against(&request).is_err(), "{operation}, mutation {mutation}");
        }
    }
}

#[test]
fn common_json_rejects_tampered_render_and_output_parse_evidence() {
    let (result, request) = run(request("merge2", "json", &["{\"add\":1}", "{\n\"keep\":2\n}"]));
    assert!(result.ok);
    for mutation in 0..5 {
        let mut forged = result.clone();
        match mutation {
            0 => {
                forged.render_report.get_mut("evidence").unwrap()["retained"][0]["sha256"] =
                    json!("0".repeat(64))
            }
            1 => forged.output.as_mut().unwrap().push(' '),
            2 => forged.verification.retained_source_regions.as_mut().unwrap().clear(),
            3 => forged.extra.get_mut("output_parse").unwrap()["backend"]["id"] = json!("wrong"),
            _ => {
                forged.render_report.remove("evidence");
            }
        }
        assert!(forged.validate_against(&request).is_err());
    }
}

#[test]
fn common_merge3_proves_selected_theirs_and_nested_independent_changes() {
    let (selected, _) =
        run(request("merge3", "json", &["{\"x\":1}", "{\"x\":1}", "{\r\n\"x\":2\r\n}"]));
    assert!(selected.ok);
    assert_eq!(selected.render_report["evidence"]["baseline"]["role"], "theirs");
    assert_eq!(selected.verification.base_participated, Some(true));
    let (merged, _) = run(request(
        "merge3",
        "json",
        &[r#"{"x":{"a":1,"b":2}}"#, r#"{"x":{"a":3,"b":2}}"#, r#"{"x":{"a":1,"b":4}}"#],
    ));
    assert!(merged.ok, "{:?}", merged.diagnostics);
    assert_eq!(
        serde_json::from_str::<Value>(merged.output.as_ref().unwrap()).unwrap(),
        json!({"x":{"a":3,"b":4}})
    );
}

#[test]
fn common_conflicts_keep_native_categories_and_real_present_absent_regions() {
    for texts in [
        [r#"{"x":1}"#, r#"{"x":2}"#, r#"{"x":3}"#],
        [r#"{"x":1}"#, "{}", r#"{"x":3}"#],
        ["1", "2", "3"],
    ] {
        let (result, _) = run(request("merge3", "json", &texts));
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(!result.conflicts.is_empty());
        let portable_conflict::ConflictRecord::Canonical(conflict) = &result.conflicts[0] else {
            panic!()
        };
        assert!(
            !conflict.classification.extra["native_conflict"]["category"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        assert_eq!(conflict.alternatives.len(), 3);
        for alternative in &conflict.alternatives {
            if alternative.state == portable_conflict::AlternativeState::Absent {
                assert!(alternative.regions.is_empty());
            } else {
                assert!(!alternative.regions.is_empty());
            }
        }
    }
}

#[test]
fn common_json_rejects_invalid_dialects_syntax_and_unimplemented_constraints() {
    for (dialect, texts) in [
        ("json", ["{}", "{"]),
        ("json", ["{}", "// forbidden\n{}"]),
        ("jsonc", ["{}", "{unquoted:1}"]),
    ] {
        let (result, _) = run(request("merge2", dialect, &texts));
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(!result.diagnostics.is_empty());
    }
    let mut unsupported = request("merge2", "json", &["{}", "{}"]);
    unsupported.provider_selection.required_capabilities.push("invented".into());
    unsupported.provider_selection.required_capabilities.sort();
    let (result, _) = run(unsupported);
    assert!(!result.ok);
    assert!(!result.extra.contains_key("input_parses"));
}

#[test]
fn exported_facade_routes_json_and_honors_cancellation() {
    register_language_pack_parser("json.common.api".into(), "json".into()).unwrap();
    let limits = ParseLimits {
        max_batch_items: 3,
        max_input_bytes: 100000,
        max_nodes: 10000,
        max_diagnostics: 100,
        timeout_millis: None,
    };
    let control = create_operation_control();
    control.cancel();
    for operation in ["merge2", "diff2", "analyze"] {
        let mut request = request(operation, "json", &["{\"add\":1}", "{}"]);
        request.parser_selection.backend = Some("json.common.api".into());
        assert_eq!(
            execute_operation_controlled(request.clone(), limits.clone(), &control)
                .unwrap_err()
                .code,
            "execution.cancelled"
        );
        assert!(execute_operation(request, limits.clone()).unwrap().ok);
    }
    let mut request = git_request("json", &["{\"x\":0}", "{\"x\":1}", "{\"x\":2}"]);
    request.parser_selection.backend = Some("json.common.api".into());
    assert_eq!(
        execute_operation_controlled(request.clone(), limits.clone(), &control).unwrap_err().code,
        "execution.cancelled"
    );
    let result = execute_operation(request, limits).unwrap();
    assert!(!result.ok);
    assert!(result.conflicted_output.unwrap().contains("<<<<<<< ours"));
    unregister_parser_provider("json.common.api".into()).unwrap();
}

#[test]
fn output_parser_failure_never_exposes_success_or_a_render() {
    struct FailingOutput(LanguagePackProvider);
    impl ParserProvider for FailingOutput {
        fn descriptor(&self) -> &ParserProviderDescriptor {
            self.0.descriptor()
        }
        fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
            self.0.probe(request)
        }
        fn parse_batch(
            &self,
            requests: Vec<ParseRequest>,
            context: &ExecutionContext,
        ) -> Result<Vec<ParseOutput>, ProviderFault> {
            if requests.iter().any(|request| request.source.descriptor.role == SourceRole::Output) {
                return Err(ProviderFault {
                    code: "test.output_refused".into(),
                    message: "private parser exception".into(),
                });
            }
            self.0.parse_batch(requests, context)
        }
    }
    let registry = ParserRegistry::default();
    registry
        .register(Arc::new(FailingOutput(
            LanguagePackProvider::new("json.common".into(), "json".into()).unwrap(),
        )))
        .unwrap();
    let context = ExecutionContext {
        cancelled: Arc::new(AtomicBool::new(false)),
        deadline: None,
        max_batch_items: 3,
        max_input_bytes: 10000,
        max_nodes: 1000,
        max_diagnostics: 20,
    };
    for operation in ["merge2", "merge3"] {
        let texts = if operation == "merge2" { vec!["{}", "{}"] } else { vec!["{}", "{}", "{}"] };
        let request = request(operation, "json", &texts).validate(10000, |_, _| panic!()).unwrap();
        let result = native_operation::execute_native_operation(
            &request,
            &registry.snapshot().unwrap(),
            &context,
        )
        .unwrap();
        assert!(!result.ok);
        assert!(result.output.is_none());
        assert!(result.render_report.is_empty());
        let portable_diagnostic::DiagnosticRecord::Canonical(diagnostic) = &result.diagnostics[0]
        else {
            panic!()
        };
        assert_eq!(diagnostic.origin.native_code.as_deref(), Some("test.output_refused"));
        assert!(!diagnostic.message.contains("private"));
        result.validate_against(&request).unwrap();
    }
}
