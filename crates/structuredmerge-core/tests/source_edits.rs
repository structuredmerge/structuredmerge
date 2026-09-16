use structuredmerge_core::*;

fn request(edits: Vec<ExplicitSourceEdit>) -> SourceEditRequest {
    SourceEditRequest {
        request_id: "edit-1".into(),
        source: source_input(
            "source-1".into(),
            SourceRole::Source,
            SourceEncoding::Utf8,
            "\u{feff}é: one\r\nlast".as_bytes().to_vec(),
        )
        .unwrap(),
        edits,
    }
}
fn edit(start_byte: usize, end_byte: usize, replacement: &str) -> ExplicitSourceEdit {
    ExplicitSourceEdit { start_byte, end_byte, replacement: replacement.into() }
}
fn limits() -> SourceEditLimits {
    SourceEditLimits { max_input_bytes: 100, max_output_bytes: 100, max_edits: 10 }
}

#[test]
fn exact_bytes_and_identity_survive_unsorted_edits_and_noop() {
    let input = request(vec![edit(12, 16, "tail"), edit(7, 10, "two")]);
    let descriptor = input.source.descriptor.clone();
    let result = apply_explicit_source_edits(input, limits()).unwrap();
    assert_eq!(result.request_id, "edit-1");
    assert_eq!(result.source, descriptor);
    assert_eq!(result.output, "\u{feff}é: two\r\ntail");
    assert_eq!(result.edit_count, 2);
    assert_eq!(
        apply_explicit_source_edits(request(vec![]), limits()).unwrap().output,
        "\u{feff}é: one\r\nlast"
    );
}

#[test]
fn invalid_ranges_fail_atomically() {
    for edits in [
        vec![edit(4, 5, "x")],
        vec![edit(8, 7, "x")],
        vec![edit(17, 17, "x")],
        vec![edit(7, 10, "x"), edit(8, 9, "y")],
        vec![edit(7, 7, "x"), edit(7, 7, "y")],
    ] {
        assert_eq!(
            apply_explicit_source_edits(request(edits), limits()).unwrap_err().code,
            "source_edit.rejected"
        );
    }
}

#[test]
fn validates_identity_source_and_resource_budgets() {
    let mut input = request(vec![]);
    input.source.bytes[7] = b'z';
    assert_eq!(apply_explicit_source_edits(input, limits()).unwrap_err().code, "source.invalid");
    let mut input = request(vec![]);
    input.request_id.clear();
    assert_eq!(apply_explicit_source_edits(input, limits()).unwrap_err().code, "request.invalid");
    let mut input = request(vec![]);
    input.source.descriptor.role = SourceRole::Current;
    assert_eq!(apply_explicit_source_edits(input, limits()).unwrap_err().code, "request.invalid");
    for budget in [
        SourceEditLimits { max_input_bytes: 15, ..limits() },
        SourceEditLimits { max_output_bytes: 15, ..limits() },
        SourceEditLimits { max_edits: 0, ..limits() },
    ] {
        assert_eq!(
            apply_explicit_source_edits(request(vec![edit(16, 16, "!")]), budget).unwrap_err().code,
            "resource.limit"
        );
    }
    // Output budget counts the edited output, not source plus all replacements.
    let result = apply_explicit_source_edits(
        request(vec![edit(0, 16, "")]),
        SourceEditLimits { max_output_bytes: 0, ..limits() },
    )
    .unwrap();
    assert_eq!(result.output, "");
}
