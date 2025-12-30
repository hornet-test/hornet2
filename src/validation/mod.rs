mod consistency;
mod data_dependencies;
mod operations;
mod parameters;
mod schemas;

pub use consistency::ArazzoOpenApiValidator;
pub use consistency::ConsistencyValidationResult;
pub use consistency::ValidationError;
pub use consistency::ValidationWarning;

/// Type of validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    // Basic checks
    OperationIdNotFound,
    OperationPathNotFound,
    WorkflowRefNotFound,

    // Parameter checks
    RequiredParameterMissing,
    ParameterTypeMismatch,
    ParameterLocationMismatch,

    // Data dependency checks
    InvalidStepReference,
    StepOrderViolation,
    InvalidInputReference,
    InvalidResponseRefContext,

    // Schema checks
    RequestBodySchemaMismatch,
    ResponseSchemaMismatch,
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::OperationIdNotFound => write!(f, "Operation ID not found"),
            ErrorType::OperationPathNotFound => write!(f, "Operation path not found"),
            ErrorType::WorkflowRefNotFound => write!(f, "Workflow reference not found"),
            ErrorType::RequiredParameterMissing => write!(f, "Required parameter missing"),
            ErrorType::ParameterTypeMismatch => write!(f, "Parameter type mismatch"),
            ErrorType::ParameterLocationMismatch => write!(f, "Parameter location mismatch"),
            ErrorType::InvalidStepReference => write!(f, "Invalid step reference"),
            ErrorType::StepOrderViolation => write!(f, "Step order violation"),
            ErrorType::InvalidInputReference => write!(f, "Invalid input reference"),
            ErrorType::InvalidResponseRefContext => write!(f, "Invalid response reference context"),
            ErrorType::RequestBodySchemaMismatch => write!(f, "Request body schema mismatch"),
            ErrorType::ResponseSchemaMismatch => write!(f, "Response schema mismatch"),
        }
    }
}
