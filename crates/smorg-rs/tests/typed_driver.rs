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
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let mut child = Command::new(bin)
        .arg("merge-driver")
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
            assert_eq!(fs::read_to_string(dir.path().join("report")).unwrap(), "sentinel");
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
