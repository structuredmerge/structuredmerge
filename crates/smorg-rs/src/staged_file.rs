//! Same-directory staging; atomic replacement per file, not a multi-file transaction.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct StagedFile {
    temporary: tempfile::NamedTempFile,
    destination: PathBuf,
}

impl StagedFile {
    pub fn new(path: &str, bytes: &[u8]) -> io::Result<Self> {
        Self::stage_with(Path::new(path), |file| file.write_all(bytes))
    }

    fn stage_with(
        path: &Path,
        write: impl FnOnce(&mut fs::File) -> io::Result<()>,
    ) -> io::Result<Self> {
        let (destination, permissions) = match fs::symlink_metadata(path) {
            Ok(_) => {
                let destination = fs::canonicalize(path)?;
                let metadata = fs::metadata(&destination)?;
                if !metadata.is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "destination is not a regular file",
                    ));
                }
                if metadata.permissions().readonly() {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "destination is read-only",
                    ));
                }
                // Check existing-file write access without truncation. Replacement
                // also requires directory access; do not bypass a read-only target.
                fs::OpenOptions::new().write(true).open(&destination)?;
                (destination, Some(metadata.permissions()))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (path.to_path_buf(), None),
            Err(error) => return Err(error),
        };
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::Builder::new().prefix(".smorg-write-").tempfile_in(parent)?;
        write(temporary.as_file_mut())?;
        if let Some(permissions) = permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.as_file().sync_all()?;
        Ok(Self { temporary, destination })
    }

    pub fn commit(self) -> io::Result<()> {
        self.temporary.persist(self.destination).map_err(|error| error.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abandoned_stage_preserves_destination_and_failed_commit_cleans_temporary() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("destination");
        fs::write(&path, b"original").unwrap();
        let staged = StagedFile::stage_with(&path, |file| file.write_all(b"discard")).unwrap();
        drop(staged);
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

        let staged = StagedFile::stage_with(&path, |file| file.write_all(b"complete")).unwrap();
        let backup = dir.path().join("original");
        fs::rename(&path, &backup).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(staged.commit().is_err());
        assert_eq!(fs::read(&backup).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn follows_symlink_and_preserves_executable_mode() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::write(&target, b"original").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o750)).unwrap();
        symlink("target", &link).unwrap();
        StagedFile::stage_with(&link, |file| file.write_all(b"complete"))
            .unwrap()
            .commit()
            .unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), b"complete");
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o750);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn partial_staging_failure_does_not_truncate_destination_or_leave_temporary() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("destination");
        fs::write(&path, b"original").unwrap();
        let result = StagedFile::stage_with(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        let staged = StagedFile::stage_with(&path, |file| file.write_all(b"complete")).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"original");
        staged.commit().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete");
    }
}
