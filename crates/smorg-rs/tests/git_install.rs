use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const BINS: [&str; 2] = [env!("CARGO_BIN_EXE_smorg"), env!("CARGO_BIN_EXE_smorg-rs")];

fn temporary() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new().prefix("git install '雪 ").tempdir_in(root).unwrap()
}
fn command(executable: &str, root: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(root);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command.env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", root.join("global-config"));
    command
}
fn run(binary: &str, root: &Path, args: &[&str]) -> (Output, Value) {
    let output =
        command(binary, root).args(["git", "install", "--json"]).args(args).output().unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}"));
    (output, value)
}
fn succeeds(binary: &str, root: &Path, args: &[&str]) -> Value {
    let (output, value) = run(binary, root, args);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    assert_eq!(value["schema"], "structuredmerge.cli-report/v1");
    assert_eq!(value["command"], "git.install");
    assert_eq!(value["git_install"]["default_approved"], false);
    value
}
fn git(root: &Path, args: &[&str]) -> Output {
    command("git", root).args(args).output().unwrap()
}

#[test]
fn local_install_check_dry_run_and_undo_preserve_unowned_exact_bytes() {
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        let path = root.join(".gitattributes");
        let original = b"# user\r\n\r\n*.json merge=smorg-rs diff=smorg-rs smorg.language=json\r\nlast-no-newline";
        fs::write(&path, original).unwrap();
        succeeds(binary, root, &["--dry-run"]);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
        assert_eq!(run(binary, root, &["--check"]).0.status.code(), Some(2));
        succeeds(binary, root, &[]);
        let installed = fs::read(&path).unwrap();
        assert!(installed.starts_with(original));
        succeeds(binary, root, &["--check"]);
        assert_eq!(succeeds(binary, root, &[])["outcome"], "clean");
        succeeds(binary, root, &["--undo", "--dry-run"]);
        assert_eq!(fs::read(&path).unwrap(), installed);
        let tail = b"# user tail\r\n";
        fs::write(&path, [installed.as_slice(), tail].concat()).unwrap();
        succeeds(binary, root, &["--undo"]);
        assert_eq!(fs::read(&path).unwrap(), [original.as_slice(), b"\n", tail].concat());
        succeeds(binary, root, &["--undo"]);
    }
}

#[test]
fn created_and_preexisting_empty_files_have_distinct_undo_behavior() {
    for binary in BINS {
        for existing in [false, true] {
            let directory = temporary();
            let root = directory.path();
            let path = root.join(".gitattributes");
            if existing {
                fs::write(&path, b"").unwrap();
            }
            succeeds(binary, root, &["--profile", "builtin-diff"]);
            succeeds(binary, root, &["--profile", "builtin-diff", "--undo"]);
            assert_eq!(path.exists(), existing);
            if existing {
                assert_eq!(fs::read(path).unwrap(), b"");
            }
        }
    }
}

#[test]
fn global_scope_uses_isolated_git_location_and_does_not_install_cat() {
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        let path = root.join("global-config");
        let original = b"# global user\r\n[alias]\n st = status";
        fs::write(&path, original).unwrap();
        succeeds(binary, root, &["--scope", "global", "--dry-run"]);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!root.join(".gitattributes").exists());
        succeeds(binary, root, &["--scope", "global"]);
        succeeds(binary, root, &["--scope", "global", "--check"]);
        assert_eq!(
            git(root, &["config", "--global", "--get", "diff.smorg-rs.command"]).stdout,
            b"smorg-rs diff-driver\n"
        );
        succeeds(binary, root, &["--scope", "global", "--profile", "builtin-diff"]);
        assert_eq!(
            git(root, &["config", "--global", "--get", "diff.smorg-rs.command"]).status.code(),
            Some(1)
        );
        succeeds(binary, root, &["--scope", "global", "--profile", "builtin-diff", "--undo"]);
        assert_eq!(fs::read(path).unwrap(), original);
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
}

#[test]
fn include_scope_preserves_other_includes_and_git_resolves_quoted_fragment() {
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        assert!(git(root, &["init", "--template=", "-b", "test"]).status.success());
        fs::write(root.join("existing.conf"), "[alias]\n st = status\n").unwrap();
        assert!(
            git(root, &["config", "--local", "--add", "include.path", "../existing.conf"])
                .status
                .success()
        );
        let config = root.join(".git/config");
        let original = fs::read(&config).unwrap();
        succeeds(binary, root, &["--scope", "include-file", "--dry-run"]);
        assert_eq!(fs::read(&config).unwrap(), original);
        assert!(!root.join(".git/smorg").exists());
        succeeds(binary, root, &["--scope", "include-file"]);
        succeeds(binary, root, &["--scope", "include-file", "--check"]);
        assert_eq!(
            git(root, &["config", "--includes", "--get", "diff.smorg-rs.command"]).stdout,
            b"smorg-rs diff-driver\n"
        );
        let includes = git(root, &["config", "--local", "--get-all", "include.path"]);
        assert_eq!(String::from_utf8(includes.stdout).unwrap().lines().count(), 2);
        succeeds(binary, root, &["--scope", "include-file", "--undo"]);
        assert_eq!(fs::read(&config).unwrap(), original);
        assert!(!root.join(".git/smorg/config").exists());
        assert_eq!(
            fs::read_to_string(root.join("existing.conf")).unwrap(),
            "[alias]\n st = status\n"
        );
    }
}

#[test]
fn modified_blocks_and_invalid_invocations_never_change_configuration() {
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        succeeds(binary, root, &[]);
        let path = root.join(".gitattributes");
        let modified = fs::read_to_string(&path).unwrap().replace("diff=smorg-rs", "diff=user");
        fs::write(&path, &modified).unwrap();
        for args in [vec![], vec!["--check"], vec!["--undo"], vec!["--dry-run"]] {
            assert_eq!(run(binary, root, &args).0.status.code(), Some(2));
            assert_eq!(fs::read_to_string(&path).unwrap(), modified);
        }
        for args in [
            vec!["--scope", "local", "--scope", "global"],
            vec!["--check", "--undo"],
            vec!["--check", "--dry-run"],
            vec!["--dry-run", "--dry-run"],
            vec!["--profile", ""],
            vec!["--scope"],
        ] {
            let output = command(binary, root)
                .args(["git", "install", "--json"])
                .args(args)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert_eq!(fs::read_to_string(&path).unwrap(), modified);
        }
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
}

#[cfg(unix)]
#[test]
fn protected_files_and_partial_include_failure_are_reported_not_hidden() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        fs::write(root.join("target"), b"user").unwrap();
        symlink("target", root.join(".gitattributes")).unwrap();
        assert_eq!(run(binary, root, &[]).0.status.code(), Some(2));
        fs::remove_file(root.join(".gitattributes")).unwrap();
        fs::hard_link(root.join("target"), root.join(".gitattributes")).unwrap();
        assert_eq!(run(binary, root, &[]).0.status.code(), Some(2));
        assert_eq!(fs::read(root.join("target")).unwrap(), b"user");
        assert!(git(root, &["init", "--template=", "-b", "test"]).status.success());
        let config = root.join(".git/config");
        let original = fs::read(&config).unwrap();
        fs::create_dir(root.join("outside")).unwrap();
        fs::write(root.join("outside/config"), b"unowned").unwrap();
        symlink("../outside", root.join(".git/smorg")).unwrap();
        assert_eq!(run(binary, root, &["--scope", "include-file"]).0.status.code(), Some(2));
        assert_eq!(fs::read(root.join("outside/config")).unwrap(), b"unowned");
        fs::remove_file(root.join(".git/smorg")).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o444)).unwrap();
        let (output, report) = run(binary, root, &["--scope", "include-file"]);
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        assert_eq!(report["git_install"]["steps"][0]["status"], "succeeded");
        assert_eq!(report["git_install"]["steps"][1]["status"], "failed");
        assert_eq!(fs::read(&config).unwrap(), original);
        assert!(root.join(".git/smorg/config").exists());
        fs::set_permissions(&config, fs::Permissions::from_mode(0o644)).unwrap();
        succeeds(binary, root, &["--scope", "include-file"]);
        succeeds(binary, root, &["--scope", "include-file", "--undo"]);
        assert_eq!(fs::read(config).unwrap(), original);
        assert!(!root.join(".git/smorg/config").exists());
        succeeds(binary, root, &["--scope", "include-file"]);
        let installed_config = fs::read(root.join(".git/config")).unwrap();
        let fragment = root.join(".git/smorg/config");
        fs::set_permissions(&fragment, fs::Permissions::from_mode(0o444)).unwrap();
        let (output, report) =
            run(binary, root, &["--scope", "include-file", "--profile", "builtin-diff"]);
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(report["git_install"]["steps"][0]["status"], "failed");
        assert_eq!(report["git_install"]["steps"][1]["status"], "not_run");
        assert_eq!(fs::read(root.join(".git/config")).unwrap(), installed_config);
        fs::set_permissions(fragment, fs::Permissions::from_mode(0o644)).unwrap();
        succeeds(binary, root, &["--scope", "include-file", "--undo"]);
        assert_eq!(fs::read(root.join(".git/config")).unwrap(), original);
    }
}

#[test]
fn include_scope_uses_shared_configuration_from_linked_worktrees() {
    use std::{io::Write, process::Stdio};
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        assert!(git(root, &["init", "--template=", "-b", "test"]).status.success());
        let mut import = command("git", root)
            .args(["fast-import", "--quiet"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        import.stdin.take().unwrap().write_all(b"commit refs/heads/test\ncommitter Fixture <fixture@example.invalid> 1000000000 +0000\ndata 4\nseed\n\n").unwrap();
        let imported = import.wait_with_output().unwrap();
        assert!(imported.status.success(), "{imported:?}");
        let linked = root.join("linked worktree");
        assert!(
            git(root, &["worktree", "add", "--detach", linked.to_str().unwrap()]).status.success()
        );
        let config = root.join(".git/config");
        let original = fs::read(&config).unwrap();
        let pointer = fs::read(linked.join(".git")).unwrap();
        succeeds(binary, &linked, &["--scope", "include-file"]);
        assert!(root.join(".git/smorg/config").is_file());
        assert_eq!(
            git(&linked, &["config", "--includes", "--get", "diff.smorg-rs.command"]).stdout,
            b"smorg-rs diff-driver\n"
        );
        succeeds(binary, &linked, &["--scope", "include-file", "--check"]);
        succeeds(binary, &linked, &["--scope", "include-file", "--undo"]);
        assert_eq!(fs::read(config).unwrap(), original);
        assert_eq!(fs::read(linked.join(".git")).unwrap(), pointer);
    }
}

#[test]
fn invalid_and_oversized_targets_are_preserved() {
    for binary in BINS {
        let directory = temporary();
        let root = directory.path();
        let path = root.join(".gitattributes");
        fs::write(&path, [255]).unwrap();
        assert_eq!(run(binary, root, &[]).0.status.code(), Some(2));
        assert_eq!(fs::read(&path).unwrap(), [255]);
        fs::File::create(&path).unwrap().set_len(1024 * 1024 + 1).unwrap();
        assert_eq!(run(binary, root, &[]).0.status.code(), Some(2));
        assert_eq!(fs::metadata(&path).unwrap().len(), 1024 * 1024 + 1);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(run(binary, root, &[]).0.status.code(), Some(2));
        assert!(path.is_dir());
    }
}
