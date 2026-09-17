use std::{fs, process::Command};

fn scratch() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root.canonicalize().unwrap()).unwrap()
}

#[test]
fn missing_and_invalid_commands_fail_without_legacy_file_interpretation() {
    let dir = scratch();
    let ours = dir.path().join("ours");
    fs::write(&ours, "must not change").unwrap();
    for name in ["py", "../escape", "--unknown", "", "x;y", "UPPER"] {
        let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .arg(name)
            .args(["base", ours.to_str().unwrap(), "theirs", "file.json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(fs::read_to_string(&ours).unwrap(), "must not change");
    }
}

#[test]
fn explicit_benchmark_command_retains_legacy_failure_behavior() {
    let dir = scratch();
    let positional = ["missing-base", "missing-ours", "missing-theirs", "file.json"];
    let legacy = Command::new(env!("CARGO_BIN_EXE_smorg-rs"))
        .current_dir(dir.path())
        .args(positional)
        .output()
        .unwrap();
    let explicit = Command::new(env!("CARGO_BIN_EXE_smorg"))
        .current_dir(dir.path())
        .arg("benchmark-provider-merge3")
        .args(positional)
        .output()
        .unwrap();
    assert!(!legacy.status.success());
    assert_eq!(explicit.status.code(), legacy.status.code());
    assert_eq!(explicit.stdout, legacy.stdout);
    assert_eq!(explicit.stderr, legacy.stderr);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{
        ffi::OsString,
        io::Write,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt, process::ExitStatusExt},
        process::Stdio,
    };

    fn executable(dir: &std::path::Path, name: &str, body: &str) {
        let path = dir.join(format!("smorg-{name}"));
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn external_process_inherits_native_arguments_streams_environment_and_directory() {
        let dir = scratch();
        executable(
            dir.path(),
            "py",
            "printf '%s\\n' \"$PWD\" \"$SMORG_DISPATCH_TEST\"; printf '<%s>\\n' \"$@\"; IFS= read -r line; printf '%s\\n' \"$line\"; printf 'child-error\\n' >&2; exit 23",
        );
        let mut child = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .env("SMORG_DISPATCH_TEST", "inherited")
            .current_dir(dir.path())
            .args(["py", "two words", "$(touch should-not-exist)", "", "--flag"])
            .arg(OsString::from_vec(vec![0xff, b'x']))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"standard input\n").unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(23));
        let mut expected = format!(
            "{}\ninherited\n<two words>\n<$(touch should-not-exist)>\n<>\n<--flag>\n<",
            dir.path().display()
        )
        .into_bytes();
        expected.extend_from_slice(b"\xffx>\nstandard input\n");
        assert_eq!(output.stdout, expected);
        assert_eq!(output.stderr, b"child-error\n");
        assert!(!dir.path().join("should-not-exist").exists());
    }

    #[test]
    fn builtins_cannot_be_shadowed_and_non_executable_targets_fail() {
        let dir = scratch();
        executable(dir.path(), "help", "exit 55");
        let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .arg("help")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains("StructuredMerge"));
        fs::write(dir.path().join("smorg-cloud"), "not executable").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .arg("cloud")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8(output.stderr).unwrap().contains("cannot execute smorg-cloud"));
    }

    #[test]
    fn unix_signal_termination_is_preserved() {
        let dir = scratch();
        executable(dir.path(), "cloud", "kill -TERM $$");
        let status = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .arg("cloud")
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(15));
    }

    #[test]
    fn executable_text_without_an_interpreter_is_not_evaluated_as_shell() {
        let dir = scratch();
        let path = dir.path().join("smorg-cloud");
        fs::write(&path, "exit 47\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .env("PATH", dir.path())
            .arg("cloud")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }

    #[test]
    fn builtin_non_utf8_arguments_are_rejected_without_panicking() {
        let output = Command::new(env!("CARGO_BIN_EXE_smorg"))
            .arg("merge-driver")
            .arg(OsString::from_vec(vec![0xff]))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8(output.stderr).unwrap().contains("must be UTF-8"));
    }

    #[test]
    fn path_search_continues_past_missing_and_denied_entries_but_requires_path() {
        let dir = scratch();
        let denied = dir.path().join("denied");
        let valid = dir.path().join("valid");
        fs::create_dir_all(&denied).unwrap();
        fs::create_dir_all(&valid).unwrap();
        fs::write(denied.join("smorg-py"), "not executable").unwrap();
        executable(&valid, "py", "exit 29");
        let path = std::env::join_paths([dir.path().join("missing"), denied, valid]).unwrap();
        for binary in [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")] {
            let status = Command::new(binary).env("PATH", &path).arg("py").status().unwrap();
            assert_eq!(status.code(), Some(29));
            let output = Command::new(binary).env_remove("PATH").arg("py").output().unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(String::from_utf8(output.stderr).unwrap().contains("PATH is unset"));
        }
    }
}
