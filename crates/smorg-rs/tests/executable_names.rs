//! Exercise actual Cargo-built entry points, not only the in-process router.
use std::process::Command;

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
