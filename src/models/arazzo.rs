use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Arazzo Specification root object
/// https://spec.openapis.org/arazzo/latest.html
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArazzoSpec {
    /// The version of the Arazzo Specification (e.g., "1.0.0")
    pub arazzo: String,

    /// Metadata about the Arazzo document
    pub info: Info,

    /// A list of source descriptions (references to OpenAPI documents)
    #[serde(default, rename = "sourceDescriptions")]
    pub source_descriptions: Vec<SourceDescription>,

    /// A list of workflows
    pub workflows: Vec<Workflow>,

    /// Additional components that can be referenced
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    /// The title of the Arazzo document
    pub title: String,

    /// A short summary of the Arazzo document
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// A description of the Arazzo document
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The version of the Arazzo document
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDescription {
    /// A unique name for the source description
    pub name: String,

    /// A URL or relative path to the OpenAPI document
    pub url: String,

    /// The type of the source description (defaults to "openapi")
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// A unique identifier for the workflow
    #[serde(rename = "workflowId")]
    pub workflow_id: String,

    /// A short summary of the workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// A detailed description of the workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input parameters for the workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,

    /// A list of steps in the workflow
    pub steps: Vec<Step>,

    /// Success criteria for the workflow
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "successCriteria")]
    pub success_criteria: Option<Vec<SuccessCriteria>>,

    /// Output values from the workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,

    /// Additional data for extensions (x-* fields)
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// A unique identifier for the step
    #[serde(rename = "stepId")]
    pub step_id: String,

    /// A description of the step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Reference to an operation by operationId
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "operationId")]
    pub operation_id: Option<String>,

    /// Reference to an operation by path and method
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "operationPath")]
    pub operation_path: Option<String>,

    /// Reference to a workflow to execute
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "workflowId")]
    pub workflow_id: Option<String>,

    /// Parameters for the operation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,

    /// Request body for the operation
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<RequestBody>,

    /// Success criteria for the step
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "successCriteria")]
    pub success_criteria: Option<Vec<SuccessCriteria>>,

    /// Actions to take on success
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "onSuccess")]
    pub on_success: Option<Vec<SuccessAction>>,

    /// Actions to take on failure
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "onFailure")]
    pub on_failure: Option<Vec<FailureAction>>,

    /// Output values from the step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// The name of the parameter
    pub name: String,

    /// The location of the parameter (query, header, path, cookie)
    #[serde(rename = "in")]
    pub location: String,

    /// The value of the parameter (can be a runtime expression)
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// The content type of the request body
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "contentType")]
    pub content_type: Option<String>,

    /// The payload (can be a runtime expression)
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    /// The context to evaluate (e.g., "$statusCode")
    pub context: String,

    /// The condition to check (e.g., "$eq")
    pub condition: String,

    /// The expected value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,

    /// The type of the criteria
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub criteria_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessAction {
    /// The name of the action
    pub name: String,

    /// The type of the action
    #[serde(rename = "type")]
    pub action_type: String,

    /// Additional criteria or configuration
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAction {
    /// The name of the action
    pub name: String,

    /// The type of the action
    #[serde(rename = "type")]
    pub action_type: String,

    /// Additional criteria or configuration
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    /// Reusable parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, Parameter>>,

    /// Reusable success criteria
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "successActions")]
    pub success_actions: Option<HashMap<String, SuccessAction>>,

    /// Reusable failure actions
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "failureActions")]
    pub failure_actions: Option<HashMap<String, FailureAction>>,

    /// Additional components
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ArazzoSpec {
    /// Validate the Arazzo specification
    pub fn validate(&self) -> Result<(), crate::error::HornetError> {
        // Check version
        if !self.arazzo.starts_with("1.") {
            return Err(crate::error::HornetError::ValidationError(
                format!("Unsupported Arazzo version: {}", self.arazzo),
            ));
        }

        // Check that all workflows have unique IDs
        let mut workflow_ids = std::collections::HashSet::new();
        for workflow in &self.workflows {
            if !workflow_ids.insert(&workflow.workflow_id) {
                return Err(crate::error::HornetError::ValidationError(
                    format!("Duplicate workflow ID: {}", workflow.workflow_id),
                ));
            }
        }

        // Validate each workflow
        for workflow in &self.workflows {
            workflow.validate()?;
        }

        Ok(())
    }
}

impl Workflow {
    /// Validate the workflow
    pub fn validate(&self) -> Result<(), crate::error::HornetError> {
        // Check that all steps have unique IDs
        let mut step_ids = std::collections::HashSet::new();
        for step in &self.steps {
            if !step_ids.insert(&step.step_id) {
                return Err(crate::error::HornetError::ValidationError(
                    format!("Duplicate step ID: {} in workflow {}", step.step_id, self.workflow_id),
                ));
            }
        }

        // Validate each step
        for step in &self.steps {
            step.validate()?;
        }

        Ok(())
    }
}

impl Step {
    /// Validate the step
    pub fn validate(&self) -> Result<(), crate::error::HornetError> {
        // Check that at least one of operationId, operationPath, or workflowId is set
        if self.operation_id.is_none() && self.operation_path.is_none() && self.workflow_id.is_none() {
            return Err(crate::error::HornetError::ValidationError(
                format!("Step {} must have either operationId, operationPath, or workflowId", self.step_id),
            ));
        }

        Ok(())
    }
}
