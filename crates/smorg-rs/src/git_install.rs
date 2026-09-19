//! Owned, byte-preserving installation steps. No default-provider authority.
use crate::{GitInstallOptions, GitInstallProfile, GitInstallScope, staged_file::StagedFile};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const LIMIT: u64 = 1024 * 1024;
const COMMAND_LIMIT: u64 = 65536;

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
    exit: i32,
}
fn invalid(message: impl ToString) -> Failure {
    Failure { code: "git.install_rejected", message: message.to_string(), exit: 2 }
}
fn io(error: std::io::Error) -> Failure {
    Failure { code: "git.install_io", message: error.to_string(), exit: 3 }
}

// Git supplies configuration locations, including linked-worktree paths. Both
// pipes are bounded and drained concurrently; no temporary capture files are
// created by check/dry-run. Git is a trusted local executable, not a sandbox.
fn git(args: &[&str]) -> Result<String, Failure> {
    let mut command = Command::new("git");
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(io)?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = vec![];
        stdout.take(COMMAND_LIMIT + 1).read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = vec![];
        stderr.take(COMMAND_LIMIT + 1).read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => break Err(invalid("Git configuration lookup exceeded deadline")),
            Err(error) => break Err(io(error)),
        }
    };
    #[cfg(unix)]
    {
        // SAFETY: the child is placed in its own process group before exec.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    if status.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let stdout = out.join().map_err(|_| invalid("Git stdout reader failed"))?.map_err(io)?;
    let stderr = err.join().map_err(|_| invalid("Git stderr reader failed"))?.map_err(io)?;
    if stdout.len() > COMMAND_LIMIT as usize || stderr.len() > COMMAND_LIMIT as usize {
        return Err(invalid("Git configuration lookup exceeded capture budget"));
    }
    if !status?.success() {
        return Err(invalid("Git configuration lookup failed; no installation was performed"));
    }
    String::from_utf8(stdout).map_err(|_| invalid("Git returned a non-UTF-8 configuration path"))
}

fn absolute(path: &str) -> Result<PathBuf, Failure> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return Err(invalid("empty or control-character configuration path"));
    }
    let path = PathBuf::from(path);
    let path =
        if path.is_absolute() { path } else { std::env::current_dir().map_err(io)?.join(path) };
    if path.to_str().is_none() {
        return Err(invalid("non-UTF-8 configuration destination"));
    }
    Ok(path)
}

fn read(path: &Path) -> Result<Option<String>, Failure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io(error)),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > LIMIT {
        return Err(invalid(
            "configuration target must be a regular non-symlink file of at most 1 MiB",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() > 1 {
            return Err(invalid("hard-linked configuration targets are not supported"));
        }
    }
    let mut bytes = vec![];
    fs::File::open(path).map_err(io)?.take(LIMIT + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 > LIMIT {
        return Err(invalid("configuration grew beyond 1 MiB"));
    }
    String::from_utf8(bytes).map(Some).map_err(|_| invalid("configuration target is not UTF-8"))
}

enum Kind {
    Attributes,
    Driver,
    Include(String),
}
impl Kind {
    fn name(&self) -> &str {
        match self {
            Self::Attributes => "attributes",
            Self::Driver => "driver",
            Self::Include(_) => "include",
        }
    }
    fn body(&self, profile: GitInstallProfile) -> String {
        match self {
            Self::Attributes => crate::git_attribute_lines(profile).join("\n") + "\n",
            Self::Driver if profile == GitInstallProfile::SemanticDiff => {
                "[diff \"smorg-rs\"]\n\tcommand = smorg-rs diff-driver\n".into()
            }
            Self::Driver => {
                "# Built-in diff uses Git attributes; no external command is installed.\n".into()
            }
            Self::Include(path) => format!(
                "[include]\n\tpath = \"{}\"\n",
                path.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
            ),
        }
    }
    fn begin(&self) -> String {
        format!("\n# BEGIN structuredmerge {} v1\n", self.name())
    }
    fn end(&self) -> String {
        format!("# END structuredmerge {} v1\n", self.name())
    }
    fn block(&self, profile: GitInstallProfile, created: bool) -> String {
        format!(
            "{}# profile: {}\n# created-file: {}\n{}{}",
            self.begin(),
            profile.as_str(),
            created,
            self.body(profile),
            self.end()
        )
    }
}

struct Step {
    path: PathBuf,
    before: Option<String>,
    after: Option<String>,
}
impl Step {
    fn changed(&self) -> bool {
        self.before != self.after
    }
    fn record(&self, options: &GitInstallOptions, status: &str) -> Value {
        let identity = |source: &Option<String>| {
            source.as_ref().map(|source| {
                structuredmerge_core::source_input(
                    "configuration".into(),
                    structuredmerge_core::SourceRole::Source,
                    structuredmerge_core::SourceEncoding::Utf8,
                    source.as_bytes().to_vec(),
                )
                .unwrap()
                .descriptor
            })
        };
        json!({"name":"git_drivers", "action": action(options), "target":self.path,
            "path":self.path, "scope":options.scope.as_str(), "profile":options.profile.as_str(),
            "status":status, "change_planned":self.changed(), "before":identity(&self.before), "planned_after":identity(&self.after),
            "diagnostics":crate::forge_diagnostics()})
    }
    fn apply(&self) -> Result<(), Failure> {
        if !self.changed() {
            return Ok(());
        }
        if read(&self.path)? != self.before {
            return Err(invalid("configuration changed after planning; retry explicitly"));
        }
        if let Some(after) = &self.after {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(io)?;
            }
            let path =
                self.path.to_str().ok_or_else(|| invalid("non-UTF-8 configuration destination"))?;
            StagedFile::new(path, after.as_bytes()).map_err(io)?.commit().map_err(io)
        } else {
            if fs::metadata(&self.path).map_err(io)?.permissions().readonly() {
                return Err(io(std::io::ErrorKind::PermissionDenied.into()));
            }
            fs::remove_file(&self.path).map_err(io)
        }
    }
}

fn plan(path: PathBuf, kind: Kind, options: &GitInstallOptions) -> Result<Step, Failure> {
    let before = read(&path)?;
    let source = before.as_deref().unwrap_or("");
    let begin = kind.begin();
    let end = kind.end();
    let starts: Vec<_> = source.match_indices(begin.trim_start_matches('\n')).collect();
    let ends: Vec<_> = source.match_indices(&end).collect();
    let mut owned = None;
    if starts.is_empty() && source.contains(&format!("# BEGIN structuredmerge {}", kind.name())) {
        return Err(invalid("unknown or edited managed-section version"));
    }
    if !starts.is_empty() || !ends.is_empty() {
        if starts.len() != 1 || ends.len() != 1 {
            return Err(invalid("ambiguous managed configuration sections"));
        }
        let start =
            source.find(&begin).ok_or_else(|| invalid("edited managed section boundary"))?;
        let stop = ends[0].0 + end.len();
        if stop <= start {
            return Err(invalid("misordered managed section boundaries"));
        }
        for profile in [GitInstallProfile::SemanticDiff, GitInstallProfile::BuiltinDiff] {
            for created in [false, true] {
                if source[start..stop] == kind.block(profile, created) {
                    owned = Some((start, stop, profile, created));
                }
            }
        }
        if owned.is_none() {
            return Err(invalid("managed section was edited; preserve it and resolve manually"));
        }
    }
    let after = if options.check {
        if owned.is_none_or(|(_, _, profile, _)| profile != options.profile) {
            return Err(invalid(
                "requested managed installation is missing or uses another profile",
            ));
        }
        before.clone()
    } else if options.undo {
        match owned {
            None => before.clone(),
            Some((start, stop, profile, created)) => {
                if profile != options.profile {
                    return Err(invalid("undo profile differs from the owned installation"));
                }
                let prefix = &source[..start];
                let suffix = &source[stop..];
                // If a user appended a new line after our section, do not join
                // it to a preexisting unterminated line when retiring framing.
                let separator = if !prefix.is_empty()
                    && !suffix.is_empty()
                    && !prefix.ends_with('\n')
                    && !suffix.starts_with('\n')
                {
                    "\n"
                } else {
                    ""
                };
                let remainder = prefix.to_owned() + separator + suffix;
                if created && remainder.is_empty() { None } else { Some(remainder) }
            }
        }
    } else {
        let replacement =
            kind.block(options.profile, owned.map_or(before.is_none(), |item| item.3));
        Some(match owned {
            Some((start, stop, _, _)) => {
                source[..start].to_owned() + &replacement + &source[stop..]
            }
            None => source.to_owned() + &replacement,
        })
    };
    if after.as_ref().is_some_and(|value| value.len() as u64 > LIMIT) {
        return Err(invalid("planned configuration exceeds 1 MiB"));
    }
    Ok(Step { path, before, after })
}

fn plans(options: &GitInstallOptions) -> Result<Vec<Step>, Failure> {
    match options.scope {
        GitInstallScope::Local => {
            Ok(vec![plan(absolute(".gitattributes")?, Kind::Attributes, options)?])
        }
        GitInstallScope::Global => {
            let paths = git(&["var", "GIT_CONFIG_GLOBAL"])?;
            // Use Git's last reported global location, or its explicit single
            // GIT_CONFIG_GLOBAL override. No HOME/XDG guessing in the installer.
            let path = paths
                .split_terminator('\n')
                .next_back()
                .ok_or_else(|| invalid("Git reported no global configuration path"))?;
            Ok(vec![plan(absolute(path)?, Kind::Driver, options)?])
        }
        GitInstallScope::IncludeFile => {
            let location = git(&["rev-parse", "--path-format=absolute", "--git-path", "config"])?;
            let config = absolute(location.strip_suffix('\n').unwrap_or(&location))?;
            let fragment = config
                .parent()
                .ok_or_else(|| invalid("configuration has no parent"))?
                .join("smorg/config");
            if let Ok(metadata) = fs::symlink_metadata(fragment.parent().unwrap()) {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(invalid(
                        "managed include directory must not be a symlink or non-directory",
                    ));
                }
            }
            if read(&fragment)?
                .is_some_and(|source| !source.is_empty() && !source.contains(&Kind::Driver.begin()))
            {
                return Err(invalid(
                    "include fragment contains unowned configuration; refusing to activate it",
                ));
            }
            let link = Kind::Include(
                fragment.to_str().ok_or_else(|| invalid("non-UTF-8 include path"))?.into(),
            );
            let included = plan(fragment, Kind::Driver, options)?;
            let include = plan(config, link, options)?;
            // Installation writes the fragment before referencing it; undo
            // removes the reference before retiring the owned fragment.
            Ok(if options.undo { vec![include, included] } else { vec![included, include] })
        }
    }
}

fn action(options: &GitInstallOptions) -> &str {
    if options.check {
        "check"
    } else if options.undo {
        "undo"
    } else {
        "install"
    }
}

pub(super) fn run(
    options: &GitInstallOptions,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let mut steps = vec![];
    let mut failure = None;
    let mut changed = false;
    match plans(options) {
        Err(error) => failure = Some(error),
        Ok(plans) => {
            for (index, step) in plans.iter().enumerate() {
                let status = if options.dry_run { "planned" } else { "succeeded" };
                if !options.check && !options.dry_run {
                    if let Err(error) = step.apply() {
                        steps.push(step.record(options, "failed"));
                        steps.extend(
                            plans[index + 1..].iter().map(|step| step.record(options, "not_run")),
                        );
                        failure = Some(error);
                        break;
                    }
                }
                changed |= step.changed();
                steps.push(step.record(options, status));
            }
        }
    }
    let code = failure.as_ref().map_or(0, |error| error.exit);
    let diagnostics: Vec<_> = failure
        .as_ref()
        .map(|error| {
            let _ = writeln!(stderr, "{}: {}", error.code, error.message);
            crate::typed_driver::adapter_diagnostic(
                if code == 3 {
                    structuredmerge_core::PortableCategory::InternalError
                } else {
                    structuredmerge_core::PortableCategory::InvalidRequest
                },
                error.code,
                error.message.clone(),
            )
        })
        .into_iter()
        .collect();
    if steps.is_empty() {
        steps.push(json!({"name":"git_drivers", "action":action(options), "target":null, "status":"failed", "diagnostics":diagnostics}));
    }
    let limitation = match options.scope {
        GitInstallScope::Local => {
            "Managed attributes only; driver configuration and provider availability are not verified."
        }
        _ if options.profile == GitInstallProfile::BuiltinDiff => {
            "No external command installed; built-in diff still requires local attributes."
        }
        _ => {
            "Managed diff configuration only; attributes, merge-driver setup and provider availability are not verified."
        }
    };
    let report = json!({
        "schema":"structuredmerge.cli-report/v1", "command":"git.install",
        "cli":{"executable":env!("CARGO_BIN_NAME"), "package":env!("CARGO_PKG_NAME"), "version":env!("CARGO_PKG_VERSION"),
            "kernel_version":structuredmerge_core::artifact_inventory::compiled_provider_inventory().kernel_version, "cli_contract":"structuredmerge.cli/v1"},
        "outcome":if code != 0 { "error" } else if changed { "changed" } else { "clean" }, "exit_code":code,
        "operation_result":null, "availability":null, "conflict_review":null,
        "git_install":{"scope":options.scope.as_str(), "profile":options.profile.as_str(), "steps":steps,
            "driver_configuration_verified":false, "default_approved":false, "setup_complete":false, "limitations":[limitation]},
        "diagnostics":diagnostics, "output_commit_verified":false,
        // Retained legacy keys for the existing installation-report consumer.
        "report_version":1, "ok":code == 0, "profile":options.profile.as_str(), "scope":options.scope.as_str(),
        "install_steps":steps, "missing":if code == 0 { json!([]) } else { json!(["git_drivers"]) },
    });
    let write = if options.json {
        serde_json::to_writer(&mut *stdout, &report)
            .map_err(std::io::Error::other)
            .and_then(|()| writeln!(stdout))
    } else {
        writeln!(
            stdout,
            "git install: {} {} {}",
            if code != 0 {
                "failed"
            } else if options.dry_run {
                "planned"
            } else {
                "succeeded"
            },
            options.profile.as_str(),
            options.scope.as_str()
        )
        .and_then(|()| writeln!(stdout, "{limitation}"))
    };
    if write.is_err() {
        let _ = writeln!(stderr, "cannot write Git installation report");
        return 3;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> GitInstallOptions {
        GitInstallOptions {
            scope: GitInstallScope::Local,
            profile: GitInstallProfile::SemanticDiff,
            check: false,
            undo: false,
            dry_run: false,
            json: true,
        }
    }

    #[test]
    fn stale_plans_and_unknown_marker_versions_reject_without_writes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        fs::create_dir_all(&root).unwrap();
        let directory = tempfile::tempdir_in(root).unwrap();
        let path = directory.path().join("attributes");
        fs::write(&path, "original").unwrap();
        let step = plan(path.clone(), Kind::Attributes, &options()).unwrap();
        fs::write(&path, "concurrent change").unwrap();
        assert!(step.apply().is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "concurrent change");
        fs::write(&path, "# BEGIN structuredmerge attributes v2\n").unwrap();
        assert!(plan(path.clone(), Kind::Attributes, &options()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "# BEGIN structuredmerge attributes v2\n");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn stdout_failure_is_not_installation_success() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut options = options();
        options.dry_run = true;
        assert_eq!(run(&options, &mut Closed, &mut vec![]), 3);
    }
}
