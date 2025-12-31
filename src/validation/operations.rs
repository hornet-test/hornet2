use super::{ErrorType, ValidationError, ValidationWarning};
use crate::error::Result;
use crate::loader::OpenApiResolver;
use crate::models::arazzo::{ArazzoSpec, Step};
use oas3::spec::Operation;

/// Validator for operation references
pub struct OperationValidator<'a> {
    arazzo: &'a ArazzoSpec,
    resolver: &'a OpenApiResolver,
}

impl<'a> OperationValidator<'a> {
    pub fn new(arazzo: &'a ArazzoSpec, resolver: &'a OpenApiResolver) -> Self {
        Self { arazzo, resolver }
    }

    /// Validate all operation references
    pub fn validate(&self) -> Result<(Vec<ValidationError>, Vec<ValidationWarning>)> {
        let mut errors = vec![];
        let warnings = vec![];

        // Build workflow ID set for workflow reference checks
        let workflow_ids: std::collections::HashSet<_> = self
            .arazzo
            .workflows
            .iter()
            .map(|w| &w.workflow_id)
            .collect();

        // Validate each workflow's steps
        for workflow in &self.arazzo.workflows {
            for step in &workflow.steps {
                // Check operationId reference
                if let Some(op_id) = &step.operation_id {
                    if let Some(op_ref) = self.resolver.find_operation(op_id) {
                        // Operation found - optionally add source info to error context if needed
                        let _ = op_ref.source_name; // Available for future use
                    } else {
                        errors.push(
                            ValidationError::new(
                                ErrorType::OperationIdNotFound,
                                format!("Operation not found: operationId '{}' does not exist in any OpenAPI source", op_id),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                    }
                }

                // Check operationPath reference
                if let Some(op_path) = &step.operation_path
                    && !self.find_operation_by_path(op_path)
                {
                    errors.push(
                            ValidationError::new(
                                ErrorType::OperationPathNotFound,
                                format!("Operation not found: operationPath '{}' does not exist in any OpenAPI source", op_path),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                }

                // Check workflowId reference
                if let Some(wf_id) = &step.workflow_id
                    && !workflow_ids.contains(wf_id)
                {
                    errors.push(
                            ValidationError::new(
                                ErrorType::WorkflowRefNotFound,
                                format!("Workflow reference not found: workflowId '{}' does not exist in Arazzo spec", wf_id),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                }
            }
        }

        Ok((errors, warnings))
    }

    /// Check if an operationPath exists in the OpenAPI spec
    /// operationPath format: "{method} {path}" (e.g., "GET /users/{id}")
    fn find_operation_by_path(&self, operation_path: &str) -> bool {
        // Parse operationPath: "METHOD /path"
        let parts: Vec<&str> = operation_path.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return false;
        }

        let method = parts[0];
        let path = parts[1];

        self.resolver
            .find_operation_by_path_with_details(path, method)
            .is_some()
    }

    /// Get operation by operationId (for use by other validators)
    pub fn get_operation_by_id(&self, operation_id: &str) -> Option<Operation> {
        self.resolver
            .find_operation_with_details(operation_id)
            .map(|(_, op)| op)
    }

    /// Get operation by operationPath (for use by other validators)
    pub fn get_operation_by_path(&self, operation_path: &str) -> Option<Operation> {
        let parts: Vec<&str> = operation_path.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return None;
        }

        let method = parts[0];
        let path = parts[1];

        self.resolver
            .find_operation_by_path_with_details(path, method)
            .map(|(_, op)| op)
    }

    /// Get operation from a step (helper method)
    pub fn get_operation_from_step(&self, step: &Step) -> Option<Operation> {
        if let Some(op_id) = &step.operation_id {
            return self.get_operation_by_id(op_id);
        }

        if let Some(op_path) = &step.operation_path {
            return self.get_operation_by_path(op_path);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader;
    use std::path::PathBuf;

    #[test]
    fn test_operation_path_parsing() {
        // Test operationPath format parsing
        let path1 = "GET /users/{id}";
        let parts: Vec<&str> = path1.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "GET");
        assert_eq!(parts[1], "/users/{id}");

        let path2 = "POST /users";
        let parts: Vec<&str> = path2.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "POST");
        assert_eq!(parts[1], "/users");
    }

    #[test]
    fn test_validate_existing_operation_id() {
        use crate::loader::OpenApiResolver;

        // Load fixtures
        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");
        let arazzo_path = PathBuf::from("tests/fixtures/arazzo.yaml");

        let arazzo = loader::load_arazzo(&arazzo_path).expect("Failed to load Arazzo");

        // Create resolver and load OpenAPI spec
        let mut resolver = OpenApiResolver::new(PathBuf::from("tests/fixtures"));
        resolver
            .load_spec("userAPI", &openapi_path)
            .expect("Failed to load OpenAPI");

        let validator = OperationValidator::new(&arazzo, &resolver);
        let (errors, _warnings) = validator.validate().expect("Validation failed");

        // All operationIds in the fixture should be valid
        let op_id_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e.error_type, ErrorType::OperationIdNotFound))
            .collect();

        assert_eq!(
            op_id_errors.len(),
            0,
            "Should have no operationId errors, but got: {:?}",
            op_id_errors
        );
    }

    #[test]
    fn test_validate_missing_operation_id() {
        use crate::loader::OpenApiResolver;

        // Load fixtures
        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");

        // Create resolver and load OpenAPI spec
        let mut resolver = OpenApiResolver::new(PathBuf::from("tests/fixtures"));
        resolver
            .load_spec("userAPI", &openapi_path)
            .expect("Failed to load OpenAPI");

        // Create a minimal Arazzo with invalid operationId
        let arazzo_yaml = r#"
arazzo: 1.0.0
info:
  title: Test
  version: 1.0.0
workflows:
  - workflowId: test-flow
    steps:
      - stepId: invalid-step
        operationId: nonExistentOperation
"#;

        let arazzo: ArazzoSpec = serde_yaml::from_str(arazzo_yaml).expect("Failed to parse Arazzo");

        let validator = OperationValidator::new(&arazzo, &resolver);
        let (errors, _warnings) = validator.validate().expect("Validation failed");

        // Should have an operationId not found error
        let op_id_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e.error_type, ErrorType::OperationIdNotFound))
            .collect();

        assert_eq!(
            op_id_errors.len(),
            1,
            "Should have 1 operationId error, got {}",
            op_id_errors.len()
        );
    }

    #[test]
    fn test_validate_workflow_reference() {
        use crate::loader::OpenApiResolver;

        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");

        // Create resolver and load OpenAPI spec
        let mut resolver = OpenApiResolver::new(PathBuf::from("tests/fixtures"));
        resolver
            .load_spec("userAPI", &openapi_path)
            .expect("Failed to load OpenAPI");

        // Create Arazzo with invalid workflowId reference
        let arazzo_yaml = r#"
arazzo: 1.0.0
info:
  title: Test
  version: 1.0.0
workflows:
  - workflowId: workflow1
    steps:
      - stepId: step1
        workflowId: nonExistentWorkflow
"#;

        let arazzo: ArazzoSpec = serde_yaml::from_str(arazzo_yaml).expect("Failed to parse Arazzo");

        let validator = OperationValidator::new(&arazzo, &resolver);
        let (errors, _warnings) = validator.validate().expect("Validation failed");

        // Should have a workflow ref not found error
        let wf_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e.error_type, ErrorType::WorkflowRefNotFound))
            .collect();

        assert_eq!(
            wf_errors.len(),
            1,
            "Should have 1 workflow ref error, got {}",
            wf_errors.len()
        );
    }
}
