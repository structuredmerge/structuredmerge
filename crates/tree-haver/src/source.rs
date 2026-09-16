//! Exact source storage for the typed kernel boundary (Slices 722, 1024, 1027).
//! Parser adapters may decode a view, but cannot replace the retained bytes.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ByteRange, SourcePoint};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    Source,
    Before,
    After,
    Incoming,
    Current,
    Base,
    Ours,
    Theirs,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEncoding {
    Utf8,
    Binary,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LineEndings {
    pub lf: u64,
    pub crlf: u64,
    pub bare_cr: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceDescriptor {
    pub source_id: String,
    pub role: SourceRole,
    pub byte_length: u64,
    pub sha256: String,
    pub encoding: SourceEncoding,
    pub bom: bool,
    pub line_endings: LineEndings,
    pub final_newline: bool,
}

/// Owned transport input. Always validate it before parsing or rendering.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceInput {
    pub descriptor: SourceDescriptor,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceErrorCode {
    EmptyId,
    DuplicateId,
    InvalidEncoding,
    DescriptorMismatch,
    InvalidRange,
    UnknownSource,
    LimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceError {
    pub code: SourceErrorCode,
    pub source_id: String,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: source {:?}", self.code, self.source_id)
    }
}

impl Error for SourceError {}

fn failure(code: SourceErrorCode, source_id: &str) -> SourceError {
    SourceError { code, source_id: source_id.to_owned() }
}

pub fn source_input(
    source_id: String,
    role: SourceRole,
    encoding: SourceEncoding,
    bytes: Vec<u8>,
) -> Result<SourceInput, SourceError> {
    if source_id.is_empty() {
        return Err(failure(SourceErrorCode::EmptyId, &source_id));
    }
    if encoding == SourceEncoding::Utf8 && std::str::from_utf8(&bytes).is_err() {
        return Err(failure(SourceErrorCode::InvalidEncoding, &source_id));
    }
    let mut line_endings = LineEndings::default();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'\n' if index > 0 && bytes[index - 1] == b'\r' => line_endings.crlf += 1,
            b'\n' => line_endings.lf += 1,
            b'\r' if bytes.get(index + 1) != Some(&b'\n') => line_endings.bare_cr += 1,
            _ => {}
        }
    }
    let descriptor = SourceDescriptor {
        source_id,
        role,
        byte_length: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        encoding,
        bom: encoding == SourceEncoding::Utf8 && bytes.starts_with(&[0xef, 0xbb, 0xbf]),
        line_endings,
        final_newline: bytes.last().is_some_and(|byte| matches!(byte, b'\r' | b'\n')),
    };
    Ok(SourceInput { descriptor, bytes })
}

/// Validated immutable source. Neither bytes nor descriptor can be mutated.
#[derive(Clone, Debug)]
pub struct SourceDocument {
    input: SourceInput,
}

impl SourceDocument {
    pub fn validate(input: SourceInput, max_bytes: u64) -> Result<Self, SourceError> {
        let descriptor = &input.descriptor;
        if input.bytes.len() as u64 > max_bytes || descriptor.byte_length > max_bytes {
            return Err(failure(SourceErrorCode::LimitExceeded, &descriptor.source_id));
        }
        let verified = source_input(
            descriptor.source_id.clone(),
            descriptor.role,
            descriptor.encoding,
            input.bytes,
        )?;
        if *descriptor != verified.descriptor {
            return Err(failure(SourceErrorCode::DescriptorMismatch, &descriptor.source_id));
        }
        Ok(Self { input: verified })
    }

    pub fn descriptor(&self) -> &SourceDescriptor {
        &self.input.descriptor
    }

    pub fn bytes(&self) -> &[u8] {
        &self.input.bytes
    }

    pub fn slice(&self, range: ByteRange) -> Result<&[u8], SourceError> {
        self.bytes()
            .get(range.start_byte..range.end_byte)
            .ok_or_else(|| failure(SourceErrorCode::InvalidRange, &self.descriptor().source_id))
    }

    pub fn range_digest(&self, range: ByteRange) -> Result<String, SourceError> {
        Ok(format!("{:x}", Sha256::digest(self.slice(range)?)))
    }

    /// Byte-oriented point; LF terminates a row, with CR retained in its bytes.
    /// Binary sources have no implied text points.
    pub fn point(&self, offset: usize) -> Result<SourcePoint, SourceError> {
        if self.descriptor().encoding != SourceEncoding::Utf8 {
            return Err(failure(SourceErrorCode::InvalidEncoding, &self.descriptor().source_id));
        }
        let prefix = self.slice(ByteRange { start_byte: 0, end_byte: offset })?;
        let row = prefix.iter().filter(|&&byte| byte == b'\n').count();
        let column = prefix
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(offset, |index| offset - index - 1);
        Ok(SourcePoint { row, column })
    }
}

/// One operation's immutable source map; IDs cannot alias different sources.
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    documents: BTreeMap<String, SourceDocument>,
}

impl SourceMap {
    pub fn validate(inputs: Vec<SourceInput>, max_total_bytes: u64) -> Result<Self, SourceError> {
        let mut documents = BTreeMap::new();
        let mut remaining = max_total_bytes;
        for input in inputs {
            let id = input.descriptor.source_id.clone();
            if documents.contains_key(&id) {
                return Err(failure(SourceErrorCode::DuplicateId, &id));
            }
            let document = SourceDocument::validate(input, remaining)?;
            remaining -= document.descriptor().byte_length;
            documents.insert(id, document);
        }
        Ok(Self { documents })
    }

    pub fn get(&self, source_id: &str) -> Result<&SourceDocument, SourceError> {
        self.documents
            .get(source_id)
            .ok_or_else(|| failure(SourceErrorCode::UnknownSource, source_id))
    }
}
