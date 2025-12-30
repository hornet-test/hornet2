pub mod builder;
pub mod exporter;
pub mod validator;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A flow graph representing an Arazzo workflow
#[derive(Debug, Clone)]
pub struct FlowGraph {
    /// The underlying directed graph
    pub graph: DiGraph<FlowNode, FlowEdge>,

    /// The workflow ID this graph represents
    pub workflow_id: String,

    /// Mapping from step_id to NodeIndex for quick lookup
    pub step_index_map: HashMap<String, NodeIndex>,
}

/// A node in the flow graph representing a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    /// The step ID
    pub step_id: String,

    /// The operation ID (if referencing OpenAPI operation)
    pub operation_id: Option<String>,

    /// The operation path (if referencing by path)
    pub operation_path: Option<String>,

    /// HTTP method (GET, POST, etc.)
    pub method: Option<String>,

    /// Description of the step
    pub description: Option<String>,

    /// Whether this step has outputs
    pub has_outputs: bool,

    /// Whether this step has success criteria
    pub has_success_criteria: bool,
}

/// An edge in the flow graph representing a relationship between steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowEdge {
    /// The type of edge
    pub edge_type: EdgeType,

    /// Data reference (e.g., "$steps.login.outputs.token")
    pub data_ref: Option<String>,

    /// Description of the relationship
    pub description: Option<String>,
}

/// Type of edge in the flow graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// Sequential execution (step1 -> step2)
    Sequential,

    /// Conditional execution based on success criteria
    Conditional,

    /// Data dependency (step2 uses output from step1)
    DataDependency,

    /// Success path (onSuccess: goto)
    OnSuccess,

    /// Failure path (onFailure: goto)
    OnFailure,
}

impl FlowGraph {
    /// Create a new empty flow graph
    pub fn new(workflow_id: String) -> Self {
        Self {
            graph: DiGraph::new(),
            workflow_id,
            step_index_map: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: FlowNode) -> NodeIndex {
        let step_id = node.step_id.clone();
        let index = self.graph.add_node(node);
        self.step_index_map.insert(step_id, index);
        index
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: FlowEdge) {
        self.graph.add_edge(from, to, edge);
    }

    /// Get a node index by step ID
    pub fn get_node_index(&self, step_id: &str) -> Option<NodeIndex> {
        self.step_index_map.get(step_id).copied()
    }

    /// Get a node by step ID
    pub fn get_node(&self, step_id: &str) -> Option<&FlowNode> {
        self.get_node_index(step_id)
            .and_then(|idx| self.graph.node_weight(idx))
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

impl FlowNode {
    /// Create a new flow node from a step
    pub fn from_step(step: &crate::models::arazzo::Step) -> Self {
        Self {
            step_id: step.step_id.clone(),
            operation_id: step.operation_id.clone(),
            operation_path: step.operation_path.clone(),
            method: None, // Will be resolved from OpenAPI
            description: step.description.clone(),
            has_outputs: step.outputs.is_some(),
            has_success_criteria: step.success_criteria.is_some(),
        }
    }
}

impl FlowEdge {
    /// Create a sequential edge
    pub fn sequential() -> Self {
        Self {
            edge_type: EdgeType::Sequential,
            data_ref: None,
            description: Some("Sequential execution".to_string()),
        }
    }

    /// Create a conditional edge
    pub fn conditional(description: String) -> Self {
        Self {
            edge_type: EdgeType::Conditional,
            data_ref: None,
            description: Some(description),
        }
    }

    /// Create a data dependency edge
    pub fn data_dependency(data_ref: String) -> Self {
        let description = format!("Data dependency: {}", data_ref);
        Self {
            edge_type: EdgeType::DataDependency,
            data_ref: Some(data_ref),
            description: Some(description),
        }
    }

    /// Create a success path edge
    pub fn on_success(description: String) -> Self {
        Self {
            edge_type: EdgeType::OnSuccess,
            data_ref: None,
            description: Some(description),
        }
    }

    /// Create a failure path edge
    pub fn on_failure(description: String) -> Self {
        Self {
            edge_type: EdgeType::OnFailure,
            data_ref: None,
            description: Some(description),
        }
    }
}
