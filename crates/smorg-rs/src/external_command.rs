//! OS-native external command routing; never invoke a shell or inspect source files.
use std::ffi::OsString;

pub fn dispatch(args: &[OsString], legacy_positional: bool) -> Option<i32> {
    let first = args.first()?;
    if matches!(
        first.to_str(),
        Some(
            "merge-driver"
                | "diff-driver"
                | "conflicts"
                | "languages"
                | "git"
                | "help"
                | "-h"
                | "--help"
                | "--version"
                | "benchmark-provider-session"
                | "benchmark-provider-diff"
                | "benchmark-provider-merge2"
                | "benchmark-provider-merge3"
        )
    ) || (legacy_positional && args.len() >= 4)
    {
        return None;
    }
    let name = first.to_str().filter(|name| {
        name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    });
    let Some(name) = name else {
        eprintln!("smorg: invalid external command name {first:?}");
        return Some(2);
    };
    let executable = format!("smorg-{name}");
    #[cfg(unix)]
    {
        let error = exec_on_path(&executable, &args[1..]);
        eprintln!("smorg: cannot execute {executable}: {error}");
        Some(2)
    }
    #[cfg(not(unix))]
    {
        let mut command = std::process::Command::new(&executable);
        command.args(&args[1..]);
        match command.status() {
            Ok(status) => Some(status.code().unwrap_or(3)),
            Err(error) => {
                eprintln!("smorg: cannot execute {executable}: {error}");
                Some(2)
            }
        }
    }
}

#[cfg(unix)]
fn exec_on_path(executable: &str, args: &[OsString]) -> std::io::Error {
    use std::{env, ffi::CString, io, os::unix::ffi::OsStrExt};
    let Some(path) = env::var_os("PATH") else {
        return io::Error::new(io::ErrorKind::NotFound, "PATH is unset");
    };
    let strings = std::iter::once(executable.as_bytes())
        .chain(args.iter().map(|arg| arg.as_bytes()))
        .map(CString::new)
        .collect::<Result<Vec<_>, _>>();
    let Ok(strings) = strings else {
        return io::Error::new(io::ErrorKind::InvalidInput, "argument contains NUL");
    };
    let mut pointers: Vec<_> = strings.iter().map(|arg| arg.as_ptr()).collect();
    pointers.push(std::ptr::null());
    let mut denied = false;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(executable);
        let Ok(candidate) = CString::new(candidate.as_os_str().as_bytes()) else {
            return io::Error::new(io::ErrorKind::InvalidInput, "PATH contains NUL");
        };
        // execvp (including Command::exec) may interpret ENOEXEC files with /bin/sh.
        // Use execv with explicit PATH iteration so an invalid executable fails
        // instead of being evaluated as shell code. The OS still handles shebangs.
        // SAFETY: candidate and every argv string remain alive and NUL-terminated;
        // argv ends with a null pointer. Dispatch runs before starting any threads.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
            libc::execv(candidate.as_ptr(), pointers.as_ptr());
        }
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ENOENT | libc::ENOTDIR) => {}
            Some(libc::EACCES) => denied = true,
            _ => return error,
        }
    }
    io::Error::from_raw_os_error(if denied { libc::EACCES } else { libc::ENOENT })
}
