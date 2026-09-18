use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const BINS: [&str; 2] = [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")];

fn fixture(grammar: Option<&Path>) -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    fs::create_dir(dir.path().join("libs")).unwrap();
    fs::create_dir(dir.path().join("cache")).unwrap();
    if let Some(grammar) = grammar {
        fs::copy(grammar, dir.path().join("libs").join(grammar.file_name().unwrap())).unwrap();
    }
    for (name, bytes) in [
        ("base", "{\"a\":1,\"b\":1}\n"),
        ("ours", "{\"a\":2,\"b\":1}\n"),
        ("theirs", "{\"a\":1,\"b\":2}\n"),
        ("report", "sentinel"),
    ] {
        fs::write(dir.path().join(name), bytes).unwrap();
    }
    dir
}

fn run(bin: &str, dir: &Path, args: &[&str]) -> Output {
    run_command(bin, dir, "merge-driver", args)
}

fn run_command(bin: &str, dir: &Path, command: &str, args: &[&str]) -> Output {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let mut child = Command::new(bin)
        .arg(command)
        .args(args)
        .current_dir(dir)
        .env("TMPDIR", dir)
        .envs(
            ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "https_proxy", "http_proxy", "all_proxy"]
                .map(|key| (key, &proxy)),
        )
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env("TREE_SITTER_LANGUAGE_PACK_LIBS_DIR", dir.join("libs"))
        .env("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", dir.join("cache"))
        .env("TREE_SITTER_LANGUAGE_PACK_CACHE_DIR", dir.join("cache"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            panic!("CLI timed out: {:?}", child.wait_with_output().unwrap());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "unexpected proxy connection"
    );
    output
}

fn diff_args() -> Vec<&'static str> {
    vec![
        "--provider",
        "kernel.json",
        "--backend",
        "kernel.tslp.json",
        "--profile",
        "kernel.json.nested.v1",
        "--json",
        "--report",
        "report",
        "base",
        "ours",
    ]
}

#[test]
fn typed_diff_cold_errors_and_invalid_invocations_are_read_only() {
    for bin in BINS {
        for scenario in
            ["cold", "provider", "merge-only-profile", "duplicate", "missing", "report-alias"]
        {
            let dir = fixture(None);
            let before = fs::read(dir.path().join("base")).unwrap();
            let after = fs::read(dir.path().join("ours")).unwrap();
            let mut invocation = diff_args();
            match scenario {
                "provider" => invocation[1] = "missing",
                "merge-only-profile" => {
                    invocation[1] = "kernel.git.json";
                    invocation[5] = "kernel.git.json.v1";
                }
                "duplicate" => invocation.push("--json"),
                "missing" => {
                    invocation.extend(["--dialect", "--json"]);
                }
                "report-alias" => invocation[8] = "ours",
                _ => {}
            }
            let output = run_command(bin, dir.path(), "diff-driver", &invocation);
            assert_eq!(output.status.code(), Some(2), "{scenario}: {output:?}");
            assert!(!output.stderr.is_empty());
            assert_eq!(fs::read(dir.path().join("base")).unwrap(), before);
            assert_eq!(fs::read(dir.path().join("ours")).unwrap(), after);
            if ["duplicate", "missing", "report-alias"].contains(&scenario) {
                assert!(output.stdout.is_empty());
                assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
            } else {
                let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(fs::read(dir.path().join("report")).unwrap(), output.stdout);
                assert_eq!(value["command"], "diff-driver");
                assert_eq!(value["outcome"], "error");
                if scenario == "cold" {
                    assert_eq!(value["operation_result"]["operation"], "diff2");
                    assert_eq!(value["operation_result"]["ok"], false);
                } else {
                    assert!(value["operation_result"].is_null());
                    let diagnostic: structuredmerge_core::PortableDiagnostic =
                        serde_json::from_value(value["diagnostics"][0].clone()).unwrap();
                    assert_eq!(
                        diagnostic.operation,
                        Some(structuredmerge_core::OperationKind::Diff2)
                    );
                    assert_eq!(diagnostic.request_id.as_deref(), Some("cli.diff2"));
                    structuredmerge_core::validate_diagnostics(
                        &[&diagnostic],
                        "cli.diff2",
                        structuredmerge_core::OperationKind::Diff2,
                        &structuredmerge_core::SourceMap::default(),
                        |_| false,
                    )
                    .unwrap();
                }
            }
            assert_eq!(fs::read_dir(dir.path().join("cache")).unwrap().count(), 0);
        }
    }
}

#[test]
#[ignore = "requires explicitly supplied existing JSON grammar; never downloads"]
fn typed_diff_uses_kernel_changes_and_git_roles_without_mutation() {
    let grammar = PathBuf::from(std::env::var_os("SMORG_TEST_JSON_GRAMMAR").unwrap());
    for bin in BINS {
        for mode in [
            "unchanged",
            "changed",
            "formatting",
            "git-seven",
            "git-nine",
            "git-added",
            "git-deleted",
            "human",
            "parse-error",
            "capability",
        ] {
            let dir = fixture(Some(&grammar));
            if mode == "unchanged" {
                fs::copy(dir.path().join("base"), dir.path().join("ours")).unwrap();
            }
            if mode == "formatting" {
                fs::write(dir.path().join("ours"), "{\r\n  \"a\":1,\"b\":1\r\n}\r\n").unwrap();
            }
            if mode == "parse-error" {
                fs::write(dir.path().join("ours"), "{broken").unwrap();
            }
            let before = fs::read(dir.path().join("base")).unwrap();
            let after = fs::read(dir.path().join("ours")).unwrap();
            let mut invocation = diff_args();
            if mode.starts_with("git-") {
                invocation.truncate(9);
                invocation.extend([
                    "logical '雪.txt",
                    "base",
                    "not-a-file-old-hash",
                    "100644",
                    "ours",
                    "not-a-file-new-hash",
                    "100644",
                ]);
                if mode == "git-added" {
                    invocation[10] = "/dev/null";
                    invocation[11] = ".";
                    invocation[12] = ".";
                }
                if mode == "git-deleted" {
                    invocation[13] = "/dev/null";
                    invocation[14] = ".";
                    invocation[15] = ".";
                }
                if mode == "git-nine" {
                    invocation.extend(["", "not-a-file-prefix/"]);
                }
            } else {
                invocation.extend(["--path-name", "logical '雪.txt"]);
            }
            invocation.extend([
                "--exit-code",
                "--require-capability",
                "diff2",
                "--require-capability",
                "diff2",
            ]);
            if mode == "human" {
                invocation.remove(6);
            }
            if mode == "capability" {
                invocation.extend(["--require-capability", "not-supported"]);
            }
            let output = run_command(bin, dir.path(), "diff-driver", &invocation);
            let error = ["parse-error", "capability"].contains(&mode);
            assert_eq!(
                output.status.code(),
                Some(if error {
                    2
                } else if mode == "unchanged" {
                    0
                } else {
                    1
                }),
                "{mode}: {output:?}"
            );
            assert_eq!(fs::read(dir.path().join("base")).unwrap(), before);
            assert_eq!(fs::read(dir.path().join("ours")).unwrap(), after);
            let bytes = fs::read(dir.path().join("report")).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["command"], "diff-driver");
            assert_eq!(value["operation_result"]["operation"], "diff2");
            if ["git-added", "git-deleted"].contains(&mode) {
                let index = if mode == "git-added" { 0 } else { 1 };
                let source = &value["operation_result"]["input_parses"][index]["parsed"]["source"];
                assert_eq!(source["byte_length"], 0);
                assert_eq!(
                    source["sha256"],
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                );
                let classification = if mode == "git-added" { "added" } else { "deleted" };
                assert!(
                    value["operation_result"]["changes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|c| c["classification"] == classification)
                );
            }
            assert_eq!(
                value["operation_result"]["request_forwarding"]["path_name"],
                "logical '雪.txt"
            );
            if !error {
                assert_eq!(
                    value["operation_result"]["provider"]["provider_id"], "kernel.json",
                    "{mode}"
                );
            }
            assert_eq!(
                value["outcome"],
                if error {
                    "error"
                } else if mode == "unchanged" {
                    "clean"
                } else {
                    "changed"
                }
            );
            if mode == "human" {
                assert!(String::from_utf8(output.stdout).unwrap().contains("status changed"));
            } else {
                assert_eq!(output.stdout, bytes);
            }
            if !error {
                assert_eq!(
                    value["operation_result"]["diff"]["change_ids"].as_array().unwrap().is_empty(),
                    mode == "unchanged"
                );
            }
        }
    }
}

fn args() -> Vec<&'static str> {
    vec![
        "--provider",
        "kernel.json",
        "--backend",
        "kernel.tslp.json",
        "--profile",
        "kernel.json.nested.v1",
        "--report",
        "report",
        "base",
        "ours",
        "theirs",
        "not-json.txt",
    ]
}

fn error_report(dir: &Path, category: &str, code: &str) {
    let bytes = fs::read(dir.join("report")).unwrap();
    assert!(bytes.ends_with(b"\n"));
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema"], "structuredmerge.cli-report/v1");
    assert_eq!(value["outcome"], "error");
    assert_eq!(value["exit_code"], 2);
    assert!(value["operation_result"].is_null());
    assert_eq!(value["output_commit_verified"], false);
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(value["diagnostics"][0]["category"], category);
    assert_eq!(value["diagnostics"][0]["code"], code);
    let diagnostic: structuredmerge_core::PortableDiagnostic =
        serde_json::from_value(value["diagnostics"][0].clone()).unwrap();
    assert!(diagnostic.blocking);
    assert!(diagnostic.origin.provider_id.is_none());
    assert!(diagnostic.origin.backend_id.is_none());
    structuredmerge_core::validate_diagnostics(
        &[&diagnostic],
        "cli.merge3",
        structuredmerge_core::OperationKind::Merge3,
        &structuredmerge_core::SourceMap::default(),
        |_| false,
    )
    .unwrap();
}

#[test]
fn explicit_selection_rejects_constraints_without_legacy_fallback_or_writes() {
    for bin in BINS {
        for (option, value) in [
            ("--provider", "unknown"),
            ("--backend", "unknown"),
            ("--profile", "unknown"),
            ("--family", "yaml"),
            ("--dialect", "yaml"),
            ("--fallback", "full-file"),
            ("--conflict-policy", "bad"),
            ("--require-profile-status", "default"),
        ] {
            let dir = fixture(None);
            let mut invocation = args();
            if let Some(index) = invocation.iter().position(|s| *s == option) {
                invocation[index + 1] = value;
            } else {
                invocation.extend([option, value]);
            }
            let output = run(bin, dir.path(), &invocation);
            assert_eq!(output.status.code(), Some(2), "{option}: {output:?}");
            assert!(!output.stderr.is_empty());
            assert_eq!(fs::read_to_string(dir.path().join("ours")).unwrap(), "{\"a\":2,\"b\":1}\n");
            if ["--provider", "--backend", "--profile", "--family", "--dialect"].contains(&option) {
                error_report(dir.path(), "selection_error", "cli.selection_rejected");
            } else {
                assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
            }
            assert_eq!(fs::read_dir(dir.path().join("cache")).unwrap().count(), 0);
        }
        for extra in [
            vec!["--output", "base"],
            vec!["--output", "theirs"],
            vec!["--exit-code"],
            vec!["--profile-report"],
        ] {
            let dir = fixture(None);
            let mut invocation = args();
            invocation.extend(extra);
            assert_eq!(run(bin, dir.path(), &invocation).status.code(), Some(2));
            assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
        }
    }
}

#[test]
fn pre_execution_source_errors_report_without_a_fabricated_operation_result() {
    for bin in BINS {
        for scenario in ["missing", "directory", "utf8", "limit"] {
            let dir = fixture(None);
            match scenario {
                "missing" => fs::remove_file(dir.path().join("base")).unwrap(),
                "directory" => {
                    fs::remove_file(dir.path().join("base")).unwrap();
                    fs::create_dir(dir.path().join("base")).unwrap();
                }
                "utf8" => fs::write(dir.path().join("base"), [255]).unwrap(),
                _ => fs::File::create(dir.path().join("base"))
                    .unwrap()
                    .set_len(8 * 1024 * 1024 + 1)
                    .unwrap(),
            }
            let output = run(bin, dir.path(), &args());
            assert_eq!(output.status.code(), Some(2), "{scenario}: {output:?}");
            assert!(!output.stderr.is_empty());
            error_report(
                dir.path(),
                if scenario == "limit" { "resource_limit" } else { "invalid_request" },
                if scenario == "limit" { "cli.source_limit" } else { "cli.source_rejected" },
            );
            assert_eq!(fs::read_to_string(dir.path().join("ours")).unwrap(), "{\"a\":2,\"b\":1}\n");
            assert_eq!(fs::read_dir(dir.path().join("cache")).unwrap().count(), 0);
        }
    }
}

#[test]
fn rejected_selection_still_obeys_report_alias_and_staging_safety() {
    for bin in BINS {
        for scenario in ["alias", "directory", "malformed"] {
            let dir = fixture(None);
            let mut invocation = args();
            invocation[1] = "not-a-provider";
            match scenario {
                "alias" => {
                    fs::remove_file(dir.path().join("report")).unwrap();
                    fs::hard_link(dir.path().join("ours"), dir.path().join("report")).unwrap();
                }
                "directory" => {
                    fs::remove_file(dir.path().join("report")).unwrap();
                    fs::create_dir(dir.path().join("report")).unwrap();
                }
                _ => invocation.extend(["--provider", "duplicate"]),
            }
            let output = run(bin, dir.path(), &invocation);
            assert_eq!(
                output.status.code(),
                Some(if scenario == "directory" { 3 } else { 2 }),
                "{output:?}"
            );
            assert_eq!(fs::read_to_string(dir.path().join("ours")).unwrap(), "{\"a\":2,\"b\":1}\n");
            if scenario == "malformed" {
                assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
            }
            assert!(!fs::read_dir(dir.path()).unwrap().any(|entry| {
                entry.unwrap().file_name().to_string_lossy().starts_with(".smorg-write-")
            }));
        }
    }
}

#[test]
fn cold_cache_returns_error_result_not_conflict_or_text_fallback() {
    for bin in BINS {
        let dir = fixture(None);
        let output = run(bin, dir.path(), &args());
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert_eq!(fs::read_to_string(dir.path().join("ours")).unwrap(), "{\"a\":2,\"b\":1}\n");
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.path().join("report")).unwrap()).unwrap();
        assert_eq!(report["outcome"], "error");
        assert_eq!(report["operation_result"]["ok"], false);
        assert_eq!(report["output_commit_verified"], false);
        assert_eq!(fs::read_dir(dir.path().join("cache")).unwrap().count(), 0);
    }
}

#[test]
#[ignore = "requires explicitly supplied existing JSON grammar; never downloads"]
fn warm_cached_typed_merge_and_conflict_policies() {
    let grammar = PathBuf::from(
        std::env::var_os("SMORG_TEST_JSON_GRAMMAR").expect("set existing grammar path"),
    );
    for bin in BINS {
        for check in [false, true] {
            let dir = fixture(Some(&grammar));
            let mut invocation = args();
            invocation.extend(["--require-capability", "merge3", "--require-capability", "merge3"]);
            if check {
                invocation.extend(["--check-only", "--exit-code"]);
            }
            let output = run(bin, dir.path(), &invocation);
            assert_eq!(output.status.code(), Some(if check { 1 } else { 0 }), "{output:?}");
            let report: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.path().join("report")).unwrap()).unwrap();
            assert_eq!(report["schema"], "structuredmerge.cli-report/v1");
            assert_eq!(report["outcome"], "changed");
            assert_eq!(report["operation_result"]["provider"]["provider_id"], "kernel.json");
            let actual: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.path().join("ours")).unwrap()).unwrap();
            assert_eq!(actual, serde_json::json!({"a":2,"b":if check {1} else {2}}));
        }
        for write in [false, true] {
            let dir = fixture(Some(&grammar));
            fs::write(dir.path().join("theirs"), "{\"a\":3,\"b\":1}\n").unwrap();
            let mut invocation = args();
            invocation[1] = "kernel.git.json";
            invocation[5] = "kernel.git.json.v1";
            if write {
                invocation.extend(["--conflict-policy", "write"]);
            }
            let output = run(bin, dir.path(), &invocation);
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            let report: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.path().join("report")).unwrap()).unwrap();
            assert_eq!(report["outcome"], "conflict");
            let actual = fs::read_to_string(dir.path().join("ours")).unwrap();
            if write {
                assert_eq!(
                    actual,
                    report["operation_result"]["conflicted_output"].as_str().unwrap()
                );
            } else {
                assert_eq!(actual, "{\"a\":2,\"b\":1}\n");
            }
        }
    }
}

#[test]
#[ignore = "requires explicitly supplied existing JSON grammar; never downloads"]
fn warm_rejections_and_staging_preserve_sources() {
    let grammar = PathBuf::from(std::env::var_os("SMORG_TEST_JSON_GRAMMAR").unwrap());
    for bin in BINS {
        for scenario in [
            "invalid-utf8",
            "oversize",
            "parse-error",
            "capability",
            "report-alias",
            "output-failure",
            "report-failure",
        ] {
            let dir = fixture(Some(&grammar));
            let mut invocation = args();
            let expected = match scenario {
                "invalid-utf8" => {
                    fs::write(dir.path().join("base"), [255]).unwrap();
                    2
                }
                "oversize" => {
                    fs::File::create(dir.path().join("base"))
                        .unwrap()
                        .set_len(8 * 1024 * 1024 + 1)
                        .unwrap();
                    2
                }
                "parse-error" => {
                    fs::write(dir.path().join("base"), "{broken").unwrap();
                    2
                }
                "capability" => {
                    invocation.extend(["--require-capability", "not-supported"]);
                    2
                }
                "report-alias" => {
                    fs::remove_file(dir.path().join("report")).unwrap();
                    fs::hard_link(dir.path().join("ours"), dir.path().join("report")).unwrap();
                    2
                }
                "output-failure" => {
                    fs::create_dir(dir.path().join("destination")).unwrap();
                    invocation.extend(["--output", "destination"]);
                    3
                }
                _ => {
                    fs::remove_file(dir.path().join("report")).unwrap();
                    fs::create_dir(dir.path().join("report")).unwrap();
                    3
                }
            };
            let before: Vec<_> = ["base", "ours", "theirs"]
                .iter()
                .map(|p| fs::read(dir.path().join(p)).unwrap())
                .collect();
            let output = run(bin, dir.path(), &invocation);
            assert_eq!(output.status.code(), Some(expected), "{scenario}: {output:?}");
            assert!(!output.stderr.is_empty(), "{scenario}");
            for (index, path) in ["base", "ours", "theirs"].iter().enumerate() {
                assert_eq!(fs::read(dir.path().join(path)).unwrap(), before[index], "{scenario}");
            }
            if scenario == "output-failure" {
                assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
            }
            assert!(!fs::read_dir(dir.path()).unwrap().any(|entry| {
                entry.unwrap().file_name().to_string_lossy().starts_with(".smorg-write-")
            }));
        }
    }
}
