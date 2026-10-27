//! Error types for ingest observation validation.

use std::fmt;

/// Error codes corresponding to the Python reference implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Schema,
    PayloadDigest,
    Json,
    Fields,
    Producer,
    Scope,
    Identity,
    Prior,
    Utf8,
    Dialect,
    Processing,
    Time,
    Scan,
    Evidence,
    Deletion,
    Authority,
    Precondition,
    Revision,
    ConflictingRetry,
    UnavailableBytes,
    Ancestry,
    Embedding,
    Lineage,
    Incorporation,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrorCode::Schema => "schema",
            ErrorCode::PayloadDigest => "payload_digest",
            ErrorCode::Json => "json",
            ErrorCode::Fields => "fields",
            ErrorCode::Producer => "producer",
            ErrorCode::Scope => "scope",
            ErrorCode::Identity => "identity",
            ErrorCode::Prior => "prior",
            ErrorCode::Utf8 => "utf8",
            ErrorCode::Dialect => "dialect",
            ErrorCode::Processing => "processing",
            ErrorCode::Time => "time",
            ErrorCode::Scan => "scan",
            ErrorCode::Evidence => "evidence",
            ErrorCode::Deletion => "deletion",
            ErrorCode::Authority => "authority",
            ErrorCode::Precondition => "precondition",
            ErrorCode::Revision => "revision",
            ErrorCode::ConflictingRetry => "conflicting_retry",
            ErrorCode::UnavailableBytes => "unavailable_bytes",
            ErrorCode::Ancestry => "ancestry",
            ErrorCode::Embedding => "embedding",
            ErrorCode::Lineage => "lineage",
            ErrorCode::Incorporation => "incorporation",
        })
    }
}

/// Validation error with an error code.
#[derive(Debug, Clone)]
pub struct Error {
    code: ErrorCode,
    message: Option<String>,
}

impl Error {
    pub fn new(code: ErrorCode) -> Self {
        Self { code, message: None }
    }

    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: Some(message.into()) }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(msg) = &self.message {
            write!(f, "{}: {}", self.code, msg)
        } else {
            write!(f, "{}", self.code)
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
