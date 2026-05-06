use std::collections::{BTreeSet, HashMap};

use tree_haver::{
    BinaryDiagnostic, BinaryMergeReport, BinaryNestedDispatch, ByteRange, ZipArchiveEntry,
    ZipArchiveInfo, ZipFamilyReport, ZipMemberDecision, ZipUnsafeEntry,
};

const LOCAL: u32 = 0x04034b50;
const CENTRAL: u32 = 0x02014b50;
const EOCD: u32 = 0x06054b50;
const DOS_EPOCH: [u8; 4] = [0, 0, 0x21, 0];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderError {
    pub diagnostic: BinaryDiagnostic,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.diagnostic.message.fmt(formatter)
    }
}

impl std::error::Error for RenderError {}

pub fn parse_zip_inventory(source: &[u8]) -> Result<ZipFamilyReport, String> {
    let central = scan_central_directory(source)?;
    let locals = scan_local_headers(source, &central.records)?;
    let mut entries = central
        .records
        .iter()
        .map(|(name, record)| {
            let local = locals.get(name).expect("local header should exist");
            ZipArchiveEntry {
                path: name.clone(),
                normalized_path: normalize_zip_path(name),
                directory: name.ends_with('/'),
                compression: compression_name(record.method),
                compressed_size: record.compressed_size,
                uncompressed_size: record.uncompressed_size,
                crc32: format!("{:08x}", record.crc32),
                local_header_range: ByteRange {
                    start_byte: record.local_offset,
                    end_byte: local.data_start,
                },
                data_range: ByteRange {
                    start_byte: local.data_start,
                    end_byte: local.data_start + record.compressed_size,
                },
                central_directory_range: record.range.clone(),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.local_header_range.start_byte);

    Ok(ZipFamilyReport {
        archive: ZipArchiveInfo {
            format: "zip".to_string(),
            schema: "zip.ksy".to_string(),
            entry_count: entries.len(),
            central_directory_range: central.range,
        },
        unsafe_entries: unsafe_entries(&entries, &central.records),
        entries,
        member_decisions: vec![],
        merge_report: empty_report(),
    })
}

pub fn plan_zip_merge(
    ancestor: &ZipFamilyReport,
    current: &ZipFamilyReport,
    incoming: &ZipFamilyReport,
) -> ZipFamilyReport {
    let ancestor_entries = entries_by_path(&ancestor.entries);
    let current_entries = entries_by_path(&current.entries);
    let incoming_entries = entries_by_path(&incoming.entries);
    let unsafe_by_path = incoming
        .unsafe_entries
        .iter()
        .map(|entry| (entry.normalized_path.clone(), entry.clone()))
        .collect::<HashMap<_, _>>();
    let paths = ancestor_entries
        .keys()
        .chain(current_entries.keys())
        .chain(incoming_entries.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut report = ZipFamilyReport {
        archive: incoming.archive.clone(),
        entries: incoming.entries.clone(),
        member_decisions: vec![],
        unsafe_entries: incoming.unsafe_entries.clone(),
        merge_report: empty_report(),
    };

    for path in paths {
        let ancestor_entry = ancestor_entries.get(&path);
        let current_entry = current_entries.get(&path);
        let incoming_entry = incoming_entries.get(&path);
        if let Some(unsafe_entry) = unsafe_by_path.get(&path) {
            report.member_decisions.push(ZipMemberDecision {
                normalized_path: path.clone(),
                operation: "reject".to_string(),
                disposition: "unsafe".to_string(),
                nested_family: None,
                reason: unsafe_entry.reason.clone(),
            });
            report.merge_report.diagnostics.push(diagnostic(
                &unsafe_entry.category,
                &schema_path(&path),
                &unsafe_entry.reason,
            ));
        } else if current_entry.is_none() && incoming_entry.is_some() {
            decision(
                &mut report,
                &path,
                "add",
                "requires_renderer",
                "member exists only in incoming archive",
            );
        } else if current_entry.is_some() && incoming_entry.is_none() {
            decision(
                &mut report,
                &path,
                "delete",
                "requires_renderer",
                "member was removed from incoming archive",
            );
        } else if ancestor_entry.is_some()
            && same_entry(current_entry.unwrap(), ancestor_entry.unwrap())
            && same_entry(incoming_entry.unwrap(), ancestor_entry.unwrap())
        {
            let entry = current_entry.unwrap();
            report.member_decisions.push(ZipMemberDecision {
                normalized_path: path.clone(),
                operation: "preserve".to_string(),
                disposition: "safe".to_string(),
                nested_family: None,
                reason: "member is unchanged from ancestor".to_string(),
            });
            report.merge_report.preserved_ranges.push(entry.local_header_range.clone());
            report.merge_report.preserved_ranges.push(entry.data_range.clone());
        } else if let Some(family) = nested_family(&path) {
            report.member_decisions.push(ZipMemberDecision {
                normalized_path: path.clone(),
                operation: "delegate".to_string(),
                disposition: "requires_renderer".to_string(),
                nested_family: Some(family.to_string()),
                reason: "structured member can be merged by a nested family before ZIP rendering"
                    .to_string(),
            });
            report.merge_report.nested_dispatches.push(BinaryNestedDispatch {
                schema_path: format!("{}/data", schema_path(&path)),
                family: family.to_string(),
                status: "planned".to_string(),
            });
            report.merge_report.rewritten_nodes.push(schema_path(&path));
            report.merge_report.checksum_updates.push(format!("{}/crc32", schema_path(&path)));
        } else {
            decision(
                &mut report,
                &path,
                "rewrite",
                "requires_renderer",
                "member bytes or metadata changed",
            );
            report.merge_report.checksum_updates.push(format!("{}/crc32", schema_path(&path)));
        }
        report.merge_report.matched_schema_paths.push(schema_path(&path));
    }

    if !report.merge_report.rewritten_nodes.is_empty()
        || !report.merge_report.checksum_updates.is_empty()
    {
        report.merge_report.rewritten_nodes.push("/central_directory".to_string());
        report.merge_report.checksum_updates.push("/central_directory/size".to_string());
        report.merge_report.checksum_updates.push("/central_directory/offset".to_string());
    }
    report
}

pub fn render_with_raw_preservation(
    source: &[u8],
    plan: &ZipFamilyReport,
    member_bytes: &HashMap<String, Vec<u8>>,
) -> Result<(Vec<u8>, ZipFamilyReport, BinaryMergeReport), RenderError> {
    let source_inventory = parse_zip_inventory(source)
        .map_err(|error| render_error("invalid_zip", "/archive", &error))?;
    let central = scan_central_directory(source)
        .map_err(|error| render_error("invalid_zip", "/archive", &error))?;
    let source_entries = entries_by_path(&source_inventory.entries);
    let raw_ranges = raw_local_record_ranges(&source_entries);
    let entries = entries_by_path(&plan.entries);
    let mut output = vec![];
    let mut central_records = vec![];

    for member in &plan.member_decisions {
        match member.operation.as_str() {
            "reject" => {
                return Err(render_error(
                    "rejected_member",
                    &schema_path(&member.normalized_path),
                    &member.reason,
                ));
            }
            "delete" => {}
            "preserve" => {
                let entry =
                    source_entries.get(&member.normalized_path).expect("source entry should exist");
                validate_raw_preserve_entry(&central, entry)?;
                let range =
                    raw_ranges.get(&member.normalized_path).expect("raw range should exist");
                let offset = output.len();
                output.extend_from_slice(&source[range.start_byte..range.end_byte]);
                central_records.push(record_from_entry(entry, offset));
            }
            "add" | "rewrite" | "delegate" => {
                let entry =
                    entries.get(&member.normalized_path).expect("planned entry should exist");
                let content =
                    member_bytes.get(&member.normalized_path).expect("member bytes should exist");
                let (local, record) = rendered_local_record(entry, content, output.len());
                output.extend_from_slice(&local);
                central_records.push(record);
            }
            operation => {
                return Err(render_error(
                    "unsupported_operation",
                    &schema_path(&member.normalized_path),
                    operation,
                ));
            }
        }
    }

    let central_start = output.len();
    for record in &central_records {
        write_central_record(&mut output, record);
    }
    let central_size = output.len() - central_start;
    write_eocd(&mut output, central_records.len(), central_size, central_start);
    let inventory = parse_zip_inventory(&output)
        .map_err(|error| render_error("invalid_rendered_zip", "/archive", &error))?;
    let mut merge_report = plan.merge_report.clone();
    merge_report.preserved_ranges = plan
        .member_decisions
        .iter()
        .filter_map(|member| {
            raw_ranges
                .get(&member.normalized_path)
                .cloned()
                .filter(|_| member.operation == "preserve")
        })
        .collect();
    Ok((output, inventory, merge_report))
}

pub fn new_stored_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut output = vec![];
    let mut central = vec![];
    let mut entries = entries.to_vec();
    entries.sort_by_key(|entry| entry.0);
    for (name, content) in entries {
        let entry = ZipArchiveEntry {
            path: name.to_string(),
            normalized_path: normalize_zip_path(name),
            directory: false,
            compression: "stored".to_string(),
            compressed_size: content.len(),
            uncompressed_size: content.len(),
            crc32: format!("{:08x}", crc32(content.as_bytes())),
            local_header_range: ByteRange { start_byte: 0, end_byte: 0 },
            data_range: ByteRange { start_byte: 0, end_byte: 0 },
            central_directory_range: ByteRange { start_byte: 0, end_byte: 0 },
        };
        let (local, record) = rendered_local_record(&entry, content.as_bytes(), output.len());
        output.extend(local);
        central.push(record);
    }
    let start = output.len();
    for record in &central {
        write_central_record(&mut output, record);
    }
    let size = output.len() - start;
    write_eocd(&mut output, central.len(), size, start);
    output
}

#[derive(Clone)]
struct CentralScan {
    range: ByteRange,
    records: HashMap<String, CentralInfo>,
    archive_comment: bool,
}

#[derive(Clone)]
struct CentralInfo {
    range: ByteRange,
    flags: u16,
    method: u16,
    crc32: u32,
    compressed_size: usize,
    uncompressed_size: usize,
    extra_length: usize,
    comment_length: usize,
    local_offset: usize,
}

struct LocalInfo {
    data_start: usize,
}

#[derive(Clone)]
struct CentralRecord {
    name: String,
    method: u16,
    crc32: u32,
    compressed_size: usize,
    uncompressed_size: usize,
    offset: usize,
    flags: u16,
}

fn scan_central_directory(source: &[u8]) -> Result<CentralScan, String> {
    let eocd = (0..source.len().saturating_sub(3))
        .rev()
        .find(|offset| read_u32(source, *offset) == EOCD)
        .ok_or_else(|| "missing ZIP end of central directory".to_string())?;
    let size = read_u32(source, eocd + 12) as usize;
    let offset = read_u32(source, eocd + 16) as usize;
    let comment_length = read_u16(source, eocd + 20) as usize;
    let mut records = HashMap::new();
    let mut cursor = offset;
    while cursor < offset + size {
        if read_u32(source, cursor) != CENTRAL {
            return Err("unexpected central directory record".to_string());
        }
        let name_len = read_u16(source, cursor + 28) as usize;
        let extra_len = read_u16(source, cursor + 30) as usize;
        let comment_len = read_u16(source, cursor + 32) as usize;
        let name =
            String::from_utf8_lossy(&source[cursor + 46..cursor + 46 + name_len]).to_string();
        let end = cursor + 46 + name_len + extra_len + comment_len;
        records.insert(
            name,
            CentralInfo {
                range: ByteRange { start_byte: cursor, end_byte: end },
                flags: read_u16(source, cursor + 8),
                method: read_u16(source, cursor + 10),
                crc32: read_u32(source, cursor + 16),
                compressed_size: read_u32(source, cursor + 20) as usize,
                uncompressed_size: read_u32(source, cursor + 24) as usize,
                extra_length: extra_len,
                comment_length: comment_len,
                local_offset: read_u32(source, cursor + 42) as usize,
            },
        );
        cursor = end;
    }
    Ok(CentralScan {
        range: ByteRange { start_byte: offset, end_byte: offset + size },
        records,
        archive_comment: comment_length > 0,
    })
}

fn scan_local_headers(
    source: &[u8],
    records: &HashMap<String, CentralInfo>,
) -> Result<HashMap<String, LocalInfo>, String> {
    let mut result = HashMap::new();
    for (name, record) in records {
        if read_u32(source, record.local_offset) != LOCAL {
            return Err("unexpected ZIP local header".to_string());
        }
        let name_len = read_u16(source, record.local_offset + 26) as usize;
        let extra_len = read_u16(source, record.local_offset + 28) as usize;
        result.insert(
            name.clone(),
            LocalInfo { data_start: record.local_offset + 30 + name_len + extra_len },
        );
    }
    Ok(result)
}

fn validate_raw_preserve_entry(
    central: &CentralScan,
    entry: &ZipArchiveEntry,
) -> Result<(), RenderError> {
    if central.archive_comment {
        return Err(render_error(
            "archive_comment",
            "/archive/comment",
            "raw-preserving ZIP renderer does not yet preserve archive comments",
        ));
    }
    let record = central.records.get(&entry.path).expect("central record should exist");
    if record.flags & 0x1 != 0 {
        return Err(render_error(
            "encrypted_member",
            &schema_path(&entry.normalized_path),
            "raw-preserving ZIP renderer rejects encrypted member",
        ));
    }
    if record.method != 0 && record.method != 8 {
        return Err(render_error(
            "unsupported_compression",
            &schema_path(&entry.normalized_path),
            "raw-preserving ZIP renderer rejects unsupported compression",
        ));
    }
    if record.extra_length != 0 {
        return Err(render_error(
            "central_directory_extra_field",
            &schema_path(&entry.normalized_path),
            "raw-preserving ZIP renderer does not yet preserve central-directory extra fields",
        ));
    }
    if record.comment_length != 0 {
        return Err(render_error(
            "member_comment",
            &schema_path(&entry.normalized_path),
            "raw-preserving ZIP renderer does not yet preserve member comments",
        ));
    }
    Ok(())
}

fn unsafe_entries(
    entries: &[ZipArchiveEntry],
    records: &HashMap<String, CentralInfo>,
) -> Vec<ZipUnsafeEntry> {
    let mut seen = HashMap::new();
    let mut result = vec![];
    for entry in entries {
        if escapes_root(&entry.path) {
            result.push(unsafe_entry(entry, "path_traversal", "entry escapes the archive root"));
        }
        if seen.insert(entry.normalized_path.clone(), entry.path.clone()).is_some() {
            result.push(unsafe_entry(
                entry,
                "duplicate_normalized_path",
                "normalized path collides with an existing entry",
            ));
        }
        if records.get(&entry.path).map(|record| record.flags & 0x1 != 0).unwrap_or(false) {
            result.push(unsafe_entry(
                entry,
                "encrypted_member",
                "encrypted member cannot be rendered by the default provider",
            ));
        }
        if signing_sensitive(&entry.normalized_path) {
            result.push(unsafe_entry(
                entry,
                "signing_sensitive_member",
                "signature-bearing member mutation is not enabled",
            ));
        }
    }
    result
}

fn rendered_local_record(
    entry: &ZipArchiveEntry,
    content: &[u8],
    offset: usize,
) -> (Vec<u8>, CentralRecord) {
    let crc = crc32(content);
    let mut output = vec![];
    write_u32(&mut output, LOCAL);
    write_u16(&mut output, 20);
    write_u16(&mut output, 0);
    write_u16(&mut output, 0);
    output.extend_from_slice(&DOS_EPOCH);
    write_u32(&mut output, crc);
    write_u32(&mut output, content.len() as u32);
    write_u32(&mut output, content.len() as u32);
    write_u16(&mut output, entry.path.len() as u16);
    write_u16(&mut output, 0);
    output.extend_from_slice(entry.path.as_bytes());
    output.extend_from_slice(content);
    (
        output,
        CentralRecord {
            name: entry.path.clone(),
            method: 0,
            crc32: crc,
            compressed_size: content.len(),
            uncompressed_size: content.len(),
            offset,
            flags: 0,
        },
    )
}

fn write_central_record(output: &mut Vec<u8>, record: &CentralRecord) {
    write_u32(output, CENTRAL);
    write_u16(output, 20);
    write_u16(output, 20);
    write_u16(output, record.flags);
    write_u16(output, record.method);
    output.extend_from_slice(&DOS_EPOCH);
    write_u32(output, record.crc32);
    write_u32(output, record.compressed_size as u32);
    write_u32(output, record.uncompressed_size as u32);
    write_u16(output, record.name.len() as u16);
    write_u16(output, 0);
    write_u16(output, 0);
    write_u16(output, 0);
    write_u16(output, 0);
    write_u32(output, 0);
    write_u32(output, record.offset as u32);
    output.extend_from_slice(record.name.as_bytes());
}

fn write_eocd(output: &mut Vec<u8>, entries: usize, size: usize, offset: usize) {
    write_u32(output, EOCD);
    write_u16(output, 0);
    write_u16(output, 0);
    write_u16(output, entries as u16);
    write_u16(output, entries as u16);
    write_u32(output, size as u32);
    write_u32(output, offset as u32);
    write_u16(output, 0);
}

fn record_from_entry(entry: &ZipArchiveEntry, offset: usize) -> CentralRecord {
    CentralRecord {
        name: entry.path.clone(),
        method: if entry.compression == "deflate" { 8 } else { 0 },
        crc32: u32::from_str_radix(&entry.crc32, 16).unwrap_or(0),
        compressed_size: entry.compressed_size,
        uncompressed_size: entry.uncompressed_size,
        offset,
        flags: 0,
    }
}

fn raw_local_record_ranges(
    entries: &HashMap<String, ZipArchiveEntry>,
) -> HashMap<String, ByteRange> {
    let mut ordered = entries.values().collect::<Vec<_>>();
    ordered.sort_by_key(|entry| entry.local_header_range.start_byte);
    ordered
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let end_byte = ordered
                .get(index + 1)
                .map(|next| next.local_header_range.start_byte)
                .unwrap_or(entry.central_directory_range.start_byte);
            (
                entry.normalized_path.clone(),
                ByteRange { start_byte: entry.local_header_range.start_byte, end_byte },
            )
        })
        .collect()
}

fn empty_report() -> BinaryMergeReport {
    BinaryMergeReport {
        format: "zip".to_string(),
        schema: "zip.ksy".to_string(),
        matched_schema_paths: vec![],
        preserved_ranges: vec![],
        rewritten_nodes: vec![],
        checksum_updates: vec![],
        nested_dispatches: vec![],
        diagnostics: vec![],
    }
}

fn decision(
    report: &mut ZipFamilyReport,
    path: &str,
    operation: &str,
    disposition: &str,
    reason: &str,
) {
    report.member_decisions.push(ZipMemberDecision {
        normalized_path: path.to_string(),
        operation: operation.to_string(),
        disposition: disposition.to_string(),
        nested_family: None,
        reason: reason.to_string(),
    });
    report.merge_report.rewritten_nodes.push(schema_path(path));
}

fn entries_by_path(entries: &[ZipArchiveEntry]) -> HashMap<String, ZipArchiveEntry> {
    entries.iter().map(|entry| (entry.normalized_path.clone(), entry.clone())).collect()
}

fn same_entry(left: &ZipArchiveEntry, right: &ZipArchiveEntry) -> bool {
    left.path == right.path
        && left.compression == right.compression
        && left.compressed_size == right.compressed_size
        && left.uncompressed_size == right.uncompressed_size
        && left.crc32 == right.crc32
}

fn normalize_zip_path(path: &str) -> String {
    let mut stack = vec![];
    let normalized = path.replace('\\', "/");
    for part in normalized.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                stack.pop();
            }
            other => stack.push(other),
        }
    }
    stack.join("/")
}

fn escapes_root(path: &str) -> bool {
    let mut depth = 0isize;
    for part in path.replace('\\', "/").split('/') {
        match part {
            "." | "" => {}
            ".." => depth -= 1,
            _ => depth += 1,
        }
        if depth < 0 {
            return true;
        }
    }
    path.starts_with('/')
}

fn signing_sensitive(path: &str) -> bool {
    let upper = path.to_uppercase();
    upper.starts_with("META-INF/")
        && [".RSA", ".DSA", ".EC", ".SF"].iter().any(|suffix| upper.ends_with(suffix))
}

fn nested_family(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        Some("markdown")
    } else if lower.ends_with(".json") {
        Some("json")
    } else if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        Some("yaml")
    } else if lower.ends_with(".xml") {
        Some("xml")
    } else {
        None
    }
}

fn compression_name(method: u16) -> String {
    match method {
        0 => "stored".to_string(),
        8 => "deflate".to_string(),
        other => format!("method-{other}"),
    }
}

fn unsafe_entry(entry: &ZipArchiveEntry, category: &str, reason: &str) -> ZipUnsafeEntry {
    ZipUnsafeEntry {
        path: entry.path.clone(),
        normalized_path: entry.normalized_path.clone(),
        category: category.to_string(),
        reason: reason.to_string(),
    }
}

fn diagnostic(category: &str, schema_path: &str, message: &str) -> BinaryDiagnostic {
    BinaryDiagnostic {
        severity: "error".to_string(),
        category: category.to_string(),
        message: message.to_string(),
        schema_path: schema_path.to_string(),
        byte_range: None,
    }
}

fn render_error(category: &str, schema_path: &str, message: &str) -> RenderError {
    RenderError { diagnostic: diagnostic(category, schema_path, message) }
}

fn schema_path(path: &str) -> String {
    format!("/entries/by_path/{path}")
}

fn read_u16(source: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([source[offset], source[offset + 1]])
}

fn read_u32(source: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([source[offset], source[offset + 1], source[offset + 2], source[offset + 3]])
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(source: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in source {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}
