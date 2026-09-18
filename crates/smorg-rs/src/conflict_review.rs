use ast_merge_git::conflict_review::{
    ConflictReview, MAX_REVIEW_BYTES, ReviewError, review_conflicts,
};
use std::{
    fs::File,
    io::{Read, Write},
};
use structuredmerge_core::{PortableCategory, PortableDiagnostic};

fn read(options: &crate::ConflictDiffOptions) -> Result<ConflictReview, ReviewError> {
    let io_error = |_| ReviewError {
        code: "conflict.source_rejected",
        message: "cannot read a regular conflict-review source",
        line: None,
    };
    let metadata = std::fs::metadata(&options.file_path).map_err(io_error)?;
    if !metadata.is_file() {
        return Err(io_error(std::io::Error::other("not regular")));
    }
    let limit = || ReviewError {
        code: "conflict.input_limit",
        message: "conflict review exceeds 8 MiB",
        line: None,
    };
    if metadata.len() > MAX_REVIEW_BYTES as u64 {
        return Err(limit());
    }
    let mut bytes = vec![];
    File::open(&options.file_path)
        .map_err(io_error)?
        .take(MAX_REVIEW_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_REVIEW_BYTES {
        return Err(limit());
    }
    let path = options.path_name.as_deref().unwrap_or(&options.file_path);
    let settings = crate::load_path_settings(path);
    review_conflicts(bytes, settings.conflict_marker_size)
}

pub(super) fn run(
    options: &crate::ConflictDiffOptions,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let path = options.path_name.as_deref().unwrap_or(&options.file_path);
    let mut diagnostics: Vec<PortableDiagnostic> = vec![];
    let (review, outcome, code) = match read(options) {
        Ok(review) => {
            let conflict = !review.regions.is_empty();
            (
                Some(review),
                if conflict { "conflict" } else { "clean" },
                i32::from(conflict && options.exit_code),
            )
        }
        Err(error) => {
            let _ = writeln!(stderr, "{}: {}", error.code, error.message);
            let category = if matches!(error.code, "conflict.input_limit" | "conflict.region_limit")
            {
                PortableCategory::ResourceLimit
            } else {
                PortableCategory::InvalidRequest
            };
            let mut diagnostic =
                crate::typed_driver::adapter_diagnostic(category, error.code, error.message.into());
            if let Some(line) = error.line {
                diagnostic.data.insert("line".into(), serde_json::json!(line));
            }
            diagnostics.push(diagnostic);
            (None, "error", 2)
        }
    };
    let write = if options.json {
        let value = serde_json::json!({
            "schema": "structuredmerge.cli-report/v1", "command": "conflicts.diff",
            "cli": {"executable": env!("CARGO_BIN_NAME"), "package": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"), "cli_contract": "structuredmerge.cli/v1",
                "kernel_version": structuredmerge_core::artifact_inventory::compiled_provider_inventory().kernel_version},
            "path_name": path, "outcome": outcome, "exit_code": code,
            "operation_result": null, "availability": null, "conflict_review": review,
            "git_install": null, "diagnostics": diagnostics, "output_commit_verified": false,
        });
        serde_json::to_writer(&mut *stdout, &value)
            .map_err(std::io::Error::other)
            .and_then(|()| writeln!(stdout))
    } else if let Some(review) = review {
        (|| {
            writeln!(stdout, "conflicts {path}\ncount {}", review.regions.len())?;
            for (index, region) in review.regions.iter().enumerate() {
                writeln!(
                    stdout,
                    "conflict {} lines {}-{} separator {}",
                    index + 1,
                    region.start_line,
                    region.end_line,
                    region.separator_line
                )?;
            }
            Ok(())
        })()
    } else {
        Ok(())
    };
    if let Err(error) = write {
        let _ = writeln!(stderr, "cannot write conflict review: {error}");
        return 3;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stdout_failure_is_internal_error() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let options = crate::ConflictDiffOptions {
            path_name: None,
            file_path: "missing-conflict-source".into(),
            exit_code: false,
            json: true,
        };
        assert_eq!(run(&options, &mut Closed, &mut vec![]), 3);
    }
}
