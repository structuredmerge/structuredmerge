//! Typed common-operation facade. No implicit filesystem/reference resolution.
use crate::operation::{OperationRequest, OperationRequestErrorCode};
use crate::operation_result::OperationResult;
use crate::{CoreError, OperationControl, ParseLimits};

pub fn execute_operation(
    request: OperationRequest,
    limits: ParseLimits,
) -> Result<OperationResult, CoreError> {
    execute_operation_controlled(request, limits, &OperationControl::new())
}

/// The deadline covers normalization, registry selection, parsing, Rust
/// analysis/merge and verification. Source references must be resolved by the
/// caller into checked inline content; this API never opens a named path.
pub fn execute_operation_controlled(
    request: OperationRequest,
    limits: ParseLimits,
    control: &OperationControl,
) -> Result<OperationResult, CoreError> {
    let context = limits.controlled_context(control)?;
    let check = || {
        context.check().map_err(|error| {
            let failure = crate::ParserFailure::from(error);
            CoreError {
                code: failure.code,
                message: "operation execution control rejected request".into(),
            }
        })
    };
    check()?;
    let validated = request.validate(context.max_input_bytes, |_, _| {
        Err("common facade requires inline source content or bytes".into())
    });
    check()?;
    let validated = validated.map_err(|error| CoreError {
        code: match error.code {
            OperationRequestErrorCode::ResourceLimit => "resource.limit",
            OperationRequestErrorCode::SourceResolution => "source.unresolved_reference",
            _ => "operation.invalid_request",
        }
        .into(),
        message: error.to_string(),
    })?;
    let snapshot = crate::host::registry()
        .snapshot()
        .map_err(|error| CoreError { code: "registry".into(), message: format!("{error:?}") })?;
    crate::native_operation::execute_native_operation(&validated, &snapshot, &context)
}
