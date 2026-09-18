//! Exercise actual Cargo-built entry points, not only the in-process router.
use std::process::Command;

#[test]
fn version_identifies_each_binary_and_linked_kernel_without_external_tools() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    for (executable, name) in
        [(env!("CARGO_BIN_EXE_smorg"), "smorg"), (env!("CARGO_BIN_EXE_smorg-rs"), "smorg-rs")]
    {
        let invoke = |args: &[&str]| {
            Command::new(executable)
                .args(args)
                .current_dir(dir.path())
                .env("PATH", dir.path())
                .env("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", dir.path().join("grammars"))
                .output()
                .unwrap()
        };
        let output = invoke(&["--version", "--json"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty());
        assert!(output.stdout.ends_with(b"\n"));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["schema"], "structuredmerge.cli-version/v1");
        assert_eq!(value["executable"], name);
        assert_eq!(value["package"], "smorg");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        let linked = structuredmerge_core::capability_manifest(
            Vec::new(),
            structuredmerge_core::ParseLimits {
                max_batch_items: 1,
                max_input_bytes: 1,
                max_nodes: 1,
                max_diagnostics: 1,
                timeout_millis: None,
            },
        )
        .unwrap();
        assert_eq!(value["kernel_version"], linked.kernel_version);
        assert_eq!(value["cli_contract"], "structuredmerge.cli/v1");
        let inventory = structuredmerge_core::artifact_inventory::compiled_provider_inventory();
        assert_eq!(value["compiled_providers"], serde_json::to_value(inventory).unwrap());
        assert_eq!(value["compiled_providers"]["runtime_availability_checked"], false);
        assert!(output.stdout.len() < 65536, "version observation must fit the assembly budget");
        let build = &value["build"];
        assert_eq!(build["schema"], "structuredmerge.cli-build/v1");
        for key in ["target", "host", "cargo_profile", "cargo_opt_level", "cargo_debug"] {
            assert!(!build[key].as_str().unwrap().is_empty());
        }
        assert!(build["cargo_feature_flags"].is_array());
        assert!(build["target_features"].is_array());
        assert_eq!(build["provenance_verified"], false);
        assert_eq!(build["source"]["verified"], false);
        let altered = Command::new(executable)
            .args(["--version", "--json"])
            .current_dir(dir.path())
            .env("PATH", dir.path())
            .env("TARGET", "runtime-cannot-change-build-target")
            .env("SMORG_BUILD_REVISION", "invalid-runtime-value")
            .env("SMORG_BUILD_SOURCE_STATE", "clean")
            .output()
            .unwrap();
        assert!(altered.status.success());
        let altered: serde_json::Value = serde_json::from_slice(&altered.stdout).unwrap();
        assert_eq!(altered["build"], *build);
        let text = invoke(&["--version"]);
        assert_eq!(text.status.code(), Some(0));
        assert!(String::from_utf8(text.stdout).unwrap().starts_with(&format!("{name} ")));
        for invalid in [
            vec!["--version", "--unknown"],
            vec!["--version", "--json", "--json"],
            vec!["--version", "base", "ours", "theirs", "a.json"],
        ] {
            let output = invoke(&invalid);
            assert_eq!(output.status.code(), Some(2), "{invalid:?}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[test]
fn option_terminators_named_sources_and_empty_git_prefixes_remain_valid() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    for name in ["--base.json", "--ours.json", "--theirs.json"] {
        std::fs::write(dir.path().join(name), b"{\"a\":1}\n").unwrap();
    }
    for executable in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for args in [
            vec![
                "merge-driver",
                "--strict",
                "--",
                "--base.json",
                "--ours.json",
                "--theirs.json",
                "a.json",
            ],
            vec![
                "merge-driver",
                "--ancestor",
                "./--base.json",
                "--current",
                "./--ours.json",
                "--other",
                "./--theirs.json",
                "--path-name",
                "a.json",
            ],
            vec!["diff-driver", "--", "--base.json", "--ours.json"],
            vec![
                "diff-driver",
                "a.json",
                "./--base.json",
                "0000000",
                "100644",
                "./--ours.json",
                "0000000",
                "100644",
                "",
                "",
            ],
            vec!["merge-driver", "--help"],
            vec!["diff-driver", "--help"],
        ] {
            let output =
                Command::new(executable).args(&args).current_dir(dir.path()).output().unwrap();
            assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
            assert_eq!(std::fs::read(dir.path().join("--ours.json")).unwrap(), b"{\"a\":1}\n");
        }
    }
}

#[test]
fn shared_cli_argument_rejections_preserve_every_input() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/conformance/cli-v1/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    for executable in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for case in manifest["cases"].as_array().unwrap() {
            // Discovery reports have separate, still-open implementation gates.
            if case["expect"]["exit_code"] != 2 {
                continue;
            }
            let dir = tempfile::tempdir_in(&root).unwrap();
            for (name, text) in manifest["files"].as_object().unwrap() {
                std::fs::write(dir.path().join(name), text.as_str().unwrap()).unwrap();
            }
            let args: Vec<_> =
                case["argv"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            let output =
                Command::new(executable).args(args).current_dir(dir.path()).output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{}: {output:?}", case["id"]);
            assert!(output.stdout.is_empty(), "{}: {output:?}", case["id"]);
            assert!(!output.stderr.is_empty(), "{}: {output:?}", case["id"]);
            for (name, text) in manifest["files"].as_object().unwrap() {
                assert_eq!(
                    std::fs::read(dir.path().join(name)).unwrap(),
                    text.as_str().unwrap().as_bytes(),
                    "{}: {name}",
                    case["id"]
                );
            }
        }
    }
}

#[test]
fn malformed_merge_options_never_write_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    for (name, content) in
        [("base", "{\"a\":1}\n"), ("ours", "{\"a\":1}\n"), ("theirs", "{\"a\":2}\n")]
    {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    for executable in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for option in [
            "--ancestor",
            "--current",
            "--other",
            "--path-name",
            "--output",
            "--report",
            "--profile",
            "--require-profile-status",
            "--fallback",
        ] {
            for tail in [vec![option], vec![option, "--check-only"], vec![option, ""]] {
                let output = Command::new(executable)
                    .current_dir(dir.path())
                    .args(["merge-driver", "base", "ours", "theirs", "file.json"])
                    .args(&tail)
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(2), "{tail:?}: {output:?}");
                assert!(output.stdout.is_empty());
                assert!(!output.stderr.is_empty());
                assert_eq!(
                    std::fs::read(dir.path().join("ours")).unwrap(),
                    b"{\"a\":1}\n",
                    "{tail:?}"
                );
            }
        }
        let output = Command::new(executable)
            .current_dir(dir.path())
            .args([
                "merge-driver",
                "base",
                "ours",
                "theirs",
                "file.json",
                "--require-profile-status",
                "recomended",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert_eq!(std::fs::read(dir.path().join("ours")).unwrap(), b"{\"a\":1}\n");
    }
}

#[test]
fn invalid_attribute_promotion_status_never_writes_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    std::fs::write(
        dir.path().join(".gitattributes"),
        "*.json smorg.requireProfileStatus=recomended\n",
    )
    .unwrap();
    for (name, text) in
        [("base", "{\"a\":1}\n"), ("ours", "{\"a\":1}\n"), ("theirs", "{\"a\":2}\n")]
    {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    for executable in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        let output = Command::new(executable)
            .current_dir(dir.path())
            .args(["merge-driver", "base", "ours", "theirs", "file.json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unsupported required profile status")
        );
        assert_eq!(std::fs::read(dir.path().join("ours")).unwrap(), b"{\"a\":1}\n");
    }
}

#[test]
fn canonical_and_compatibility_executables_preserve_command_results() {
    for args in [
        vec!["--help"],
        vec!["languages", "--gitattributes"],
        vec!["merge-driver"],
        vec!["definitely-not-a-command"],
    ] {
        let canonical = Command::new(env!("CARGO_BIN_EXE_smorg")).args(&args).output().unwrap();
        let compatibility =
            Command::new(env!("CARGO_BIN_EXE_smorg-rs")).args(&args).output().unwrap();
        assert_eq!(canonical.status.code(), compatibility.status.code(), "{args:?}");
        assert_eq!(canonical.stdout, compatibility.stdout, "{args:?}");
        assert_eq!(canonical.stderr, compatibility.stderr, "{args:?}");
        if args[0] == "--help" {
            assert!(canonical.status.success());
            assert!(String::from_utf8(canonical.stdout).unwrap().starts_with("smorg:"));
        } else if args[0] == "languages" {
            assert!(canonical.status.success());
            // Driver selection is deliberately not promoted by a naming change.
            assert!(String::from_utf8(canonical.stdout).unwrap().contains("merge=smorg-rs"));
        } else {
            assert_eq!(canonical.status.code(), Some(2));
        }
    }
}
