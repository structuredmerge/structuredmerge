use structuredmerge_core::{
    CoreError, ParserSelection, SelectionReport,
    parsed::ParseValidationError,
    service::{ProviderFault, ServiceError},
    source::{SourceError, SourceErrorCode},
};

#[test]
fn service_failures_have_stable_distinct_codes() {
    let cases = [
        (ServiceError::InvalidRequest, "request.invalid"),
        (ServiceError::LimitExceeded, "resource.limit"),
        (ServiceError::Cancelled, "execution.cancelled"),
        (ServiceError::DeadlineExceeded, "execution.deadline_exceeded"),
        (
            ServiceError::Source(SourceError {
                code: SourceErrorCode::DescriptorMismatch,
                source_id: "ours".into(),
            }),
            "source.invalid",
        ),
        (
            ServiceError::Selection(Box::new(SelectionReport {
                requested: ParserSelection {
                    backend_id: Some("missing".into()),
                    preference: vec![],
                    required_capabilities: vec![],
                },
                generation: 0,
                digest: "snapshot".into(),
                candidates: vec![],
                selected_backend: None,
            })),
            "selection.no_parser",
        ),
        (
            ServiceError::Provider {
                backend_id: "native".into(),
                fault: ProviderFault {
                    code: "native_code".into(),
                    message: "native exception".into(),
                },
            },
            "parser.provider_fault",
        ),
        (ServiceError::ProviderPanic { backend_id: "native".into() }, "parser.provider_panic"),
        (ServiceError::InvalidBatch { backend_id: "native".into() }, "parser.invalid_batch"),
        (
            ServiceError::InvalidResult {
                backend_id: "native".into(),
                error: ParseValidationError::IdentityMismatch,
            },
            "parser.invalid_result",
        ),
        (
            ServiceError::InvalidResult {
                backend_id: "native".into(),
                error: ParseValidationError::LimitExceeded,
            },
            "resource.limit",
        ),
    ];
    for (error, expected) in cases {
        let projected = CoreError::from(error);
        assert_eq!(projected.code, expected);
        assert!(!projected.message.is_empty());
        assert!(projected.to_string().starts_with(&format!("{expected}: ")));
    }
}

#[test]
fn provider_native_code_cannot_replace_the_portable_code() {
    let error = CoreError::from(ServiceError::Provider {
        backend_id: "ruby.psych".into(),
        fault: ProviderFault {
            code: "execution.cancelled".into(),
            message: "provider says cancelled".into(),
        },
    });
    assert_eq!(error.code, "parser.provider_fault");
    assert!(error.message.contains("ruby.psych"));
    assert!(error.message.contains("execution.cancelled"));
}
