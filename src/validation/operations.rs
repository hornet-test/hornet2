use super::{ErrorType, ValidationError, ValidationWarning};
use crate::error::Result;
use crate::models::arazzo::{ArazzoSpec, Step};
use oas3::spec::Operation;
use oas3::OpenApiV3Spec;
use std::collections::HashMap;

/// Validator for operation references
pub struct OperationValidator<'a> {
    arazzo: &'a ArazzoSpec,
    openapi: &'a OpenApiV3Spec,
}

impl<'a> OperationValidator<'a> {
    pub fn new(arazzo: &'a ArazzoSpec, openapi: &'a OpenApiV3Spec) -> Self {
        Self { arazzo, openapi }
    }

    /// Validate all operation references
    pub fn validate(&self) -> Result<(Vec<ValidationError>, Vec<ValidationWarning>)> {
        let mut errors = vec![];
        let warnings = vec![];

        // Build operation cache for faster lookups
        let op_cache = self.build_operation_cache();

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
                    if !op_cache.contains_key(op_id.as_str()) {
                        errors.push(
                            ValidationError::new(
                                ErrorType::OperationIdNotFound,
                                format!("Operation not found: operationId '{}' does not exist in OpenAPI spec", op_id),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                    }
                }

                // Check operationPath reference
                if let Some(op_path) = &step.operation_path {
                    if !self.find_operation_by_path(op_path) {
                        errors.push(
                            ValidationError::new(
                                ErrorType::OperationPathNotFound,
                                format!("Operation not found: operationPath '{}' does not exist in OpenAPI spec", op_path),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                    }
                }

                // Check workflowId reference
                if let Some(wf_id) = &step.workflow_id {
                    if !workflow_ids.contains(wf_id) {
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
        }

        Ok((errors, warnings))
    }

    /// Build a cache of all operations by operationId
    fn build_operation_cache(&self) -> HashMap<String, Operation> {
        let mut cache = HashMap::new();

        if let Some(paths) = &self.openapi.paths {
            for (_path, path_item) in paths.iter() {
                let operations = [
                    &path_item.get,
                    &path_item.post,
                    &path_item.put,
                    &path_item.delete,
                    &path_item.patch,
                    &path_item.options,
                    &path_item.head,
                    &path_item.trace,
                ];

                for op in operations.iter().copied().flatten() {
                    if let Some(op_id) = &op.operation_id {
                        cache.insert(op_id.clone(), op.clone());
                    }
                }
            }
        }

        cache
    }

    /// Check if an operationPath exists in the OpenAPI spec
    /// operationPath format: "{method} {path}" (e.g., "GET /users/{id}")
    fn find_operation_by_path(&self, operation_path: &str) -> bool {
        // Parse operationPath: "METHOD /path"
        let parts: Vec<&str> = operation_path.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return false;
        }

        let method = parts[0].to_uppercase();
        let path = parts[1];

        if let Some(paths) = &self.openapi.paths {
            if let Some(path_item) = paths.get(path) {
                let op_option = match method.as_str() {
                    "GET" => &path_item.get,
                    "POST" => &path_item.post,
                    "PUT" => &path_item.put,
                    "DELETE" => &path_item.delete,
                    "PATCH" => &path_item.patch,
                    "OPTIONS" => &path_item.options,
                    "HEAD" => &path_item.head,
                    "TRACE" => &path_item.trace,
                    _ => &None,
                };

                return op_option.is_some();
            }
        }

        false
    }

    /// Get operation by operationId (for use by other validators)
    pub fn get_operation_by_id(&self, operation_id: &str) -> Option<Operation> {
        let cache = self.build_operation_cache();
        cache.get(operation_id).cloned()
    }

    /// Get operation by operationPath (for use by other validators)
    pub fn get_operation_by_path(&self, operation_path: &str) -> Option<Operation> {
        let parts: Vec<&str> = operation_path.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return None;
        }

        let method = parts[0].to_uppercase();
        let path = parts[1];

        if let Some(paths) = &self.openapi.paths {
            if let Some(path_item) = paths.get(path) {
                let op_option = match method.as_str() {
                    "GET" => &path_item.get,
                    "POST" => &path_item.post,
                    "PUT" => &path_item.put,
                    "DELETE" => &path_item.delete,
                    "PATCH" => &path_item.patch,
                    "OPTIONS" => &path_item.options,
                    "HEAD" => &path_item.head,
                    "TRACE" => &path_item.trace,
                    _ => &None,
                };

                return op_option.as_ref().map(|op| (*op).clone());
            }
        }

        None
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
        // Load fixtures
        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");
        let arazzo_path = PathBuf::from("tests/fixtures/arazzo.yaml");

        let openapi = loader::load_openapi(&openapi_path).expect("Failed to load OpenAPI");
        let arazzo = loader::load_arazzo(&arazzo_path).expect("Failed to load Arazzo");

        let validator = OperationValidator::new(&arazzo, &openapi);
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
        // Load fixtures
        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");
        let openapi = loader::load_openapi(&openapi_path).expect("Failed to load OpenAPI");

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

        let validator = OperationValidator::new(&arazzo, &openapi);
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
        let openapi_path = PathBuf::from("tests/fixtures/openapi.yaml");
        let openapi = loader::load_openapi(&openapi_path).expect("Failed to load OpenAPI");

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

        let validator = OperationValidator::new(&arazzo, &openapi);
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
