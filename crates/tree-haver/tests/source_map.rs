use tree_haver::source::{
    SourceDocument, SourceEncoding, SourceErrorCode, SourceInput, SourceMap, SourceRole,
    source_input,
};
use tree_haver::{ByteRange, SourcePoint};

fn input(bytes: &[u8]) -> SourceInput {
    source_input("source-1".into(), SourceRole::Current, SourceEncoding::Utf8, bytes.to_vec())
        .unwrap()
}

#[test]
fn exact_bytes_include_bom_crlf_unicode_and_no_final_newline() {
    let bytes = "\u{feff}é\r\n中\nlast\rbare".as_bytes();
    let input = input(bytes);
    assert!(input.descriptor.bom);
    assert_eq!(input.descriptor.line_endings.crlf, 1);
    assert_eq!(input.descriptor.line_endings.lf, 1);
    assert_eq!(input.descriptor.line_endings.bare_cr, 1);
    assert!(!input.descriptor.final_newline);
    let document = SourceDocument::validate(input, 100).unwrap();
    assert_eq!(document.bytes(), bytes);
    assert_eq!(document.point(5).unwrap(), SourcePoint { row: 0, column: 5 });
    assert_eq!(document.point(7).unwrap(), SourcePoint { row: 1, column: 0 });
    assert_eq!(document.point(10).unwrap(), SourcePoint { row: 1, column: 3 });
    assert_eq!(document.slice(ByteRange { start_byte: 7, end_byte: 10 }).unwrap(), "中".as_bytes());
}

#[test]
fn independently_known_sha256_and_empty_range() {
    let document = SourceDocument::validate(input(b"abc"), 3).unwrap();
    assert_eq!(
        document.descriptor().sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        document.range_digest(ByteRange { start_byte: 3, end_byte: 3 }).unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn rejects_every_falsified_preservation_field() {
    let original = input(b"abc\r\n");
    let mutations: Vec<fn(&mut SourceInput)> = vec![
        |source| source.descriptor.byte_length += 1,
        |source| source.descriptor.sha256 = "0".repeat(64),
        |source| source.descriptor.bom = true,
        |source| source.descriptor.line_endings.crlf = 0,
        |source| source.descriptor.line_endings.lf = 1,
        |source| source.descriptor.line_endings.bare_cr = 1,
        |source| source.descriptor.final_newline = false,
        |source| source.bytes[0] = b'z',
    ];
    for mutate in mutations {
        let mut source = original.clone();
        mutate(&mut source);
        assert_eq!(
            SourceDocument::validate(source, 100).unwrap_err().code,
            SourceErrorCode::DescriptorMismatch
        );
    }
}

#[test]
fn binary_source_is_retained_without_lossy_decoding() {
    let bytes = vec![0xff, 0, 0xfe, 13, 10];
    assert_eq!(
        source_input("binary".into(), SourceRole::Base, SourceEncoding::Utf8, bytes.clone())
            .unwrap_err()
            .code,
        SourceErrorCode::InvalidEncoding
    );
    let binary =
        source_input("binary".into(), SourceRole::Base, SourceEncoding::Binary, bytes.clone())
            .unwrap();
    let document = SourceDocument::validate(binary, 5).unwrap();
    assert_eq!(document.bytes(), bytes);
    assert_eq!(document.point(0).unwrap_err().code, SourceErrorCode::InvalidEncoding);
}

#[test]
fn ranges_and_total_limits_fail_closed() {
    let document = SourceDocument::validate(input(b"abc"), 3).unwrap();
    for (start_byte, end_byte) in [(3, 2), (0, 4), (usize::MAX, usize::MAX)] {
        assert_eq!(
            document.slice(ByteRange { start_byte, end_byte }).unwrap_err().code,
            SourceErrorCode::InvalidRange
        );
    }
    assert_eq!(document.point(4).unwrap_err().code, SourceErrorCode::InvalidRange);
    let mut second = input(b"xyz");
    second.descriptor.source_id = "source-2".into();
    assert_eq!(
        SourceMap::validate(vec![input(b"abc"), second], 5).unwrap_err().code,
        SourceErrorCode::LimitExceeded
    );
}

#[test]
fn source_map_cannot_replace_an_existing_identity() {
    assert_eq!(
        SourceMap::validate(vec![input(b"a"), input(b"b")], 10).unwrap_err().code,
        SourceErrorCode::DuplicateId
    );
    let map = SourceMap::validate(vec![input(b"a")], 1).unwrap();
    assert_eq!(map.get("missing").unwrap_err().code, SourceErrorCode::UnknownSource);
    assert_eq!(map.get("source-1").unwrap().bytes(), b"a");
}

#[test]
fn indexed_points_match_byte_positions_at_every_offset() {
    for bytes in [b"".as_slice(), b"\n\n", "é\r\n中\nlast\rbare".as_bytes()] {
        let document = SourceDocument::validate(input(bytes), 100).unwrap();
        let mut expected = SourcePoint { row: 0, column: 0 };
        assert_eq!(document.point(0).unwrap(), expected);
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                expected.row += 1;
                expected.column = 0;
            } else {
                expected.column += 1;
            }
            assert_eq!(document.point(index + 1).unwrap(), expected);
        }
        assert_eq!(
            document.point(bytes.len() + 1).unwrap_err().code,
            SourceErrorCode::InvalidRange
        );
    }
}
