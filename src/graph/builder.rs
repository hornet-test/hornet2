use super::{FlowEdge, FlowGraph, FlowNode};
use crate::error::Result;
use crate::models::OpenApiV3Spec;
use crate::models::arazzo::{Step, Workflow};
use regex::Regex;
use std::collections::HashSet;

/// Builder for constructing flow graphs from Arazzo workflows
pub struct FlowGraphBuilder<'a> {
    workflow: &'a Workflow,
    openapi: Option<&'a OpenApiV3Spec>,
}

impl<'a> FlowGraphBuilder<'a> {
    /// Create a new builder
    pub fn new(workflow: &'a Workflow) -> Self {
        Self {
            workflow,
            openapi: None,
        }
    }

    /// Set the OpenAPI specification (optional, for resolving operation details)
    pub fn with_openapi(mut self, openapi: &'a OpenApiV3Spec) -> Self {
        self.openapi = Some(openapi);
        self
    }

    /// Build the flow graph
    pub fn build(self) -> Result<FlowGraph> {
        let mut graph = FlowGraph::new(self.workflow.workflow_id.clone());

        // Step 1: Add all nodes
        let node_indices: Vec<_> = self
            .workflow
            .steps
            .iter()
            .map(|step| {
                let mut node = FlowNode::from_step(step);

                // Resolve HTTP method from OpenAPI if available
                if let Some(openapi) = self.openapi
                    && let Some(operation_id) = &step.operation_id
                {
                    node.method = self.resolve_method_by_operation_id(openapi, operation_id);
                }

                graph.add_node(node)
            })
            .collect();

        // Step 2: Add sequential edges (step order)
        // Only add sequential edge if there are no explicit "goto" actions that override the flow
        for i in 0..node_indices.len().saturating_sub(1) {
            let current_step = &self.workflow.steps[i];
            let mut has_goto = false;

            // Check onSuccess for goto
            if let Some(ref actions) = current_step.on_success
                && actions.iter().any(|a| a.action_type == "goto")
            {
                has_goto = true;
            }

            // Check onFailure for goto
            if let Some(ref actions) = current_step.on_failure
                && actions.iter().any(|a| a.action_type == "goto")
            {
                has_goto = true;
            }

            if !has_goto {
                graph.add_edge(node_indices[i], node_indices[i + 1], FlowEdge::sequential());
            }
        }

        // Step 3: Analyze data dependencies
        // Note: We don't add separate edges for data dependencies since
        // the sequential edges already represent the execution order.
        // We just annotate the existing edges with data dependency information.
        for step in self.workflow.steps.iter() {
            let dependencies = self.extract_data_dependencies(step);

            if !dependencies.is_empty() {
                // Mark the node as having data dependencies (for visualization)
                // The sequential edges already ensure correct execution order
            }
        }

        // Step 4: Add conditional edges based on success criteria
        for (i, step) in self.workflow.steps.iter().enumerate() {
            if let Some(ref criteria) = step.success_criteria
                && !criteria.is_empty()
            {
                let description = format!("Success criteria: {} conditions", criteria.len());
                // Add self-loop to indicate conditional execution
                graph.add_edge(
                    node_indices[i],
                    node_indices[i],
                    FlowEdge::conditional(description),
                );
            }
        }

        // Step 5: Add branching edges based on onSuccess/onFailure
        for (i, step) in self.workflow.steps.iter().enumerate() {
            let source_idx = node_indices[i];

            // Handle onSuccess
            if let Some(ref actions) = step.on_success {
                for action in actions {
                    if action.action_type == "goto"
                        && let Some(target_id) =
                            action.config.get("stepId").and_then(|v| v.as_str())
                        && let Some(target_idx) = graph.get_node_index(target_id)
                    {
                        graph.add_edge(
                            source_idx,
                            target_idx,
                            FlowEdge::on_success(action.name.clone()),
                        );
                    }
                }
            }

            // Handle onFailure
            if let Some(ref actions) = step.on_failure {
                for action in actions {
                    if action.action_type == "goto"
                        && let Some(target_id) =
                            action.config.get("stepId").and_then(|v| v.as_str())
                        && let Some(target_idx) = graph.get_node_index(target_id)
                    {
                        graph.add_edge(
                            source_idx,
                            target_idx,
                            FlowEdge::on_failure(action.name.clone()),
                        );
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Extract data dependencies from a step
    /// Returns a set of step IDs that this step depends on
    fn extract_data_dependencies(&self, step: &Step) -> HashSet<String> {
        let mut dependencies = HashSet::new();

        // Regex to match $steps.{step_id}.outputs references
        let re = Regex::new(r"\$steps\.([a-zA-Z0-9_-]+)\.outputs").unwrap();

        // Check parameters
        for param in &step.parameters {
            if let Some(value_str) = param.value.as_str() {
                for cap in re.captures_iter(value_str) {
                    if let Some(step_id) = cap.get(1) {
                        dependencies.insert(step_id.as_str().to_string());
                    }
                }
            }
        }

        // Check request body
        if let Some(ref body) = step.request_body {
            let body_str = serde_json::to_string(&body.payload).unwrap_or_default();
            for cap in re.captures_iter(&body_str) {
                if let Some(step_id) = cap.get(1) {
                    dependencies.insert(step_id.as_str().to_string());
                }
            }
        }

        dependencies
    }

    /// Resolve HTTP method from OpenAPI by operation ID
    fn resolve_method_by_operation_id(
        &self,
        openapi: &OpenApiV3Spec,
        operation_id: &str,
    ) -> Option<String> {
        if let Some(ref paths) = openapi.paths {
            for (_path, path_item) in paths.iter() {
                // Check all HTTP methods
                if let Some(ref op) = path_item.get
                    && op.operation_id.as_deref() == Some(operation_id)
                {
                    return Some("GET".to_string());
                }
                if let Some(ref op) = path_item.post
                    && op.operation_id.as_deref() == Some(operation_id)
                {
                    return Some("POST".to_string());
                }
                if let Some(ref op) = path_item.put
                    && op.operation_id.as_deref() == Some(operation_id)
                {
                    return Some("PUT".to_string());
                }
                if let Some(ref op) = path_item.delete
                    && op.operation_id.as_deref() == Some(operation_id)
                {
                    return Some("DELETE".to_string());
                }
                if let Some(ref op) = path_item.patch
                    && op.operation_id.as_deref() == Some(operation_id)
                {
                    return Some("PATCH".to_string());
                }
            }
        }
        None
    }
}

/// Build a flow graph from a workflow
pub fn build_flow_graph(workflow: &Workflow, openapi: Option<&OpenApiV3Spec>) -> Result<FlowGraph> {
    let mut builder = FlowGraphBuilder::new(workflow);
    if let Some(spec) = openapi {
        builder = builder.with_openapi(spec);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::arazzo::Parameter;

    #[test]
    fn test_simple_workflow() {
        let workflow = Workflow {
            workflow_id: "test".to_string(),
            summary: None,
            description: None,
            inputs: None,
            steps: vec![
                Step {
                    step_id: "step1".to_string(),
                    description: None,
                    operation_id: Some("op1".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: None,
                },
                Step {
                    step_id: "step2".to_string(),
                    description: None,
                    operation_id: Some("op2".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: None,
                },
            ],
            success_criteria: None,
            outputs: None,
            extensions: Default::default(),
        };

        let graph = build_flow_graph(&workflow, None).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1); // Sequential edge
    }

    #[test]
    fn test_data_dependency() {
        let workflow = Workflow {
            workflow_id: "test".to_string(),
            summary: None,
            description: None,
            inputs: None,
            steps: vec![
                Step {
                    step_id: "login".to_string(),
                    description: None,
                    operation_id: Some("loginUser".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: Some(serde_json::json!({"token": "$response.body.token"})),
                },
                Step {
                    step_id: "getProfile".to_string(),
                    description: None,
                    operation_id: Some("getProfile".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![Parameter {
                        name: "Authorization".to_string(),
                        location: "header".to_string(),
                        value: serde_json::json!("Bearer $steps.login.outputs.token"),
                    }],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: None,
                },
            ],
            success_criteria: None,
            outputs: None,
            extensions: Default::default(),
        };

        let graph = build_flow_graph(&workflow, None).unwrap();

        assert_eq!(graph.node_count(), 2);
        // Only sequential edge (data dependencies are implicit in execution order)
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_branching_workflow() {
        use crate::graph::EdgeType;
        use crate::models::arazzo::{FailureAction, SuccessAction};
        use std::collections::HashMap;

        let mut success_config = HashMap::new();
        success_config.insert("stepId".to_string(), serde_json::json!("step3"));

        let mut failure_config = HashMap::new();
        failure_config.insert("stepId".to_string(), serde_json::json!("step2"));

        let workflow = Workflow {
            workflow_id: "test".to_string(),
            summary: None,
            description: None,
            inputs: None,
            steps: vec![
                Step {
                    step_id: "step1".to_string(),
                    description: None,
                    operation_id: Some("op1".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: Some(vec![SuccessAction {
                        name: "goto-step3".to_string(),
                        action_type: "goto".to_string(),
                        config: success_config,
                    }]),
                    on_failure: Some(vec![FailureAction {
                        name: "goto-step2".to_string(),
                        action_type: "goto".to_string(),
                        config: failure_config,
                    }]),
                    outputs: None,
                },
                Step {
                    step_id: "step2".to_string(),
                    description: None,
                    operation_id: Some("op2".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: None,
                },
                Step {
                    step_id: "step3".to_string(),
                    description: None,
                    operation_id: Some("op3".to_string()),
                    operation_path: None,
                    workflow_id: None,
                    parameters: vec![],
                    request_body: None,
                    success_criteria: None,
                    on_success: None,
                    on_failure: None,
                    outputs: None,
                },
            ],
            success_criteria: None,
            outputs: None,
            extensions: Default::default(),
        };

        let graph = build_flow_graph(&workflow, None).unwrap();

        assert_eq!(graph.node_count(), 3);
        // Edges:
        // step2->step3 (sequential)
        // step1->step3 (OnSuccess)
        // step1->step2 (OnFailure)
        // Note: step1->step2 sequential is NOT added because step1 has explicit branching
        assert_eq!(graph.edge_count(), 3);

        // Find OnSuccess edge
        let n1_idx = graph.get_node_index("step1").unwrap();
        let n3_idx = graph.get_node_index("step3").unwrap();
        let edge = graph.graph.find_edge(n1_idx, n3_idx).unwrap();
        assert!(matches!(graph.graph[edge].edge_type, EdgeType::OnSuccess));
    }
}
