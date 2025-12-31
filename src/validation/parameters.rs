use super::operations::OperationValidator;
use super::{ErrorType, ValidationError, ValidationWarning};
use crate::error::Result;
use crate::loader::OpenApiResolver;
use crate::models::arazzo::ArazzoSpec;
use oas3::spec::ObjectOrReference;

/// Validator for parameter compatibility
pub struct ParameterValidator<'a> {
    arazzo: &'a ArazzoSpec,
    resolver: &'a OpenApiResolver,
}

impl<'a> ParameterValidator<'a> {
    pub fn new(arazzo: &'a ArazzoSpec, resolver: &'a OpenApiResolver) -> Self {
        Self { arazzo, resolver }
    }

    /// Validate parameter compatibility
    pub fn validate(&self) -> Result<(Vec<ValidationError>, Vec<ValidationWarning>)> {
        let mut errors = vec![];
        let mut warnings = vec![];

        let op_validator = OperationValidator::new(self.arazzo, self.resolver);

        for workflow in &self.arazzo.workflows {
            for step in &workflow.steps {
                // Skip steps that reference workflows (no operation to validate)
                if step.workflow_id.is_some() {
                    continue;
                }

                // Get the operation for this step
                let operation = match op_validator.get_operation_from_step(step) {
                    Some(op) => op,
                    None => continue, // Operation not found - will be caught by operations validator
                };

                // Extract OpenAPI parameters
                let openapi_params = self.extract_parameters(&operation);

                // Check required parameters
                for openapi_param in &openapi_params {
                    if openapi_param.required.unwrap_or(false) {
                        // Check if this required parameter is provided in the step
                        let param_provided = step.parameters.iter().any(|p| {
                            p.name == openapi_param.name && p.location == openapi_param.location
                        });

                        if !param_provided {
                            errors.push(
                                ValidationError::new(
                                    ErrorType::RequiredParameterMissing,
                                    format!(
                                        "Required parameter missing: '{}' (in: {}) is required by OpenAPI but not provided in step",
                                        openapi_param.name, openapi_param.location
                                    ),
                                )
                                .with_workflow(&workflow.workflow_id)
                                .with_step(&step.step_id),
                            );
                        }
                    }
                }

                // Check for extra parameters not defined in OpenAPI
                for step_param in &step.parameters {
                    let param_defined = openapi_params.iter().any(|op_param| {
                        op_param.name == step_param.name && op_param.location == step_param.location
                    });

                    if !param_defined {
                        // Check if it's a runtime expression (starts with $)
                        let is_runtime_expr = step_param
                            .value
                            .as_str()
                            .map(|s| s.starts_with('$'))
                            .unwrap_or(false);

                        if !is_runtime_expr {
                            warnings.push(
                                ValidationWarning::new(format!(
                                    "Extra parameter: '{}' (in: {}) is not defined in OpenAPI operation",
                                    step_param.name, step_param.location
                                ))
                                .with_workflow(&workflow.workflow_id)
                                .with_step(&step.step_id),
                            );
                        }
                    }
                }

                // Check parameter locations match
                for step_param in &step.parameters {
                    if let Some(openapi_param) =
                        openapi_params.iter().find(|p| p.name == step_param.name)
                        && openapi_param.location != step_param.location
                    {
                        errors.push(
                                ValidationError::new(
                                    ErrorType::ParameterLocationMismatch,
                                    format!(
                                        "Parameter location mismatch: '{}' is defined as 'in: {}' in OpenAPI but provided as 'in: {}' in step",
                                        step_param.name, openapi_param.location, step_param.location
                                    ),
                                )
                                .with_workflow(&workflow.workflow_id)
                                .with_step(&step.step_id),
                            );
                    }
                }
            }
        }

        Ok((errors, warnings))
    }

    /// Extract parameters from an operation
    fn extract_parameters(&self, operation: &oas3::spec::Operation) -> Vec<ParameterInfo> {
        let mut params = vec![];

        // operation.parameters is a Vec<ObjectOrReference<Parameter>>
        for param_ref in &operation.parameters {
            match param_ref {
                ObjectOrReference::Object(param) => {
                    params.push(ParameterInfo {
                        name: param.name.clone(),
                        location: format!("{:?}", param.location).to_lowercase(),
                        required: param.required,
                    });
                }
                ObjectOrReference::Ref { .. } => {
                    // For now, skip $ref resolution
                    // Future enhancement: resolve $ref to actual parameter
                }
            }
        }

        params
    }
}

/// Information about a parameter from OpenAPI
#[derive(Debug, Clone)]
struct ParameterInfo {
    name: String,
    location: String,
    required: Option<bool>,
}
