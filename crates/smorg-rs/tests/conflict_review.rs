use serde_json::Value;
use std::{path::Path, process::Command};

fn temporary() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

#[test]
fn shared_conflict_review_cases_preserve_bytes_and_report_typed_ranges() {
    let fixtures: Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/conformance/cli-v1/conflict-review.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fixtures["schema"], "structuredmerge.cli-conflict-review-cases/v1");
    for binary in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for case in fixtures["cases"].as_array().unwrap() {
            let directory = temporary();
            let source = case["source"].as_str().unwrap().as_bytes();
            std::fs::write(directory.path().join("--source.txt"), source).unwrap();
            let attributes = format!(
                "*.txt conflict-marker-size={}\n",
                case["marker_size"].as_u64().unwrap_or(7)
            );
            std::fs::write(directory.path().join(".gitattributes"), &attributes).unwrap();
            let mut command = Command::new(binary);
            command
                .current_dir(directory.path())
                .env("PATH", directory.path())
                .env("TREE_HAVER_LANGUAGE_PACK_CACHE_DIR", directory.path().join("cache"))
                .args(["conflicts", "diff", "--json", "--path-name", "logical '雪.txt"]);
            if case["exit_code"] == true {
                command.arg("--exit-code");
            }
            let output = command.args(["--", "--source.txt"]).output().unwrap();
            assert_eq!(
                output.status.code().unwrap() as i64,
                case["expected"]["exit"].as_i64().unwrap(),
                "{}: {output:?}",
                case["id"]
            );
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["schema"], "structuredmerge.cli-report/v1");
            assert_eq!(report["command"], "conflicts.diff");
            assert_eq!(report["outcome"], case["expected"]["outcome"]);
            assert_eq!(report["exit_code"], case["expected"]["exit"]);
            assert_eq!(report["path_name"], "logical '雪.txt");
            for field in ["operation_result", "availability", "git_install"] {
                assert!(report[field].is_null());
            }
            let review = &report["conflict_review"];
            if case["expected"]["exit"] == 2 {
                assert!(review.is_null());
                assert_eq!(report["diagnostics"][0]["code"], case["expected"]["code"]);
                let _: structuredmerge_core::PortableDiagnostic =
                    serde_json::from_value(report["diagnostics"][0].clone()).unwrap();
                assert!(!output.stderr.is_empty());
            } else {
                assert!(output.stderr.is_empty());
                assert_eq!(review["source"]["byte_length"], source.len());
                let descriptor = structuredmerge_core::source_input(
                    "review".into(),
                    structuredmerge_core::SourceRole::Source,
                    structuredmerge_core::SourceEncoding::Utf8,
                    source.to_vec(),
                )
                .unwrap()
                .descriptor;
                assert_eq!(review["source"]["sha256"], descriptor.sha256);
                assert_eq!(review["semantic_conflicts_verified"], false);
                let regions = review["regions"].as_array().unwrap();
                let expected = case["expected"]["regions"].as_array().unwrap();
                assert_eq!(regions.len(), expected.len());
                let mut previous_end = 0;
                for (region, expected) in regions.iter().zip(expected) {
                    let start = region["range"]["start_byte"].as_u64().unwrap() as usize;
                    let end = region["range"]["end_byte"].as_u64().unwrap() as usize;
                    assert!(previous_end <= start && start <= end && end <= source.len());
                    previous_end = end;
                    for role in ["ours", "base", "theirs"] {
                        if expected[role].is_null() {
                            assert!(region[role].is_null());
                            continue;
                        }
                        let first = region[role]["start_byte"].as_u64().unwrap() as usize;
                        let last = region[role]["end_byte"].as_u64().unwrap() as usize;
                        assert!(start <= first && first <= last && last <= end);
                        assert_eq!(
                            &source[first..last],
                            expected[role].as_str().unwrap().as_bytes()
                        );
                    }
                }
            }
            assert_eq!(std::fs::read(directory.path().join("--source.txt")).unwrap(), source);
            assert_eq!(
                std::fs::read_to_string(directory.path().join(".gitattributes")).unwrap(),
                attributes
            );
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
        }
    }
}

#[test]
fn invalid_options_fail_before_reading_or_emitting_json() {
    for binary in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for arguments in [
            vec!["--json", "--json", "missing"],
            vec!["--path-name"],
            vec!["--path-name", "--json", "missing"],
            vec!["--path-name", "", "missing"],
            vec!["--exit-code", "--exit-code", "missing"],
            vec!["--json", "missing", "extra"],
        ] {
            let output =
                Command::new(binary).args(["conflicts", "diff"]).args(arguments).output().unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
        }
    }
}

#[test]
fn missing_nonregular_invalid_utf8_and_oversized_sources_fail_closed() {
    let directory = temporary();
    std::fs::write(directory.path().join("invalid"), [255]).unwrap();
    std::fs::File::create(directory.path().join("large"))
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    for binary in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for (path, code) in [
            ("missing", "conflict.source_rejected"),
            (".", "conflict.source_rejected"),
            ("invalid", "conflict.invalid_utf8"),
            ("large", "conflict.input_limit"),
        ] {
            let output = Command::new(binary)
                .current_dir(directory.path())
                .args(["conflicts", "diff", "--json", path])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(report["conflict_review"].is_null());
            assert_eq!(report["diagnostics"][0]["code"], code);
        }
    }
    assert_eq!(std::fs::read(directory.path().join("invalid")).unwrap(), [255]);
    assert_eq!(
        std::fs::metadata(directory.path().join("large")).unwrap().len(),
        8 * 1024 * 1024 + 1
    );
}
