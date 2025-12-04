use super::FlowGraph;
use crate::error::{HornetError, Result};
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::visit::EdgeRef;

/// Validator for flow graphs
pub struct FlowGraphValidator<'a> {
    graph: &'a FlowGraph,
}

impl<'a> FlowGraphValidator<'a> {
    /// Create a new validator
    pub fn new(graph: &'a FlowGraph) -> Self {
        Self { graph }
    }

    /// Validate the flow graph
    pub fn validate(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        // Check for cycles (excluding self-loops for conditional edges)
        if self.has_cycles()? {
            result.is_valid = false;
            result
                .errors
                .push("Graph contains cycles (not a DAG)".to_string());
        }

        // Check for unreachable nodes
        let unreachable = self.find_unreachable_nodes();
        if !unreachable.is_empty() {
            result.warnings.push(format!(
                "Found {} unreachable nodes: {:?}",
                unreachable.len(),
                unreachable
            ));
        }

        // Check for nodes with no outgoing edges (except the last step)
        let dead_ends = self.find_dead_end_nodes();
        if dead_ends.len() > 1 {
            result.warnings.push(format!(
                "Found {} nodes with no outgoing edges: {:?}",
                dead_ends.len(),
                dead_ends
            ));
        }

        // Try topological sort
        match self.topological_order() {
            Ok(order) => {
                result
                    .warnings
                    .push(format!("Topological order: {}", order.join(" → ")));
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to compute topological order: {}", e));
                result.is_valid = false;
            }
        }

        Ok(result)
    }

    /// Check if the graph has cycles (excluding self-loops)
    fn has_cycles(&self) -> Result<bool> {
        use petgraph::graph::EdgeIndex;

        // Create a copy without self-loops
        let mut graph_copy = self.graph.graph.clone();

        // Collect self-loops first, then remove them
        let self_loops: Vec<EdgeIndex> = graph_copy
            .edge_indices()
            .filter(|&e| {
                if let Some((source, target)) = graph_copy.edge_endpoints(e) {
                    source == target
                } else {
                    false
                }
            })
            .collect();

        // Remove in reverse order to maintain indices
        for edge in self_loops.into_iter().rev() {
            graph_copy.remove_edge(edge);
        }

        Ok(is_cyclic_directed(&graph_copy))
    }

    /// Find unreachable nodes (nodes with no incoming edges, except the first)
    fn find_unreachable_nodes(&self) -> Vec<String> {
        let mut unreachable = vec![];
        let mut has_incoming = std::collections::HashSet::new();

        // Find all nodes with incoming edges
        for edge in self.graph.graph.edge_references() {
            let target = edge.target();
            has_incoming.insert(target);
        }

        // Find nodes without incoming edges (except if they're the first node)
        let mut node_indices: Vec<_> = self.graph.graph.node_indices().collect();
        if node_indices.len() > 1 {
            node_indices.remove(0); // Remove first node from check

            for node_idx in node_indices {
                if !has_incoming.contains(&node_idx) {
                    if let Some(node) = self.graph.graph.node_weight(node_idx) {
                        unreachable.push(node.step_id.clone());
                    }
                }
            }
        }

        unreachable
    }

    /// Find nodes with no outgoing edges
    fn find_dead_end_nodes(&self) -> Vec<String> {
        let mut dead_ends = vec![];

        for node_idx in self.graph.graph.node_indices() {
            let outgoing = self.graph.graph.edges(node_idx).count();
            if outgoing == 0 {
                if let Some(node) = self.graph.graph.node_weight(node_idx) {
                    dead_ends.push(node.step_id.clone());
                }
            }
        }

        dead_ends
    }

    /// Compute topological order
    fn topological_order(&self) -> Result<Vec<String>> {
        use petgraph::graph::EdgeIndex;

        // Create a copy without self-loops
        let mut graph_copy = self.graph.graph.clone();

        let self_loops: Vec<EdgeIndex> = graph_copy
            .edge_indices()
            .filter(|&e| {
                if let Some((source, target)) = graph_copy.edge_endpoints(e) {
                    source == target
                } else {
                    false
                }
            })
            .collect();

        // Remove in reverse order
        for edge in self_loops.into_iter().rev() {
            graph_copy.remove_edge(edge);
        }

        match toposort(&graph_copy, None) {
            Ok(order) => {
                let step_ids: Vec<String> = order
                    .into_iter()
                    .filter_map(|idx| graph_copy.node_weight(idx).map(|node| node.step_id.clone()))
                    .collect();
                Ok(step_ids)
            }
            Err(_) => Err(HornetError::GraphError(
                "Graph contains cycles, cannot compute topological order".to_string(),
            )),
        }
    }
}

/// Result of graph validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the graph is valid
    pub is_valid: bool,

    /// List of errors
    pub errors: Vec<String>,

    /// List of warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Check if the validation passed
    pub fn is_ok(&self) -> bool {
        self.is_valid
    }

    /// Get a summary message
    pub fn summary(&self) -> String {
        if self.is_valid {
            if self.warnings.is_empty() {
                "Graph is valid with no warnings".to_string()
            } else {
                format!("Graph is valid with {} warnings", self.warnings.len())
            }
        } else {
            format!("Graph is invalid with {} errors", self.errors.len())
        }
    }
}

/// Validate a flow graph
pub fn validate_flow_graph(graph: &FlowGraph) -> Result<ValidationResult> {
    FlowGraphValidator::new(graph).validate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{FlowEdge, FlowNode};

    #[test]
    fn test_valid_linear_graph() {
        let mut graph = FlowGraph::new("test".to_string());

        let n1 = graph.add_node(FlowNode {
            step_id: "step1".to_string(),
            operation_id: None,
            operation_path: None,
            method: None,
            description: None,
            has_outputs: false,
            has_success_criteria: false,
        });

        let n2 = graph.add_node(FlowNode {
            step_id: "step2".to_string(),
            operation_id: None,
            operation_path: None,
            method: None,
            description: None,
            has_outputs: false,
            has_success_criteria: false,
        });

        graph.add_edge(n1, n2, FlowEdge::sequential());

        let result = validate_flow_graph(&graph).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_self_loop_allowed() {
        let mut graph = FlowGraph::new("test".to_string());

        let n1 = graph.add_node(FlowNode {
            step_id: "step1".to_string(),
            operation_id: None,
            operation_path: None,
            method: None,
            description: None,
            has_outputs: false,
            has_success_criteria: true,
        });

        // Self-loop for conditional edge
        graph.add_edge(n1, n1, FlowEdge::conditional("condition".to_string()));

        let result = validate_flow_graph(&graph).unwrap();
        assert!(result.is_ok());
    }
}
