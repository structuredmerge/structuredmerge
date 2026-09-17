//! Preflight report/source separation. This is not a concurrent-filesystem lock.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn destination(path: &Path) -> io::Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(_) => fs::canonicalize(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
            })?;
            Ok(fs::canonicalize(parent)?.join(name))
        }
        Err(error) => Err(error),
    }
}

pub fn report_is_distinct(report: &str, protected: &[&str]) -> io::Result<bool> {
    let report = Path::new(report);
    let resolved_report = destination(report)?;
    for path in protected {
        let path = Path::new(path);
        if resolved_report == destination(path)? {
            return Ok(false);
        }
        // File identity also catches distinct hard-link names. Errors other than
        // a not-yet-created destination fail closed rather than guessing identity.
        match same_file::is_same_file(report, path) {
            Ok(true) => return Ok(false),
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}
