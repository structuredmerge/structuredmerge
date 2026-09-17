use std::{fs, process::Command};

fn scratch() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

#[test]
fn report_paths_cannot_overwrite_sources_or_output() {
    for executable in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
        for report in ["base", "ours", "theirs", "./ours", "result", "./result"] {
            for check_only in [false, true] {
                let dir = scratch();
                for (name, text) in
                    [("base", "{\"a\":1}\n"), ("ours", "{\"a\":1}\n"), ("theirs", "{\"a\":2}\n")]
                {
                    fs::write(dir.path().join(name), text).unwrap();
                }
                let mut command = Command::new(executable);
                command.current_dir(dir.path()).args([
                    "merge-driver",
                    "base",
                    "ours",
                    "theirs",
                    "file.json",
                    "--output",
                    "result",
                    "--report",
                    report,
                ]);
                if check_only {
                    command.arg("--check-only");
                }
                let output = command.output().unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(2),
                    "report={report} check={check_only}: {output:?}"
                );
                assert_eq!(fs::read(dir.path().join("base")).unwrap(), b"{\"a\":1}\n");
                assert_eq!(fs::read(dir.path().join("ours")).unwrap(), b"{\"a\":1}\n");
                assert_eq!(fs::read(dir.path().join("theirs")).unwrap(), b"{\"a\":2}\n");
                assert!(!dir.path().join("result").exists());
            }
        }
    }
}

#[test]
fn report_hard_links_cannot_overwrite_current() {
    let dir = scratch();
    for name in ["base", "ours", "theirs"] {
        fs::write(dir.path().join(name), "{}\n").unwrap();
    }
    fs::hard_link(dir.path().join("ours"), dir.path().join("report")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
        .current_dir(dir.path())
        .args(["merge-driver", "base", "ours", "theirs", "file.json", "--report", "report"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(fs::read(dir.path().join("ours")).unwrap(), b"{}\n");
}

#[cfg(unix)]
#[test]
fn report_symlinks_cannot_overwrite_current() {
    let dir = scratch();
    for name in ["base", "ours", "theirs"] {
        fs::write(dir.path().join(name), "{}\n").unwrap();
    }
    std::os::unix::fs::symlink("ours", dir.path().join("report")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
        .current_dir(dir.path())
        .args(["merge-driver", "base", "ours", "theirs", "file.json", "--report", "report"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(fs::read(dir.path().join("ours")).unwrap(), b"{}\n");
}
