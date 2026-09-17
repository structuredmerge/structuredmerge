//! Exercise actual Cargo-built entry points, not only the in-process router.
use std::process::Command;

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
